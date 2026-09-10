package pattern

import (
	"encoding/binary"
	"testing"
)

// The shared cross-platform fragment vectors
// (core/vor-core/tests/vectors/decision-vectors.json) pin this port to the
// Rust reference. Same ClientHello construction as every port.

func clientHello(t *testing.T) []byte {
	t.Helper()
	name := []byte("example.com")
	ext := make([]byte, 0, 16)
	ext = binary.BigEndian.AppendUint16(ext, uint16(1+2+len(name)))
	ext = append(ext, 0x00)
	ext = binary.BigEndian.AppendUint16(ext, uint16(len(name)))
	ext = append(ext, name...)

	trailing := []byte{0x00, 0x2B, 0x00, 0x02, 0x03, 0x04}
	extTotal := len(ext) + 4 + len(trailing)

	handshake := make([]byte, 0, 64)
	handshake = append(handshake, 0x03, 0x03)
	handshake = append(handshake, make([]byte, 32)...)      // random
	handshake = append(handshake, 0x00)                     // session id len
	handshake = binary.BigEndian.AppendUint16(handshake, 2) // cipher len
	handshake = append(handshake, 0x13, 0x01)
	handshake = append(handshake, 0x01, 0x00) // comp
	handshake = binary.BigEndian.AppendUint16(handshake, uint16(extTotal))
	handshake = binary.BigEndian.AppendUint16(handshake, 0x0000) // server_name
	handshake = binary.BigEndian.AppendUint16(handshake, uint16(len(ext)))
	handshake = append(handshake, ext...)
	handshake = append(handshake, trailing...)

	record := make([]byte, 0, len(handshake)+9)
	record = append(record, 0x16, 0x03, 0x01)
	record = binary.BigEndian.AppendUint16(record, uint16(len(handshake)+4))
	record = append(record, 0x01)
	length := uint32(len(handshake))
	record = append(record, byte(length>>16), byte(length>>8), byte(length))
	record = append(record, handshake...)
	return record
}

func TestFindsSni(t *testing.T) {
	record := clientHello(t)
	if !IsClientHello(record) {
		t.Fatal("not detected as client hello")
	}
	sni := FindSNI(record)
	if sni == nil {
		t.Fatal("SNI not found")
	}
	if string(record[sni.HostnameStart:sni.HostnameStart+sni.HostnameLen]) != "example.com" {
		t.Fatalf("hostname mismatch: %q", record[sni.HostnameStart:sni.HostnameStart+sni.HostnameLen])
	}
}

func TestPlanInvariants(t *testing.T) {
	record := clientHello(t)
	strategies := []string{
		StrategySniSplit, StrategyRecordSplit, StrategyHalf, StrategyFull5,
		StrategyFull10, StrategyFull20, StrategyMulti64, StrategySniBoundary,
		StrategyTlsSniRecords, StrategyTlsRecordFrag, StrategyRaw,
	}
	for _, strategy := range strategies {
		writes := Plan(record, strategy, 5)
		if len(writes) == 0 {
			t.Fatalf("%s: no writes", strategy)
		}
		cursor := 0
		for _, w := range writes {
			if w.Start != cursor {
				t.Fatalf("%s: non-contiguous (start %d, cursor %d)", strategy, w.Start, cursor)
			}
			cursor = w.End
		}
		if cursor != len(record) {
			t.Fatalf("%s: coverage %d != %d", strategy, cursor, len(record))
		}
	}
	// raw and non-hello data: single write
	if writes := Plan(record, StrategyRaw, 5); len(writes) != 1 {
		t.Fatalf("raw: %d writes", len(writes))
	}
	junk := make([]byte, 64)
	if writes := Plan(junk, StrategySniSplit, 5); len(writes) != 1 {
		t.Fatalf("junk: %d writes", len(writes))
	}
}

func TestRecordSplitAtFive(t *testing.T) {
	record := clientHello(t)
	writes := Plan(record, StrategyRecordSplit, 10)
	if writes[0].End != 5 {
		t.Fatalf("record_split first end %d", writes[0].End)
	}
	if writes[1].DelayMs != 10 {
		t.Fatalf("record_split second delay %d", writes[1].DelayMs)
	}
	if !writes[0].More {
		t.Fatal("record_split first write should hint more")
	}
}

func TestFull5Chunks(t *testing.T) {
	record := clientHello(t)
	writes := Plan(record, StrategyFull5, 0)
	for _, w := range writes[:len(writes)-1] {
		if w.End-w.Start != 5 {
			t.Fatalf("full5 chunk size %d", w.End-w.Start)
		}
	}
}

func TestSniSplitBracketsSni(t *testing.T) {
	record := clientHello(t)
	sni := FindSNI(record)
	writes := Plan(record, StrategySniSplit, 0)
	if len(writes) < 3 {
		t.Fatalf("sni_split writes %d", len(writes))
	}
	if writes[0].End != sni.ExtensionStart {
		t.Fatalf("first write end %d != sni start %d", writes[0].End, sni.ExtensionStart)
	}
	foundEnd := false
	for _, w := range writes {
		if w.End == sni.ExtensionEnd {
			foundEnd = true
		}
	}
	if !foundEnd {
		t.Fatalf("no write ends at SNI end %d", sni.ExtensionEnd)
	}
}

func TestWellFormedRecords(t *testing.T) {
	record := clientHello(t)
	pieces := SplitIntoWellFormedRecords(record)
	if len(pieces) != 2 {
		t.Fatalf("pieces %d", len(pieces))
	}
	if len(pieces[0]) != 5 {
		t.Fatalf("prefix length %d", len(pieces[0]))
	}
	// second record must reassemble to the original payload
	rest := pieces[1][5:]
	if string(rest) != string(record[5:]) {
		t.Fatal("well-formed records lost payload bytes")
	}
	// its header must declare the exact remaining length
	if binary.BigEndian.Uint16(pieces[1][3:5]) != uint16(len(rest)) {
		t.Fatal("second record header length mismatch")
	}
}

func TestRandomSplitDeterministic(t *testing.T) {
	record := clientHello(t)
	a := Plan(record, StrategyRandomSplit, 0)
	b := Plan(record, StrategyRandomSplit, 0)
	if len(a) != len(b) {
		t.Fatalf("random plans differ in length")
	}
	for i := range a {
		if a[i] != b[i] {
			t.Fatalf("random plans differ at %d", i)
		}
	}
}
