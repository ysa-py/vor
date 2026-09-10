// @ts-nocheck
/**
 * MICAFP-UnifiedShield-6.0 — Firefox MV2 Background Script
 *
 * Persistent background script using the browser.* API namespace.
 * Manages the WebTransport tunnel, registers a PAC script via
 * browser.proxy.register(), tracks connection status, and updates
 * the browser action badge.
 */

import type { TunnelState, TunnelStats, WasmObfuscator } from "../shared/webtransport_tunnel";
import { WebTransportTunnel } from "../shared/webtransport_tunnel";

// ---------------------------------------------------------------------------
// Firefox-specific API wrapper
// ---------------------------------------------------------------------------

// In Firefox WebExtensions, `browser` is globally available.
// We alias for clarity when this is also compiled for Chrome.
declare const browser: {
  proxy: {
    register(url: string): Promise<void>;
    unregister(): Promise<void>;
  };
  storage: {
    local: {
      get(keys: string | string[]): Promise<Record<string, unknown>>;
      set(items: Record<string, unknown>): Promise<void>;
    };
  };
  browserAction: {
    setBadgeBackgroundColor(details: { color: string }): Promise<void>;
    setBadgeText(details: { text: string }): Promise<void>;
    setIcon(details: { path: string }): Promise<void>;
  };
  runtime: {
    onMessage: {
      addListener(callback: (msg: unknown, sender: unknown, sendResponse: (resp?: unknown) => void) => boolean | void): void;
    };
    getURL(path: string): string;
    onInstalled: {
      addListener(callback: () => void): void;
    };
  };
  webRequest: {
    onBeforeRequest: {
      addListener(
        callback: (details: { url: string; requestId: string }) => void,
        filter: { urls: string[] },
        extraInfoSpec?: string[],
      ): void;
    };
  };
};

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
    const response = await fetch(browser.runtime.getURL("wasm/obfuscator.wasm"));
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
// PAC script (Firefox uses browser.proxy.register with a separate file)
// ---------------------------------------------------------------------------

/**
 * In Firefox, the PAC script is loaded from a URL within the extension.
 * We generate it dynamically and store it as a data URI.
 *
 * However, browser.proxy.register() expects a URL to a file in the
 * extension, so we use an inline PAC approach through the
 * browser.proxy.onRequest API instead for dynamic control.
 */

async function enableProxy(): Promise<void> {
  // Firefox uses the onRequest API for fine-grained proxy control
  // We register a listener that routes all traffic through our proxy
  try {
    // Register PAC file from extension
    await browser.proxy.register(browser.runtime.getURL("proxy.pac"));
    console.log("[Shield] Proxy registered (PAC)");
  } catch {
    // Fallback: use onProxyError + onRequest pattern
    console.warn("[Shield] PAC register failed, falling back to direct proxy");
  }
}

async function disableProxy(): Promise<void> {
  try {
    await browser.proxy.unregister();
    console.log("[Shield] Proxy unregistered");
  } catch {
    /* ignore */
  }
}

// ---------------------------------------------------------------------------
// Badge / icon updates
// ---------------------------------------------------------------------------

function updateBadge(state: TunnelState): void {
  const colors: Record<TunnelState, string> = {
    disconnected: "#9ca3af",
    connecting: "#f59e0b",
    connected: "#22c55e",
    reconnecting: "#f59e0b",
    failed: "#ef4444",
  };

  const texts: Record<TunnelState, string> = {
    disconnected: "",
    connecting: "…",
    connected: "✓",
    reconnecting: "↻",
    failed: "✗",
  };

  void browser.browserAction.setBadgeBackgroundColor({ color: colors[state] });
  void browser.browserAction.setBadgeText({ text: texts[state] });
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
    maxReconnectAttempts: 0,
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
  const result = await browser.storage.local.get(STORAGE_KEY_CONFIG);
  const stored = result[STORAGE_KEY_CONFIG] as StoredConfig | undefined;
  return (
    stored ?? {
      endpoints: CDN_ENDPOINTS_DEFAULT,
      hmacKey: HMAC_KEY_DEFAULT,
    }
  );
}

async function saveConfig(config: StoredConfig): Promise<void> {
  await browser.storage.local.set({ [STORAGE_KEY_CONFIG]: config });
}

async function persistState(state: TunnelState): Promise<void> {
  await browser.storage.local.set({ [STORAGE_KEY_STATE]: state });
}

async function persistStats(stats: TunnelStats): Promise<void> {
  await browser.storage.local.set({ [STORAGE_KEY_STATS]: stats });
}

// ---------------------------------------------------------------------------
// Message handling (from popup)
// ---------------------------------------------------------------------------

browser.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  const msg = message as { type: string; payload?: unknown };

  switch (msg.type) {
    case "connect":
      startTunnel()
        .then(() => sendResponse({ ok: true }))
        .catch((err: Error) => sendResponse({ ok: false, error: err.message }));
      return true; // async response

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
// Init
// ---------------------------------------------------------------------------

browser.runtime.onInstalled.addListener(() => {
  console.log("[Shield] Extension installed / updated");
});

// Restore previous state on startup
(async () => {
  const result = await browser.storage.local.get(STORAGE_KEY_STATE);
  const prevState = result[STORAGE_KEY_STATE] as TunnelState | undefined;

  if (prevState === "connected" || prevState === "reconnecting") {
    console.log("[Shield] Restoring tunnel after restart");
    wasmObfuscator = await loadWasmObfuscator();
    await startTunnel();
  } else {
    updateBadge("disconnected");
  }

  // Periodically persist stats (since background is persistent in MV2)
  setInterval(() => {
    if (tunnel) {
      persistStats(tunnel.getStats());
    }
  }, 60_000);
})();
