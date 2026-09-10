// go-bridge/yggdrasil-mobile/main.go
// ==============================================================================
// Yggdrasil Mobile Bridge for MICAFP-UnifiedShield
// ==============================================================================
// Builds as a C-archive (static library + header) for linking into
// Android (via JNI), iOS (via Swift/ObjC bridge), and Linux (via FFI).
// Provides functions to start/stop a Yggdrasil node, expose a SOCKS5 proxy,
// query peer count, check international reachability, and receive state
// change callbacks.
//
// Build targets:
//   android/arm64:  CGO_ENABLED=1 GOOS=android GOARCH=arm64 go build -buildmode=c-archive
//   android/arm:    CGO_ENABLED=1 GOOS=android GOARCH=arm go build -buildmode=c-archive
//   android/amd64:  CGO_ENABLED=1 GOOS=android GOARCH=amd64 go build -buildmode=c-archive
//   ios/arm64:      CGO_ENABLED=1 GOOS=ios GOARCH=arm64 go build -buildmode=c-archive
//   linux/amd64:    CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -buildmode=c-archive
//   linux/arm64:    CGO_ENABLED=0 GOOS=linux GOARCH=arm64 go build -buildmode=c-archive

package main

/*
#cgo CFLAGS: -Wall -Wextra
#include <stdint.h>
#include <stdlib.h>

// State change callback type: called when Yggdrasil node state changes
// state: 0=stopped, 1=starting, 2=running, 3=error
typedef void (*StateCallback)(int state, const char* message);

// Exported C functions (defined in Go below)
extern int YggdrasilStart(const char* config_json);
extern int YggdrasilStop();
extern int YggdrasilRestart();
extern int YggdrasilGetStatus();
extern int YggdrasilGetPeerCount();
extern int YggdrasilCheckReachability();
extern char* YggdrasilGetConfig();
extern int YggdrasilSetConfig(const char* config_json);
extern char* YggdrasilGetStats();
extern void YggdrasilSetStateCallback(StateCallback cb);
extern void YggdrasilFreeString(char* s);
*/
import "C"

import (
	"encoding/json"
	"fmt"
	"net"
	"os"
	"os/signal"
	"strconv"
	"sync"
	"syscall"
	"time"
	"unsafe"
)

// ==============================================================================
// Types and Constants
// ==============================================================================

// NodeState represents the current state of the Yggdrasil node
type NodeState int

const (
	StateStopped  NodeState = 0
	StateStarting NodeState = 1
	StateRunning  NodeState = 2
	StateError    NodeState = 3
)

// NodeConfig holds the Yggdrasil configuration passed from the mobile app
type NodeConfig struct {
	ListenAddresses    []string `json:"listen_addresses"`
	Peers              []string `json:"peers"`
	InterfacePeers     map[string][]string `json:"interface_peers"`
	AdminListen        string   `json:"admin_listen"`
	MulticastInterfaces []string `json:"multicast_interfaces"`
	PrivateKey         string   `json:"private_key"`
	IfName             string   `json:"if_name"`
	IfMTU              int      `json:"if_mtu"`
	SessionFirewall    SessionFirewallConfig `json:"session_firewall"`
	TunnelRouting      TunnelRoutingConfig   `json:"tunnel_routing"`
	SOCKS5Port         int      `json:"socks5_port"`
	SOCKS5Listen       string   `json:"socks5_listen"`
	AutoStartPeers     bool     `json:"auto_start_peers"`
	ReachabilityTarget string   `json:"reachability_target"`
}

// SessionFirewallConfig controls which sessions are allowed
type SessionFirewallConfig struct {
	Enable                 bool     `json:"enable"`
	AllowFromDirect        bool     `json:"allow_from_direct"`
	AllowFromRemote        bool     `json:"allow_from_remote"`
	AlwaysAllowOutbound    bool     `json:"always_allow_outbound"`
	WhitelistEncryptionPublicKeys []string `json:"whitelist_encryption_public_keys"`
	BlacklistEncryptionPublicKeys []string `json:"blacklist_encryption_public_keys"`
}

// TunnelRoutingConfig for routing specific subnets through Yggdrasil
type TunnelRoutingConfig struct {
	Enable bool `json:"enable"`
}

// NodeStats holds runtime statistics from the Yggdrasil node
type NodeStats struct {
	PeersConnected   int       `json:"peers_connected"`
	BytesSent        int64     `json:"bytes_sent"`
	BytesReceived    int64     `json:"bytes_received"`
	UptimeSeconds    int64     `json:"uptime_seconds"`
	IsReachable      bool      `json:"is_reachable"`
	LocalAddress     string    `json:"local_address"`
	SubnetAddress    string    `json:"subnet_address"`
	LastPeerUpdate   time.Time `json:"last_peer_update"`
	SOCKS5Active     bool      `json:"socks5_active"`
	SOCKS5Port       int       `json:"socks5_port"`
}

// ==============================================================================
// Global State (thread-safe via mutex)
// ==============================================================================

var (
	nodeMutex       sync.RWMutex
	currentState    NodeState    = StateStopped
	currentConfig   *NodeConfig  = nil
	currentStats    *NodeStats   = &NodeStats{}
	startTime       time.Time
	socks5Listener  net.Listener
	stopChan        chan struct{} = make(chan struct{})
	stateCallback   C.StateCallback
	peerConnections map[string]net.Conn = make(map[string]net.Conn)
	errorMessage    string = ""
)

// ==============================================================================
// State Management
// ==============================================================================

func setState(state NodeState, message string) {
	nodeMutex.Lock()
	defer nodeMutex.Unlock()

	currentState = state
	errorMessage = message

	// Invoke the C callback if registered
	if stateCallback != nil {
		cMsg := C.CString(message)
		defer C.free(unsafe.Pointer(cMsg))
		C.stateCallback(C.int(state), cMsg)
	}
}

func getState() NodeState {
	nodeMutex.RLock()
	defer nodeMutex.RUnlock()
	return currentState
}

// ==============================================================================
// SOCKS5 Proxy Server
// ==============================================================================

// startSOCKS5 starts a minimal SOCKS5 proxy that routes traffic through Yggdrasil
func startSOCKS5(listenAddr string) error {
	var err error
	socks5Listener, err = net.Listen("tcp", listenAddr)
	if err != nil {
		return fmt.Errorf("SOCKS5 listen failed: %w", err)
	}

	go func() {
		for {
			conn, err := socks5Listener.Accept()
			if err != nil {
				select {
				case <-stopChan:
					return
				default:
					continue
				}
			}
			go handleSOCKS5Connection(conn)
		}
	}()

	return nil
}

// handleSOCKS5Connection handles a single SOCKS5 client connection
func handleSOCKS5Connection(clientConn net.Conn) {
	defer clientConn.Close()

	// SOCKS5 greeting
	buf := make([]byte, 256)
	n, err := clientConn.Read(buf)
	if err != nil || n < 2 {
		return
	}

	// Verify SOCKS version
	if buf[0] != 0x05 {
		return
	}

	// No auth required
	clientConn.Write([]byte{0x05, 0x00})

	// Read connect request
	n, err = clientConn.Read(buf)
	if err != nil || n < 7 {
		return
	}

	if buf[0] != 0x05 || buf[1] != 0x01 {
		// Only support CONNECT command
		clientConn.Write([]byte{0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0})
		return
	}

	var targetAddr string
	var targetPort int

	switch buf[3] {
	case 0x01: // IPv4
		if n < 10 {
			return
		}
		targetAddr = net.IPv4(buf[4], buf[5], buf[6], buf[7]).String()
		targetPort = int(buf[8])<<8 | int(buf[9])
	case 0x03: // Domain name
		domainLen := int(buf[4])
		if n < 5+domainLen+2 {
			return
		}
		targetAddr = string(buf[5 : 5+domainLen])
		targetPort = int(buf[5+domainLen])<<8 | int(buf[5+domainLen+1])
	case 0x04: // IPv6
		if n < 22 {
			return
		}
		targetAddr = net.IP(buf[4:20]).String()
		targetPort = int(buf[20])<<8 | int(buf[21])
	default:
		clientConn.Write([]byte{0x05, 0x08, 0x00, 0x01, 0, 0, 0, 0, 0, 0})
		return
	}

	// Connect to target (through Yggdrasil tunnel in production)
	targetConn, err := net.DialTimeout("tcp",
		net.JoinHostPort(targetAddr, strconv.Itoa(targetPort)),
		10*time.Second,
	)
	if err != nil {
		clientConn.Write([]byte{0x05, 0x05, 0x00, 0x01, 0, 0, 0, 0, 0, 0})
		return
	}
	defer targetConn.Close()

	// Success response
	clientConn.Write([]byte{0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0})

	// Bidirectional relay
	go relay(clientConn, targetConn)
	relay(targetConn, clientConn)
}

// relay copies data between two connections and updates stats
func relay(dst, src net.Conn) {
	buf := make([]byte, 32*1024)
	for {
		n, err := src.Read(buf)
		if err != nil {
			return
		}
		_, err = dst.Write(buf[:n])
		if err != nil {
			return
		}

		// Update statistics
		nodeMutex.Lock()
		currentStats.BytesSent += int64(n)
		nodeMutex.Unlock()
	}
}

// ==============================================================================
// Reachability Check
// ==============================================================================

// checkInternationalReachability tests if the node can reach international endpoints
func checkInternationalReachability() bool {
	testEndpoints := []string{
		"1.1.1.1:443",    // Cloudflare DNS
		"8.8.8.8:443",    // Google DNS
		"208.67.222.222:443", // OpenDNS
	}

	for _, endpoint := range testEndpoints {
		conn, err := net.DialTimeout("tcp", endpoint, 5*time.Second)
		if err != nil {
			continue
		}
		conn.Close()
		return true
	}

	// Check via DNS resolution
	_, err := net.LookupHost("www.google.com")
	if err == nil {
		return true
	}

	return false
}

// ==============================================================================
// Simulated Yggdrasil Node Operations
// (In production, these would call the actual Yggdrasil library)
// ==============================================================================

func simulateYggdrasilStart(config *NodeConfig) error {
	// In a real implementation, this would:
	// 1. Parse the Yggdrasil configuration
	// 2. Initialize the Yggdrasil core
	// 3. Set up the TUN interface
	// 4. Connect to configured peers
	// 5. Start the SOCKS5 proxy

	setState(StateStarting, "Initializing Yggdrasil node...")

	// Simulate connection to peers
	for i, peer := range config.Peers {
		conn, err := net.DialTimeout("tcp", peer, 5*time.Second)
		if err == nil {
			nodeMutex.Lock()
			peerConnections[peer] = conn
			currentStats.PeersConnected++
			nodeMutex.Unlock()
		}
		setState(StateStarting, fmt.Sprintf("Connecting to peers (%d/%d)...", i+1, len(config.Peers)))
	}

	// Start SOCKS5 proxy
	socks5Addr := config.SOCKS5Listen
	if socks5Addr == "" {
		socks5Addr = "127.0.0.1:" + strconv.Itoa(config.SOCKS5Port)
	}
	if config.SOCKS5Port > 0 {
		if err := startSOCKS5(socks5Addr); err != nil {
			setState(StateError, "SOCKS5 start failed: "+err.Error())
			return err
		}
		nodeMutex.Lock()
		currentStats.SOCKS5Active = true
		currentStats.SOCKS5Port = config.SOCKS5Port
		nodeMutex.Unlock()
	}

	// Check reachability
	reachable := checkInternationalReachability()
	nodeMutex.Lock()
	currentStats.IsReachable = reachable
	startTime = time.Now()
	currentStats.LocalAddress = "200::1"  // Simulated Yggdrasil address
	currentStats.SubnetAddress = "300::/64"
	nodeMutex.Unlock()

	setState(StateRunning, "Yggdrasil node running")
	return nil
}

func simulateYggdrasilStop() {
	// Stop SOCKS5 listener
	if socks5Listener != nil {
		socks5Listener.Close()
	}

	// Close all peer connections
	nodeMutex.Lock()
	for addr, conn := range peerConnections {
		conn.Close()
		delete(peerConnections, addr)
	}
	currentStats = &NodeStats{}
	nodeMutex.Unlock()

	setState(StateStopped, "Yggdrasil node stopped")
}

// ==============================================================================
// Exported C Functions
// ==============================================================================

//export YggdrasilStart
func YggdrasilStart(configJSON *C.char) C.int {
	if getState() == StateRunning {
		return C.int(1) // Already running
	}

	// Parse configuration
	goConfig := C.GoString(configJSON)
	var config NodeConfig
	if err := json.Unmarshal([]byte(goConfig), &config); err != nil {
		setState(StateError, "Config parse error: "+err.Error())
		return C.int(-1)
	}

	// Validate minimum config
	if config.SOCKS5Port == 0 {
		config.SOCKS5Port = 1080 // Default SOCKS5 port
	}
	if len(config.ListenAddresses) == 0 {
		config.ListenAddresses = []string{"tcp://0.0.0.0:0"}
	}

	nodeMutex.Lock()
	currentConfig = &config
	nodeMutex.Unlock()

	// Start node in a goroutine to avoid blocking the C caller
	go func() {
		if err := simulateYggdrasilStart(&config); err != nil {
			return
		}

		// Wait for stop signal
		sigChan := make(chan os.Signal, 1)
		signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)

		select {
		case <-stopChan:
		case <-sigChan:
		}

		simulateYggdrasilStop()
	}()

	// Brief wait for state transition
	for i := 0; i < 50; i++ {
		if getState() == StateRunning {
			return C.int(0) // Success
		}
		if getState() == StateError {
			return C.int(-2) // Start failed
		}
		time.Sleep(100 * time.Millisecond)
	}

	return C.int(0)
}

//export YggdrasilStop
func YggdrasilStop() C.int {
	if getState() != StateRunning {
		return C.int(1) // Not running
	}

	select {
	case stopChan <- struct{}{}:
	default:
	}

	// Wait for state transition
	for i := 0; i < 30; i++ {
		if getState() == StateStopped {
			return C.int(0)
		}
		time.Sleep(100 * time.Millisecond)
	}

	return C.int(0)
}

//export YggdrasilRestart
func YggdrasilRestart() C.int {
	YggdrasilStop()
	time.Sleep(500 * time.Millisecond)

	nodeMutex.RLock()
	config := currentConfig
	nodeMutex.RUnlock()

	if config != nil {
		configJSON, _ := json.Marshal(config)
		return YggdrasilStart(C.CString(string(configJSON)))
	}
	return C.int(-1)
}

//export YggdrasilGetStatus
func YggdrasilGetStatus() C.int {
	return C.int(getState())
}

//export YggdrasilGetPeerCount
func YggdrasilGetPeerCount() C.int {
	nodeMutex.RLock()
	defer nodeMutex.RUnlock()
	return C.int(currentStats.PeersConnected)
}

//export YggdrasilCheckReachability
func YggdrasilCheckReachability() C.int {
	if checkInternationalReachability() {
		return C.int(1)
	}
	return C.int(0)
}

//export YggdrasilGetConfig
func YggdrasilGetConfig() *C.char {
	nodeMutex.RLock()
	defer nodeMutex.RUnlock()

	if currentConfig == nil {
		return C.CString("{}")
	}

	configJSON, err := json.Marshal(currentConfig)
	if err != nil {
		return C.CString("{}")
	}
	return C.CString(string(configJSON))
}

//export YggdrasilSetConfig
func YggdrasilSetConfig(configJSON *C.char) C.int {
	goConfig := C.GoString(configJSON)
	var config NodeConfig
	if err := json.Unmarshal([]byte(goConfig), &config); err != nil {
		return C.int(-1)
	}

	nodeMutex.Lock()
	currentConfig = &config
	nodeMutex.Unlock()

	// If running, restart with new config
	if getState() == StateRunning {
		return YggdrasilRestart()
	}
	return C.int(0)
}

//export YggdrasilGetStats
func YggdrasilGetStats() *C.char {
	nodeMutex.RLock()
	defer nodeMutex.RUnlock()

	// Update uptime
	if getState() == StateRunning && !startTime.IsZero() {
		currentStats.UptimeSeconds = int64(time.Since(startTime).Seconds())
	}

	statsJSON, err := json.Marshal(currentStats)
	if err != nil {
		return C.CString("{}")
	}
	return C.CString(string(statsJSON))
}

//export YggdrasilSetStateCallback
func YggdrasilSetStateCallback(cb C.StateCallback) {
	nodeMutex.Lock()
	defer nodeMutex.Unlock()
	stateCallback = cb
}

//export YggdrasilFreeString
func YggdrasilFreeString(s *C.char) {
	C.free(unsafe.Pointer(s))
}

// ==============================================================================
// Main (required for c-archive, but not called)
// ==============================================================================

func main() {
	// When built as c-archive, main() is not executed.
	// This is only used for standalone testing.
	fmt.Println("MICAFP-UnifiedShield Yggdrasil Mobile Bridge v6.0.0")
	fmt.Println("This binary is designed to be built as a C-archive.")
	fmt.Println("Use -buildmode=c-archive when building.")
}
