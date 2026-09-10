// Package sshchain implements SSH-based tunnel transports for Vor.
//
// Modes (each independently selectable, mirroring the tunnel-chaining ideas
// documented in public anti-censorship literature — implemented from scratch
// against the SSH/TLS/WebSocket/HTTP specs, no third-party app code ported):
//
//   - standalone SSH: dynamic SOCKS5 forwarding over direct-tcpip channels
//   - SSH-over-TLS: the SSH stream is wrapped in TLS with a custom SNI, so
//     the connection looks like ordinary HTTPS to a middlebox and the SSH
//     banner is never plaintext on the wire
//   - SSH-over-WebSocket: the SSH stream is carried inside RFC 6455 binary
//     frames (ws:// or wss://), which CDN proxies (Cloudflare etc.) accept
//   - SSH-over-HTTP-CONNECT: the SSH stream is routed through an HTTP
//     CONNECT proxy with a custom Host header
//   - pre-handshake payload injection: arbitrary bytes are sent before the
//     SSH banner so the first bytes on the wire are not "SSH-2.0" — a cheap
//     DPI signature break (the server must tolerate and discard them)
//
// Cipher selection is supported (aes128-gcm@openssh.com,
// chacha20-poly1305@openssh.com, aes128-ctr) to shape the SSH handshake
// fingerprint.
//
// The public surface is deliberately small: configure via Config, run via
// Start (a local SOCKS5 listener), and everything below is a plain SSH
// client connection — the same protocol tunneled through composable
// transport wraps.
package sshchain

import (
	"context"
	"crypto/tls"
	"encoding/binary"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"net"
	"strconv"
	"sync"
	"time"

	"golang.org/x/crypto/ssh"
)

// WrapMode selects how the SSH stream is carried.
type WrapMode string

const (
	WrapNone WrapMode = "none"         // plain TCP
	WrapTLS  WrapMode = "tls"          // SSH-over-TLS, custom SNI
	WrapWS   WrapMode = "ws"           // SSH-over-WebSocket (plaintext)
	WrapWSS  WrapMode = "wss"          // SSH-over-WebSocket inside TLS
	WrapHTTP WrapMode = "http-connect" // SSH-over-HTTP-CONNECT
)

// AuthMethod selects authentication.
type AuthMethod string

const (
	AuthPassword AuthMethod = "password"
	AuthKey      AuthMethod = "key"
)

// Config configures one SSH tunnel chain.
type Config struct {
	// Addr is the SSH server "host:port". With TLS/WS wraps the dialed
	// host is the wrap endpoint; Addr stays the SSH identity.
	Addr string `json:"addr"`
	User string `json:"user"`

	Auth   AuthMethod `json:"auth"`
	Secret string     `json:"secret"` // password, or PEM private key data

	Wrap WrapMode `json:"wrap"`

	// TLS wrap options.
	TLSSNI      string `json:"tls_sni"` // custom SNI (domain fronting)
	TLSInsecure bool   `json:"tls_insecure"`

	// WebSocket wrap options.
	WSPath string `json:"ws_path"` // e.g. "/tunnel"
	WSHost string `json:"ws_host"` // Host header override

	// HTTP CONNECT wrap options.
	// ProxyAddr is the HTTP proxy for wrap=http-connect (Addr stays the SSH
	// endpoint); HTTPHost overrides the CONNECT Host header.
	ProxyAddr string `json:"proxy_addr"`
	HTTPHost  string `json:"http_host"`

	// InjectHex is optional hex bytes written before the SSH banner
	// (pre-handshake payload injection for DPI signature breaking).
	InjectHex string `json:"inject_hex"`

	// Ciphers optionally restrict the SSH cipher list (in preference
	// order). Empty = library default (all supported).
	Ciphers []string `json:"ciphers"`

	// ListenAddr is the local SOCKS5 listener "host:port".
	ListenAddr string `json:"listen_addr"`

	HandshakeTimeout time.Duration `json:"handshake_timeout"`
}

// Defaults returns a Config with sane non-zero defaults.
func Defaults() Config {
	return Config{
		User:             "vor",
		Auth:             AuthPassword,
		Wrap:             WrapNone,
		HandshakeTimeout: 15 * time.Second,
	}
}

// SupportedCiphers is the subset of SSH ciphers this engine exposes.
var SupportedCiphers = []string{
	"aes128-gcm@openssh.com",
	"chacha20-poly1305@openssh.com",
	"aes128-ctr",
}

// Validate checks the config for structural errors.
func (c Config) Validate() error {
	if _, _, err := net.SplitHostPort(c.Addr); err != nil {
		return fmt.Errorf("sshchain: bad addr %q: %w", c.Addr, err)
	}
	if c.User == "" {
		return errors.New("sshchain: user is required")
	}
	switch c.Auth {
	case AuthPassword:
		if c.Secret == "" {
			return errors.New("sshchain: password auth requires a secret")
		}
	case AuthKey:
		if c.Secret == "" {
			return errors.New("sshchain: key auth requires PEM data")
		}
	default:
		return fmt.Errorf("sshchain: unknown auth %q", c.Auth)
	}
	switch c.Wrap {
	case WrapNone, WrapTLS, WrapWS, WrapWSS, WrapHTTP:
	default:
		return fmt.Errorf("sshchain: unknown wrap %q", c.Wrap)
	}
	if c.Wrap == WrapTLS || c.Wrap == WrapWSS {
		if c.TLSSNI == "" {
			return fmt.Errorf("sshchain: wrap %q requires tls_sni", c.Wrap)
		}
	}
	if c.Wrap == WrapHTTP {
		if c.ProxyAddr == "" {
			return errors.New("sshchain: wrap http-connect requires proxy_addr")
		}
	}
	if c.InjectHex != "" {
		if _, err := hex.DecodeString(c.InjectHex); err != nil {
			return fmt.Errorf("sshchain: bad inject_hex: %w", err)
		}
	}
	for _, cipher := range c.Ciphers {
		if !contains(SupportedCiphers, cipher) {
			return fmt.Errorf("sshchain: unsupported cipher %q (supported: %v)", cipher, SupportedCiphers)
		}
	}
	if c.ListenAddr == "" {
		return errors.New("sshchain: listen_addr is required")
	}
	return nil
}

// Engine is a running SSH-chain tunnel with a local SOCKS5 front.
type Engine struct {
	cfg    Config
	log    LogFunc
	mu     sync.Mutex
	ln     net.Listener
	client *ssh.Client
	stop   chan struct{}
	done   sync.WaitGroup
	state  string // "stopped" | "starting" | "running"
}

// LogFunc receives human-readable status lines (level: info/warn/error).
type LogFunc func(msg, level string)

// New creates an Engine. log may be nil.
func New(cfg Config, log LogFunc) *Engine {
	if log == nil {
		log = func(string, string) {}
	}
	return &Engine{cfg: cfg, log: log, state: "stopped", stop: make(chan struct{})}
}

// State reports the engine lifecycle state.
func (e *Engine) State() string {
	e.mu.Lock()
	defer e.mu.Unlock()
	return e.state
}

// Start brings up the SSH chain and the local SOCKS5 listener. The SSH
// connection is established eagerly so start errors surface immediately.
func (e *Engine) Start() error {
	if err := e.cfg.Validate(); err != nil {
		return err
	}
	e.mu.Lock()
	if e.state != "stopped" {
		e.mu.Unlock()
		return errors.New("sshchain: already started")
	}
	e.state = "starting"
	e.mu.Unlock()

	client, err := e.dial()
	if err != nil {
		e.mu.Lock()
		e.state = "stopped"
		e.mu.Unlock()
		return err
	}
	ln, err := net.Listen("tcp", e.cfg.ListenAddr)
	if err != nil {
		client.Close()
		e.mu.Lock()
		e.state = "stopped"
		e.mu.Unlock()
		return fmt.Errorf("sshchain: listen %s: %w", e.cfg.ListenAddr, err)
	}
	e.mu.Lock()
	e.ln, e.client, e.state = ln, client, "running"
	e.mu.Unlock()
	e.log(fmt.Sprintf("sshchain: running socks=%s -> ssh=%s wrap=%s",
		e.cfg.ListenAddr, e.cfg.Addr, e.cfg.Wrap), "info")

	e.done.Add(1)
	go e.acceptLoop()
	return nil
}

// Stop tears the tunnel down and waits for sessions to drain.
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
	if e.client != nil {
		e.client.Close()
	}
	e.state = "stopped"
	e.mu.Unlock()
	e.done.Wait()
}

// dial establishes the wrapped, authenticated SSH client connection.
func (e *Engine) dial() (*ssh.Client, error) {
	timeout := e.cfg.HandshakeTimeout
	if timeout == 0 {
		timeout = 15 * time.Second
	}
	ctx, cancel := context.WithTimeout(context.Background(), timeout)
	defer cancel()

	conn, err := e.transport(ctx)
	if err != nil {
		return nil, err
	}

	// Pre-handshake payload injection (DPI signature break). The remote
	// side must tolerate these bytes before its SSH banner.
	if raw, decErr := hex.DecodeString(e.cfg.InjectHex); decErr == nil && len(raw) > 0 {
		if _, err := conn.Write(raw); err != nil {
			conn.Close()
			return nil, fmt.Errorf("sshchain: payload injection: %w", err)
		}
	}

	auth, err := e.authMethod()
	if err != nil {
		conn.Close()
		return nil, err
	}
	sshConfig := &ssh.ClientConfig{
		User:            e.cfg.User,
		Auth:            []ssh.AuthMethod{auth},
		HostKeyCallback: ssh.InsecureIgnoreHostKey(), // identity lives in the transport config, not host keys
		Timeout:         timeout,
	}
	if len(e.cfg.Ciphers) > 0 {
		sshConfig.Config.Ciphers = e.cfg.Ciphers
	}

	sshConn, chans, reqs, err := ssh.NewClientConn(conn, e.cfg.Addr, sshConfig)
	if err != nil {
		conn.Close()
		return nil, fmt.Errorf("sshchain: ssh handshake: %w", err)
	}
	return ssh.NewClient(sshConn, chans, reqs), nil
}

// transport dials the connection with the configured wrap applied.
func (e *Engine) transport(ctx context.Context) (net.Conn, error) {
	dialer := &net.Dialer{Timeout: 10 * time.Second}
	switch e.cfg.Wrap {
	case WrapNone:
		return dialContextConn(ctx, dialer, e.cfg.Addr)
	case WrapHTTP:
		return httpConnectDial(ctx, e.cfg.ProxyAddr, e.cfg.Addr, e.cfg.HTTPHost)
	case WrapTLS:
		return e.tlsDial(ctx, dialer, false)
	case WrapWSS:
		return e.tlsDial(ctx, dialer, true)
	case WrapWS:
		return wsDial(ctx, e.cfg.Addr, e.cfg.WSHost, e.cfg.WSPath, nil)
	}
	return nil, fmt.Errorf("sshchain: unhandled wrap %q", e.cfg.Wrap)
}

func (e *Engine) tlsDial(ctx context.Context, dialer *net.Dialer, innerWS bool) (net.Conn, error) {
	raw, err := dialContextConn(ctx, dialer, e.cfg.Addr)
	if err != nil {
		return nil, err
	}
	tlsConfig := &tls.Config{
		ServerName:         e.cfg.TLSSNI,
		InsecureSkipVerify: e.cfg.TLSInsecure, // fronting domains never match the certificate
	}
	tlsConn := tls.Client(raw, tlsConfig)
	if err := tlsConn.HandshakeContext(ctx); err != nil {
		raw.Close()
		return nil, fmt.Errorf("sshchain: tls handshake: %w", err)
	}
	if !innerWS {
		return tlsConn, nil
	}
	// wss: the WebSocket upgrade runs inside the TLS connection.
	return wsDial(ctx, "", e.cfg.WSHost, e.cfg.WSPath, tlsConn)
}

func (e *Engine) authMethod() (ssh.AuthMethod, error) {
	switch e.cfg.Auth {
	case AuthPassword:
		return ssh.Password(e.cfg.Secret), nil
	case AuthKey:
		signer, err := ssh.ParsePrivateKey([]byte(e.cfg.Secret))
		if err != nil {
			return nil, fmt.Errorf("sshchain: parse key: %w", err)
		}
		return ssh.PublicKeys(signer), nil
	}
	return nil, fmt.Errorf("sshchain: unknown auth %q", e.cfg.Auth)
}

func dialContextConn(ctx context.Context, d *net.Dialer, addr string) (net.Conn, error) {
	netD := *d
	conn, err := netD.DialContext(ctx, "tcp", addr)
	if err != nil {
		return nil, fmt.Errorf("sshchain: dial %s: %w", addr, err)
	}
	return conn, nil
}

// acceptLoop runs the local SOCKS5 listener.
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
			e.log("sshchain: accept: "+err.Error(), "error")
			return
		}
		e.done.Add(1)
		go func(conn net.Conn) {
			defer e.done.Done()
			defer conn.Close()
			if err := e.socksServe(conn); err != nil {
				e.log("sshchain: session: "+err.Error(), "debug")
			}
		}(conn)
	}
}

// socksServe speaks RFC 1928 (no-auth + CONNECT, IPv4/IPv6/domain targets)
// on one client connection and bridges it through the SSH tunnel.
func (e *Engine) socksServe(conn net.Conn) error {
	// Greeting: VER NMETHODS METHODS...
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
	if _, err := conn.Write([]byte{0x05, 0x00}); err != nil { // NO-AUTH
		return err
	}

	// Request: VER CMD RSV ATYP DST.ADDR DST.PORT
	req := make([]byte, 4)
	if _, err := io.ReadFull(conn, req); err != nil {
		return err
	}
	if req[0] != 0x05 || req[1] != 0x01 { // CONNECT only
		conn.Write([]byte{0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0})
		return errors.New("only CONNECT supported")
	}
	var host string
	switch req[3] {
	case 0x01: // IPv4
		buf := make([]byte, 4)
		if _, err := io.ReadFull(conn, buf); err != nil {
			return err
		}
		host = net.IP(buf).String()
	case 0x04: // IPv6
		buf := make([]byte, 16)
		if _, err := io.ReadFull(conn, buf); err != nil {
			return err
		}
		host = net.IP(buf).String()
	case 0x03: // domain
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

	e.mu.Lock()
	client := e.client
	e.mu.Unlock()
	if client == nil {
		return errors.New("tunnel is down")
	}

	target := net.JoinHostPort(host, strconv.Itoa(int(port)))
	remote, err := client.Dial("tcp", target)
	if err != nil {
		conn.Write([]byte{0x05, 0x04, 0x00, 0x01, 0, 0, 0, 0, 0, 0}) // host unreachable
		return fmt.Errorf("ssh dial %s: %w", target, err)
	}
	defer remote.Close()
	if _, err := conn.Write([]byte{0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0}); err != nil {
		return err
	}

	// Bidirectional bridge; first side finishing closes the session.
	done := make(chan error, 2)
	go func() { _, err := io.Copy(remote, conn); done <- err }()
	go func() { _, err := io.Copy(conn, remote); done <- err }()
	<-done
	return nil
}

func contains(haystack []string, needle string) bool {
	for _, item := range haystack {
		if item == needle {
			return true
		}
	}
	return false
}
