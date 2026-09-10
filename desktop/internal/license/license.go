// Package license implements offline Ed25519 license-token verification
// for Vor Desktop (see license/SPEC.md).
//
// This is the Go port of the shared reference implementation
// (core/vor-core/src/license.rs, license/python/vor_license.py,
// android/core-license). All ports are pinned to the same conformance
// vectors (license/vectors.json) so every platform provably agrees.
package license

import (
	"crypto/ed25519"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"strings"
	"time"
)

// Status is the verification outcome.
type Status string

// Verification outcomes (wire names shared by every platform).
const (
	Valid   Status = "VALID"
	Expired Status = "EXPIRED"
	Invalid Status = "INVALID"
)

// Payload is the decoded license payload.
type Payload struct {
	Version      int            `json:"v"`
	ID           string         `json:"id"`
	Product      string         `json:"product"`
	IssuedAt     string         `json:"issued_at"`
	ExpiresAt    string         `json:"expires_at"`
	Entitlements map[string]any `json:"entitlements,omitempty"`
	Platforms    []string       `json:"-"`
}

// Result carries the status plus the payload when decodable.
type Result struct {
	Status  Status
	Payload *Payload
}

// B64URL is unpadded base64url (RFC 4648 §5).
func B64URL(data []byte) string {
	return base64.RawURLEncoding.EncodeToString(data)
}

// Verify checks a token against a base64url Ed25519 public key at time now.
// It never touches the network.
func Verify(publicKeyB64, token string, now time.Time) Result {
	invalid := Result{Status: Invalid, Payload: nil}

	keyBytes, err := base64.RawURLEncoding.DecodeString(strings.TrimSpace(publicKeyB64))
	if err != nil || len(keyBytes) != ed25519.PublicKeySize {
		return invalid
	}
	publicKey := ed25519.PublicKey(keyBytes)

	parts := strings.Split(strings.TrimSpace(token), ".")
	if len(parts) != 3 || parts[0] != "VOR1" {
		return invalid
	}
	payloadBytes, err := base64.RawURLEncoding.DecodeString(parts[1])
	if err != nil {
		return invalid
	}
	signature, err := base64.RawURLEncoding.DecodeString(parts[2])
	if err != nil || len(signature) != ed25519.SignatureSize {
		return invalid
	}

	if !ed25519.Verify(publicKey, payloadBytes, signature) {
		return invalid
	}

	var payload Payload
	if err := json.Unmarshal(payloadBytes, &payload); err != nil {
		return invalid
	}
	if payload.Version != 1 || payload.Product != "vor" ||
		payload.ID == "" || payload.IssuedAt == "" || payload.ExpiresAt == "" {
		return Result{Status: Invalid, Payload: &payload}
	}
	if platforms, ok := payload.Entitlements["platforms"].([]any); ok {
		for _, platform := range platforms {
			if text, ok := platform.(string); ok {
				payload.Platforms = append(payload.Platforms, text)
			}
		}
	}

	expires, ok := ParseRFC3339(payload.ExpiresAt)
	if !ok {
		return Result{Status: Invalid, Payload: &payload}
	}
	if !now.Before(expires) {
		return Result{Status: Expired, Payload: &payload}
	}
	return Result{Status: Valid, Payload: &payload}
}

// ParseRFC3339 parses the issuer's timestamp forms ("2027-01-01T00:00:00Z",
// fractional seconds, ±hh:mm offsets) to time.Time in UTC.
func ParseRFC3339(text string) (time.Time, bool) {
	text = strings.TrimSpace(text)
	// Normalize a lowercase trailing 'z' (some tools emit it).
	if strings.HasSuffix(text, "z") {
		text = text[:len(text)-1] + "Z"
	}
	parsed, err := time.Parse(time.RFC3339, text)
	if err != nil {
		// Tolerate a missing seconds field ("2027-01-01T00:00Z").
		parsed, err = time.Parse("2006-01-02T15:04Z07:00", text)
		if err != nil {
			return time.Time{}, false
		}
	}
	return parsed.UTC(), true
}

// FormatRFC3339 renders a UTC time the canonical issuer form.
func FormatRFC3339(t time.Time) string {
	return t.UTC().Format("2006-01-02T15:04:05Z")
}

// ErrNoLicense is returned by the gate when no valid token is installed.
var ErrNoLicense = fmt.Errorf("license: no valid license installed")
