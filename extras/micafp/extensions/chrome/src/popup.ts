/**
 * MICAFP-UnifiedShield Chrome Extension Popup UI
 * 
 * Features:
 * - Connection status display with visual indicator
 * - Connect/disconnect button
 * - Server selection dropdown
 * - Settings panel
 * - Import WebTransport tunnel from shared module
 */

// ── Types ──────────────────────────────────────────────────────────────────

interface PopupState {
  connectionState: string;
  stats: any;
  settings: any;
  servers: any[];
  pings: Record<string, number>;
  showSettings: boolean;
}

// ── State ──────────────────────────────────────────────────────────────────

let popupState: PopupState = {
  connectionState: "disconnected",
  stats: null,
  settings: null,
  servers: [],
  pings: {},
  showSettings: false,
};

// ── DOM References ─────────────────────────────────────────────────────────

function $(id: string): HTMLElement | null { return document.getElementById(id); }
function $<T extends HTMLElement>(id: string): T | null { return document.getElementById(id) as T; }

// ── State Helpers ──────────────────────────────────────────────────────────

function stateColor(state: string): string {
  const colors: Record<string, string> = {
    disconnected: "#6b7280",
    connecting: "#f59e0b",
    connected: "#10b981",
    reconnecting: "#f59e0b",
    error: "#ef4444",
  };
  return colors[state] || "#6b7280";
}

function stateLabel(state: string): string {
  const labels: Record<string, string> = {
    disconnected: "Disconnected",
    connecting: "Connecting…",
    connected: "Connected",
    reconnecting: "Reconnecting…",
    error: "Error",
  };
  return labels[state] || state;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatDuration(ms: number): string {
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  if (hours > 0) return `${hours}h ${minutes % 60}m`;
  if (minutes > 0) return `${minutes}m ${seconds % 60}s`;
  return `${seconds}s`;
}

// ── Render Functions ───────────────────────────────────────────────────────

function renderStatusIndicator(): string {
  const color = stateColor(popupState.connectionState);
  const label = stateLabel(popupState.connectionState);
  const pulseClass = ["connecting", "reconnecting"].includes(popupState.connectionState) ? "animate-pulse" : "";

  return `
    <div class="flex items-center gap-3 mb-4">
      <div class="relative">
        <div class="w-4 h-4 rounded-full ${pulseClass}" style="background-color: ${color};"></div>
        ${popupState.connectionState === "connected" ? '<div class="absolute inset-0 w-4 h-4 rounded-full animate-ping opacity-30" style="background-color: ' + color + ';"></div>' : ''}
      </div>
      <div>
        <div class="text-sm font-semibold text-gray-900">${label}</div>
        ${popupState.stats?.connectedSince ? '<div class="text-xs text-gray-500">Uptime: ' + formatDuration(Date.now() - popupState.stats.connectedSince) + '</div>' : ''}
      </div>
    </div>
  `;
}

function renderStats(): string {
  const stats = popupState.stats;
  if (!stats) return "";

  return `
    <div class="grid grid-cols-2 gap-2 mb-4 text-xs">
      <div class="bg-gray-50 rounded-lg p-2">
        <div class="text-gray-500">↓ Received</div>
        <div class="font-semibold text-gray-900">${formatBytes(stats.bytesReceived || 0)}</div>
      </div>
      <div class="bg-gray-50 rounded-lg p-2">
        <div class="text-gray-500">↑ Sent</div>
        <div class="font-semibold text-gray-900">${formatBytes(stats.bytesSent || 0)}</div>
      </div>
      <div class="bg-gray-50 rounded-lg p-2">
        <div class="text-gray-500">Latency</div>
        <div class="font-semibold text-gray-900">${stats.latencyMs !== null ? stats.latencyMs + ' ms' : '—'}</div>
      </div>
      <div class="bg-gray-50 rounded-lg p-2">
        <div class="text-gray-500">Streams</div>
        <div class="font-semibold text-gray-900">${stats.streamsOpened || 0}</div>
      </div>
    </div>
  `;
}

function renderConnectButton(): string {
  const isConnected = popupState.connectionState === "connected";
  const isTransitioning = ["connecting", "reconnecting"].includes(popupState.connectionState);

  return `
    <button id="connect-btn"
      class="w-full py-2.5 px-4 rounded-lg font-semibold text-sm transition-all duration-200
        ${isConnected
          ? "bg-red-500 hover:bg-red-600 text-white"
          : isTransitioning
          ? "bg-yellow-500 text-white cursor-wait"
          : "bg-emerald-500 hover:bg-emerald-600 text-white"
        }"
      ${isTransitioning ? "disabled" : ""}>
      ${isConnected ? "Disconnect" : isTransitioning ? "Connecting…" : "Connect"}
    </button>
  `;
}

function renderServerSelector(): string {
  const servers = popupState.servers;
  const selected = popupState.settings?.selectedServer || "auto";

  const options = servers.map((s: any) => {
    const ping = popupState.pings[s.id];
    const pingLabel = ping !== undefined ? (ping >= 0 ? `${ping}ms` : "—") : "";
    const selectedAttr = s.id === selected ? "selected" : "";
    return `<option value="${s.id}" ${selectedAttr}>${s.name} ${pingLabel ? '(' + pingLabel + ')' : ''}</option>`;
  }).join("");

  return `
    <div class="mb-4">
      <label class="block text-xs font-medium text-gray-700 mb-1">Server</label>
      <select id="server-select" class="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm bg-white focus:ring-2 focus:ring-emerald-500 focus:border-emerald-500">
        <option value="auto" ${selected === "auto" ? "selected" : ""}>Auto (lowest latency)</option>
        ${options}
      </select>
    </div>
  `;
}

function renderSettingsPanel(): string {
  if (!popupState.showSettings) return "";

  const settings = popupState.settings || {};

  return `
    <div class="border-t border-gray-200 pt-4 mt-4 space-y-3">
      <div class="text-sm font-semibold text-gray-700">Settings</div>

      <div>
        <label class="block text-xs font-medium text-gray-700 mb-1">Proxy Mode</label>
        <select id="proxy-mode" class="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm bg-white">
          <option value="iran-only" ${settings.proxyMode === "iran-only" ? "selected" : ""}>Iran Only (recommended)</option>
          <option value="all" ${settings.proxyMode === "all" ? "selected" : ""}>All Traffic</option>
          <option value="selective" ${settings.proxyMode === "selective" ? "selected" : ""}>Selective</option>
        </select>
      </div>

      <div>
        <label class="block text-xs font-medium text-gray-700 mb-1">Auth Token</label>
        <input id="auth-token" type="password"
          class="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-emerald-500"
          value="${settings.authToken || ''}" placeholder="Enter your auth token" />
      </div>

      <div>
        <label class="block text-xs font-medium text-gray-700 mb-1">Obfuscation Key</label>
        <input id="obfuscation-key" type="password"
          class="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-emerald-500"
          value="${settings.obfuscationKey || ''}" placeholder="Obfuscation key" />
      </div>

      <div class="flex items-center gap-2">
        <input id="auto-connect" type="checkbox" ${settings.autoConnect ? "checked" : ""}
          class="rounded border-gray-300 text-emerald-500 focus:ring-emerald-500" />
        <label for="auto-connect" class="text-xs text-gray-700">Auto-connect on startup</label>
      </div>

      <button id="save-settings" class="w-full bg-gray-800 hover:bg-gray-900 text-white rounded-lg py-2 px-4 text-sm font-medium">
        Save Settings
      </button>
    </div>
  `;
}

function renderPage(): void {
  const container = $("app");
  if (!container) return;

  container.innerHTML = `
    <div class="w-80 p-4 bg-white font-sans">
      <!-- Header -->
      <div class="flex items-center justify-between mb-4">
        <div class="flex items-center gap-2">
          <div class="w-8 h-8 bg-emerald-500 rounded-lg flex items-center justify-center">
            <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
            </svg>
          </div>
          <div>
            <div class="text-sm font-bold text-gray-900">UnifiedShield</div>
            <div class="text-xs text-gray-500">v6.0.0</div>
          </div>
        </div>
        <button id="settings-toggle" class="p-1.5 rounded-lg hover:bg-gray-100 transition-colors">
          <svg class="w-5 h-5 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
        </button>
      </div>

      <!-- Status -->
      ${renderStatusIndicator()}

      <!-- Stats -->
      ${renderStats()}

      <!-- Server Selection -->
      ${renderServerSelector()}

      <!-- Connect Button -->
      ${renderConnectButton()}

      <!-- Settings Panel (collapsible) -->
      ${renderSettingsPanel()}

      <!-- Footer -->
      <div class="mt-4 pt-3 border-t border-gray-100 text-center">
        <div class="text-xs text-gray-400">MICAFP-UnifiedShield • End-to-end encrypted</div>
      </div>
    </div>
  `;

  attachEventListeners();
}

// ── Event Listeners ────────────────────────────────────────────────────────

function attachEventListeners(): void {
  // Connect/Disconnect button
  const connectBtn = $<HTMLButtonElement>("connect-btn");
  if (connectBtn) {
    connectBtn.addEventListener("click", () => {
      const isConnected = popupState.connectionState === "connected";
      chrome.runtime.sendMessage(
        { type: isConnected ? "disconnect" : "connect" },
        (response) => {
          if (response?.success !== false) {
            refreshState();
          }
        }
      );
    });
  }

  // Server selector
  const serverSelect = $<HTMLSelectElement>("server-select");
  if (serverSelect) {
    serverSelect.addEventListener("change", () => {
      chrome.runtime.sendMessage({
        type: "update_settings",
        settings: { selectedServer: serverSelect.value },
      }, () => refreshState());
    });
  }

  // Settings toggle
  const settingsToggle = $("settings-toggle");
  if (settingsToggle) {
    settingsToggle.addEventListener("click", () => {
      popupState.showSettings = !popupState.showSettings;
      renderPage();
    });
  }

  // Save settings button
  const saveBtn = $("save-settings");
  if (saveBtn) {
    saveBtn.addEventListener("click", () => {
      const proxyMode = ($<HTMLSelectElement>("proxy-mode"))?.value;
      const authToken = ($<HTMLInputElement>("auth-token"))?.value;
      const obfuscationKey = ($<HTMLInputElement>("obfuscation-key"))?.value;
      const autoConnect = ($<HTMLInputElement>("auto-connect"))?.checked;

      chrome.runtime.sendMessage({
        type: "update_settings",
        settings: {
          proxyMode,
          authToken,
          obfuscationKey,
          autoConnect,
        },
      }, () => {
        saveBtn.textContent = "Saved ✓";
        setTimeout(() => { saveBtn.textContent = "Save Settings"; }, 1500);
        refreshState();
      });
    });
  }
}

// ── Data Refresh ───────────────────────────────────────────────────────────

function refreshState(): void {
  chrome.runtime.sendMessage({ type: "get_state" }, (response) => {
    if (response) {
      popupState.connectionState = response.state;
      popupState.stats = response.stats;
      popupState.settings = response.settings;
    }
    renderPage();
  });

  chrome.runtime.sendMessage({ type: "get_servers" }, (response) => {
    if (response) {
      popupState.servers = response.servers;
      popupState.pings = response.pings;
      renderPage();
    }
  });
}

// ── Live Updates ───────────────────────────────────────────────────────────

chrome.runtime.onMessage.addListener((message) => {
  if (message.type === "state_change") {
    popupState.connectionState = message.state;
    popupState.stats = message.stats;
    renderPage();
  }
  if (message.type === "tunnel_error") {
    console.error("Tunnel error:", message.error);
  }
});

// ── Initialize ─────────────────────────────────────────────────────────────

document.addEventListener("DOMContentLoaded", () => {
  refreshState();

  // Periodic refresh for stats (every 3 seconds)
  setInterval(() => {
    chrome.runtime.sendMessage({ type: "get_stats" }, (response) => {
      if (response?.stats) {
        popupState.stats = response.stats;
        renderPage();
      }
    });
  }, 3000);

  // Ping servers on load
  chrome.runtime.sendMessage({ type: "ping_servers" }, (response) => {
    if (response?.pings) {
      popupState.pings = response.pings;
      renderPage();
    }
  });
});
