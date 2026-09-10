// Package pattern is the Vor port of the UAC-SNI-Spoofer-Windows engines:
// the Patterniha wrong-sequence injection technique plus the UAC fragment
// strategy matrix and carrier tuning, re-implemented in Go so the desktop
// ships ONE static binary (vs the 150–250 MB Python runtime of upstream —
// see docs/ENGINES.md for the tradeoff analysis).
//
// Technique provenance (patterniha/SNI-Spoofing, GPL-3.0 upstream, source
// preserved at extras/uac-sni-spoofer-windows/third_party/): after the
// TCP handshake completes, inject a forged packet carrying a fake-SNI
// ClientHello at seq = syn_seq + 1 - len(fake). The DPI middlebox parses
// the fake SNI while the real server discards the out-of-window segment;
// success is confirmed when the server ACKs exactly past the fake bytes,
// then the real ClientHello flows normally. Any anomaly falls back to
// pure fragmentation.
package pattern

import (
	"encoding/binary"
)

// Strategy names — the union taxonomy shared with vor-core and the UAC
// fragmenters (see core/vor-core/src/fragment.rs).
const (
	StrategySniSplit      = "sni_split"
	StrategyRecordSplit   = "record_split"
	StrategyRandomSplit   = "random_split"
	StrategyFull5         = "full5"
	StrategyFull10        = "full10"
	StrategyFull20        = "full20"
	StrategyMulti64       = "multi64"
	StrategySniBoundary   = "sni_boundary"
	StrategyTlsSniRecords = "tls_sni_records"
	StrategyTlsRecordFrag = "tls_record_frag"
	StrategyHalf          = "half"
	StrategyRaw           = "raw"
)

// TLS record constants.
const (
	tlsContentHandshake     = 0x16
	tlsHandshakeClientHello = 0x01
	tlsExtServerName        = 0x0000
)

// SniLocation marks the server_name extension inside a ClientHello record.
type SniLocation struct {
	ExtensionStart int
	ExtensionEnd   int
	HostnameStart  int
	HostnameLen    int
}

// IsClientHello reports whether the buffer looks like a TLS ClientHello
// handshake record (same heuristic as every port).
func IsClientHello(data []byte) bool {
	return len(data) >= 9 &&
		data[0] == tlsContentHandshake &&
		data[1] == 0x03 &&
		data[5] == tlsHandshakeClientHello
}

// FindSNI locates the server_name extension (mirrors vor-core find_sni).
func FindSNI(data []byte) *SniLocation {
	if !IsClientHello(data) {
		return nil
	}
	pos := 5 + 4
	pos += 2 + 32
	sessionLen := int(data[pos])
	pos += 1 + sessionLen
	if pos+2 > len(data) {
		return nil
	}
	cipherLen := int(binary.BigEndian.Uint16(data[pos : pos+2]))
	pos += 2 + cipherLen
	if pos >= len(data) {
		return nil
	}
	compLen := int(data[pos])
	pos += 1 + compLen
	if pos+2 > len(data) {
		return nil
	}
	extensionsLen := int(binary.BigEndian.Uint16(data[pos : pos+2]))
	pos += 2
	extensionsEnd := min(pos+extensionsLen, len(data))

	for pos+4 <= extensionsEnd {
		extType := binary.BigEndian.Uint16(data[pos : pos+2])
		extLen := int(binary.BigEndian.Uint16(data[pos+2 : pos+4]))
		extStart := pos
		extEnd := min(pos+4+extLen, extensionsEnd)
		if extType == tlsExtServerName {
			p := extStart + 4 + 2 // skip the list length
			if p < extEnd && data[p] == 0x00 {
				p++
				if p+2 <= extEnd {
					nameLen := int(binary.BigEndian.Uint16(data[p : p+2]))
					p += 2
					hostLen := min(nameLen, extEnd-p)
					if hostLen < 0 {
						hostLen = 0
					}
					return &SniLocation{
						ExtensionStart: extStart,
						ExtensionEnd:   extEnd,
						HostnameStart:  p,
						HostnameLen:    hostLen,
					}
				}
			}
			return &SniLocation{
				ExtensionStart: extStart,
				ExtensionEnd:   extEnd,
				HostnameStart:  extStart + 4,
				HostnameLen:    0,
			}
		}
		pos = extEnd
	}
	return nil
}

// Write is one planned byte-range write (mirrors vor-core PlannedWrite).
type Write struct {
	Start   int
	End     int
	DelayMs int
	More    bool
}

// Plan computes the write boundaries for a strategy (mirrors vor-core
// fragment.rs plan() — pinned by the shared conformance vectors).
func Plan(data []byte, strategy string, interWriteDelayMs int) []Write {
	single := func() []Write {
		return []Write{{Start: 0, End: len(data), DelayMs: 0, More: false}}
	}
	if !IsClientHello(data) || len(data) < 10 || strategy == StrategyRaw {
		return single()
	}
	sni := FindSNI(data)
	mid := len(data) / 2
	if sni == nil {
		mid = len(data) / 2
	}

	var points []int
	switch strategy {
	case StrategyHalf:
		points = []int{len(data) / 2}
	case StrategyRecordSplit, StrategyTlsRecordFrag:
		points = []int{5}
	case StrategyFull5:
		points = chunkPoints(len(data), 5)
	case StrategyFull10:
		points = chunkPoints(len(data), 10)
	case StrategyFull20:
		points = chunkPoints(len(data), 20)
	case StrategyMulti64:
		points = chunkPoints(len(data), 64)
	case StrategySniBoundary:
		boundary := mid
		if sni != nil {
			boundary = sni.ExtensionStart
		}
		points = []int{boundary}
	case StrategySniSplit, StrategyTlsSniRecords:
		start, end := mid, mid
		if sni != nil {
			start, end = sni.ExtensionStart, sni.ExtensionEnd
		}
		points = []int{start, end}
	case StrategyRandomSplit:
		sniEnd := mid
		if sni != nil {
			sniEnd = sni.ExtensionEnd
		}
		points = pseudoRandomSplits(data, 2, 4)
		filtered := points[:0]
		for _, p := range points {
			if p < sniEnd {
				filtered = append(filtered, p)
			}
		}
		points = append(filtered, sniEnd)
	default:
		return single()
	}

	points = sortedUnique(points)
	writes := make([]Write, 0, len(points)+1)
	prev := 0
	for _, point := range points {
		if point <= prev || point >= len(data) {
			continue
		}
		delay := 0
		if prev != 0 {
			delay = interWriteDelayMs
		}
		writes = append(writes, Write{Start: prev, End: point, DelayMs: delay, More: true})
		prev = point
	}
	if prev < len(data) {
		delay := 0
		if prev != 0 {
			delay = interWriteDelayMs
		}
		writes = append(writes, Write{Start: prev, End: len(data), DelayMs: delay, More: false})
	}
	if len(writes) <= 1 {
		return single()
	}
	return writes
}

// SplitIntoWellFormedRecords is the external-finalmask record rewrite (UAC
// Edge Bridge semantics: a 5-byte prefix record, then the remainder
// re-framed with a fresh record header of the same type/version).
func SplitIntoWellFormedRecords(data []byte) [][]byte {
	if len(data) <= 5 {
		return [][]byte{data}
	}
	restLen := len(data) - 5
	secondHeader := []byte{
		data[0], data[1], data[2],
		byte(restLen >> 8), byte(restLen),
	}
	second := append(secondHeader, data[5:]...)
	return [][]byte{data[0:5], second}
}

// ---------------------------------------------------------------- helpers

func chunkPoints(length, chunk int) []int {
	var points []int
	for p := chunk; p < length; p += chunk {
		points = append(points, p)
	}
	return points
}

// pseudoRandomSplits derives deterministic split points (xorshift over the
// record bytes — identical to the Rust/Kotlin references).
func pseudoRandomSplits(data []byte, minValue, maxValue int) []int {
	seed := uint32(0x9E3779B9)
	for _, b := range data {
		seed = (seed<<5 | seed>>27) ^ uint32(b)
	}
	if seed == 0 {
		seed = 0x12345678
	}
	count := minValue + int(seed)%max(1, maxValue-minValue+1)
	points := make([]int, 0, count)
	value := seed
	for i := 0; i < count; i++ {
		value ^= value << 13
		value ^= value >> 17
		value ^= value << 5
		point := 1 + int(uint64(uint32(value))%uint64(len(data)-1))
		points = append(points, point)
	}
	return sortedUnique(points)
}

func sortedUnique(points []int) []int {
	if len(points) == 0 {
		return points
	}
	// insertion sort (inputs are tiny)
	for i := 1; i < len(points); i++ {
		for j := i; j > 0 && points[j] < points[j-1]; j-- {
			points[j], points[j-1] = points[j-1], points[j]
		}
	}
	unique := points[:1]
	for _, p := range points[1:] {
		if p != unique[len(unique)-1] {
			unique = append(unique, p)
		}
	}
	return unique
}

func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}

func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}
