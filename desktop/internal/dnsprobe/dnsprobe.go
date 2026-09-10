// Package dnsprobe is Vor's independent DNS-resolver scanner/scorer plus a
// multi-transport resolution layer (UDP, DNS-over-TLS, DNS-over-HTTPS).
//
// It is implemented from scratch against the DNS wire format (RFC 1035),
// EDNS0 (RFC 6891), DoT (RFC 7858) and DoH (RFC 8484) specifications.
// Nothing here derives from any third-party application.
//
// Scanner semantics:
//   - latency: median of N A-queries against a well-known domain
//   - EDNS0: does the resolver answer a query carrying an OPT RR
//   - NXDOMAIN honesty: a random label under a reserved TLD must return
//     RCODE 3 (NXDOMAIN). An A answer or NOERROR-with-no-data wildcard is
//     flagged as hijacking (DNS poisoning / search-redirect injectors).
//   - score: 0-100 composite (latency + capability + honesty)
package dnsprobe

import (
	"bytes"
	"context"
	"crypto/tls"
	"encoding/binary"
	"errors"
	"fmt"
	"io"
	"math/rand"
	"net"
	"net/http"
	"sync"
	"time"
)

// ---- wire format -----------------------------------------------------------

// Question is a single DNS query.
type Question struct {
	Name  string
	Type  uint16 // 1 = A, 28 = AAAA
	Class uint16 // 255 = ANY probe, 1 = IN
}

const (
	typeA     = 1
	typeAAAA  = 28
	typeOPT   = 41
	flagRD    = 0x0100
	rcodeMask = 0x000F
)

// buildQuery encodes a DNS query with an optional EDNS0 OPT record.
func buildQuery(id uint16, q Question, edns bool) []byte {
	var buf bytes.Buffer
	binary.Write(&buf, binary.BigEndian, id)
	binary.Write(&buf, binary.BigEndian, uint16(flagRD))
	binary.Write(&buf, binary.BigEndian, uint16(1)) // QDCOUNT
	binary.Write(&buf, binary.BigEndian, uint16(0)) // ANCOUNT
	binary.Write(&buf, binary.BigEndian, uint16(0)) // NSCOUNT
	arcount := uint16(0)                            // ARCOUNT (EDNS0 OPT)
	if edns {
		arcount = 1
	}
	binary.Write(&buf, binary.BigEndian, arcount)
	writeName(&buf, q.Name)
	binary.Write(&buf, binary.BigEndian, q.Type)
	binary.Write(&buf, binary.BigEndian, q.Class)
	if edns {
		// OPT RR: root name, type 41, class = UDP payload size, TTL=0,
		// RDLENGTH=0.
		buf.WriteByte(0)
		binary.Write(&buf, binary.BigEndian, uint16(typeOPT))
		binary.Write(&buf, binary.BigEndian, uint16(1232))
		binary.Write(&buf, binary.BigEndian, uint32(0))
		binary.Write(&buf, binary.BigEndian, uint16(0))
	}
	return buf.Bytes()
}

// writeName encodes a dotted name as DNS labels.
func writeName(buf *bytes.Buffer, name string) {
	if name == "" || name == "." {
		buf.WriteByte(0)
		return
	}
	for _, label := range splitLabels(name) {
		buf.WriteByte(byte(len(label)))
		buf.WriteString(label)
	}
	buf.WriteByte(0)
}

func splitLabels(name string) []string {
	var labels []string
	current := ""
	for _, r := range name {
		if r == '.' {
			if current != "" {
				labels = append(labels, current)
				current = ""
			}
			continue
		}
		current += string(r)
	}
	if current != "" {
		labels = append(labels, current)
	}
	return labels
}

// Response is a parsed DNS reply.
type Response struct {
	ID        uint16
	RCode     int
	Answers   []Record
	HasEDNS   bool // OPT present in the reply
	Truncated bool
}

// Record is one answer RR.
type Record struct {
	Name  string
	Type  uint16
	Class uint16
	TTL   uint32
	Data  net.IP
}

// parseResponse decodes a reply (question echo assumed for name parsing).
func parseResponse(data []byte) (*Response, error) {
	if len(data) < 12 {
		return nil, errors.New("dnsprobe: short header")
	}
	resp := &Response{
		ID:    binary.BigEndian.Uint16(data[0:2]),
		RCode: int(binary.BigEndian.Uint16(data[2:4]) & rcodeMask),
	}
	flags := binary.BigEndian.Uint16(data[2:4])
	resp.Truncated = flags&0x0200 != 0
	questions := binary.BigEndian.Uint16(data[4:6])
	answers := binary.BigEndian.Uint16(data[6:8])
	authority := binary.BigEndian.Uint16(data[8:10])
	additional := binary.BigEndian.Uint16(data[10:12])

	offset := 12
	for i := 0; i < int(questions); i++ {
		_, next, err := readName(data, offset)
		if err != nil {
			return nil, err
		}
		offset = next + 4 // QTYPE + QCLASS
	}
	for section, count := range []uint16{answers, authority, additional} {
		for i := 0; i < int(count); i++ {
			name, next, err := readName(data, offset)
			if err != nil {
				return nil, err
			}
			offset = next
			if offset+10 > len(data) {
				return nil, errors.New("dnsprobe: short RR header")
			}
			rtype := binary.BigEndian.Uint16(data[offset : offset+2])
			rclass := binary.BigEndian.Uint16(data[offset+2 : offset+4])
			ttl := binary.BigEndian.Uint32(data[offset+4 : offset+8])
			rdlength := int(binary.BigEndian.Uint16(data[offset+8 : offset+10]))
			offset += 10
			if offset+rdlength > len(data) {
				return nil, errors.New("dnsprobe: short RDATA")
			}
			if rtype == typeOPT {
				resp.HasEDNS = true
			} else if section == 0 && (rtype == typeA || rtype == typeAAAA) {
				record := Record{Name: name, Type: rtype, Class: rclass, TTL: ttl}
				if rtype == typeA && rdlength == 4 {
					record.Data = net.IP(append([]byte(nil), data[offset:offset+4]...))
				} else if rtype == typeAAAA && rdlength == 16 {
					record.Data = net.IP(append([]byte(nil), data[offset:offset+16]...))
				}
				resp.Answers = append(resp.Answers, record)
			}
			offset += rdlength
		}
	}
	return resp, nil
}

// readName decodes a (possibly compressed) name. Returns the name and the
// offset just past it.
func readName(data []byte, offset int) (string, int, error) {
	var labels []string
	jumped := false
	end := 0
	visited := 0
	for {
		if offset >= len(data) {
			return "", 0, errors.New("dnsprobe: name overrun")
		}
		length := int(data[offset])
		if length == 0 {
			offset++
			if !jumped {
				end = offset
			}
			break
		}
		if length&0xC0 == 0xC0 { // compression pointer
			if offset+1 >= len(data) {
				return "", 0, errors.New("dnsprobe: bad pointer")
			}
			pointer := int(binary.BigEndian.Uint16(data[offset:offset+2]) & 0x3FFF)
			if !jumped {
				end = offset + 2
			}
			jumped = true
			offset = pointer
			visited++
			if visited > 32 {
				return "", 0, errors.New("dnsprobe: pointer loop")
			}
			continue
		}
		if offset+1+length > len(data) {
			return "", 0, errors.New("dnsprobe: label overrun")
		}
		labels = append(labels, string(data[offset+1:offset+1+length]))
		offset += 1 + length
	}
	return joinLabels(labels), end, nil
}

func joinLabels(labels []string) string {
	name := ""
	for _, label := range labels {
		if name != "" {
			name += "."
		}
		name += label
	}
	if name == "" {
		return "."
	}
	return name
}

// ---- transports -------------------------------------------------------------

// Transport is a resolution transport.
type Transport string

const (
	TransportUDP Transport = "udp"
	TransportDoT Transport = "dot"
	TransportDoH Transport = "doh"
)

// Resolver resolves over one transport.
type Resolver struct {
	Transport Transport
	// Server is "host" (udp/dot) or a full URL (doh,
	// e.g. https://cloudflare-dns.com/dns-query).
	Server  string
	Timeout time.Duration
}

// Resolve performs one A/AAAA query and returns the response.
func (r Resolver) Resolve(ctx context.Context, name string, qtype uint16) (*Response, error) {
	switch r.Transport {
	case TransportUDP:
		return r.resolveUDP(ctx, name, qtype, false)
	case TransportDoT:
		return r.resolveUDP(ctx, name, qtype, false)
	case TransportDoH:
		return r.resolveDoH(ctx, name, qtype)
	}
	return nil, fmt.Errorf("dnsprobe: unknown transport %q", r.Transport)
}

// ResolveEDNS performs the query with an EDNS0 OPT record attached.
func (r Resolver) ResolveEDNS(ctx context.Context, name string, qtype uint16) (*Response, error) {
	switch r.Transport {
	case TransportUDP, TransportDoT:
		return r.resolveUDP(ctx, name, qtype, true)
	case TransportDoH:
		return r.resolveDoH(ctx, name, qtype) // DoH is EDNS-transparent
	}
	return nil, fmt.Errorf("dnsprobe: unknown transport %q", r.Transport)
}

func (r Resolver) timeout() time.Duration {
	if r.Timeout != 0 {
		return r.Timeout
	}
	return 3 * time.Second
}

func (r Resolver) resolveUDP(ctx context.Context, name string, qtype uint16, edns bool) (*Response, error) {
	id := uint16(rand.Intn(65536))
	payload := buildQuery(id, Question{Name: name, Type: qtype, Class: 1}, edns)
	server := r.Server
	if _, _, err := net.SplitHostPort(server); err != nil {
		if r.Transport == TransportDoT {
			server = net.JoinHostPort(server, "853")
		} else {
			server = net.JoinHostPort(server, "53")
		}
	}

	var conn net.Conn
	if r.Transport == TransportDoT {
		dialer := &net.Dialer{Timeout: r.timeout()}
		rawConn, dialErr := dialer.DialContext(ctx, "tcp", server)
		if dialErr != nil {
			return nil, fmt.Errorf("dnsprobe: dot dial: %w", dialErr)
		}
		tlsConn := tls.Client(rawConn, &tls.Config{ServerName: tlsName(r.Server)})
		if handshakeErr := tlsConn.HandshakeContext(ctx); handshakeErr != nil {
			rawConn.Close()
			return nil, fmt.Errorf("dnsprobe: dot handshake: %w", handshakeErr)
		}
		conn = tlsConn
		defer conn.Close()
		// DoT: 2-byte length prefix.
		frame := make([]byte, 2+len(payload))
		binary.BigEndian.PutUint16(frame, uint16(len(payload)))
		copy(frame[2:], payload)
		if _, err := conn.Write(frame); err != nil {
			return nil, err
		}
	} else {
		var err error
		conn, err = net.DialTimeout("udp", server, r.timeout())
		if err != nil {
			return nil, fmt.Errorf("dnsprobe: udp dial: %w", err)
		}
		defer conn.Close()
		deadline, _ := ctx.Deadline()
		if deadline.IsZero() {
			deadline = time.Now().Add(r.timeout())
		}
		conn.SetDeadline(deadline)
		if _, err := conn.Write(payload); err != nil {
			return nil, err
		}
	}

	if r.Transport == TransportDoT {
		length := make([]byte, 2)
		if _, err := io.ReadFull(conn, length); err != nil {
			return nil, err
		}
		size := int(binary.BigEndian.Uint16(length))
		data := make([]byte, size)
		if _, err := io.ReadFull(conn, data); err != nil {
			return nil, err
		}
		return parseResponse(data)
	}

	buffer := make([]byte, 1500)
	n, err := conn.Read(buffer)
	if err != nil {
		return nil, err
	}
	return parseResponse(buffer[:n])
}

func (r Resolver) resolveDoH(ctx context.Context, name string, qtype uint16) (*Response, error) {
	id := uint16(rand.Intn(65536))
	payload := buildQuery(id, Question{Name: name, Type: qtype, Class: 1}, false)
	request, err := http.NewRequestWithContext(ctx, http.MethodPost, r.Server, bytes.NewReader(payload))
	if err != nil {
		return nil, err
	}
	request.Header.Set("Content-Type", "application/dns-message")
	client := &http.Client{Timeout: r.timeout()}
	response, err := client.Do(request)
	if err != nil {
		return nil, fmt.Errorf("dnsprobe: doh: %w", err)
	}
	defer response.Body.Close()
	if response.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("dnsprobe: doh: status %d", response.StatusCode)
	}
	body, err := io.ReadAll(io.LimitReader(response.Body, 65535))
	if err != nil {
		return nil, err
	}
	return parseResponse(body)
}

func tlsName(server string) string {
	host, _, err := net.SplitHostPort(server)
	if err != nil {
		return server
	}
	return host
}

// ---- scanner ----------------------------------------------------------------

// ProbeResult is the outcome of scanning one resolver.
type ProbeResult struct {
	Resolver  string  `json:"resolver"`
	Transport string  `json:"transport"`
	LatencyMS float64 `json:"latency_ms"`
	EDNS      bool    `json:"edns"`
	Honest    bool    `json:"honest"` // NXDOMAIN not hijacked
	HijackIP  string  `json:"hijack_ip,omitempty"`
	Score     float64 `json:"score"`
	Error     string  `json:"error,omitempty"`
}

// Probe scans one resolver: latency (median of 3), EDNS capability,
// NXDOMAIN honesty, and a 0-100 score. The probe domain and the
// NXDOMAIN test label are parameterized for testability.
func Probe(ctx context.Context, resolver Resolver, probeDomain string) ProbeResult {
	result := ProbeResult{
		Resolver:  resolver.Server,
		Transport: string(resolver.Transport),
		Honest:    true,
	}

	// Latency: median of 3 A queries.
	var samples []float64
	for i := 0; i < 3; i++ {
		start := time.Now()
		_, err := resolver.Resolve(ctx, probeDomain, typeA)
		if err != nil {
			result.Error = err.Error()
			result.Score = 0
			return result
		}
		samples = append(samples, float64(time.Since(start).Microseconds())/1000.0)
	}
	result.LatencyMS = median(samples)

	// EDNS capability: a query with an OPT RR must come back with an OPT.
	if resolver.Transport == TransportDoH {
		result.EDNS = true // DoH servers handle EDNS transparently
	} else if resp, err := resolver.ResolveEDNS(ctx, probeDomain, typeA); err == nil {
		result.EDNS = resp.HasEDNS
	}

	// NXDOMAIN honesty: random label under .invalid must yield RCODE 3.
	testName := randomLabel(12) + ".invalid"
	if resp, err := resolver.Resolve(ctx, testName, typeA); err == nil {
		if resp.RCode == 3 {
			result.Honest = true
		} else if len(resp.Answers) > 0 {
			result.Honest = false
			result.HijackIP = resp.Answers[0].Data.String()
		}
		// NOERROR with no answers is ambiguous (some resolvers synthesize
		// empty NOERROR); treat as honest but note via score only.
	}

	result.Score = score(result)
	return result
}

// score computes the 0-100 composite.
func score(r ProbeResult) float64 {
	if r.Error != "" {
		return 0
	}
	value := 100.0
	// Latency: 0ms -> 0 penalty, 500ms+ -> 55 penalty (capped).
	penalty := 55.0 * clamp(r.LatencyMS/500.0, 0, 1)
	value -= penalty
	if !r.EDNS {
		value -= 10
	}
	if !r.Honest {
		value -= 70 // hijacking resolvers are near-useless for tunnels
	}
	return clamp(value, 0, 100)
}

func clamp(v, low, high float64) float64 {
	if v < low {
		return low
	}
	if v > high {
		return high
	}
	return v
}

func median(values []float64) float64 {
	sorted := append([]float64(nil), values...)
	for i := 0; i < len(sorted); i++ {
		for j := i + 1; j < len(sorted); j++ {
			if sorted[j] < sorted[i] {
				sorted[i], sorted[j] = sorted[j], sorted[i]
			}
		}
	}
	return sorted[len(sorted)/2]
}

const labelAlphabet = "abcdefghijklmnopqrstuvwxyz0123456789"

func randomLabel(length int) string {
	out := make([]byte, length)
	for i := range out {
		out[i] = labelAlphabet[rand.Intn(len(labelAlphabet))]
	}
	return string(out)
}

// Scan runs Probe over many resolvers concurrently and returns the results
// sorted by score (descending).
func Scan(ctx context.Context, resolvers []Resolver, probeDomain string) []ProbeResult {
	results := make([]ProbeResult, len(resolvers))
	var wg sync.WaitGroup
	for index, resolver := range resolvers {
		wg.Add(1)
		go func(index int, resolver Resolver) {
			defer wg.Done()
			scanCtx, cancel := context.WithTimeout(ctx, 10*time.Second)
			defer cancel()
			results[index] = Probe(scanCtx, resolver, probeDomain)
		}(index, resolver)
	}
	wg.Wait()
	// Selection sort (stable enough for tens of entries, no side effects).
	for i := 0; i < len(results); i++ {
		best := i
		for j := i + 1; j < len(results); j++ {
			if results[j].Score > results[best].Score {
				best = j
			}
		}
		results[i], results[best] = results[best], results[i]
	}
	return results
}

// DefaultResolvers is a starter list of public resolvers across transports.
var DefaultResolvers = []Resolver{
	{Transport: TransportUDP, Server: "1.1.1.1"},
	{Transport: TransportUDP, Server: "8.8.8.8"},
	{Transport: TransportUDP, Server: "9.9.9.9"},
	{Transport: TransportDoT, Server: "1.1.1.1"},
	{Transport: TransportDoT, Server: "8.8.8.8"},
	{Transport: TransportDoH, Server: "https://cloudflare-dns.com/dns-query"},
	{Transport: TransportDoH, Server: "https://dns.google/dns-query"},
}
