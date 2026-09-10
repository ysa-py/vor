// @ts-nocheck
/**
 * MICAFP-UnifiedShield Firefox Manifest V2 Background Script
 * 
 * Features:
 * - browser.proxy API for proxy configuration
 * - Same functionality as Chrome but with Firefox APIs
 * - WebTransport tunnel from shared module
 * - browser.storage.local for settings
 * - browser.browserAction for status
 * - Compatible with Firefox 109+
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
  "bbc.com", "cnn.com", "reuters.com",
  "nytimes.com", "washingtonpost.com",
];

// ── State ──────────────────────────────────────────────────────────────────

let tunnel: WebTransportClient | null = null;
let currentState: ConnectionState = "disconnected";
let currentSettings: ShieldSettings = { ...DEFAULT_SETTINGS };
let serverPings: Map<string, number> = new Map();

// ── Badge Management (Firefox uses browser.browserAction) ──────────────────

function updateBadge(state: ConnectionState): void {
  const badgeConfig: Record<ConnectionState, { text: string; color: string }> = {
    disconnected: { text: "OFF", color: "#6b7280" },
    connecting: { text: "…", color: "#f59e0b" },
    connected: { text: "ON", color: "#10b981" },
    reconnecting: { text: "R", color: "#f59e0b" },
    error: { text: "!", color: "#ef4444" },
  };

  const config = badgeConfig[state] || badgeConfig.disconnected;
  browser.browserAction.setBadgeText({ text: config.text });
  browser.browserAction.setBadgeBackgroundColor({ color: config.color });
}

// ── Firefox Proxy API ──────────────────────────────────────────────────────

/**
 * Firefox uses browser.proxy.onRequest for dynamic proxying.
 * This is the Firefox equivalent of Chrome's PAC script approach.
 * It intercepts requests and decides whether to proxy them.
 */

function shouldProxy(url: URL): boolean {
  // Never proxy local/private addresses
  const hostname = url.hostname;
  if (
    hostname === "localhost" ||
    hostname === "127.0.0.1" ||
    hostname === "::1" ||
    hostname.endsWith(".local") ||
    hostname.startsWith("10.") ||
    hostname.startsWith("172.16.") ||
    hostname.startsWith("192.168.")
  ) {
    return false;
  }

  if (currentSettings.proxyMode === "all") {
    return true;
  }

  if (currentSettings.proxyMode === "iran-only") {
    return IRANIAN_DOMAINS.some((domain) => {
      return hostname === domain || hostname.endsWith("." + domain);
    });
  }

  return false;
}

// Register the proxy request listener
browser.proxy.onRequest.addListener(
  (details: any) => {
    if (!currentSettings.enabled) {
      return { type: "direct" };
    }

    try {
      const url = new URL(details.url);
      if (shouldProxy(url)) {
        return {
          type: "socks",
          host: "127.0.0.1",
          port: 1080,
          proxyDNS: true,
        };
      }
    } catch {}

    return { type: "direct" };
  },
  { urls: ["<all_urls>"] }
);

// Handle proxy errors
browser.proxy.onError.addListener((error: any) => {
  console.error("Proxy error:", error);
});

// ── Settings Management ────────────────────────────────────────────────────

async function loadSettings(): Promise<ShieldSettings> {
  const result = await browser.storage.local.get("shieldSettings");
  if (result.shieldSettings) {
    currentSettings = { ...DEFAULT_SETTINGS, ...result.shieldSettings };
  } else {
    currentSettings = { ...DEFAULT_SETTINGS };
  }
  return currentSettings;
}

async function saveSettings(settings: Partial<ShieldSettings>): Promise<void> {
  currentSettings = { ...currentSettings, ...settings };
  await browser.storage.local.set({ shieldSettings: currentSettings });
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

  tunnel.addStateChangeListener((state, prev) => {
    currentState = state;
    updateBadge(state);
    // Broadcast to popup
    browser.runtime.sendMessage({
      type: "state_change",
      state,
      prev,
      stats: tunnel?.getStats() || null,
    }).catch(() => {});
  });

  tunnel.addErrorListener((error) => {
    browser.runtime.sendMessage({
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
}

// ── Server Ping ────────────────────────────────────────────────────────────

async function pingServer(server: ServerEntry): Promise<number> {
  const start = performance.now();
  try {
    const response = await fetch(`${server.url.replace("/wt", "")}/health`, {
      method: "GET",
      signal: AbortSignal.timeout(5000),
    });
    return response.ok ? Math.round(performance.now() - start) : -1;
  } catch {
    return -1;
  }
}

async function pingAllServers(): Promise<Map<string, number>> {
  const results = new Map<string, number>();
  await Promise.allSettled(
    SERVER_LIST.map(async (server) => {
      const latency = await pingServer(server);
      results.set(server.id, latency);
      serverPings.set(server.id, latency);
    })
  );
  return results;
}

// ── Message Handler (Firefox uses browser.runtime) ─────────────────────────

browser.runtime.onMessage.addListener((message: any, sender: any) => {
  switch (message.type) {
    case "connect":
      return startTunnel()
        .then(() => ({ success: true }))
        .catch((err) => ({ success: false, error: err.message }));

    case "disconnect":
      return stopTunnel()
        .then(() => ({ success: true }))
        .catch((err) => ({ success: false, error: err.message }));

    case "get_state":
      return Promise.resolve({
        state: currentState,
        stats: tunnel?.getStats() || null,
        settings: currentSettings,
      });

    case "update_settings":
      return saveSettings(message.settings)
        .then(() => ({ success: true }))
        .catch((err) => ({ success: false, error: err.message }));

    case "get_servers":
      return Promise.resolve({
        servers: SERVER_LIST,
        pings: Object.fromEntries(serverPings),
      });

    case "ping_servers":
      return pingAllServers()
        .then((pings) => ({ pings: Object.fromEntries(pings) }))
        .catch((err) => ({ error: err.message }));

    case "get_stats":
      return Promise.resolve({
        stats: tunnel?.getStats() || null,
      });

    default:
      return Promise.resolve({ error: "Unknown message type" });
  }
});

// ── WebRequest Listener for Header Modification ────────────────────────────

/**
 * Modify request headers to add auth and shield identification
 * This works with Firefox's webRequest API in MV2
 */
browser.webRequest.onBeforeSendHeaders.addListener(
  (details: any) => {
    if (!currentSettings.enabled) return;

    // Only modify requests going to our CDN endpoints
    const url = new URL(details.url);
    const isShieldRequest = url.hostname.includes("unifiedshield") ||
      url.hostname.includes("shield") ||
      SERVER_LIST.some((s) => url.hostname.includes(new URL(s.url).hostname));

    if (isShieldRequest) {
      const headers = details.requestHeaders || [];
      headers.push({
        name: "X-Shield-Client",
        value: "firefox-extension/6.0.0",
      });
      if (currentSettings.authToken) {
        headers.push({
          name: "Authorization",
          value: `Bearer ${currentSettings.authToken}`,
        });
      }
      return { requestHeaders: headers };
    }
  },
  { urls: ["<all_urls>"] },
  ["blocking", "requestHeaders"]
);

// ── Initialization ─────────────────────────────────────────────────────────

async function initialize(): Promise<void> {
  await loadSettings();
  updateBadge(currentState);

  if (currentSettings.enabled && currentSettings.autoConnect) {
    await startTunnel();
  }
}

initialize().catch(console.error);
