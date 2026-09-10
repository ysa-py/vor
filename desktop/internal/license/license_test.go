package license

import (
	"encoding/json"
	"os"
	"strings"
	"testing"
	"time"
)

// The exact same license/vectors.json is consumed by the Rust reference,
// the Kotlin ports (main app + License Manager) and the Python reference —
// every platform provably agrees on accept/reject decisions.
type vectorsFile struct {
	Spec struct {
		Now     string `json:"now"`
		Product string `json:"product"`
	} `json:"spec"`
	PublicKey string `json:"public_key"`
	Cases     []struct {
		Name     string          `json:"name"`
		Token    string          `json:"token"`
		Expected Status          `json:"expected"`
		Note     json.RawMessage `json:"note"`
	} `json:"cases"`
}

func loadVectors(t *testing.T) (*vectorsFile, string) {
	t.Helper()
	raw, err := os.ReadFile("testdata/license-vectors.json")
	if err != nil {
		// Also try the canonical repo-root location (go test runs in the
		// package dir; walking up keeps this robust from any depth).
		raw, err = os.ReadFile("../../../license/vectors.json")
		if err != nil {
			t.Fatalf("vectors file: %v", err)
		}
	}
	var vectors vectorsFile
	if err := json.Unmarshal(raw, &vectors); err != nil {
		t.Fatalf("vectors json: %v", err)
	}
	key, err := os.ReadFile("testdata/VOR_LICENSE_PUBLIC_KEY.txt")
	if err != nil {
		key, err = os.ReadFile("../../../license/keys/dev/VOR_LICENSE_PUBLIC_KEY.txt")
		if err != nil {
			t.Fatalf("dev public key: %v", err)
		}
	}
	return &vectors, strings.TrimSpace(string(key))
}

func TestSharedVectors(t *testing.T) {
	vectors, key := loadVectors(t)
	if key == "" {
		t.Fatal("empty dev key")
	}
	now, ok := ParseRFC3339(vectors.Spec.Now)
	if !ok {
		t.Fatalf("vector now %q unparseable", vectors.Spec.Now)
	}
	if vectors.PublicKey != key {
		t.Fatalf("vector key mismatch: %q vs %q", vectors.PublicKey, key)
	}
	for _, testCase := range vectors.Cases {
		result := Verify(key, testCase.Token, now)
		if result.Status != testCase.Expected {
			t.Errorf("case %s: got %s, want %s", testCase.Name, result.Status, testCase.Expected)
		}
	}
}

func TestParseRFC3339ReferenceConstants(t *testing.T) {
	cases := []struct {
		text string
		want int64
	}{
		{"1970-01-01T00:00:00Z", 0},
		{"2024-01-01T00:00:00Z", 1704067200},
		{"2027-01-01T00:00:00Z", 1798761600},
		{"2026-09-10T12:00:00Z", 1789041600},
		{"2026-09-10T12:00:00+02:00", 1789034400},
		{"2026-09-10T12:00:00.123Z", 1789041600},
	}
	for _, testCase := range cases {
		parsed, ok := ParseRFC3339(testCase.text)
		if !ok {
			t.Errorf("%s: parse failed", testCase.text)
			continue
		}
		if got := parsed.Unix(); got != testCase.want {
			t.Errorf("%s: got %d, want %d", testCase.text, got, testCase.want)
		}
	}
	if _, ok := ParseRFC3339("garbage"); ok {
		t.Error("garbage parsed")
	}
	if _, ok := ParseRFC3339("2026-13-01T00:00:00Z"); ok {
		t.Error("month 13 parsed")
	}
}

func TestRejectsMalformed(t *testing.T) {
	_, key := loadVectors(t)
	zero := time.Unix(0, 0).UTC()
	for _, token := range []string{
		"",
		"VOR1.only-two",
		"VORX.a.b",
		"VOR1.!!not-base64!!.AAAA",
		"VOR1.eyJ2IjoxfQ.AAAA",
	} {
		if result := Verify(key, token, zero); result.Status != Invalid {
			t.Errorf("token %q: got %s, want INVALID", token, result.Status)
		}
	}
}
