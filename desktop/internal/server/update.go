package server

import (
	"encoding/json"
	"os"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"

	"ezsni/internal/ghdl"
)

// AppVersion is the current Vor Desktop version.
const AppVersion = "1.0.4"

// updateRepo is the official GitHub repository for Vor. Updates are resolved
// from its GitHub releases via github.com (no api.github.com, which is
// censored in some regions). Release tags follow the vor-v<semver> scheme.
const updateRepo = "ysa-py/Vor"

// updateManifestURL is an optional fallback used only if no GitHub release is
// found: a small JSON file {"version","url","notes"} in the repo.
const updateManifestURL = "https://raw.githubusercontent.com/ysa-py/Vor/refs/heads/main/desktop/repo/update.json"

// normalizeReleaseTag maps a release tag ("vor-v1.0.4", "v1.0.4", "1.0.4")
// onto the plain semver the version comparison and UI expect.
func normalizeReleaseTag(tag string) string {
	v := strings.TrimSpace(tag)
	v = strings.TrimPrefix(v, "vor-v")
	v = strings.TrimPrefix(v, "vor-")
	v = strings.TrimPrefix(v, "v")
	return v
}

func (s *Server) handleAppVersion(json.RawMessage) (any, error) {
	return map[string]any{"version": AppVersion, "repo": "https://github.com/" + updateRepo}, nil
}

type updateManifest struct {
	Version string `json:"version"`
	URL     string `json:"url"`
	Notes   string `json:"notes"`
}

// cmpVersions returns 1 if a>b, -1 if a<b, 0 if equal (dotted numeric compare).
func cmpVersions(a, b string) int {
	clean := func(v string) []int {
		v = strings.TrimPrefix(strings.TrimSpace(v), "v")
		parts := strings.FieldsFunc(v, func(r rune) bool { return r == '.' || r == '-' || r == '+' })
		nums := make([]int, 0, len(parts))
		for _, p := range parts {
			n, err := strconv.Atoi(p)
			if err != nil {
				break
			}
			nums = append(nums, n)
		}
		return nums
	}
	an, bn := clean(a), clean(b)
	for i := 0; i < len(an) || i < len(bn); i++ {
		var x, y int
		if i < len(an) {
			x = an[i]
		}
		if i < len(bn) {
			y = bn[i]
		}
		if x > y {
			return 1
		}
		if x < y {
			return -1
		}
	}
	return 0
}

// pickReleaseAsset chooses the best download for this OS/arch from a release's
// asset list (exact-name match first: Vor-desktop.exe / vor-desktop, then
// OS+arch, then a runnable file).
func pickReleaseAsset(assets []string) string {
	goos, arch := runtime.GOOS, runtime.GOARCH
	// Exact names the Vor release pipeline publishes.
	if goos == "windows" {
		for _, a := range assets {
			if strings.EqualFold(a, "Vor-desktop.exe") {
				return a
			}
		}
	} else {
		for _, a := range assets {
			if strings.EqualFold(a, "vor-desktop") {
				return a
			}
		}
	}
	want := []string{goos + "-" + arch, goos, ".exe"}
	if goos == "windows" {
		want = []string{"windows-" + arch, "windows", ".exe", ".zip"}
	}
	low := strings.ToLower
	for _, key := range want {
		for _, a := range assets {
			if strings.Contains(low(a), low(key)) {
				return a
			}
		}
	}
	if len(assets) > 0 {
		return assets[0]
	}
	return ""
}

func (s *Server) handleAppUpdateCheck(body json.RawMessage) (any, error) {
	var req struct {
		Mirror string `json:"mirror"`
	}
	_ = json.Unmarshal(body, &req)
	ghdl.SetMirror(req.Mirror)

	// 1) Official GitHub releases (github.com redirect, no api.github.com).
	if tag, err := ghdl.LatestTag(updateRepo); err == nil && tag != "" {
		assets, _ := ghdl.ListAssets(updateRepo, tag)
		asset := pickReleaseAsset(assets)
		url := ""
		if asset != "" {
			url = ghdl.AssetURL(updateRepo, tag, asset)
		}
		latest := normalizeReleaseTag(tag)
		return map[string]any{
			"ok": true, "current": AppVersion, "latest": latest,
			"update_available": cmpVersions(latest, AppVersion) > 0,
			"url":              url, "asset": asset, "source": "github",
			"notes": "https://github.com/" + updateRepo + "/releases/tag/" + tag,
		}, nil
	}

	// 2) Fallback: update.json manifest.
	data, err := ghdl.Download(updateManifestURL)
	if err != nil {
		return map[string]any{"ok": false, "current": AppVersion, "error": err.Error()}, nil
	}
	var m updateManifest
	if err := json.Unmarshal(data, &m); err != nil {
		return map[string]any{"ok": false, "current": AppVersion, "error": "no GitHub release and invalid manifest"}, nil
	}
	return map[string]any{
		"ok": true, "current": AppVersion, "latest": m.Version,
		"update_available": m.Version != "" && cmpVersions(m.Version, AppVersion) > 0,
		"url":              m.URL, "notes": m.Notes, "source": "manifest",
	}, nil
}

func (s *Server) handleAppUpdateDownload(body json.RawMessage) (any, error) {
	var req struct {
		URL    string `json:"url"`
		Mirror string `json:"mirror"`
	}
	if err := json.Unmarshal(body, &req); err != nil {
		return nil, err
	}
	ghdl.SetMirror(req.Mirror)
	url := strings.TrimSpace(req.URL)
	if url == "" {
		return map[string]any{"ok": false, "error": "no download URL"}, nil
	}
	s.log("Downloading app update…", "ACCENT")
	data, err := ghdl.Download(url)
	if err != nil {
		s.log("✗ update download: "+err.Error(), "ERROR")
		return map[string]any{"ok": false, "error": err.Error()}, nil
	}
	dir := filepath.Join(appDir(), "update")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return map[string]any{"ok": false, "error": err.Error()}, nil
	}
	if strings.HasSuffix(strings.ToLower(url), ".zip") {
		paths, err := ghdl.ExtractZip(data, dir)
		if err != nil {
			return map[string]any{"ok": false, "error": err.Error()}, nil
		}
		bin := ghdl.PickBinary(paths, "Vor-desktop"+exeExt(), "vor")
		s.log("✓ Update extracted to "+dir+" — close Vor Desktop and run the new executable.", "OK")
		return map[string]any{"ok": true, "path": bin, "dir": dir}, nil
	}
	dest := filepath.Join(dir, "Vor-desktop-update"+exeExt())
	if err := os.WriteFile(dest, data, 0o755); err != nil {
		return map[string]any{"ok": false, "error": err.Error()}, nil
	}
	s.log("✓ Update saved to "+dest+" — close Vor Desktop and replace the old executable with it.", "OK")
	return map[string]any{"ok": true, "path": dest}, nil
}

func exeExt() string {
	if filepath.Ext(os.Args[0]) == ".exe" || strings.Contains(strings.ToLower(os.Args[0]), "windows") {
		return ".exe"
	}
	if os.PathSeparator == '\\' {
		return ".exe"
	}
	return ""
}
