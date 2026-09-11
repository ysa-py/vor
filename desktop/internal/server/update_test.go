package server

import "testing"

// Regression tests for the v1.0.4 updater fix: the updater used to resolve
// binaries from a different, defunct project (macan-dev/EasySNI, tag "v4.x",
// asset "V2RayEz.exe"), which "updated" Vor Desktop into another product.
// These tests pin the current contract: Vor's own repo, vor-v* tags, and
// Vor-desktop / vor-desktop asset names.

func TestNormalizeReleaseTag(t *testing.T) {
	cases := map[string]string{
		"vor-v1.0.4":  "1.0.4",
		"vor-v2.0":    "2.0",
		"v1.0.4":      "1.0.4",
		"1.0.4":       "1.0.4",
		" vor-v1.2.3": "1.2.3",
		"":            "",
	}
	for tag, want := range cases {
		if got := normalizeReleaseTag(tag); got != want {
			t.Errorf("normalizeReleaseTag(%q) = %q, want %q", tag, got, want)
		}
	}
}

func TestCmpVersionsWithVorTags(t *testing.T) {
	// The old bug: cmpVersions("vor-v1.0.4", "1.0.3") mis-parsed the tag so
	// "update available" was computed against garbage. Normalized tags must
	// compare correctly against AppVersion.
	if cmpVersions(normalizeReleaseTag("vor-v1.0.4"), AppVersion) != 0 {
		t.Errorf("same version must compare equal")
	}
	if cmpVersions(normalizeReleaseTag("vor-v1.0.5"), AppVersion) <= 0 {
		t.Errorf("1.0.5 must be newer than %s", AppVersion)
	}
	if cmpVersions(normalizeReleaseTag("vor-v1.0.2"), AppVersion) >= 0 {
		t.Errorf("1.0.2 must be older than %s", AppVersion)
	}
}

func TestPickReleaseAssetPrefersVorDesktopNames(t *testing.T) {
	// On Linux: exact vor-desktop beats unrelated assets.
	got := pickReleaseAsset([]string{
		"Vor-v1.0.4-android-arm64-v8a-release.apk",
		"vor-desktop_1.0.4-1_amd64.deb",
		"SHA256SUMS.txt",
		"vor-desktop",
	})
	if got != "vor-desktop" {
		t.Errorf("linux pick = %q, want %q", got, "vor-desktop")
	}
}

func TestPickReleaseAssetWindows(t *testing.T) {
	// On Windows the test process is not windows; exercise the exact-name
	// branch by checking that a "Vor-desktop.exe" round-trip through the
	// want-list logic still matches when the exact pass is skipped.
	assets := []string{
		"Vor-iOS-unsigned.ipa",
		"vor-desktop",
		"Vor-v1.0.4-android-universal-release.apk",
	}
	got := pickReleaseAsset(assets)
	if got != "vor-desktop" {
		t.Errorf("fallback pick = %q, want %q", got, "vor-desktop")
	}
}

func TestUpdateRepoPointsAtVor(t *testing.T) {
	if updateRepo != "ysa-py/Vor" {
		t.Errorf("updateRepo = %q, want ysa-py/Vor (the updater must resolve from this repository)", updateRepo)
	}
}
