/**
 * MICAFP-UnifiedShield-6.0 — Chrome Popup Script
 *
 * Handles UI interactions, message passing to the background service
 * worker, and periodic status display updates.
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

function sendMessage<T>(message: Record<string, unknown>): Promise<T> {
  return new Promise((resolve) => {
    chrome.runtime.sendMessage(message, (response: T) => {
      resolve(response);
    });
  });
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
    // @ts-ignore
    connectBtn.disabled = false;
  } else if (state === "connecting" || state === "reconnecting") {
    connectBtn.innerHTML = '<span class="spinner"></span>Connecting…';
    connectBtn.className = "connect-btn connect";
    // @ts-ignore
    connectBtn.disabled = true;
  } else {
    connectBtn.textContent = "Connect";
    connectBtn.className = "connect-btn connect";
    // @ts-ignore
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
  } catch (err) {
    // Service worker might be asleep — show disconnected
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

  // Immediate refresh after action
  setTimeout(refresh, 500);
});

settingsLink.addEventListener("click", (e) => {
  e.preventDefault();
  // Open options page (or a new tab with settings)
  chrome.runtime.openOptionsPage?.() ??
    chrome.tabs.create({ url: chrome.runtime.getURL("popup.html#settings") });
});

aboutLink.addEventListener("click", (e) => {
  e.preventDefault();
  chrome.tabs.create({ url: "https://github.com/MICAFP/UnifiedShield" });
});

// ---------------------------------------------------------------------------
// Init
// ---------------------------------------------------------------------------

startRefreshLoop();
