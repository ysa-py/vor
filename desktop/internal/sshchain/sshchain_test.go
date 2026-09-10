package sshchain

import (
	"bufio"
	"context"
	"crypto/rand"
	"crypto/rsa"
	"crypto/tls"
	"crypto/x509"
	"encoding/binary"
	"encoding/pem"
	"fmt"
	"io"
	"net"
	"net/http"
	"strconv"
	"strings"
	"sync"
	"testing"
	"time"

	"golang.org/x/crypto/ssh"
)

// ---- test infrastructure -------------------------------------------------

// echoServer is a TCP server that prefixes every chunk with "echo:".
func echoServer(t *testing.T) (addr string, stop func()) {
	t.Helper()
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	done := make(chan struct{})
	go func() {
		defer close(done)
		for {
			conn, err := ln.Accept()
			if err != nil {
				return
			}
			go func(conn net.Conn) {
				defer conn.Close()
				io.Copy(conn, conn)
			}(conn)
		}
	}()
	return ln.Addr().String(), func() { ln.Close(); <-done }
}

// sshTestServer runs an in-process SSH server with password auth and
// direct-tcpip forwarding. Optional wraps: TLS listener, prefix bytes to
// strip before the SSH stream, and a readiness callback.
type sshTestServer struct {
	tlsConfig    *tls.Config // non-nil => TLS-wrapped listener
	strip        int         // bytes to discard before the SSH stream (payload-injection shim)
	acceptAnyKey bool        // enable public-key auth
	gotPrefix    []byte

	ln   net.Listener
	addr string
	mu   sync.Mutex
}

func (s *sshTestServer) start(t *testing.T) {
	t.Helper()
	raw, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	var ln net.Listener = raw
	if s.tlsConfig != nil {
		ln = tls.NewListener(raw, s.tlsConfig)
	}
	s.ln = ln
	s.addr = ln.Addr().String()

	config := &ssh.ServerConfig{
		PasswordCallback: func(conn ssh.ConnMetadata, password []byte) (*ssh.Permissions, error) {
			if conn.User() == "vor" && string(password) == "sekrit" {
				return &ssh.Permissions{}, nil
			}
			return nil, fmt.Errorf("auth rejected")
		},
	}
	if s.acceptAnyKey {
		config.PublicKeyCallback = func(conn ssh.ConnMetadata, key ssh.PublicKey) (*ssh.Permissions, error) {
			if conn.User() == "vor" {
				return &ssh.Permissions{}, nil
			}
			return nil, fmt.Errorf("auth rejected")
		}
	}
	hostKey, err := rsa.GenerateKey(rand.Reader, 2048)
	if err != nil {
		t.Fatal(err)
	}
	signer, err := ssh.NewSignerFromKey(hostKey)
	if err != nil {
		t.Fatal(err)
	}
	config.AddHostKey(signer)

	go func() {
		for {
			conn, err := ln.Accept()
			if err != nil {
				return
			}
			go s.handle(conn, config)
		}
	}()
}

func (s *sshTestServer) handle(conn net.Conn, config *ssh.ServerConfig) {
	defer conn.Close()
	if s.strip > 0 {
		prefix := make([]byte, s.strip)
		if _, err := io.ReadFull(conn, prefix); err != nil {
			return
		}
		s.mu.Lock()
		s.gotPrefix = append(s.gotPrefix, prefix...)
		s.mu.Unlock()
	}
	serverConn, chans, reqs, err := ssh.NewServerConn(conn, config)
	if err != nil {
		return
	}
	defer serverConn.Close()
	go ssh.DiscardRequests(reqs)
	for newChannel := range chans {
		if newChannel.ChannelType() != "session" && newChannel.ChannelType() != "direct-tcpip" {
			newChannel.Reject(ssh.UnknownChannelType, "unsupported")
			continue
		}
		if newChannel.ChannelType() == "direct-tcpip" {
			// The target address travels in the channel open payload —
			// capture it BEFORE accepting.
			openPayload := newChannel.ExtraData()
			channel, requests, err := newChannel.Accept()
			if err != nil {
				continue
			}
			go ssh.DiscardRequests(requests)
			go forwardChannel(channel, openPayload)
			continue
		}
		channel, requests, err := newChannel.Accept()
		if err != nil {
			continue
		}
		go ssh.DiscardRequests(requests)
		channel.Close()
	}
}

// forwardChannel dials the target requested in a direct-tcpip payload and
// bridges both directions.
func forwardChannel(channel ssh.Channel, openPayload []byte) {
	defer channel.Close()
	target, rest, ok := parseSSHString(openPayload)
	if !ok {
		return
	}
	portBytes, _, ok := parseSSHUint32(rest)
	if !ok {
		return
	}
	targetAddr := net.JoinHostPort(target, strconv.Itoa(int(portBytes)))

	conn, err := net.DialTimeout("tcp", targetAddr, 5*time.Second)
	if err != nil {
		channel.Close()
		return
	}
	defer conn.Close()
	done := make(chan struct{}, 2)
	go func() { io.Copy(conn, channel); done <- struct{}{} }()
	go func() { io.Copy(channel, conn); done <- struct{}{} }()
	<-done
}

func parseSSHString(data []byte) (string, []byte, bool) {
	if len(data) < 4 {
		return "", nil, false
	}
	length := int(binary.BigEndian.Uint32(data[:4]))
	if len(data) < 4+length {
		return "", nil, false
	}
	return string(data[4 : 4+length]), data[4+length:], true
}

func parseSSHUint32(data []byte) (uint32, []byte, bool) {
	if len(data) < 4 {
		return 0, nil, false
	}
	return binary.BigEndian.Uint32(data[:4]), data[4:], true
}

// socksRequest performs a raw SOCKS5 CONNECT through the engine's listener
// and returns the connected net.Conn.
func socksRequest(t *testing.T, socksAddr, targetHost string, targetPort uint16) net.Conn {
	t.Helper()
	conn, err := net.DialTimeout("tcp", socksAddr, 5*time.Second)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := conn.Write([]byte{0x05, 0x01, 0x00}); err != nil { // no-auth offered
		t.Fatal(err)
	}
	reply := make([]byte, 2)
	if _, err := io.ReadFull(conn, reply); err != nil {
		t.Fatal(err)
	}
	if reply[0] != 0x05 || reply[1] != 0x00 {
		t.Fatalf("bad greeting reply %v", reply)
	}
	host := []byte(targetHost)
	request := []byte{0x05, 0x01, 0x00, 0x03, byte(len(host))}
	request = append(request, host...)
	request = append(request, byte(targetPort>>8), byte(targetPort&0xFF))
	if _, err := conn.Write(request); err != nil {
		t.Fatal(err)
	}
	// Success reply is 10 bytes (ATYP IPv4).
	head := make([]byte, 4)
	if _, err := io.ReadFull(conn, head); err != nil {
		t.Fatal(err)
	}
	if head[1] != 0x00 {
		t.Fatalf("CONNECT failed: rep=%d", head[1])
	}
	rest := make([]byte, 6)
	if _, err := io.ReadFull(conn, rest); err != nil {
		t.Fatal(err)
	}
	return conn
}

func portOf(addr string) uint16 {
	_, port, _ := net.SplitHostPort(addr)
	var p uint16
	fmt.Sscanf(port, "%d", &p)
	return p
}

// freePort gives a usable local listen address.
func freePort(t *testing.T) string {
	t.Helper()
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	defer ln.Close()
	return ln.Addr().String()
}

func selfSignedTLS(t *testing.T) *tls.Config {
	t.Helper()
	key, err := rsa.GenerateKey(rand.Reader, 2048)
	if err != nil {
		t.Fatal(err)
	}
	template := &x509.Certificate{
		DNSNames:  []string{"front.example"},
		PublicKey: &key.PublicKey,
	}
	der, err := x509.CreateCertificate(rand.Reader, template, template, &key.PublicKey, key)
	if err != nil {
		t.Fatal(err)
	}
	cert := tls.Certificate{Certificate: [][]byte{der}, PrivateKey: key}
	return &tls.Config{Certificates: []tls.Certificate{cert}}
}

// runThroughEngine starts an engine against an SSH server and round-trips
// a payload through the SOCKS5 front to the echo server.
func runThroughEngine(t *testing.T, cfg Config) {
	t.Helper()
	echoAddr, stopEcho := echoServer(t)
	defer stopEcho()

	engine := New(cfg, func(msg, level string) { t.Logf("[%s] %s", level, msg) })
	if err := engine.Start(); err != nil {
		t.Fatalf("engine start: %v", err)
	}
	defer engine.Stop()
	if engine.State() != "running" {
		t.Fatalf("state=%s", engine.State())
	}

	// "localhost" exercises the SOCKS5 domain ATYP with a name the SSH
	// server can actually resolve to the local echo server.
	conn := socksRequest(t, cfg.ListenAddr, "localhost", portOf(echoAddr))
	defer conn.Close()
	message := []byte("vor-roundtrip-payload")
	if _, err := conn.Write(message); err != nil {
		t.Fatal(err)
	}
	conn.SetReadDeadline(time.Now().Add(5 * time.Second))
	got := make([]byte, len(message))
	if _, err := io.ReadFull(conn, got); err != nil {
		t.Fatalf("roundtrip read: %v", err)
	}
	if string(got) != string(message) {
		t.Fatalf("roundtrip mismatch: %q", got)
	}
}

// ---- tests ---------------------------------------------------------------

func TestStandaloneSSHTunnel(t *testing.T) {
	server := &sshTestServer{}
	server.start(t)
	cfg := Defaults()
	cfg.Addr = server.addr
	cfg.Secret = "sekrit"
	cfg.ListenAddr = freePort(t)
	runThroughEngine(t, cfg)
}

func TestSSHTunnelTLSWrap(t *testing.T) {
	server := &sshTestServer{tlsConfig: selfSignedTLS(t)}
	server.start(t)
	cfg := Defaults()
	cfg.Addr = server.addr
	cfg.Secret = "sekrit"
	cfg.ListenAddr = freePort(t)
	cfg.Wrap = WrapTLS
	cfg.TLSSNI = "front.example"
	cfg.TLSInsecure = true
	runThroughEngine(t, cfg)
}

func TestSSHTunnelWebSocketWrap(t *testing.T) {
	server := &sshTestServer{}
	server.start(t)
	// A WebSocket bridge that unframes client frames and pipes the bytes
	// to the SSH server (what a CDN ws endpoint effectively does).
	bridgeAddr := wsBridge(t, server.addr, "")
	cfg := Defaults()
	cfg.Addr = bridgeAddr
	cfg.Secret = "sekrit"
	cfg.ListenAddr = freePort(t)
	cfg.Wrap = WrapWS
	cfg.WSPath = "/tunnel"
	runThroughEngine(t, cfg)
}

func TestSSHTunnelHTTPConnectWrap(t *testing.T) {
	server := &sshTestServer{}
	server.start(t)
	proxyAddr := connectProxy(t, server.addr)
	cfg := Defaults()
	cfg.Addr = server.addr
	cfg.Secret = "sekrit"
	cfg.ListenAddr = freePort(t)
	cfg.Wrap = WrapHTTP
	cfg.ProxyAddr = proxyAddr
	cfg.HTTPHost = "ssh.internal"
	runThroughEngine(t, cfg)
}

func TestPayloadInjection(t *testing.T) {
	server := &sshTestServer{strip: 6}
	server.start(t)
	cfg := Defaults()
	cfg.Addr = server.addr
	cfg.Secret = "sekrit"
	cfg.ListenAddr = freePort(t)
	cfg.InjectHex = "deadbeef9001"
	runThroughEngine(t, cfg)

	server.mu.Lock()
	prefix := string(server.gotPrefix)
	server.mu.Unlock()
	if prefix != "\xde\xad\xbe\xef\x90\x01" {
		t.Fatalf("server did not receive injected payload: % x", prefix)
	}
}

func TestKeyAuth(t *testing.T) {
	key, err := rsa.GenerateKey(rand.Reader, 2048)
	if err != nil {
		t.Fatal(err)
	}
	pemBytes := pem.EncodeToMemory(&pem.Block{Type: "RSA PRIVATE KEY", Bytes: x509.MarshalPKCS1PrivateKey(key)})

	server := &sshTestServer{acceptAnyKey: true}
	server.start(t)
	cfg := Defaults()
	cfg.Addr = server.addr
	cfg.Auth = AuthKey
	cfg.Secret = string(pemBytes)
	cfg.ListenAddr = freePort(t)
	runThroughEngine(t, cfg)
}

func TestCipherSelection(t *testing.T) {
	server := &sshTestServer{}
	server.start(t)
	cfg := Defaults()
	cfg.Addr = server.addr
	cfg.Secret = "sekrit"
	cfg.ListenAddr = freePort(t)
	cfg.Ciphers = []string{"aes128-ctr"}
	runThroughEngine(t, cfg)
}

func TestValidateRejects(t *testing.T) {
	cases := []Config{
		{Addr: "no-port", User: "u", Auth: AuthPassword, Secret: "s", ListenAddr: "x"},
		{Addr: "h:1", User: "", Auth: AuthPassword, Secret: "s", ListenAddr: "x"},
		{Addr: "h:1", User: "u", Auth: AuthPassword, Secret: "", ListenAddr: "x"},
		{Addr: "h:1", User: "u", Auth: AuthKey, Secret: "", ListenAddr: "x"},
		{Addr: "h:1", User: "u", Auth: "bogus", Secret: "s", ListenAddr: "x"},
		{Addr: "h:1", User: "u", Auth: AuthPassword, Secret: "s", Wrap: "bogus", ListenAddr: "x"},
		{Addr: "h:1", User: "u", Auth: AuthPassword, Secret: "s", Wrap: WrapTLS, ListenAddr: "x"},
		{Addr: "h:1", User: "u", Auth: AuthPassword, Secret: "s", Wrap: WrapHTTP, ListenAddr: "x"},
		{Addr: "h:1", User: "u", Auth: AuthPassword, Secret: "s", InjectHex: "zz", ListenAddr: "x"},
		{Addr: "h:1", User: "u", Auth: AuthPassword, Secret: "s", Ciphers: []string{"rot13"}, ListenAddr: "x"},
		{Addr: "h:1", User: "u", Auth: AuthPassword, Secret: "s"},
	}
	for index, cfg := range cases {
		if err := cfg.Validate(); err == nil {
			t.Errorf("case %d: expected error", index)
		}
	}
}

// ---- extra server helpers -------------------------------------------------

// wsBridge is a WebSocket endpoint that unframes and pipes to the backend.
func wsBridge(t *testing.T, backend, expectedHost string) string {
	t.Helper()
	mux := http.NewServeMux()
	mux.HandleFunc("/tunnel", func(w http.ResponseWriter, r *http.Request) {
		if expectedHost != "" && r.Host != expectedHost {
			http.Error(w, "bad host", http.StatusForbidden)
			return
		}
		if !strings.EqualFold(r.Header.Get("Upgrade"), "websocket") {
			http.Error(w, "not ws", http.StatusBadRequest)
			return
		}
		key := r.Header.Get("Sec-WebSocket-Key")
		accept := wsAcceptKey(key)
		hijacker, ok := w.(http.Hijacker)
		if !ok {
			http.Error(w, "no hijack", http.StatusInternalServerError)
			return
		}
		conn, bufio, err := hijacker.Hijack()
		if err != nil {
			return
		}
		defer conn.Close()
		response := "HTTP/1.1 101 Switching Protocols\r\n" +
			"Upgrade: websocket\r\nConnection: Upgrade\r\n" +
			"Sec-WebSocket-Accept: " + accept + "\r\n\r\n"
		if _, err := conn.Write([]byte(response)); err != nil {
			return
		}
		// pipe to the SSH backend
		upstream, err := net.DialTimeout("tcp", backend, 5*time.Second)
		if err != nil {
			return
		}
		defer upstream.Close()
		if bufio.Reader.Buffered() > 0 {
			buffered, _ := bufio.Reader.Peek(bufio.Reader.Buffered())
			upstream.Write(buffered)
		}
		done := make(chan struct{}, 2)
		go func() {
			// client -> (unframe) -> backend. Frames may coalesce or split
			// across reads; decode incrementally and keep the remainder.
			pumpUnframe(bufio.Reader, upstream)
			done <- struct{}{}
		}()
		go func() {
			// backend -> (frame) -> client
			buf := make([]byte, 32*1024)
			for {
				n, err := upstream.Read(buf)
				if n > 0 {
					frame := frameServerMessage(buf[:n])
					conn.Write(frame)
				}
				if err != nil {
					done <- struct{}{}
					return
				}
			}
		}()
		<-done
	})
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	go http.Serve(ln, mux)
	return ln.Addr().String()
}

// unframe decodes one (masked, client-bound) frame.
func unframe(data []byte) ([]byte, byte, error) {
	if len(data) < 2 {
		return nil, 0, io.ErrShortBuffer
	}
	opcode := data[0] & 0x0F
	masked := data[1]&0x80 != 0
	length := int(data[1] & 0x7F)
	offset := 2
	switch length {
	case 126:
		if len(data) < 4 {
			return nil, 0, io.ErrShortBuffer
		}
		length = int(binary.BigEndian.Uint16(data[2:4]))
		offset = 4
	case 127:
		if len(data) < 10 {
			return nil, 0, io.ErrShortBuffer
		}
		length = int(binary.BigEndian.Uint64(data[2:10]))
		offset = 10
	}
	var mask []byte
	if masked {
		if len(data) < offset+4 {
			return nil, 0, io.ErrShortBuffer
		}
		mask = data[offset : offset+4]
		offset += 4
	}
	if len(data) < offset+length {
		return nil, 0, io.ErrShortBuffer
	}
	payload := make([]byte, length)
	copy(payload, data[offset:offset+length])
	if masked {
		for i := 0; i < length; i++ {
			payload[i] ^= mask[i%4]
		}
	}
	// Return the payload plus the remaining bytes so the caller can loop.
	return payload, opcode, nil
}

// pumpUnframe reads framed bytes from src, decodes every complete frame in
// each chunk, and writes the payloads to dst. Partial frames are buffered.
func pumpUnframe(src *bufio.Reader, dst io.Writer) {
	buffer := make([]byte, 0, 64*1024)
	chunk := make([]byte, 32*1024)
	for {
		n, err := src.Read(chunk)
		if n > 0 {
			buffer = append(buffer, chunk[:n]...)
			for len(buffer) >= 2 {
				// Decode one frame; determine its total length to slice.
				need := frameLength(buffer)
				if need < 0 || len(buffer) < need {
					break // wait for more bytes
				}
				payload, _, ferr := unframe(buffer[:need])
				if ferr != nil {
					return // protocol violation
				}
				if len(payload) > 0 {
					if _, werr := dst.Write(payload); werr != nil {
						return
					}
				}
				buffer = buffer[need:]
				// Compact to keep memory bounded.
				if len(buffer) == 0 {
					buffer = buffer[:0]
				}
			}
		}
		if err != nil {
			return
		}
	}
}

// frameLength returns the total wire length of the first frame in data, or
// -1 when there are not enough bytes to know yet.
func frameLength(data []byte) int {
	if len(data) < 2 {
		return -1
	}
	length := int(data[1] & 0x7F)
	offset := 2
	switch length {
	case 126:
		if len(data) < 4 {
			return -1
		}
		length = int(binary.BigEndian.Uint16(data[2:4]))
		offset = 4
	case 127:
		if len(data) < 10 {
			return -1
		}
		length = int(binary.BigEndian.Uint64(data[2:10]))
		offset = 10
	}
	if data[1]&0x80 != 0 { // masked
		offset += 4
	}
	return offset + length
}

// frameServerMessage encodes one unmasked binary frame.
func frameServerMessage(payload []byte) []byte {
	header := []byte{0x82} // FIN + binary
	length := len(payload)
	switch {
	case length < 126:
		header = append(header, byte(length))
	case length <= 0xFFFF:
		header = append(header, 126)
		ext := make([]byte, 2)
		binary.BigEndian.PutUint16(ext, uint16(length))
		header = append(header, ext...)
	default:
		header = append(header, 127)
		ext := make([]byte, 8)
		binary.BigEndian.PutUint64(ext, uint64(length))
		header = append(header, ext...)
	}
	return append(header, payload...)
}

// connectProxy is a minimal HTTP CONNECT proxy.
func connectProxy(t *testing.T, backend string) string {
	t.Helper()
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	go func() {
		for {
			conn, err := ln.Accept()
			if err != nil {
				return
			}
			go func(conn net.Conn) {
				defer conn.Close()
				request, err := http.ReadRequest(bufio.NewReader(conn))
				if err != nil || request.Method != http.MethodConnect {
					return
				}
				_, port, _ := net.SplitHostPort(request.RequestURI)
				_ = port
				upstream, err := net.DialTimeout("tcp", backend, 5*time.Second)
				if err != nil {
					conn.Write([]byte("HTTP/1.1 502 Bad Gateway\r\n\r\n"))
					return
				}
				defer upstream.Close()
				if _, err := conn.Write([]byte("HTTP/1.1 200 OK\r\n\r\n")); err != nil {
					return
				}
				done := make(chan struct{}, 2)
				go func() { io.Copy(upstream, conn); done <- struct{}{} }()
				go func() { io.Copy(conn, upstream); done <- struct{}{} }()
				<-done
			}(conn)
		}
	}()
	return ln.Addr().String()
}

var _ = context.Background
