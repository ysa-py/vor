/**
 * MICAFP-UnifiedShield-6.0 — Chrome MV3 Service Worker
 *
 * Manages the WebTransport tunnel, sets the PAC proxy configuration,
 * tracks connection status, and updates the badge icon.
 *
 * In MV3 the service worker can be terminated at any time; all state
 * is persisted in chrome.storage.session so we can restore it on wake-up.
 */

// Re-export types for bundling (the build step will inline these)
import type { TunnelState, TunnelStats, WasmObfuscator } from "../shared/webtransport_tunnel";
import { WebTransportTunnel } from "../shared/webtransport_tunnel";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const STORAGE_KEY_STATE = "shield_state";
const STORAGE_KEY_STATS = "shield_stats";
const STORAGE_KEY_CONFIG = "shield_config";

const CDN_ENDPOINTS_DEFAULT: string[] = [
  "https://shield-deno-ist.deno.dev",
  "https://shield-deno-dub.deno.dev",
  "https://shield-valtown-ist.val.town",
  "https://shield-supabase-dub.supabase.co/functions/v1/shield",
  "https://shield-netlify-ist.netlify.app",
  "https://shield-arvan-teh.arvancloud.ir/faas/shield",
];

const HMAC_KEY_DEFAULT = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

// ---------------------------------------------------------------------------
// Global tunnel reference
// ---------------------------------------------------------------------------

let tunnel: WebTransportTunnel | null = null;
let wasmObfuscator: WasmObfuscator | null = null;

// ---------------------------------------------------------------------------
// WASM loader
// ---------------------------------------------------------------------------

async function loadWasmObfuscator(): Promise<WasmObfuscator | null> {
  try {
    // In MV3 we can fetch WASM from the extension package
    const response = await fetch(chrome.runtime.getURL("wasm/obfuscator.wasm"));
    const bytes = await response.arrayBuffer();
    const { instance } = await WebAssembly.instantiate(bytes, {});

    const exports = instance.exports as unknown as {
      transform: (dataPtr: number, dataLen: number, keyPtr: number, keyLen: number) => number;
      reverse_transform: (dataPtr: number, dataLen: number, keyPtr: number, keyLen: number) => number;
      memory: WebAssembly.Memory;
      alloc: (size: number) => number;
      dealloc: (ptr: number, size: number) => void;
      get_output_len: () => number;
    };

    const memory = exports.memory;

    function writeBytes(data: Uint8Array): number {
      const ptr = exports.alloc(data.length);
      new Uint8Array(memory.buffer).set(data, ptr);
      return ptr;
    }

    function readOutput(): Uint8Array {
      const len = exports.get_output_len();
      // Output is written to a region starting at offset returned by the call
      // For simplicity assume the transform functions return the output pointer
      return new Uint8Array(memory.buffer).slice(0, len);
    }

    return {
      transform(data: Uint8Array, key: Uint8Array): Uint8Array {
        const dataPtr = writeBytes(data);
        const keyPtr = writeBytes(key);
        exports.transform(dataPtr, data.length, keyPtr, key.length);
        const result = readOutput();
        exports.dealloc(dataPtr, data.length);
        exports.dealloc(keyPtr, key.length);
        return result;
      },
      reverseTransform(data: Uint8Array, key: Uint8Array): Uint8Array {
        const dataPtr = writeBytes(data);
        const keyPtr = writeBytes(key);
        exports.reverse_transform(dataPtr, data.length, keyPtr, key.length);
        const result = readOutput();
        exports.dealloc(dataPtr, data.length);
        exports.dealloc(keyPtr, key.length);
        return result;
      },
    };
  } catch (err) {
    console.warn("[Shield] WASM obfuscator load failed, running without:", err);
    return null;
  }
}

// ---------------------------------------------------------------------------
// PAC script generation
// ---------------------------------------------------------------------------

function generatePacScript(proxyHost: string, proxyPort: number): string {
  return `
    function FindProxyForURL(url, host) {
      // Never proxy extension pages
      if (host === "127.0.0.1" || host === "localhost") {
        return "DIRECT";
      }
      // Never proxy local network
      if (isPlainHostName(host) ||
          dnsDomainIs(host, ".local") ||
          isInNet(host, "10.0.0.0", "255.0.0.0") ||
          isInNet(host, "172.16.0.0", "255.240.0.0") ||
          isInNet(host, "192.168.0.0", "255.255.0.0")) {
        return "DIRECT";
      }
      // Route all other traffic through the shield proxy
      return "HTTPS ${proxyHost}:${proxyPort}; SOCKS5 127.0.0.1:1080; DIRECT";
    }
  `;
}

// ---------------------------------------------------------------------------
// Proxy management
// ---------------------------------------------------------------------------

async function enableProxy(): Promise<void> {
  const pacScript = generatePacScript("shield.proxy", 443);
  await chrome.proxy.settings.set({
    value: {
      mode: "pac_script",
      pacScript: { data: pacScript },
    },
    scope: "regular",
  });
  console.log("[Shield] Proxy enabled (PAC)");
}

async function disableProxy(): Promise<void> {
  await chrome.proxy.settings.clear({ scope: "regular" });
  console.log("[Shield] Proxy disabled");
}

// ---------------------------------------------------------------------------
// Badge / icon updates
// ---------------------------------------------------------------------------

function updateBadge(state: TunnelState): void {
  const colors: Record<TunnelState, string> = {
    disconnected: "#9ca3af", // gray
    connecting: "#f59e0b",   // amber
    connected: "#22c55e",    // green
    reconnecting: "#f59e0b", // amber
    failed: "#ef4444",       // red
  };

  const texts: Record<TunnelState, string> = {
    disconnected: "",
    connecting: "…",
    connected: "✓",
    reconnecting: "↻",
    failed: "✗",
  };

  void chrome.action.setBadgeBackgroundColor({ color: colors[state] });
  void chrome.action.setBadgeText({ text: texts[state] });
}

// ---------------------------------------------------------------------------
// Tunnel lifecycle
// ---------------------------------------------------------------------------

async function startTunnel(): Promise<void> {
  if (tunnel) {
    tunnel.destroy();
    tunnel = null;
  }

  const config = await loadConfig();

  tunnel = new WebTransportTunnel({
    endpoints: config.endpoints,
    hmacKey: config.hmacKey,
    wasmObfuscator: wasmObfuscator,
    maxReconnectAttempts: 0, // infinite
    initialBackoffMs: 1000,
    maxBackoffMs: 60_000,
    keepaliveIntervalMs: 25_000,
  });

  tunnel.onStateChange(async (state, prev) => {
    console.log(`[Shield] State: ${prev} → ${state}`);
    updateBadge(state);
    await persistState(state);
    if (state === "connected") {
      await enableProxy();
    } else if (state === "disconnected" || state === "failed") {
      await disableProxy();
    }
  });

  tunnel.onData((data) => {
    // Data received from tunnel — handled by proxy layer
    // In a full implementation this feeds the SOCKS5/HTTPS proxy server
    console.debug("[Shield] Received", data.length, "bytes from tunnel");
  });

  await tunnel.connect();
}

function stopTunnel(): void {
  if (tunnel) {
    tunnel.disconnect();
    tunnel = null;
  }
  void disableProxy();
  void persistState("disconnected");
  updateBadge("disconnected");
}

// ---------------------------------------------------------------------------
// Config persistence
// ---------------------------------------------------------------------------

interface StoredConfig {
  endpoints: string[];
  hmacKey: string;
}

async function loadConfig(): Promise<StoredConfig> {
  return new Promise((resolve) => {
    chrome.storage.local.get(STORAGE_KEY_CONFIG, (result) => {
      const stored = result[STORAGE_KEY_CONFIG] as StoredConfig | undefined;
      resolve(
        stored ?? {
          endpoints: CDN_ENDPOINTS_DEFAULT,
          hmacKey: HMAC_KEY_DEFAULT,
        },
      );
    });
  });
}

async function saveConfig(config: StoredConfig): Promise<void> {
  await chrome.storage.local.set({ [STORAGE_KEY_CONFIG]: config });
}

async function persistState(state: TunnelState): Promise<void> {
  await chrome.storage.session.set({ [STORAGE_KEY_STATE]: state });
}

async function persistStats(stats: TunnelStats): Promise<void> {
  await chrome.storage.session.set({ [STORAGE_KEY_STATS]: stats });
}

// ---------------------------------------------------------------------------
// Message handling (from popup)
// ---------------------------------------------------------------------------

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  const msg = message as { type: string; payload?: unknown };

  switch (msg.type) {
    case "connect":
      startTunnel()
        .then(() => sendResponse({ ok: true }))
        .catch((err: Error) => sendResponse({ ok: false, error: err.message }));
      return true; // keep channel open for async response

    case "disconnect":
      stopTunnel();
      sendResponse({ ok: true });
      return false;

    case "getState": {
      const state = tunnel?.currentState ?? "disconnected";
      sendResponse({ state });
      return false;
    }

    case "getStats": {
      const stats = tunnel?.getStats() ?? {
        bytesSent: 0,
        bytesReceived: 0,
        packetsSent: 0,
        packetsReceived: 0,
        connectTime: null,
        uptimeMs: 0,
      };
      sendResponse({ stats });
      return false;
    }

    case "updateConfig": {
      const newConfig = msg.payload as StoredConfig;
      saveConfig(newConfig)
        .then(() => sendResponse({ ok: true }))
        .catch((err: Error) => sendResponse({ ok: false, error: err.message }));
      return true;
    }

    default:
      sendResponse({ error: "Unknown message type" });
      return false;
  }
});

// ---------------------------------------------------------------------------
// Wake-up restore
// ---------------------------------------------------------------------------

chrome.runtime.onInstalled.addListener(() => {
  console.log("[Shield] Extension installed / updated");
});

// On service-worker wake-up, restore the previous state
chrome.storage.session.get(STORAGE_KEY_STATE, (result) => {
  const prevState = result[STORAGE_KEY_STATE] as TunnelState | undefined;
  if (prevState === "connected" || prevState === "reconnecting") {
    console.log("[Shield] Restoring tunnel after wake-up");
    loadWasmObfuscator().then((wasm) => {
      wasmObfuscator = wasm;
      startTunnel();
    });
  } else {
    updateBadge("disconnected");
  }
});

// Periodically persist stats (MV3 alarms API for periodic wake-ups)
chrome.alarms.create("persistStats", { periodInMinutes: 1 });
chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === "persistStats" && tunnel) {
    persistStats(tunnel.getStats());
  }
});
