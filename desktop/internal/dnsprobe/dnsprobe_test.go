package dnsprobe

import (
	"context"
	"encoding/binary"
	"net"
	"net/http"
	"strings"
	"sync"
	"testing"
	"time"
)

// fakeUDPResolver is a scripted UDP DNS server.
type fakeUDPResolver struct {
	ln        *net.UDPConn
	addr      string
	edns      bool // echo OPT in replies
	hijack    bool // answer NXDOMAIN queries with a fake A record
	mu        sync.Mutex
	queries   int
	ednsSeen  bool
	closeOnce sync.Once
}

func startFakeUDP(t *testing.T, edns, hijack bool) *fakeUDPResolver {
	t.Helper()
	addr, err := net.ResolveUDPAddr("udp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	conn, err := net.ListenUDP("udp", addr)
	if err != nil {
		t.Fatal(err)
	}
	server := &fakeUDPResolver{ln: conn, addr: conn.LocalAddr().String(), edns: edns, hijack: hijack}
	go server.loop()
	t.Cleanup(func() { server.closeOnce.Do(func() { conn.Close() }) })
	return server
}

func (f *fakeUDPResolver) loop() {
	buffer := make([]byte, 1500)
	for {
		n, client, err := f.ln.ReadFromUDP(buffer)
		if err != nil {
			return
		}
		go f.handle(buffer[:n], client)
	}
}

func (f *fakeUDPResolver) handle(query []byte, client *net.UDPAddr) {
	if len(query) < 12 {
		return
	}
	id := binary.BigEndian.Uint16(query[0:2])
	// Find the question name.
	name, after, err := readName(query, 12)
	if err != nil {
		return
	}
	// Detect an OPT record (EDNS0) in the additional section.
	arcount := int(binary.BigEndian.Uint16(query[10:12]))
	hasOPT := strings.Count(string(query[after:]), "\x00\x29") > 0 || arcount > 0
	f.mu.Lock()
	f.queries++
	if hasOPT {
		f.ednsSeen = true
	}
	f.mu.Unlock()

	response := f.composeReply(id, name, query[after:after+4])
	if f.edns {
		// Echo an OPT RR in the additional section.
		response = append(response, 0x00)
		response = append(response, 0x00, 0x29)                         // type OPT
		response = append(response, 0x04, 0xD0)                         // class 1232
		response = append(response, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00) // TTL + RDLENGTH
		binary.BigEndian.PutUint16(response[10:12], 1)                  // ARCOUNT = 1
	}
	f.ln.WriteToUDP(response, client)
}

// composeReply builds an answer. NXDOMAIN requests (name under .invalid)
// get RCODE 3 unless the server is a hijacker.
func (f *fakeUDPResolver) composeReply(id uint16, name string, tail []byte) []byte {
	var rcode uint16
	var answers []byte
	hijacked := false
	if strings.HasSuffix(name, ".invalid") {
		if f.hijack {
			// Hijacker: NOERROR + fake A record (search-redirect style).
			rcode = 0
			hijacked = true
			answers = appendRecord(answers, name, 1, net.IPv4(146, 112, 61, 106))
		} else {
			rcode = 3 // NXDOMAIN
		}
	} else {
		rcode = 0
		answers = appendRecord(answers, name, 1, net.IPv4(203, 0, 113, 7))
	}

	response := make([]byte, 12)
	binary.BigEndian.PutUint16(response[0:2], id)
	binary.BigEndian.PutUint16(response[2:4], 0x8180|rcode)            // QR + RD + RA
	binary.BigEndian.PutUint16(response[4:6], 1)                       // QDCOUNT
	binary.BigEndian.PutUint16(response[6:8], uint16(len(answers)/16)) // placeholder, fixed below
	binary.BigEndian.PutUint16(response[8:10], 0)
	binary.BigEndian.PutUint16(response[10:12], 0)

	// Echo the question.
	response = appendName(response, name)
	response = append(response, tail...) // QTYPE + QCLASS
	// Answers.
	answerCount := 0
	if hijacked {
		answerCount = 1
	} else if !strings.HasSuffix(name, ".invalid") {
		answerCount = 1
	}
	binary.BigEndian.PutUint16(response[6:8], uint16(answerCount))
	return append(response, answers...)
}

func appendRecord(buf []byte, name string, rtype uint16, ip net.IP) []byte {
	buf = appendName(buf, name)
	var typeBytes [2]byte
	binary.BigEndian.PutUint16(typeBytes[:], rtype)
	buf = append(buf, typeBytes[:]...)
	buf = append(buf, 0x00, 0x01)             // IN
	buf = append(buf, 0x00, 0x00, 0x00, 0x3C) // TTL 60
	buf = append(buf, 0x00, 0x04)             // RDLENGTH 4
	return append(buf, ip.To4()...)
}

func appendName(buf []byte, name string) []byte {
	if name == "." {
		return append(buf, 0)
	}
	for _, label := range strings.Split(strings.TrimSuffix(name, "."), ".") {
		buf = append(buf, byte(len(label)))
		buf = append(buf, label...)
	}
	return append(buf, 0)
}

// ---- tests -----------------------------------------------------------------

func TestParseResponseRoundtrip(t *testing.T) {
	server := startFakeUDP(t, true, false)
	resolver := Resolver{Transport: TransportUDP, Server: server.addr, Timeout: time.Second}
	response, err := resolver.Resolve(context.Background(), "example.com", typeA)
	if err != nil {
		t.Fatal(err)
	}
	if response.RCode != 0 || len(response.Answers) != 1 {
		t.Fatalf("unexpected response: %+v", response)
	}
	if !response.Answers[0].Data.Equal(net.IPv4(203, 0, 113, 7).To4()) {
		t.Fatalf("unexpected answer IP: %v", response.Answers[0].Data)
	}
	if !response.HasEDNS {
		t.Fatal("EDNS OPT missing from reply")
	}
}

func TestProbeHealthyResolver(t *testing.T) {
	server := startFakeUDP(t, true, false)
	resolver := Resolver{Transport: TransportUDP, Server: server.addr, Timeout: time.Second}
	result := Probe(context.Background(), resolver, "example.com")
	if result.Error != "" {
		t.Fatalf("probe error: %s", result.Error)
	}
	if !result.EDNS {
		t.Fatal("expected EDNS support")
	}
	if !result.Honest {
		t.Fatal("expected honest NXDOMAIN behavior")
	}
	if result.Score < 80 {
		t.Fatalf("healthy resolver scored %.1f", result.Score)
	}
}

func TestProbeDetectsHijacker(t *testing.T) {
	server := startFakeUDP(t, false, true)
	resolver := Resolver{Transport: TransportUDP, Server: server.addr, Timeout: time.Second}
	result := Probe(context.Background(), resolver, "example.com")
	if result.Honest {
		t.Fatal("hijacker must be detected")
	}
	if result.HijackIP != "146.112.61.106" {
		t.Fatalf("hijack IP: %q", result.HijackIP)
	}
	if result.Score > 30 {
		t.Fatalf("hijacker scored %.1f — must be heavily penalized", result.Score)
	}
}

func TestProbeNoEDNS(t *testing.T) {
	server := startFakeUDP(t, false, false)
	resolver := Resolver{Transport: TransportUDP, Server: server.addr, Timeout: time.Second}
	result := Probe(context.Background(), resolver, "example.com")
	if result.EDNS {
		t.Fatal("unexpected EDNS support")
	}
	if result.Score >= 90 {
		t.Fatalf("no-EDNS resolver scored %.1f — expected small penalty", result.Score)
	}
}

func TestProbeUnreachable(t *testing.T) {
	resolver := Resolver{Transport: TransportUDP, Server: "127.0.0.1:1", Timeout: 300 * time.Millisecond}
	result := Probe(context.Background(), resolver, "example.com")
	if result.Error == "" || result.Score != 0 {
		t.Fatalf("expected error + zero score, got %+v", result)
	}
}

func TestScanSortsByScore(t *testing.T) {
	healthy := startFakeUDP(t, true, false)
	hijacker := startFakeUDP(t, false, true)
	resolvers := []Resolver{
		{Transport: TransportUDP, Server: "127.0.0.1:1", Timeout: 300 * time.Millisecond}, // dead
		{Transport: TransportUDP, Server: hijacker.addr, Timeout: time.Second},
		{Transport: TransportUDP, Server: healthy.addr, Timeout: time.Second},
	}
	results := Scan(context.Background(), resolvers, "example.com")
	if len(results) != 3 {
		t.Fatalf("got %d results", len(results))
	}
	if results[0].Resolver != healthy.addr || results[0].Score <= results[1].Score {
		t.Fatalf("results not sorted by score: %+v", results)
	}
	if results[2].Score != 0 {
		t.Fatalf("dead resolver must score 0")
	}
}

func TestDoHResolver(t *testing.T) {
	// A DoH endpoint backed by the fake UDP resolver logic.
	underlying := startFakeUDP(t, false, false)
	mux := http.NewServeMux()
	mux.HandleFunc("/dns-query", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost || r.Header.Get("Content-Type") != "application/dns-message" {
			http.Error(w, "bad request", http.StatusBadRequest)
			return
		}
		query := make([]byte, 1500)
		n, _ := r.Body.Read(query)
		query = query[:n]
		if len(query) < 12 {
			http.Error(w, "short", http.StatusBadRequest)
			return
		}
		id := binary.BigEndian.Uint16(query[0:2])
		name, after, err := readName(query, 12)
		if err != nil {
			http.Error(w, "bad name", http.StatusBadRequest)
			return
		}
		var response []byte
		if strings.HasSuffix(name, ".invalid") {
			response = composeStaticReply(id, name, query[after:after+4], nil, 3)
		} else {
			response = composeStaticReply(id, name, query[after:after+4], net.IPv4(203, 0, 113, 8), 0)
		}
		w.Header().Set("Content-Type", "application/dns-message")
		w.Write(response)
	})
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	go http.Serve(ln, mux)
	t.Cleanup(func() { ln.Close() })

	resolver := Resolver{Transport: TransportDoH, Server: "http://" + ln.Addr().String() + "/dns-query", Timeout: 2 * time.Second}
	result := Probe(context.Background(), resolver, "example.com")
	if result.Error != "" {
		t.Fatalf("DoH probe failed: %s", result.Error)
	}
	if !result.Honest || !result.EDNS {
		t.Fatalf("DoH result: %+v", result)
	}
	if result.LatencyMS <= 0 {
		t.Fatal("latency must be positive")
	}
	_ = underlying
}

// composeStaticReply mirrors fakeUDPResolver.composeReply for the DoH path.
func composeStaticReply(id uint16, name string, tail []byte, ip net.IP, rcode uint16) []byte {
	response := make([]byte, 12)
	binary.BigEndian.PutUint16(response[0:2], id)
	binary.BigEndian.PutUint16(response[2:4], 0x8180|rcode)
	binary.BigEndian.PutUint16(response[4:6], 1)
	answers := 0
	if ip != nil {
		answers = 1
	}
	binary.BigEndian.PutUint16(response[6:8], uint16(answers))
	response = appendName(response, name)
	response = append(response, tail...)
	if ip != nil {
		response = appendRecord(response, name, 1, ip)
	}
	return response
}

func TestWireQueryCounts(t *testing.T) {
	plain := buildQuery(0x1234, Question{Name: "a.example", Type: typeA, Class: 1}, false)
	if binary.BigEndian.Uint16(plain[4:6]) != 1 {
		t.Fatal("QDCOUNT")
	}
	if binary.BigEndian.Uint16(plain[6:8]) != 0 || binary.BigEndian.Uint16(plain[10:12]) != 0 {
		t.Fatal("plain query must have no ARCOUNT")
	}
	ednsQuery := buildQuery(0x1234, Question{Name: "a.example", Type: typeA, Class: 1}, true)
	if binary.BigEndian.Uint16(ednsQuery[6:8]) != 0 {
		t.Fatal("EDNS query must have zero ANCOUNT")
	}
	if binary.BigEndian.Uint16(ednsQuery[10:12]) != 1 {
		t.Fatal("EDNS query must have ARCOUNT 1")
	}
	// The OPT record decodes back.
	response := append(ednsQuery[:0:0], ednsQuery...) // copy
	response[2] |= 0x80                               // QR bit
	parsed, err := parseResponse(response)
	if err != nil {
		t.Fatal(err)
	}
	if !parsed.HasEDNS {
		t.Fatal("OPT not parsed back")
	}
}

func TestNameCompression(t *testing.T) {
	// Reply with a compressed answer name (pointer to offset 12).
	server := startFakeUDP(t, false, false)
	_ = server
	response := make([]byte, 12)
	binary.BigEndian.PutUint16(response[0:2], 1)
	binary.BigEndian.PutUint16(response[2:4], 0x8180)
	binary.BigEndian.PutUint16(response[4:6], 1) // QDCOUNT
	binary.BigEndian.PutUint16(response[6:8], 1) // ANCOUNT
	// question: 3www7example3com0
	question := appendName(nil, "www.example.com")
	response = append(response, question...)
	response = append(response, 0x00, 0x01, 0x00, 0x01) // A, IN
	// answer name = pointer 0xC00C (offset 12)
	response = append(response, 0xC0, 0x0C)
	response = append(response, 0x00, 0x01, 0x00, 0x01)
	response = append(response, 0x00, 0x00, 0x00, 0x3C, 0x00, 0x04)
	response = append(response, 1, 2, 3, 4)
	parsed, err := parseResponse(response)
	if err != nil {
		t.Fatal(err)
	}
	if len(parsed.Answers) != 1 || parsed.Answers[0].Name != "www.example.com" {
		t.Fatalf("compressed name not resolved: %+v", parsed.Answers)
	}
	if !parsed.Answers[0].Data.Equal(net.IPv4(1, 2, 3, 4).To4()) {
		t.Fatalf("bad IP: %v", parsed.Answers[0].Data)
	}
}
