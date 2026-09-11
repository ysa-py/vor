package server

import "testing"

// Regression tests for the v1.0.4 updater fix: the updater used to resolve
// binaries from a different, defunct project (macan-dev/EasySNI, tag "v4.x",
// asset "V2RayEz.exe"), which "updated" Vor Desktop into another product.
// These tests pin the current contract: Vor's own repo, vor-v* tags, and
// Vor-desktop / vor-desktop asset names. All tests are OS-parameterized via
// pickReleaseAssetFor so they behave identically on Linux, macOS and Windows
// runners (the first CI run failed on the Windows runner exactly because the
// original tests depended on runtime.GOOS).

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

func TestPickReleaseAssetLinuxPrefersVorDesktop(t *testing.T) {
	got := pickReleaseAssetFor("linux", "amd64", []string{
		"Vor-v1.0.4-android-arm64-v8a-release.apk",
		"vor-desktop_1.0.4-1_amd64.deb",
		"SHA256SUMS.txt",
		"vor-desktop",
	})
	if got != "vor-desktop" {
		t.Errorf("linux pick = %q, want %q", got, "vor-desktop")
	}
}

func TestPickReleaseAssetWindowsPrefersVorDesktopExe(t *testing.T) {
	got := pickReleaseAssetFor("windows", "amd64", []string{
		"Vor-v1.0.4-android-universal-release.apk",
		"Vor-iOS-unsigned.ipa",
		"Vor-desktop.exe",
		"vor-desktop_1.0.4-1_amd64.deb",
		"SHA256SUMS.txt",
	})
	if got != "Vor-desktop.exe" {
		t.Errorf("windows pick = %q, want %q", got, "Vor-desktop.exe")
	}
}

func TestPickReleaseAssetWindowsFallbacks(t *testing.T) {
	// No exact-name asset: the windows want-list (windows-amd64 / windows /
	// .exe / .zip) must pick the archived windows binary, never an APK/ipa.
	got := pickReleaseAssetFor("windows", "amd64", []string{
		"Vor-v1.0.4-android-universal-release.apk",
		"Vor-iOS-unsigned.ipa",
		"Vor-v1.0.4-windows-amd64.zip",
	})
	if got != "Vor-v1.0.4-windows-amd64.zip" {
		t.Errorf("windows fallback pick = %q, want the windows zip", got)
	}
}

func TestPickReleaseAssetLinuxFallbackPrefersPlatformName(t *testing.T) {
	got := pickReleaseAssetFor("linux", "arm64", []string{
		"Vor-v1.0.4-android-universal-release.apk",
		"Vor-v1.0.4-linux-arm64.tar.gz",
	})
	if got != "Vor-v1.0.4-linux-arm64.tar.gz" {
		t.Errorf("linux arm64 fallback pick = %q, want the linux-arm64 archive", got)
	}
}

func TestUpdateRepoPointsAtVor(t *testing.T) {
	if updateRepo != "ysa-py/Vor" {
		t.Errorf("updateRepo = %q, want ysa-py/Vor (the updater must resolve from this repository)", updateRepo)
	}
}
