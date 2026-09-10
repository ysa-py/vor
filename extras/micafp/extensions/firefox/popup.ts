// @ts-nocheck
/**
 * MICAFP-UnifiedShield-6.0 — Firefox Popup Script
 *
 * Same UI logic as the Chrome popup but using the `browser.*` API
 * namespace which returns Promises (no callbacks).
 */

// ---------------------------------------------------------------------------
// DOM references
// ---------------------------------------------------------------------------

const statusDot = document.getElementById("statusDot")!;
const statusLabel = document.getElementById("statusLabel")!;
const endpointRow = document.getElementById("endpointRow")!;
const connectBtn = document.getElementById("connectBtn")!;
const bytesSentEl = document.getElementById("bytesSent")!;
const bytesReceivedEl = document.getElementById("bytesReceived")!;
const uptimeEl = document.getElementById("uptime")!;
const packetsEl = document.getElementById("packets")!;
const settingsLink = document.getElementById("settingsLink")!;
const aboutLink = document.getElementById("aboutLink")!;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

type TunnelState = "disconnected" | "connecting" | "connected" | "reconnecting" | "failed";

interface Stats {
  bytesSent: number;
  bytesReceived: number;
  packetsSent: number;
  packetsReceived: number;
  connectTime: number | null;
  uptimeMs: number;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  const value = bytes / Math.pow(1024, i);
  return `${value.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

function formatUptime(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
  }
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

// Firefox's browser.runtime.sendMessage returns a Promise directly
declare const browser: {
  runtime: {
    sendMessage(message: Record<string, unknown>): Promise<unknown>;
    getURL(path: string): string;
  };
};

async function sendMessage<T>(message: Record<string, unknown>): Promise<T> {
  return (await browser.runtime.sendMessage(message)) as T;
}

// ---------------------------------------------------------------------------
// UI update
// ---------------------------------------------------------------------------

const STATUS_TEXT: Record<TunnelState, string> = {
  disconnected: "Disconnected",
  connecting: "Connecting…",
  connected: "Connected",
  reconnecting: "Reconnecting…",
  failed: "Connection Failed",
};

function updateUI(state: TunnelState, stats: Stats): void {
  // Status dot
  statusDot.className = "status-dot";
  if (state === "connected") statusDot.classList.add("connected");
  else if (state === "connecting" || state === "reconnecting") statusDot.classList.add("connecting");
  else if (state === "failed") statusDot.classList.add("failed");

  // Status label
  statusLabel.textContent = STATUS_TEXT[state] ?? state;

  // Endpoint row
  endpointRow.style.display = state === "connected" ? "flex" : "none";

  // Connect button
  if (state === "connected") {
    connectBtn.textContent = "Disconnect";
    connectBtn.className = "connect-btn disconnect";
    connectBtn.disabled = false;
  } else if (state === "connecting" || state === "reconnecting") {
    connectBtn.innerHTML = '<span class="spinner"></span>Connecting…';
    connectBtn.className = "connect-btn connect";
    connectBtn.disabled = true;
  } else {
    connectBtn.textContent = "Connect";
    connectBtn.className = "connect-btn connect";
    connectBtn.disabled = false;
  }

  // Stats
  bytesSentEl.textContent = formatBytes(stats.bytesSent);
  bytesReceivedEl.textContent = formatBytes(stats.bytesReceived);
  uptimeEl.textContent = formatUptime(stats.uptimeMs);
  packetsEl.textContent = (stats.packetsSent + stats.packetsReceived).toString();
}

// ---------------------------------------------------------------------------
// Refresh loop
// ---------------------------------------------------------------------------

let refreshTimer: ReturnType<typeof setInterval> | null = null;

async function refresh(): Promise<void> {
  try {
    const stateResp = await sendMessage<{ state: TunnelState }>({ type: "getState" });
    const statsResp = await sendMessage<{ stats: Stats }>({ type: "getStats" });

    if (stateResp && statsResp) {
      updateUI(stateResp.state, statsResp.stats);
    }
  } catch {
    updateUI("disconnected", {
      bytesSent: 0,
      bytesReceived: 0,
      packetsSent: 0,
      packetsReceived: 0,
      connectTime: null,
      uptimeMs: 0,
    });
  }
}

function startRefreshLoop(): void {
  if (refreshTimer) clearInterval(refreshTimer);
  refreshTimer = setInterval(refresh, 2000);
  refresh();
}

// ---------------------------------------------------------------------------
// Event handlers
// ---------------------------------------------------------------------------

connectBtn.addEventListener("click", async () => {
  const stateResp = await sendMessage<{ state: TunnelState }>({ type: "getState" });

  if (!stateResp) return;

  if (stateResp.state === "connected") {
    await sendMessage({ type: "disconnect" });
  } else {
    await sendMessage({ type: "connect" });
  }

  setTimeout(refresh, 500);
});

settingsLink.addEventListener("click", (e) => {
  e.preventDefault();
  // Firefox: open a new tab with settings
  void browser.runtime.sendMessage({ type: "openSettings" });
});

aboutLink.addEventListener("click", (e) => {
  e.preventDefault();
  window.open("https://github.com/MICAFP/UnifiedShield", "_blank");
});

// ---------------------------------------------------------------------------
// Init
// ---------------------------------------------------------------------------

startRefreshLoop();
