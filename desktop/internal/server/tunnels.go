package server

import (
	"context"
	"encoding/json"
	"errors"
	"sync"
	"time"

	"ezsni/internal/dnsprobe"
	"ezsni/internal/naive"
	"ezsni/internal/sshchain"
)

// In-process tunnel engines (SlipNet-inspired coverage, implemented from
// scratch — see ENGINES.md): SSH chaining (plain / TLS / WebSocket /
// HTTP-CONNECT wraps, payload injection, cipher selection) and the
// NaiveProxy-style padded HTTPS tunnel. Both surface as local SOCKS5
// listeners and register in the engine selector vocabulary.
//
// All start endpoints are license-gated like every other engine.

var (
	sshMu     sync.Mutex
	sshEngine *sshchain.Engine
	sshConfig sshchain.Config

	naiveMu     sync.Mutex
	naiveEngine *naive.Engine
	naiveConfig naive.Config
)

func (s *Server) handleSSHChainStart(body json.RawMessage) (any, error) {
	if LicenseArmed() {
		return nil, errLicenseRequired
	}
	sshMu.Lock()
	defer sshMu.Unlock()
	if sshEngine != nil && sshEngine.State() != "stopped" {
		return nil, errors.New("ssh-chain already running")
	}
	cfg := sshchain.Defaults()
	if len(body) > 0 {
		if err := json.Unmarshal(body, &cfg); err != nil {
			return nil, err
		}
	}
	engine := sshchain.New(cfg, s.log)
	if err := engine.Start(); err != nil {
		return nil, err
	}
	sshEngine, sshConfig = engine, cfg
	return map[string]any{
		"state":       engine.State(),
		"listen_addr": cfg.ListenAddr,
		"wrap":        cfg.Wrap,
		"engine":      "ssh-chain",
	}, nil
}

func (s *Server) handleSSHChainStop(body json.RawMessage) (any, error) {
	sshMu.Lock()
	defer sshMu.Unlock()
	if sshEngine == nil {
		return map[string]string{"state": "stopped"}, nil
	}
	sshEngine.Stop()
	sshEngine = nil
	return map[string]string{"state": "stopped"}, nil
}

func (s *Server) handleSSHChainStatus(body json.RawMessage) (any, error) {
	sshMu.Lock()
	defer sshMu.Unlock()
	state := "stopped"
	if sshEngine != nil {
		state = sshEngine.State()
	}
	return map[string]any{
		"engine":  "ssh-chain",
		"state":   state,
		"config":  sshConfig,
		"ciphers": sshchain.SupportedCiphers,
	}, nil
}

func (s *Server) handleNaiveStart(body json.RawMessage) (any, error) {
	if LicenseArmed() {
		return nil, errLicenseRequired
	}
	naiveMu.Lock()
	defer naiveMu.Unlock()
	if naiveEngine != nil && naiveEngine.State() != "stopped" {
		return nil, errors.New("naive-https already running")
	}
	cfg := naive.Defaults()
	if len(body) > 0 {
		if err := json.Unmarshal(body, &cfg); err != nil {
			return nil, err
		}
	}
	engine := naive.New(cfg, s.log)
	if err := engine.Start(); err != nil {
		return nil, err
	}
	naiveEngine, naiveConfig = engine, cfg
	return map[string]any{
		"state":       engine.State(),
		"listen_addr": cfg.ListenAddr,
		"proxy_addr":  cfg.ProxyAddr,
		"engine":      "naive-https",
	}, nil
}

func (s *Server) handleNaiveStop(body json.RawMessage) (any, error) {
	naiveMu.Lock()
	defer naiveMu.Unlock()
	if naiveEngine == nil {
		return map[string]string{"state": "stopped"}, nil
	}
	naiveEngine.Stop()
	naiveEngine = nil
	return map[string]string{"state": "stopped"}, nil
}

func (s *Server) handleNaiveStatus(body json.RawMessage) (any, error) {
	naiveMu.Lock()
	defer naiveMu.Unlock()
	state := "stopped"
	if naiveEngine != nil {
		state = naiveEngine.State()
	}
	return map[string]any{
		"engine": "naive-https",
		"state":  state,
		"config": naiveConfig,
	}, nil
}

// handleDNSScan runs the resolver scanner/scorer over the supplied or
// default resolver list. Diagnostics are license-free (they leak nothing
// about engine internals and help users pick a resolver before setup).
func (s *Server) handleDNSScan(body json.RawMessage) (any, error) {
	var request struct {
		Resolvers   []dnsprobe.Resolver `json:"resolvers"`
		ProbeDomain string              `json:"probe_domain"`
		TimeoutMS   int                 `json:"timeout_ms"`
	}
	if len(body) > 0 {
		if err := json.Unmarshal(body, &request); err != nil {
			return nil, err
		}
	}
	resolvers := request.Resolvers
	if len(resolvers) == 0 {
		resolvers = dnsprobe.DefaultResolvers
	}
	probeDomain := request.ProbeDomain
	if probeDomain == "" {
		probeDomain = "example.com"
	}
	timeout := 12 * time.Second
	if request.TimeoutMS > 0 {
		timeout = time.Duration(request.TimeoutMS) * time.Millisecond
	}
	ctx, cancel := context.WithTimeout(context.Background(), timeout)
	defer cancel()
	started := time.Now()
	results := dnsprobe.Scan(ctx, resolvers, probeDomain)
	return map[string]any{
		"results":     results,
		"scanned":     len(resolvers),
		"duration_ms": float64(time.Since(started).Microseconds()) / 1000.0,
	}, nil
}

// handleDNSTransports lists the resolution transports (UDP/DoT/DoH) and the
// default resolver set so the dashboard can present the selection UI.
func (s *Server) handleDNSTransports(body json.RawMessage) (any, error) {
	return map[string]any{
		"transports": []string{"udp", "dot", "doh"},
		"defaults":   dnsprobe.DefaultResolvers,
	}, nil
}
