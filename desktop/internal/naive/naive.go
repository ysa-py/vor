// Package naive implements a NaiveProxy-style HTTPS tunnel client: a local
// SOCKS5 front whose traffic is carried through an HTTPS CONNECT proxy as
// padded HTTP requests, wrapped in a TLS session using a Chrome-parity
// User-Agent. The technique (HTTPS CONNECT + padding to defeat DPI
// length/flow analysis) is generic and publicly documented; no third-party
// application code is ported.
//
// Interop note (documented in ENGINES.md): authentic Chromium TLS
// fingerprinting byte-for-byte requires the uTLS library, which is flagged
// as an enhancement; this engine approximates the fingerprint via a
// standard TLS 1.3 handshake plus Chrome headers. Caddy forward_proxy
// servers accept this client's padded CONNECT.
package naive

import (
	"bufio"
	"crypto/tls"
	"encoding/binary"
	"errors"
	"fmt"
	"io"
	"math/rand"
	"net"
	"strconv"
	"strings"
	"sync"
	"time"
)

// Config configures the naive HTTPS tunnel.
type Config struct {
	// ProxyAddr is the HTTPS proxy "host:port" (the CDN / forward_proxy
	// endpoint).
	ProxyAddr string `json:"proxy_addr"`
	// SNI override for the TLS layer (defaults to the proxy host).
	SNI string `json:"sni"`
	// Insecure skips certificate validation (fronting domains).
	Insecure bool `json:"insecure"`

	// PaddingBytes is the maximum random padding added to each request
	// (0 disables padding). NaiveProxy-style padding defeats simple
	// length-fingerprinting.
	PaddingBytes int `json:"padding_bytes"`

	// ListenAddr is the local SOCKS5 listener.
	ListenAddr string `json:"listen_addr"`

	// HandshakeTimeout bounds TLS + CONNECT.
	HandshakeTimeout time.Duration `json:"handshake_timeout"`
}

// Defaults returns a sane config.
func Defaults() Config {
	return Config{PaddingBytes: 128, HandshakeTimeout: 15 * time.Second}
}

// Validate checks the config.
func (c Config) Validate() error {
	if _, _, err := net.SplitHostPort(c.ProxyAddr); err != nil {
		return fmt.Errorf("naive: bad proxy addr %q: %w", c.ProxyAddr, err)
	}
	if c.PaddingBytes < 0 || c.PaddingBytes > 65535 {
		return errors.New("naive: padding_bytes out of range")
	}
	if c.ListenAddr == "" {
		return errors.New("naive: listen_addr is required")
	}
	return nil
}

// Engine is a running naive HTTPS tunnel with a local SOCKS5 front.
type Engine struct {
	cfg      Config
	log      LogFunc
	mu       sync.Mutex
	ln       net.Listener
	stop     chan struct{}
	done     sync.WaitGroup
	sessions sync.WaitGroup
	state    string
}

// LogFunc receives status lines.
type LogFunc func(msg, level string)

// New creates an Engine.
func New(cfg Config, log LogFunc) *Engine {
	if log == nil {
		log = func(string, string) {}
	}
	return &Engine{cfg: cfg, log: log, state: "stopped", stop: make(chan struct{})}
}

// State reports the engine state.
func (e *Engine) State() string {
	e.mu.Lock()
	defer e.mu.Unlock()
	return e.state
}

// Start opens the local SOCKS5 listener (upstreams are dialed per session).
func (e *Engine) Start() error {
	if err := e.cfg.Validate(); err != nil {
		return err
	}
	e.mu.Lock()
	if e.state != "stopped" {
		e.mu.Unlock()
		return errors.New("naive: already started")
	}
	e.mu.Unlock()
	ln, err := net.Listen("tcp", e.cfg.ListenAddr)
	if err != nil {
		return fmt.Errorf("naive: listen: %w", err)
	}
	e.mu.Lock()
	e.ln, e.state = ln, "running"
	e.mu.Unlock()
	e.log(fmt.Sprintf("naive: running socks=%s -> https=%s", e.cfg.ListenAddr, e.cfg.ProxyAddr), "info")
	e.done.Add(1)
	go e.acceptLoop()
	return nil
}

// Stop closes the listener and waits for sessions.
func (e *Engine) Stop() {
	e.mu.Lock()
	if e.state == "stopped" {
		e.mu.Unlock()
		return
	}
	close(e.stop)
	if e.ln != nil {
		e.ln.Close()
	}
	e.state = "stopped"
	e.mu.Unlock()
	e.done.Wait()
	e.sessions.Wait()
}

func (e *Engine) acceptLoop() {
	defer e.done.Done()
	for {
		conn, err := e.ln.Accept()
		if err != nil {
			select {
			case <-e.stop:
				return
			default:
			}
			e.log("naive: accept: "+err.Error(), "error")
			return
		}
		e.sessions.Add(1)
		go func(conn net.Conn) {
			defer e.sessions.Done()
			defer conn.Close()
			if err := e.socksServe(conn); err != nil {
				e.log("naive: session: "+err.Error(), "debug")
			}
		}(conn)
	}
}

// dialUpstream opens a padded HTTPS CONNECT tunnel to the target.
func (e *Engine) dialUpstream(target string) (net.Conn, error) {
	timeout := e.cfg.HandshakeTimeout
	if timeout == 0 {
		timeout = 15 * time.Second
	}
	raw, err := net.DialTimeout("tcp", e.cfg.ProxyAddr, timeout)
	if err != nil {
		return nil, fmt.Errorf("naive: dial: %w", err)
	}
	host, _, splitErr := net.SplitHostPort(e.cfg.ProxyAddr)
	if splitErr != nil {
		host = e.cfg.ProxyAddr
	}
	sni := e.cfg.SNI
	if sni == "" {
		sni = host
	}
	tlsConn := tls.Client(raw, &tls.Config{
		ServerName:         sni,
		InsecureSkipVerify: e.cfg.Insecure,
		MinVersion:         tls.VersionTLS12,
	})
	if err := tlsConn.Handshake(); err != nil {
		raw.Close()
		return nil, fmt.Errorf("naive: tls: %w", err)
	}

	// Padded CONNECT (the NaiveProxy-style length randomization).
	request := "CONNECT " + target + " HTTP/1.1\r\n" +
		"Host: " + target + "\r\n" +
		"User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36\r\n"
	if e.cfg.PaddingBytes > 0 {
		size := 16 + rand.Intn(e.cfg.PaddingBytes)
		buffer := make([]byte, size)
		for i := range buffer {
			buffer[i] = byte('a' + rand.Intn(26))
		}
		request += "X-Padding: " + string(buffer) + "\r\n"
	}
	request += "\r\n"
	if _, err := tlsConn.Write([]byte(request)); err != nil {
		tlsConn.Close()
		return nil, fmt.Errorf("naive: connect write: %w", err)
	}
	reader := bufio.NewReader(tlsConn)
	line, err := reader.ReadString('\n')
	if err != nil {
		tlsConn.Close()
		return nil, fmt.Errorf("naive: connect response: %w", err)
	}
	fields := strings.Fields(strings.TrimSpace(line))
	code := 0
	if len(fields) >= 2 {
		parsed, atoiErr := strconv.Atoi(fields[1])
		if atoiErr == nil {
			code = parsed
		}
	}
	if code < 200 || code >= 300 {
		tlsConn.Close()
		return nil, fmt.Errorf("naive: connect status %q", line)
	}
	// Drain remaining response headers.
	for {
		header, err := reader.ReadString('\n')
		if err != nil {
			tlsConn.Close()
			return nil, err
		}
		if header == "\r\n" || header == "\n" {
			break
		}
	}
	if reader.Buffered() > 0 {
		buffered, _ := reader.Peek(reader.Buffered())
		return &prefixConn{Conn: tlsConn, prefix: buffered}, nil
	}
	return tlsConn, nil
}

// socksServe speaks RFC 1928 (no-auth CONNECT) and bridges through the
// HTTPS tunnel.
func (e *Engine) socksServe(conn net.Conn) error {
	head := make([]byte, 2)
	if _, err := io.ReadFull(conn, head); err != nil {
		return err
	}
	if head[0] != 0x05 {
		return errors.New("bad socks version")
	}
	methods := make([]byte, int(head[1]))
	if _, err := io.ReadFull(conn, methods); err != nil {
		return err
	}
	if _, err := conn.Write([]byte{0x05, 0x00}); err != nil {
		return err
	}
	req := make([]byte, 4)
	if _, err := io.ReadFull(conn, req); err != nil {
		return err
	}
	if req[0] != 0x05 || req[1] != 0x01 {
		conn.Write([]byte{0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0})
		return errors.New("only CONNECT supported")
	}
	var host string
	switch req[3] {
	case 0x01:
		buf := make([]byte, 4)
		if _, err := io.ReadFull(conn, buf); err != nil {
			return err
		}
		host = net.IP(buf).String()
	case 0x04:
		buf := make([]byte, 16)
		if _, err := io.ReadFull(conn, buf); err != nil {
			return err
		}
		host = net.IP(buf).String()
	case 0x03:
		length := make([]byte, 1)
		if _, err := io.ReadFull(conn, length); err != nil {
			return err
		}
		buf := make([]byte, int(length[0]))
		if _, err := io.ReadFull(conn, buf); err != nil {
			return err
		}
		host = string(buf)
	default:
		conn.Write([]byte{0x05, 0x08, 0x00, 0x01, 0, 0, 0, 0, 0, 0})
		return errors.New("bad address type")
	}
	portBuf := make([]byte, 2)
	if _, err := io.ReadFull(conn, portBuf); err != nil {
		return err
	}
	port := binary.BigEndian.Uint16(portBuf)

	target := net.JoinHostPort(host, strconv.Itoa(int(port)))
	upstream, err := e.dialUpstream(target)
	if err != nil {
		conn.Write([]byte{0x05, 0x04, 0x00, 0x01, 0, 0, 0, 0, 0, 0})
		return err
	}
	defer upstream.Close()
	if _, err := conn.Write([]byte{0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0}); err != nil {
		return err
	}
	done := make(chan error, 2)
	go func() { _, err := io.Copy(upstream, conn); done <- err }()
	go func() { _, err := io.Copy(conn, upstream); done <- err }()
	<-done
	return nil
}

// prefixConn replays read-ahead bytes.
type prefixConn struct {
	net.Conn
	prefix []byte
}

func (c *prefixConn) Read(p []byte) (int, error) {
	if len(c.prefix) > 0 {
		n := copy(p, c.prefix)
		c.prefix = c.prefix[n:]
		return n, nil
	}
	return c.Conn.Read(p)
}
