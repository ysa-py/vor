package server

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"sync"
)

// MasterDns engine adapter — runs the imported MasterDnsVPN Go client
// (repo root: dns-tunnel/, built by CI as `vor-dns-client`) as a process
// engine, mapping Vor's unified engine-selection settings onto the client's
// TOML config. This keeps the DNS-tunnel mode of the merged config schema
// runnable end-to-end from the desktop dashboard (the dnstt addon remains
// selectable too — both engines preserved per the merge spec).
//
// The client exposes a local SOCKS5 listener (default 127.0.0.1:18000) that
// the dashboard's other tools can point at, exactly like the xray runner's
// SOCKS inbound.

var (
	masterDnsMu      sync.Mutex
	masterDnsCmd     *exec.Cmd
	masterDnsRunning bool
	masterDnsPort    int
)

// MasterDnsOptions maps the Vor settings onto the client config.
type MasterDnsOptions struct {
	Domains       []string `json:"domains"`
	EncryptionKey string   `json:"encryption_key"`
	ListenPort    int      `json:"listen_port"`
	Resolvers     string   `json:"resolvers"`
	Binary        string   `json:"binary"`
}

// masterDnsConfigTOML renders the client TOML (schema: dns-tunnel's
// client_config.toml.simple; every key documented there).
func masterDnsConfigTOML(opts MasterDnsOptions) string {
	port := opts.ListenPort
	if port == 0 {
		port = 18000
	}
	var domains []string
	for _, domain := range opts.Domains {
		if strings.TrimSpace(domain) != "" {
			domains = append(domains, fmt.Sprintf("%q", strings.TrimSpace(domain)))
		}
	}
	if len(domains) == 0 {
		domains = []string{`"tunnel.example.com"`}
	}
	resolvers := strings.TrimSpace(opts.Resolvers)
	if resolvers == "" {
		resolvers = "8.8.8.8"
	}
	key := strings.TrimSpace(opts.EncryptionKey)
	if key == "" {
		key = "vor-change-me"
	}
	return fmt.Sprintf(`# Vor — MasterDns engine config (generated)
DOMAINS = [%s]
DATA_ENCRYPTION_METHOD = 4
ENCRYPTION_KEY = %q

[protocol]
PROTOCOL_TYPE = "SOCKS5"
LISTEN_IP = "127.0.0.1"
LISTEN_PORT = %d

[resolver]
RESOLVER_BALANCING_STRATEGY = 5
PACKET_DUPLICATION_COUNT = 3

[compression]
UPLOAD_COMPRESSION_TYPE = 1
DOWNLOAD_COMPRESSION_TYPE = 1

[log]
LOG_LEVEL = "info"
`, strings.Join(domains, ", "), key, port)
}

// StartMasterDns launches the DNS-tunnel client with the given options.
func (s *Server) StartMasterDns(body json.RawMessage) (any, error) {
	if LicenseArmed() {
		return nil, errLicenseRequired
	}
	var opts MasterDnsOptions
	if len(body) > 0 {
		if err := json.Unmarshal(body, &opts); err != nil {
			return nil, err
		}
	}
	binaryPath := masterDnsBinaryPath(opts.Binary)
	if binaryPath == "" {
		return nil, fmt.Errorf("masterdns binary not found — build dns-tunnel/ or set binary path")
	}
	masterDnsMu.Lock()
	defer masterDnsMu.Unlock()
	masterDnsStopLocked()

	configPath := filepath.Join(configsDir(), "vor-masterdns.toml")
	if err := writeSideFile("vor-masterdns.toml", []byte(masterDnsConfigTOML(opts))); err != nil {
		return nil, err
	}
	resolvers := strings.TrimSpace(opts.Resolvers)
	if resolvers == "" {
		resolvers = "8.8.8.8"
	}
	if err := writeSideFile("vor-masterdns-resolvers.txt", []byte(resolvers+"\n")); err != nil {
		return nil, err
	}

	cmd := exec.Command(binaryPath, "-config", configPath, "-resolvers", filepath.Join(configsDir(), "vor-masterdns-resolvers.txt"))
	cmd.Dir = configsDir()
	cmd.Stderr = os.Stderr
	if err := cmd.Start(); err != nil {
		return nil, err
	}
	masterDnsCmd = cmd
	masterDnsRunning = true
	port := opts.ListenPort
	if port == 0 {
		port = 18000
	}
	masterDnsPort = port
	go func() { _ = cmd.Wait() }()
	s.bus.Log("masterdns engine started (socks 127.0.0.1:"+fmt.Sprint(port)+")", "info")
	return map[string]any{"running": true, "socks": fmt.Sprintf("127.0.0.1:%d", port)}, nil
}

// StopMasterDns terminates the engine.
func (s *Server) StopMasterDns(body json.RawMessage) (any, error) {
	masterDnsMu.Lock()
	defer masterDnsMu.Unlock()
	masterDnsStopLocked()
	return map[string]any{"running": false}, nil
}

// StatusMasterDns reports the engine state.
func (s *Server) StatusMasterDns(body json.RawMessage) (any, error) {
	masterDnsMu.Lock()
	defer masterDnsMu.Unlock()
	return map[string]any{
		"running": masterDnsRunning,
		"socks":   fmt.Sprintf("127.0.0.1:%d", masterDnsPort),
		"engine":  "dns-tunnel-masterdns",
	}, nil
}

func masterDnsStopLocked() {
	if masterDnsCmd != nil && masterDnsCmd.Process != nil {
		_ = masterDnsCmd.Process.Kill()
	}
	masterDnsCmd = nil
	masterDnsRunning = false
}

// masterDnsBinaryPath locates the client binary: explicit path, next to the
// exe, or in the dns-tunnel build output of the monorepo checkout.
func masterDnsBinaryPath(explicit string) string {
	candidates := func() []string {
		list := []string{}
		if strings.TrimSpace(explicit) != "" {
			list = append(list, strings.TrimSpace(explicit))
		}
		if exe, err := os.Executable(); err == nil {
			dir := filepath.Dir(exe)
			suffix := ""
			if runtime.GOOS == "windows" {
				suffix = ".exe"
			}
			list = append(list,
				filepath.Join(dir, "vor-dns-client"+suffix),
				filepath.Join(dir, "masterdnsvpn-client"+suffix),
			)
		}
		return list
	}
	for _, path := range candidates() {
		if st, err := os.Stat(path); err == nil && !st.IsDir() {
			return path
		}
	}
	return ""
}
