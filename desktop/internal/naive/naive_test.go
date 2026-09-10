package naive

import (
	"bufio"
	"crypto/rand"
	"crypto/rsa"
	"crypto/tls"
	"crypto/x509"
	"io"
	"net"
	"net/http"
	"strconv"
	"testing"
	"time"
)

// httpsProxy is an in-process TLS CONNECT proxy that records padding.
type httpsProxy struct {
	ln         net.Listener
	addr       string
	sawPadding int
	sawTarget  string
}

func startHTTPSProxy(t *testing.T) *httpsProxy {
	t.Helper()
	key, err := rsa.GenerateKey(rand.Reader, 2048)
	if err != nil {
		t.Fatal(err)
	}
	template := &x509.Certificate{DNSNames: []string{"cdn.example"}, PublicKey: &key.PublicKey}
	der, err := x509.CreateCertificate(rand.Reader, template, template, &key.PublicKey, key)
	if err != nil {
		t.Fatal(err)
	}
	cert := tls.Certificate{Certificate: [][]byte{der}, PrivateKey: key}
	inner, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	ln := tls.NewListener(inner, &tls.Config{Certificates: []tls.Certificate{cert}})
	proxy := &httpsProxy{ln: ln, addr: ln.Addr().String()}
	go func() {
		for {
			conn, err := ln.Accept()
			if err != nil {
				return
			}
			go proxy.handle(conn)
		}
	}()
	t.Cleanup(func() { ln.Close() })
	return proxy
}

func (p *httpsProxy) handle(conn net.Conn) {
	defer conn.Close()
	_ = conn.SetDeadline(time.Now().Add(10 * time.Second))
	reader := bufio.NewReader(conn)
	request, err := http.ReadRequest(reader)
	if err != nil || request.Method != http.MethodConnect {
		return
	}
	p.sawTarget = request.RequestURI
	if pad := request.Header.Get("X-Padding"); pad != "" {
		p.sawPadding = len(pad)
	}
	if _, err := conn.Write([]byte("HTTP/1.1 200 OK\r\n\r\n")); err != nil {
		return
	}
	// Bridge to the requested target.
	target, err := net.DialTimeout("tcp", request.RequestURI, 5*time.Second)
	if err != nil {
		return
	}
	defer target.Close()
	if reader.Buffered() > 0 {
		buffered, _ := reader.Peek(reader.Buffered())
		target.Write(buffered)
	}
	done := make(chan struct{}, 2)
	go func() { io.Copy(target, conn); done <- struct{}{} }()
	go func() { io.Copy(conn, target); done <- struct{}{} }()
	<-done
}

func freePort(t *testing.T) string {
	t.Helper()
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	defer ln.Close()
	return ln.Addr().String()
}

func socksRoundtrip(t *testing.T, socksAddr, targetHost string, targetPort uint16) {
	t.Helper()
	conn, err := net.DialTimeout("tcp", socksAddr, 5*time.Second)
	if err != nil {
		t.Fatal(err)
	}
	defer conn.Close()
	if _, err := conn.Write([]byte{0x05, 0x01, 0x00}); err != nil {
		t.Fatal(err)
	}
	reply := make([]byte, 2)
	if _, err := io.ReadFull(conn, reply); err != nil {
		t.Fatal(err)
	}
	host := []byte(targetHost)
	request := []byte{0x05, 0x01, 0x00, 0x03, byte(len(host))}
	request = append(request, host...)
	request = append(request, byte(targetPort>>8), byte(targetPort&0xFF))
	if _, err := conn.Write(request); err != nil {
		t.Fatal(err)
	}
	head := make([]byte, 4)
	if _, err := io.ReadFull(conn, head); err != nil {
		t.Fatal(err)
	}
	if head[1] != 0x00 {
		t.Fatalf("CONNECT failed: %d", head[1])
	}
	if _, err := io.ReadFull(conn, make([]byte, 6)); err != nil {
		t.Fatal(err)
	}
	message := []byte("naive-roundtrip")
	if _, err := conn.Write(message); err != nil {
		t.Fatal(err)
	}
	conn.SetReadDeadline(time.Now().Add(5 * time.Second))
	got := make([]byte, len(message))
	if _, err := io.ReadFull(conn, got); err != nil {
		t.Fatalf("roundtrip: %v", err)
	}
	if string(got) != string(message) {
		t.Fatalf("mismatch: %q", got)
	}
}

func TestNaiveTunnelRoundtrip(t *testing.T) {
	// Echo server as the CONNECT target.
	echoLn, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	defer echoLn.Close()
	go func() {
		for {
			conn, err := echoLn.Accept()
			if err != nil {
				return
			}
			go func(conn net.Conn) { defer conn.Close(); io.Copy(conn, conn) }(conn)
		}
	}()

	proxy := startHTTPSProxy(t)
	cfg := Defaults()
	cfg.ProxyAddr = proxy.addr
	cfg.Insecure = true
	cfg.PaddingBytes = 256
	cfg.ListenAddr = freePort(t)

	engine := New(cfg, func(msg, level string) { t.Logf("[%s] %s", level, msg) })
	if err := engine.Start(); err != nil {
		t.Fatal(err)
	}
	defer engine.Stop()
	if engine.State() != "running" {
		t.Fatalf("state=%s", engine.State())
	}

	_, port, _ := net.SplitHostPort(echoLn.Addr().String())
	var targetPort uint16
	fmtScan(port, &targetPort)
	socksRoundtrip(t, cfg.ListenAddr, "localhost", targetPort)

	if proxy.sawPadding < 16 {
		t.Fatalf("padding missing/short: %d", proxy.sawPadding)
	}
	_ = strconv.Itoa
}

func TestNaiveNoPadding(t *testing.T) {
	echoLn, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	defer echoLn.Close()
	go func() {
		for {
			conn, err := echoLn.Accept()
			if err != nil {
				return
			}
			go func(conn net.Conn) { defer conn.Close(); io.Copy(conn, conn) }(conn)
		}
	}()
	proxy := startHTTPSProxy(t)
	cfg := Defaults()
	cfg.ProxyAddr = proxy.addr
	cfg.Insecure = true
	cfg.PaddingBytes = 0
	cfg.ListenAddr = freePort(t)
	engine := New(cfg, nil)
	if err := engine.Start(); err != nil {
		t.Fatal(err)
	}
	defer engine.Stop()
	_, port, _ := net.SplitHostPort(echoLn.Addr().String())
	var targetPort uint16
	fmtScan(port, &targetPort)
	socksRoundtrip(t, cfg.ListenAddr, "localhost", targetPort)
	if proxy.sawPadding != 0 {
		t.Fatalf("unexpected padding: %d", proxy.sawPadding)
	}
}

func TestValidateRejects(t *testing.T) {
	cases := []Config{
		{ProxyAddr: "no-port", ListenAddr: "x"},
		{ProxyAddr: "h:1", PaddingBytes: -1, ListenAddr: "x"},
		{ProxyAddr: "h:1", PaddingBytes: 70000, ListenAddr: "x"},
		{ProxyAddr: "h:1"},
	}
	for index, cfg := range cases {
		if err := cfg.Validate(); err == nil {
			t.Errorf("case %d: expected error", index)
		}
	}
	if err := (Config{ProxyAddr: "h:1", ListenAddr: "x"}).Validate(); err != nil {
		t.Errorf("minimal valid config rejected: %v", err)
	}
}

func fmtScan(port string, out *uint16) {
	value, _ := strconv.Atoi(port)
	*out = uint16(value)
}
