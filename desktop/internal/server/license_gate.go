package server

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"sync"
	"time"

	"ezsni/internal/engine"
	"ezsni/internal/license"
)

// Vor additions on top of the EasySNI dashboard:
//   - the offline license gate (spec: license/SPEC.md)
//   - the transport-engine selector (Auto / AI or manual) backed by the
//     shared decision model (internal/engine — vector-pinned to
//     core/vor-core).
//
// Gate semantics: when a license gate is ARMED (no valid token), every
// engine-start endpoint is refused and the dashboard shows the gate first.
// Verification is fully offline. Expiry re-locks automatically because the
// stored token is re-verified on each status call.

const licenseSideFile = "vor-license.json"

var (
	licenseMu       sync.Mutex
	licenseToken    string
	licenseVerified bool
)

func licensePublicKey() string {
	if key := os.Getenv("VOR_LICENSE_PUBLIC_KEY"); key != "" {
		return key
	}
	// Dev key (license/keys/dev) — CI release builds embed the production
	// key via -ldflags or the env var above.
	return "f54nNpWuth1MHZsbi6sdEODSDvWp7V6XSSDqWtmCyMA"
}

func initLicenseState() {
	licenseMu.Lock()
	defer licenseMu.Unlock()
	raw, err := readSideFile(licenseSideFile)
	if err != nil {
		return
	}
	var stored struct {
		Token string `json:"token"`
	}
	if json.Unmarshal([]byte(raw), &stored) != nil {
		return
	}
	licenseToken = stored.Token
	licenseVerified = license.Verify(licensePublicKey(), licenseToken, time.Now()).Status == license.Valid
}

// LicenseArmed reports whether the gate currently blocks the engines.
func LicenseArmed() bool {
	licenseMu.Lock()
	defer licenseMu.Unlock()
	if licenseToken == "" {
		return true
	}
	// Re-verify so expiry re-locks without any admin action.
	result := license.Verify(licensePublicKey(), licenseToken, time.Now())
	return result.Status != license.Valid
}

type licenseStatusResponse struct {
	Armed   bool             `json:"armed"`
	Status  string           `json:"status"`
	Payload *license.Payload `json:"payload,omitempty"`
	DevKey  bool             `json:"dev_key"`
}

func (s *Server) handleLicenseStatus(body json.RawMessage) (any, error) {
	licenseMu.Lock()
	token := licenseToken
	licenseMu.Unlock()
	response := licenseStatusResponse{Armed: true, DevKey: licensePublicKey() == "f54nNpWuth1MHZsbi6sdEODSDvWp7V6XSSDqWtmCyMA"}
	if token != "" {
		result := license.Verify(licensePublicKey(), token, time.Now())
		response.Status = string(result.Status)
		response.Payload = result.Payload
		response.Armed = result.Status != license.Valid
	}
	return response, nil
}

func (s *Server) handleLicenseActivate(body json.RawMessage) (any, error) {
	var request struct {
		Token string `json:"token"`
	}
	if err := json.Unmarshal(body, &request); err != nil {
		return nil, err
	}
	result := license.Verify(licensePublicKey(), request.Token, time.Now())
	if result.Status == license.Invalid {
		return map[string]any{
			"status": "INVALID",
			"armed":  true,
		}, nil
	}
	licenseMu.Lock()
	licenseToken = request.Token
	licenseVerified = result.Status == license.Valid
	licenseMu.Unlock()
	if err := writeSideFile(licenseSideFile, []byte(`{"token":`+quoteJSON(request.Token)+`}`)); err != nil {
		s.bus.Log("license: persist failed: "+err.Error(), "warn")
	}
	return map[string]any{
		"status":  string(result.Status),
		"armed":   result.Status != license.Valid,
		"payload": result.Payload,
	}, nil
}

func (s *Server) handleLicenseClear(body json.RawMessage) (any, error) {
	licenseMu.Lock()
	licenseToken = ""
	licenseVerified = false
	licenseMu.Unlock()
	_ = os.Remove(filepath.Join(configsDir(), licenseSideFile))
	return map[string]any{"armed": true}, nil
}

// ---------------------------------------------------------------- engine selector

var (
	engineMu         sync.Mutex
	engineStatePath  = "vor-engine-state.json"
	engineSelection  = "auto" // auto | engine id
	enginesAvailable []string
)

// RegisterEngines declares which engines this build can actually start
// (used by the decision context's available_engines).
func RegisterEngines(engines []string) {
	engineMu.Lock()
	defer engineMu.Unlock()
	enginesAvailable = engines
}

func (s *Server) handleEngineList(body json.RawMessage) (any, error) {
	return map[string]any{
		"engines":   engine.KnownEngines,
		"selection": engineSelection,
		"available": enginesAvailable,
	}, nil
}

func (s *Server) handleEngineSelect(body json.RawMessage) (any, error) {
	var request struct {
		Engine string `json:"engine"`
	}
	if err := json.Unmarshal(body, &request); err != nil {
		return nil, err
	}
	if request.Engine != "auto" && !containsEngine(engine.KnownEngines, request.Engine) {
		return nil, fmt.Errorf("unknown engine %q", request.Engine)
	}
	engineMu.Lock()
	engineSelection = request.Engine
	engineMu.Unlock()
	if raw, err := readSideFile(engineStatePath); err == nil {
		var state map[string]any
		if json.Unmarshal(raw, &state) == nil {
			state["selection"] = request.Engine
			if encoded, err := json.Marshal(state); err == nil {
				_ = writeSideFile(engineStatePath, encoded)
			}
		}
	} else {
		encoded, _ := json.Marshal(map[string]string{"selection": request.Engine})
		_ = writeSideFile(engineStatePath, encoded)
	}
	return map[string]string{"selection": request.Engine}, nil
}

func (s *Server) handleEngineDecide(body json.RawMessage) (any, error) {
	if LicenseArmed() {
		return nil, errLicenseRequired
	}
	raw, _ := readSideFile(engineStatePath)
	state := engine.ParseSelectorState(string(raw))
	if state == nil {
		state = engine.NewSelectorState()
	}
	var context engine.DecisionContext
	if len(body) > 0 {
		if err := json.Unmarshal(body, &context); err != nil {
			return nil, err
		}
	}
	engineMu.Lock()
	context.AvailableEngines = enginesAvailable
	selection := engineSelection
	engineMu.Unlock()
	if selection != "auto" && containsEngine(engine.KnownEngines, selection) {
		context.AvailableEngines = []string{selection}
		context.PinnedEngines = []string{selection}
	}
	return engine.Decide(state, &context), nil
}

func (s *Server) handleEngineObserve(body json.RawMessage) (any, error) {
	var outcome engine.Outcome
	if err := json.Unmarshal(body, &outcome); err != nil {
		return nil, err
	}
	raw, _ := readSideFile(engineStatePath)
	state := engine.ParseSelectorState(string(raw))
	if state == nil {
		state = engine.NewSelectorState()
	}
	engine.Observe(state, &outcome, time.Now().Unix())
	if encoded, err := json.Marshal(state); err == nil {
		_ = writeSideFile(engineStatePath, encoded)
	}
	return map[string]string{"ok": "observed"}, nil
}

func containsEngine(engines []string, id string) bool {
	for _, engineID := range engines {
		if engineID == id {
			return true
		}
	}
	return false
}

func quoteJSON(text string) string {
	encoded, _ := json.Marshal(text)
	return string(encoded)
}

// errLicenseRequired gates engine-start endpoints.
var errLicenseRequired = fmt.Errorf("license required")

// defaultAvailableEngines lists the engines this build can actually start.
func defaultAvailableEngines() []string {
	engines := []string{"xray", "singbox", "sni-tunnel", "tor", "mitm-fronting", "dns-tunnel-dnstt", "dns-tunnel-masterdns", "psiphon"}
	if runtime.GOOS == "windows" {
		engines = append(engines, "pattern")
	}
	return engines
}
