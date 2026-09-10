// MICAFP-UnifiedShield-6.0 — Go Bridge for Yggdrasil c-archive
//
// This Go program bridges the Yggdrasil mesh network into the Rust daemon
// via a C-archive interface. It exports C functions that the Rust daemon
// calls to manage the Yggdrasil node, and it provides a SOCKS5 proxy
// interface for tunneling traffic through the mesh.
//
// Build as c-archive:
//   CGO_ENABLED=1 go build -buildmode=c-archive -o libshield_bridge.a .
//
// Or as standalone binary:
//   go build -o shield-bridge .

package main

/*
#include <stdint.h>
#include <stdlib.h>

// Forward declarations for exported functions
extern void StartNode(const char* configJSON, int configLen);
extern void StopNode();
extern char* GetAddress();
extern int SendPacket(const char* dest, int destLen, const char* data, int dataLen);
extern void FreeCString(char* s);
*/
import "C"

import (
	"encoding/json"
	"fmt"
	"log"
	"net"
	"os"
	"os/signal"
	"strconv"
	"strings"
	"sync"
	"syscall"
	"time"
	"unsafe"

	"github.com/yggdrasil-network/yggdrasil-go/src/core"
	"github.com/yggdrasil-network/yggdrasil-go/src/multicast"
	"github.com/yggdrasil-network/yggdrasil-go/src/tuntap"
)

// ---------------------------------------------------------------------------
// Global state
// ---------------------------------------------------------------------------

var (
	node        *core.Core
	multicastIF *multicast.Multicast
	tap         *tuntap.Tuntap
	socks5Ln    net.Listener
	mu          sync.Mutex
	running     bool
	nodeAddr    string
	peers       map[string]PeerInfo
)

// PeerInfo tracks a Yggdrasil peer.
type PeerInfo struct {
	Address   string    `json:"address"`
	PublicKey string    `json:"public_key"`
	Connected bool      `json:"connected"`
	LastSeen  time.Time `json:"last_seen"`
}

// NodeConfig mirrors the Yggdrasil configuration.
type NodeConfig struct {
	Listen         []string `json:"listen"`
	Peers          []string `json:"peers"`
	PrivateKey     string   `json:"private_key,omitempty"`
	IfName         string   `json:"if_name"`
	IfMTU          int      `json:"if_mtu"`
	Encryption     string   `json:"encryption"`
	MulticastListen bool    `json:"multicast_listen"`
}

// ---------------------------------------------------------------------------
// Yggdrasil node management
// ---------------------------------------------------------------------------

// StartNode initializes and starts the Yggdrasil node.
//
//export StartNode
func StartNode(configJSON *C.char, configLen C.int) {
	mu.Lock()
	defer mu.Unlock()

	if running {
		log.Println("[Bridge] Node already running")
		return
	}

	// Parse configuration
	cfgBytes := C.GoBytes(unsafe.Pointer(configJSON), configLen)
	var cfg NodeConfig
	if err := json.Unmarshal(cfgBytes, &cfg); err != nil {
		log.Printf("[Bridge] Config parse error: %v", err)
		return
	}

	// Set defaults
	if len(cfg.Listen) == 0 {
		cfg.Listen = []string{"tcp://0.0.0.0:0"}
	}
	if cfg.IfName == "" {
		cfg.IfName = "tun0"
	}
	if cfg.IfMTU == 0 {
		cfg.IfMTU = 65535
	}

	// Create Yggdrasil node state
	state := &core.Configuration{}
	state.Listen = cfg.Listen
	state.Peers = cfg.Peers
	state.IfName = cfg.IfName
	state.IfMTU = cfg.IfMTU

	// Initialize the core
	var err error
	node, err = core.New(state)
	if err != nil {
		log.Printf("[Bridge] Failed to create Yggdrasil node: %v", err)
		return
	}

	// Start the node
	if err := node.Start(); err != nil {
		log.Printf("[Bridge] Failed to start Yggdrasil node: %v", err)
		return
	}

	// Get our address
	nodeAddr = node.Address().String()
	log.Printf("[Bridge] Yggdrasil node started, address: %s", nodeAddr)

	// Set up multicast discovery if enabled
	if cfg.MulticastListen {
		multicastIF = &multicast.Multicast{Core: node}
		if err := multicastIF.Start(); err != nil {
			log.Printf("[Bridge] Multicast start failed: %v", err)
		}
	}

	// Initialize peer tracking
	peers = make(map[string]PeerInfo)
	for _, peer := range cfg.Peers {
		peers[peer] = PeerInfo{
			Address:   peer,
			Connected: false,
			LastSeen:  time.Now(),
		}
	}

	// Start SOCKS5 proxy
	go startSOCKS5Proxy("127.0.0.1:1080")

	running = true
	log.Println("[Bridge] Node started successfully")
}

// StopNode shuts down the Yggdrasil node.
//
//export StopNode
func StopNode() {
	mu.Lock()
	defer mu.Unlock()

	if !running {
		return
	}

	// Stop SOCKS5 proxy
	if socks5Ln != nil {
		socks5Ln.Close()
	}

	// Stop multicast
	if multicastIF != nil {
		multicastIF.Stop()
	}

	// Stop the node
	if node != nil {
		node.Stop()
	}

	running = false
	nodeAddr = ""
	log.Println("[Bridge] Node stopped")
}

// GetAddress returns the Yggdrasil IPv6 address of the running node.
// The caller must free the returned C string with FreeCString.
//
//export GetAddress
func GetAddress() *C.char {
	mu.Lock()
	defer mu.Unlock()

	if !running || nodeAddr == "" {
		return C.CString("")
	}
	return C.CString(nodeAddr)
}

// SendPacket sends a data packet to a destination address through
// the Yggdrasil mesh. Returns 0 on success, -1 on error.
//
//export SendPacket
func SendPacket(dest *C.char, destLen C.int, data *C.char, dataLen C.int) C.int {
	mu.Lock()
	defer mu.Unlock()

	if !running || node == nil {
		return -1
	}

	destStr := C.GoStringN(dest, destLen)
	dataBytes := C.GoBytes(unsafe.Pointer(data), dataLen)

	// Send via Yggdrasil
	if err := node.SendTo(destStr, dataBytes); err != nil {
		log.Printf("[Bridge] SendPacket error: %v", err)
		return -1
	}

	return 0
}

// FreeCString frees a C string allocated by Go.
//
//export FreeCString
func FreeCString(s *C.char) {
	C.free(unsafe.Pointer(s))
}

// ---------------------------------------------------------------------------
// SOCKS5 proxy server
// ---------------------------------------------------------------------------

func startSOCKS5Proxy(addr string) {
	var err error
	socks5Ln, err = net.Listen("tcp", addr)
	if err != nil {
		log.Printf("[Bridge] SOCKS5 listen error: %v", err)
		return
	}
	defer socks5Ln.Close()

	log.Printf("[Bridge] SOCKS5 proxy listening on %s", addr)

	for {
		conn, err := socks5Ln.Accept()
		if err != nil {
			if !running {
				return
			}
			log.Printf("[Bridge] SOCKS5 accept error: %v", err)
			continue
		}
		go handleSOCKS5Connection(conn)
	}
}

func handleSOCKS5Connection(conn net.Conn) {
	defer conn.Close()

	// SOCKS5 handshake
	buf := make([]byte, 262)

	// Read version and auth methods
	n, err := conn.Read(buf)
	if err != nil || n < 2 {
		return
	}

	if buf[0] != 0x05 { // SOCKS version 5
		return
	}

	// No auth required
	conn.Write([]byte{0x05, 0x00})

	// Read connect request
	n, err = conn.Read(buf)
	if err != nil || n < 7 {
		return
	}

	if buf[0] != 0x05 || buf[1] != 0x01 { // CONNECT command
		conn.Write([]byte{0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0}) // Command not supported
		return
	}

	var targetAddr string
	switch buf[3] {
	case 0x01: // IPv4
		if n < 10 {
			return
		}
		targetAddr = net.IPv4(buf[4], buf[5], buf[6], buf[7]).String()
	case 0x03: // Domain name
		domainLen := int(buf[4])
		if n < 5+domainLen {
			return
		}
		targetAddr = string(buf[5 : 5+domainLen])
	case 0x04: // IPv6
		if n < 22 {
			return
		}
		targetAddr = net.IP(buf[4:20]).String()
	default:
		conn.Write([]byte{0x05, 0x08, 0x00, 0x01, 0, 0, 0, 0, 0, 0}) // Address type not supported
		return
	}

	port := int(buf[n-2])<<8 | int(buf[n-1])
	target := net.JoinHostPort(targetAddr, strconv.Itoa(port))

	// Connect through Yggdrasil
	var remote net.Conn
	if node != nil {
		remote, err = node.Dial(targetAddr, strconv.Itoa(port))
	}
	if err != nil || remote == nil {
		// Fallback to direct connection
		remote, err = net.DialTimeout("tcp", target, 10*time.Second)
		if err != nil {
			conn.Write([]byte{0x05, 0x05, 0x00, 0x01, 0, 0, 0, 0, 0, 0}) // Connection refused
			return
		}
	}
	defer remote.Close()

	// Send success response
	conn.Write([]byte{0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0})

	// Bidirectional copy
	go relay(conn, remote)
	relay(remote, conn)
}

func relay(dst, src net.Conn) {
	buf := make([]byte, 32*1024)
	for {
		n, err := src.Read(buf)
		if n > 0 {
			if _, err := dst.Write(buf[:n]); err != nil {
				return
			}
		}
		if err != nil {
			return
		}
	}
}

// ---------------------------------------------------------------------------
// Peer management
// ---------------------------------------------------------------------------

// AddPeer connects to a new Yggdrasil peer.
func AddPeer(peerAddr string) error {
	if node == nil {
		return fmt.Errorf("node not running")
	}
	return node.AddPeer(peerAddr, "")
}

// RemovePeer disconnects from a Yggdrasil peer.
func RemovePeer(peerAddr string) error {
	if node == nil {
		return fmt.Errorf("node not running")
	}
	return node.RemovePeer(peerAddr, "")
}

// GetPeers returns the list of connected peers.
func GetPeers() []PeerInfo {
	mu.Lock()
	defer mu.Unlock()

	result := make([]PeerInfo, 0, len(peers))
	for _, p := range peers {
		result = append(result, p)
	}
	return result
}

// ---------------------------------------------------------------------------
// Standalone main (when not built as c-archive)
// ---------------------------------------------------------------------------

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintf(os.Stderr, "Usage: %s <config.json>\n", os.Args[0])
		fmt.Fprintf(os.Stderr, "       %s --generate-config\n", os.Args[0])
		os.Exit(1)
	}

	if os.Args[1] == "--generate-config" {
		cfg := NodeConfig{
			Listen:         []string{"tcp://0.0.0.0:0"},
			Peers:          []string{},
			IfName:         "tun0",
			IfMTU:          65535,
			Encryption:     "nacl",
			MulticastListen: true,
		}
		data, _ := json.MarshalIndent(cfg, "", "  ")
		fmt.Println(string(data))
		return
	}

	// Read config file
	configData, err := os.ReadFile(os.Args[1])
	if err != nil {
		log.Fatalf("Failed to read config: %v", err)
	}

	// Start node
	cConfig := C.CString(string(configData))
	defer C.free(unsafe.Pointer(cConfig))
	StartNode(cConfig, C.int(len(configData)))

	// Wait for signal
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM, syscall.SIGHUP)

	for {
		select {
		case sig := <-sigCh:
			if sig == syscall.SIGHUP {
				// Reload configuration
				log.Println("[Bridge] Received SIGHUP, reloading config...")
				StopNode()
				StartNode(cConfig, C.int(len(configData)))
			} else {
				log.Printf("[Bridge] Received %v, shutting down...", sig)
				StopNode()
				return
			}
		case <-time.After(30 * time.Second):
			// Periodic peer status update
			if running && node != nil {
				peerCount := len(node.GetPeers())
				log.Printf("[Bridge] Status: connected, %d peers", peerCount)

				// Write status file for LuCI
				status := fmt.Sprintf("connected=true\naddress=%s\npeers=%d\nuptime=%d\n",
					nodeAddr, peerCount, time.Now().Unix())
				_ = os.WriteFile("/var/run/shield.status", []byte(status), 0644)
			}
		}
	}
}

// Ensure unused imports are referenced
var _ = strings.TrimSpace
