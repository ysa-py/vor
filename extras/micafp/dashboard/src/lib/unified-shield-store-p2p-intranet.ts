import { create } from "zustand";
import type {
  CoreState,
  ConnectionStats,
  ThreatIntel,
  P2PPeer,
  IntranetMode,
} from "./unified-shield-types-extended";

interface UnifiedShieldState {
  // Connection
  isConnected: boolean;
  isConnecting: boolean;
  activeCore: string;
  connectionStats: ConnectionStats;

  // Cores
  cores: CoreState[];

  // P2P
  p2pPeers: P2PPeer[];
  p2pEnabled: boolean;

  // Intranet
  intranetMode: IntranetMode;

  // Security
  threatIntel: ThreatIntel;

  // UI
  isLoading: boolean;
  error: string | null;

  // Actions
  toggleConnection: () => Promise<void>;
  switchCore: (coreId: string) => Promise<void>;
  fetchStatus: () => Promise<void>;
  enableP2P: () => Promise<void>;
  disableP2P: () => Promise<void>;
  setIntranetMode: (mode: IntranetMode) => Promise<void>;
  setError: (error: string | null) => void;
}

export const useUnifiedShieldStore = create<UnifiedShieldState>((set, get) => ({
  // Initial state
  isConnected: false,
  isConnecting: false,
  activeCore: "xtls-reality",
  connectionStats: {
    speedDown: 0,
    speedUp: 0,
    latency: 0,
    jitter: 0,
    packetLoss: 0,
    totalBytesDown: 0,
    totalBytesUp: 0,
    uptime: 0,
    activeCore: "",
    connectedServer: "",
    connectedCountry: "",
    protocol: "",
    speedHistory: [],
    latencyHistory: [],
  },

  cores: [
    { id: "xtls-reality", name: "XTLS-Reality", iconEmoji: "🔮", protocol: "xtls", status: "idle", latency: 42, bandwidth: 5120, successCount: 45, failureCount: 3, successRate: 0.938, ucb1Score: 0.94, description: "Direct TLS with reality handshake", isAvailable: true, port: 443 },
    { id: "hysteria2", name: "Hysteria2", iconEmoji: "⚡", protocol: "quic", status: "idle", latency: 38, bandwidth: 8192, successCount: 52, failureCount: 5, successRate: 0.912, ucb1Score: 0.91, description: "QUIC-based with Brutal CC", isAvailable: true, port: 8443 },
    { id: "tuicv5", name: "TUICv5", iconEmoji: "🚀", protocol: "quic", status: "idle", latency: 55, bandwidth: 6144, successCount: 38, failureCount: 7, successRate: 0.844, ucb1Score: 0.85, description: "QUIC proxy with UDP relay", isAvailable: true, port: 8443 },
    { id: "shadowsocks", name: "Shadowsocks", iconEmoji: "🕶️", protocol: "socks5", status: "idle", latency: 85, bandwidth: 3072, successCount: 20, failureCount: 15, successRate: 0.571, ucb1Score: 0.6, description: "Classic SOCKS5 with AEAD", isAvailable: true, port: 8388 },
    { id: "vless", name: "VLESS", iconEmoji: "💫", protocol: "xtls", status: "idle", latency: 48, bandwidth: 5120, successCount: 40, failureCount: 6, successRate: 0.870, ucb1Score: 0.87, description: "Lightweight proxy with XTLS", isAvailable: true, port: 443 },
    { id: "wireguard", name: "WireGuard", iconEmoji: "🛡️", protocol: "wireguard", status: "idle", latency: 120, bandwidth: 4096, successCount: 15, failureCount: 20, successRate: 0.429, ucb1Score: 0.5, description: "Kernel-level VPN tunnel", isAvailable: true, port: 51820 },
    { id: "trojan", name: "Trojan", iconEmoji: "🐴", protocol: "tls", status: "idle", latency: 50, bandwidth: 5120, successCount: 35, failureCount: 8, successRate: 0.814, ucb1Score: 0.82, description: "TLS-based HTTPS mimicry", isAvailable: true, port: 443 },
    { id: "naiveproxy", name: "NaïveProxy", iconEmoji: "🌐", protocol: "http2", status: "idle", latency: 60, bandwidth: 4096, successCount: 30, failureCount: 5, successRate: 0.857, ucb1Score: 0.86, description: "Chrome network stack camouflage", isAvailable: true, port: 443 },
    { id: "p2p-relay", name: "P2P-Relay", iconEmoji: "🤝", protocol: "p2p", status: "idle", latency: 150, bandwidth: 2048, successCount: 12, failureCount: 8, successRate: 0.600, ucb1Score: 0.65, description: "Serverless peer-to-peer relay", isAvailable: true, port: 0 },
  ],

  p2pPeers: [
    { id: "QmPeer1", country: "DE", isRelay: true, latency: 95, bandwidth: 15000, isOnline: true },
    { id: "QmPeer2", country: "NL", isRelay: true, latency: 88, bandwidth: 20000, isOnline: true },
    { id: "QmPeer3", country: "TR", isRelay: false, latency: 35, bandwidth: 5000, isOnline: true },
    { id: "QmPeer4", country: "AE", isRelay: true, latency: 42, bandwidth: 25000, isOnline: true },
    { id: "QmPeer5", country: "IR", isRelay: false, latency: 15, bandwidth: 3000, isOnline: true },
  ],
  p2pEnabled: false,

  intranetMode: "disabled",

  threatIntel: {
    dpiBlocked: 1247,
    ipBlocked: 389,
    dnsBlocked: 2156,
    allowed: 15420,
    level: "elevated",
    lastUpdated: new Date().toISOString(),
  },

  isLoading: false,
  error: null,

  // Actions
  toggleConnection: async () => {
    const state = get();
    if (state.isConnecting) return;

    if (state.isConnected) {
      set({ isConnected: false, activeCore: "", isConnecting: false });
    } else {
      set({ isConnecting: true, error: null });
      try {
        // Simulate connection
        await new Promise((resolve) => setTimeout(resolve, 1500));
        set({
          isConnected: true,
          isConnecting: false,
          activeCore: state.cores.sort((a, b) => b.ucb1Score - a.ucb1Score)[0]?.id || "xtls-reality",
        });
      } catch {
        set({ isConnecting: false, error: "Connection failed" });
      }
    }
  },

  switchCore: async (coreId: string) => {
    set({ isConnecting: true });
    try {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      set({ activeCore: coreId, isConnecting: true, isConnected: true });
      // Simulate connection stats update
      const core = get().cores.find((c) => c.id === coreId);
      if (core) {
        set((state) => ({
          connectionStats: {
            ...state.connectionStats,
            latency: core.latency,
            speedDown: core.bandwidth * 200,
            speedUp: core.bandwidth * 80,
          },
          isConnecting: false,
        }));
      }
    } catch {
      set({ isConnecting: false, error: "Core switch failed" });
    }
  },

  fetchStatus: async () => {
    set({ isLoading: true });
    try {
      // Simulate fetching status from daemon
      if (get().isConnected) {
        set((state) => ({
          connectionStats: {
            ...state.connectionStats,
            speedDown: state.connectionStats.speedDown + (Math.random() - 0.5) * 500000,
            speedUp: state.connectionStats.speedUp + (Math.random() - 0.5) * 200000,
            latency: Math.max(10, state.connectionStats.latency + (Math.random() - 0.5) * 10),
            speedHistory: [
              ...state.connectionStats.speedHistory.slice(-59),
              state.connectionStats.speedDown + Math.random() * 500000,
            ],
          },
          isLoading: false,
        }));
      } else {
        set({ isLoading: false });
      }
    } catch {
      set({ isLoading: false, error: "Status fetch failed" });
    }
  },

  enableP2P: async () => {
    set({ p2pEnabled: true });
  },

  disableP2P: async () => {
    set({ p2pEnabled: false, p2pPeers: [] });
  },

  setIntranetMode: async (mode: IntranetMode) => {
    set({ intranetMode: mode });
  },

  setError: (error: string | null) => {
    set({ error });
  },
}));
