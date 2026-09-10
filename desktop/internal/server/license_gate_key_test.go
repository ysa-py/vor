package server

import (
	"os"
	"testing"
)

// The release pipeline embeds the production license public key at link time
// (-X ezsni/internal/server.vorLicensePublicKey=...). These tests pin the
// lookup priority: env override > link-time value > dev default. A regression
// here would ship release binaries that verify tokens against the dev key
// while licenses are issued with the production key.
func TestLicensePublicKeyPriority(t *testing.T) {
	const devKey = "f54nNpWuth1MHZsbi6sdEODSDvWp7V6XSSDqWtmCyMA"
	const linkKey = "iVBsYnvBTIyXcWQprbDU0unxxfUJaK3VT-ngvUTZx_A"
	const envKey = "env-env-env-env"

	t.Setenv("VOR_LICENSE_PUBLIC_KEY", "")
	if got := licensePublicKey(); got != devKey {
		t.Fatalf("default: got %q, want dev key", got)
	}

	old := vorLicensePublicKey
	defer func() { vorLicensePublicKey = old }()
	vorLicensePublicKey = linkKey
	if got := licensePublicKey(); got != linkKey {
		t.Fatalf("link-time: got %q, want %q", got, linkKey)
	}

	t.Setenv("VOR_LICENSE_PUBLIC_KEY", envKey)
	if got := licensePublicKey(); got != envKey {
		t.Fatalf("env must win over link-time: got %q, want %q", got, envKey)
	}
	_ = os.Unsetenv("VOR_LICENSE_PUBLIC_KEY")
}
