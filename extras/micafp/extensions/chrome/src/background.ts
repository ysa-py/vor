/**
 * MICAFP-UnifiedShield Chrome Manifest V3 Background Service Worker
 * 
 * Features:
 * - chrome.proxy.settings API for proxy configuration
 * - WebTransport tunnel initialization
 * - PAC script generation for selective proxying
 * - Connection state management
 * - chrome.storage.local for settings
 * - chrome.action badge for status indicator
 * - Message passing with popup
 * - No eval(), no remote code execution
 */

import { WebTransportClient, ConnectionState, TunnelStats, TunnelError } from "../../shared/webtransport_tunnel";

// ── Types ──────────────────────────────────────────────────────────────────

interface ShieldSettings {
  serverUrl: string;
  authToken: string;
  obfuscationKey: string;
  proxyMode: "all" | "selective" | "iran-only";
  enabled: boolean;
  selectedServer: string;
  autoConnect: boolean;
}

interface ServerEntry {
  id: string;
  name: string;
  url: string;
  region: string;
  ping: number | null;
}

// ── Constants ──────────────────────────────────────────────────────────────

const DEFAULT_SETTINGS: ShieldSettings = {
  serverUrl: "https://shield.unifiedshield.io/webtransport",
  authToken: "",
  obfuscationKey: "default-obfuscation-key",
  proxyMode: "iran-only",
  enabled: false,
  selectedServer: "auto",
  autoConnect: true,
};

const SERVER_LIST: ServerEntry[] = [
  { id: "arvan-teh", name: "Arvan Tehran", url: "https://arvan-cdn.ir/wt", region: "iran", ping: null },
  { id: "alibaba-hk", name: "Alibaba HK", url: "https://alibaba-edge.cn/wt", region: "asia", ping: null },
  { id: "bytedance-sg", name: "ByteDance SG", url: "https://bd-edge.sg/wt", region: "asia", ping: null },
  { id: "tencent-hk", name: "Tencent HK", url: "https://eo-edge.hk/wt", region: "asia", ping: null },
  { id: "huawei-sg", name: "Huawei SG", url: "https://fg-edge.sg/wt", region: "asia", ping: null },
  { id: "universal-eu", name: "Universal EU", url: "https://shield-eu.unifiedshield.io/wt", region: "europe", ping: null },
];

// Iranian domains that should be proxied in "iran-only" mode
const IRANIAN_DOMAINS = [
  "telegram.org", "telegram.me", "t.me",
  "youtube.com", "youtu.be",
  "twitter.com", "x.com",
  "facebook.com", "instagram.com", "whatsapp.com",
  "google.com", "googleapis.com",
  "github.com", "githubusercontent.com",
  "wikipedia.org",
  "reddit.com",
  "netflix.com",
  "spotify.com",
  "discord.com", "discord.gg",
  "medium.com",
  "linkedin.com",
  "tiktok.com",
  "amazon.com", "amazonaws.com",
  "microsoft.com", "office.com",
  "apple.com",
  "dropbox.com",
  "wordpress.com",
  "soundcloud.com",
  "pinterest.com",
  "tumblr.com",
  "vimeo.com",
  "bbc.com", "cnn.com", "reuters.com",
  "nytimes.com", "washingtonpost.com",
  "theguardian.com", "dw.com",
];

// ── State ──────────────────────────────────────────────────────────────────

let tunnel: WebTransportClient | null = null;
let currentState: ConnectionState = "disconnected";
let currentSettings: ShieldSettings = { ...DEFAULT_SETTINGS };
let serverPings: Map<string, number> = new Map();

// ── Badge Management ───────────────────────────────────────────────────────

function updateBadge(state: ConnectionState): void {
  const badgeConfig: Record<ConnectionState, { text: string; color: string }> = {
    disconnected: { text: "OFF", color: "#6b7280" },
    connecting: { text: "…", color: "#f59e0b" },
    connected: { text: "ON", color: "#10b981" },
    reconnecting: { text: "R", color: "#f59e0b" },
    error: { text: "!", color: "#ef4444" },
  };

  const config = badgeConfig[state] || badgeConfig.disconnected;
  chrome.action.setBadgeText({ text: config.text });
  chrome.action.setBadgeBackgroundColor({ color: config.color });
}

// ── PAC Script Generation ──────────────────────────────────────────────────

/**
 * Generate a PAC script for selective proxying.
 * IMPORTANT: No eval(), no remote code — the PAC script is generated inline.
 */
function generatePacScript(settings: ShieldSettings, proxyPort: number): string {
  const domains = settings.proxyMode === "iran-only"
    ? IRANIAN_DOMAINS
    : settings.proxyMode === "all"
    ? ["*"] // Proxy everything
    : [];

  const domainChecks = domains.map((d) => {
    if (d === "*") return "true";
    // Match domain and all subdomains
    return `dnsDomainIs(host, "${d}") || shExpMatch(host, "*.${d}")`;
  }).join(" || ");

  // The PAC script format required by Chrome
  return `
function FindProxyForURL(url, host) {
  // Direct connections for local/private addresses
  if (isPlainHostName(host) ||
      dnsDomainIs(host, ".local") ||
      isInNet(host, "10.0.0.0", "255.0.0.0") ||
      isInNet(host, "172.16.0.0", "255.240.0.0") ||
      isInNet(host, "192.168.0.0", "255.255.0.0") ||
      isInNet(host, "127.0.0.0", "255.255.255.0")) {
    return "DIRECT";
  }

  // Check if this domain should be proxied
  if (${domainChecks || "false"}) {
    return "SOCKS5 127.0.0.1:${proxyPort}; DIRECT";
  }

  // Default: direct connection
  return "DIRECT";
}`;
}

// ── Proxy Configuration ────────────────────────────────────────────────────

function configureProxy(settings: ShieldSettings): void {
  if (!settings.enabled) {
    // Clear proxy settings
    chrome.proxy.settings.clear({ scope: "regular" }, () => {
      if (chrome.runtime.lastError) {
        console.error("Failed to clear proxy:", chrome.runtime.lastError.message);
      }
    });
    return;
  }

  // Use a fixed local SOCKS port (the tunnel listens here)
  const socksPort = 1080;

  if (settings.proxyMode === "all") {
    // Direct SOCKS5 proxy for all traffic
    chrome.proxy.settings.set({
      scope: "regular",
      value: {
        mode: "fixed_servers",
        rules: {
          singleProxy: {
            scheme: "socks5",
            host: "127.0.0.1",
            port: socksPort,
          },
          bypassList: ["localhost", "127.0.0.1", "::1", "*.local"],
        },
      },
    }, () => {
      if (chrome.runtime.lastError) {
        console.error("Failed to set proxy:", chrome.runtime.lastError.message);
      }
    });
  } else {
    // PAC script for selective proxying
    const pacScript = generatePacScript(settings, socksPort);
    chrome.proxy.settings.set({
      scope: "regular",
      value: {
        mode: "pac_script",
        pacScript: {
          data: pacScript,
          mandatory: false,
        },
      },
    }, () => {
      if (chrome.runtime.lastError) {
        console.error("Failed to set PAC proxy:", chrome.runtime.lastError.message);
      }
    });
  }
}

// ── Tunnel Management ──────────────────────────────────────────────────────

async function startTunnel(): Promise<void> {
  if (tunnel) {
    await tunnel.disconnect();
    tunnel = null;
  }

  const server = SERVER_LIST.find((s) => s.id === currentSettings.selectedServer) ||
    SERVER_LIST.find((s) => s.id === "arvan-teh")!;

  tunnel = new WebTransportClient({
    serverUrl: server.url,
    authToken: currentSettings.authToken,
    obfuscationKey: currentSettings.obfuscationKey,
  });

  // Register state change listener
  tunnel.addStateChangeListener((state, prev) => {
    currentState = state;
    updateBadge(state);
    // Broadcast state to all extension pages
    chrome.runtime.sendMessage({
      type: "state_change",
      state,
      prev,
      stats: tunnel?.getStats() || null,
    }).catch(() => {}); // Ignore if no listeners
  });

  // Register error listener
  tunnel.addErrorListener((error) => {
    chrome.runtime.sendMessage({
      type: "tunnel_error",
      error: error.toJSON(),
    }).catch(() => {});
  });

  await tunnel.connect();
}

async function stopTunnel(): Promise<void> {
  if (tunnel) {
    await tunnel.disconnect();
    tunnel = null;
  }
  currentState = "disconnected";
  updateBadge("disconnected");
  configureProxy({ ...currentSettings, enabled: false });
}

// ── Server Ping ────────────────────────────────────────────────────────────

async function pingServer(server: ServerEntry): Promise<number> {
  const start = performance.now();
  try {
    const response = await fetch(`${server.url.replace("/wt", "")}/health`, {
      method: "GET",
      signal: AbortSignal.timeout(5000),
    });
    if (response.ok) {
      return Math.round(performance.now() - start);
    }
    return -1;
  } catch {
    return -1;
  }
}

async function pingAllServers(): Promise<Map<string, number>> {
  const results = new Map<string, number>();
  const promises = SERVER_LIST.map(async (server) => {
    const latency = await pingServer(server);
    results.set(server.id, latency);
    serverPings.set(server.id, latency);
  });
  await Promise.allSettled(promises);
  return results;
}

// ── Settings Management ────────────────────────────────────────────────────

async function loadSettings(): Promise<ShieldSettings> {
  return new Promise((resolve) => {
    chrome.storage.local.get("shieldSettings", (result) => {
      if (result.shieldSettings) {
        currentSettings = { ...DEFAULT_SETTINGS, ...result.shieldSettings };
      } else {
        currentSettings = { ...DEFAULT_SETTINGS };
      }
      resolve(currentSettings);
    });
  });
}

async function saveSettings(settings: Partial<ShieldSettings>): Promise<void> {
  currentSettings = { ...currentSettings, ...settings };
  return new Promise((resolve) => {
    chrome.storage.local.set({ shieldSettings: currentSettings }, () => {
      if (settings.enabled !== undefined || settings.proxyMode !== undefined || settings.selectedServer !== undefined) {
        configureProxy(currentSettings);
      }
      resolve();
    });
  });
}

// ── Message Handler ────────────────────────────────────────────────────────

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  switch (message.type) {
    case "connect":
      startTunnel()
        .then(() => sendResponse({ success: true }))
        .catch((err) => sendResponse({ success: false, error: err.message }));
      return true; // async response

    case "disconnect":
      stopTunnel()
        .then(() => sendResponse({ success: true }))
        .catch((err) => sendResponse({ success: false, error: err.message }));
      return true;

    case "get_state":
      sendResponse({
        state: currentState,
        stats: tunnel?.getStats() || null,
        settings: currentSettings,
      });
      return false;

    case "update_settings":
      saveSettings(message.settings)
        .then(() => sendResponse({ success: true }))
        .catch((err) => sendResponse({ success: false, error: err.message }));
      return true;

    case "get_servers":
      sendResponse({ servers: SERVER_LIST, pings: Object.fromEntries(serverPings) });
      return false;

    case "ping_servers":
      pingAllServers()
        .then((pings) => sendResponse({ pings: Object.fromEntries(pings) }))
        .catch((err) => sendResponse({ error: err.message }));
      return true;

    case "get_stats":
      sendResponse({ stats: tunnel?.getStats() || null });
      return false;

    default:
      sendResponse({ error: "Unknown message type" });
      return false;
  }
});

// ── Proxy Error Handler ────────────────────────────────────────────────────

chrome.proxy.onProxyError.addListener((details) => {
  console.error("Proxy error:", details);
  chrome.runtime.sendMessage({
    type: "proxy_error",
    error: details,
  }).catch(() => {});
});

// ── Initialization ─────────────────────────────────────────────────────────

async function initialize(): Promise<void> {
  await loadSettings();
  updateBadge(currentState);

  if (currentSettings.enabled && currentSettings.autoConnect) {
    configureProxy(currentSettings);
    await startTunnel();
  }
}

// Start the service worker
initialize().catch(console.error);
