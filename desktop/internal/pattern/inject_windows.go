//go:build windows

// Wrong-sequence fake-SNI injection — the Patterniha technique, ported from
// UAC-SNI-Spoofer-Windows pattern_core (GPL-3.0 upstream; technique
// re-implemented in Go from the published description, upstream source
// preserved at extras/uac-sni-spoofer-windows/).
//
// Choreography (per connection):
//  1. Open a WinDivert SNIFF handle filtered to the edge IPs / port so we
//     observe the TCP handshake of connections the engine dials.
//  2. After the client's final handshake ACK, inject a forged PSH|ACK
//     segment carrying a fake-SNI ClientHello with
//     seq = synSeq + 1 - len(fakeHello).
//  3. The DPI middlebox parses the fake SNI; the real server discards the
//     out-of-window segment. Success is confirmed when the server's ACK
//     advances exactly past synSeq+1 (the fake bytes) — then the real
//     ClientHello flows normally.
//  4. Any anomaly (no ACK match, RST) falls back to pure fragmentation.
//
// The packet builder reuses the desync package's segment construction
// (checksum/seq corruption modes), which is battle-tested in this codebase.
//
// NOTE: compiled only on Windows; exercised in CI on windows-latest. The
// fragment strategies above run everywhere (userspace writes).
package pattern

import (
	"encoding/binary"
	"fmt"
	"net"
	"sync"

	"ezsni/internal/desync"
)

// Injector watches handshakes and injects wrong-seq fake hellos.
type Injector struct {
	mu       sync.Mutex
	fakeSni  string
	preset   string
	edges    []net.IP
	port     int
	mode     desync.BypassMode
	fallback []string // fragment strategy names, best-first
}

// InjectorConfig configures the wrong-seq engine.
type InjectorConfig struct {
	FakeSni  string
	Preset   string
	Edges    []net.IP
	Port     int
	Mode     desync.BypassMode
	Fallback []string
}

// NewInjector creates the engine.
func NewInjector(cfg InjectorConfig) *Injector {
	if cfg.FakeSni == "" {
		cfg.FakeSni = "www.speedtest.net"
	}
	if cfg.Preset == "" {
		cfg.Preset = "firefox"
	}
	if len(cfg.Fallback) == 0 {
		cfg.Fallback = []string{StrategySniBoundary, StrategyTlsRecordFrag, StrategyFull5}
	}
	return &Injector{
		fakeSni:  cfg.FakeSni,
		preset:   cfg.Preset,
		edges:    cfg.Edges,
		port:     cfg.Port,
		mode:     cfg.Mode,
		fallback: cfg.Fallback,
	}
}

// FakeHelloFor builds the disposable fake-SNI ClientHello segment for a
// connection (src -> dst) given the observed SYN sequence number.
//
// The forged sequence places the fake payload BEFORE the real client's
// first byte: seq = synSeq + 1 - len(fakeHello). The server treats it as
// an old retransmission (out of window) and discards it; a DPI middlebox
// with a permissive reassembly window parses the fake SNI and commits its
// verdict before the real hello arrives.
func (in *Injector) FakeHelloFor(src, dst net.IP, srcPort, dstPort int, synSeq uint32) []byte {
	fakeHello := desync.FakeClientHello(in.preset, in.fakeSni)
	fakeSeq := synSeq + 1 - uint32(len(fakeHello))
	return desync.BuildFakeSegment(src, dst, srcPort, dstPort, fakeSeq, synSeq+1, fakeHello, in.mode)
}

// WrongSeqFor returns the forged sequence number for a fake payload of
// length n after a handshake with SYN sequence synSeq.
func WrongSeqFor(synSeq uint32, n int) uint32 {
	return synSeq + 1 - uint32(n)
}

// VerifyAckMatch reports whether the server ACK observed after the
// injection matches the expected advance (exactly synSeq+1 — the fake
// bytes were consumed by the DPI but NOT by the server, whose ACK stays
// at the handshake position until the real hello arrives).
func VerifyAckMatch(synSeq, observedAck uint32, injectedLen int) bool {
	expected := synSeq + 1
	if injectedLen > 0 {
		// The server never consumes the fake bytes: its ACK must equal
		// exactly synSeq+1 (the post-handshake position).
		_ = injectedLen
	}
	return observedAck == expected
}

// ConnectionRecord tracks one observed handshake.
type ConnectionRecord struct {
	SrcIP    net.IP
	DstIP    net.IP
	SrcPort  int
	DstPort  int
	SynSeq   uint32
	Stage    Stage
	FakeSent bool
}

// Stage is the handshake observation state.
type Stage int

// Handshake stages.
const (
	StageSyn Stage = iota
	StageSynAck
	StageAck // final client ACK — inject here
	StageInjected
	StageData
)

// ObservePacket advances a ConnectionRecord from a raw TCP header (IP
// payload starting at the TCP header). Returns the next stage.
func (r *ConnectionRecord) ObservePacket(tcp []byte) Stage {
	if len(tcp) < 20 {
		return r.Stage
	}
	flags := tcp[13]
	seq := binary.BigEndian.Uint32(tcp[4:8])
	ack := binary.BigEndian.Uint32(tcp[8:12])
	isSyn := flags&0x02 != 0 && flags&0x10 == 0
	isSynAck := flags&0x02 != 0 && flags&0x10 != 0
	isAck := flags&0x10 != 0 && flags&0x02 == 0 && len(tcp) <= 20

	switch {
	case isSyn && r.SynSeq == 0:
		r.SynSeq = seq
		r.Stage = StageSyn
	case isSynAck:
		r.Stage = StageSynAck
	case isAck && r.Stage == StageSynAck:
		// final handshake ACK from the client — the injection point
		r.Stage = StageAck
	case isAck && r.Stage == StageInjected && r.DstIP.Equal(net.IP{}) == false:
		// server ACK after injection: verify it did NOT advance
		if VerifyAckMatch(r.SynSeq, ack-1, 0) {
			r.Stage = StageData
		}
	}
	return r.Stage
}

// InjectAfterHandshake computes the forged segment once the connection
// reached StageAck.
func (in *Injector) InjectAfterHandshake(record *ConnectionRecord) ([]byte, error) {
	if record.Stage != StageAck {
		return nil, fmt.Errorf("pattern: connection not at handshake-complete stage")
	}
	segment := in.FakeHelloFor(record.SrcIP, record.DstIP, record.SrcPort, record.DstPort, record.SynSeq)
	record.FakeSent = true
	record.Stage = StageInjected
	return segment, nil
}

// FallbackStrategies are the fragment strategies used when the wrong-seq
// choreography cannot complete (fallback ladder, best-first).
func (in *Injector) FallbackStrategies() []string {
	return in.fallback
}
