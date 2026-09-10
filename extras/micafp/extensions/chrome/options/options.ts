/**
 * UnifiedShield Options — Settings page logic
 */

import type { UnifiedShieldConfig } from '../../shared/protocol';
import { DEFAULT_CONFIG, StorageKeys } from '../../shared/protocol';

/* ────────── DOM Elements ────────── */

const els = {
  socksHost: document.getElementById('socksHost') as HTMLInputElement,
  socksPort: document.getElementById('socksPort') as HTMLInputElement,
  socksUsername: document.getElementById('socksUsername') as HTMLInputElement,
  socksPassword: document.getElementById('socksPassword') as HTMLInputElement,
  socksEnabled: document.getElementById('socksEnabled') as HTMLInputElement,
  webrtcFallback: document.getElementById('webrtcFallback') as HTMLInputElement,
  relaySignalingUrl: document.getElementById('relaySignalingUrl') as HTMLInputElement,
  webrtcLocalPort: document.getElementById('webrtcLocalPort') as HTMLInputElement,
  dohEnabled: document.getElementById('dohEnabled') as HTMLInputElement,
  turnServers: document.getElementById('turnServers') as HTMLTextAreaElement,
  turnUsername: document.getElementById('turnUsername') as HTMLInputElement,
  turnPassword: document.getElementById('turnPassword') as HTMLInputElement,
  dpibypassEnabled: document.getElementById('dpibypassEnabled') as HTMLInputElement,
  webrtcRelayEnabled: document.getElementById('webrtcRelayEnabled') as HTMLInputElement,
  autoStart: document.getElementById('autoStart') as HTMLInputElement,
  nativeAppEnabled: document.getElementById('nativeAppEnabled') as HTMLInputElement,
  preferredMode: document.getElementById('preferredMode') as HTMLSelectElement,
  dohBlocklist: document.getElementById('dohBlocklist') as HTMLTextAreaElement,
  saveBtn: document.getElementById('saveBtn') as HTMLButtonElement,
  resetBtn: document.getElementById('resetBtn') as HTMLButtonElement,
  statusBar: document.getElementById('statusBar') as HTMLDivElement,
  statusMessage: document.getElementById('statusMessage') as HTMLSpanElement,
};

const dohCheckboxes = document.querySelectorAll<HTMLInputElement>(
  'input[data-doh]'
);

/* ────────── Init ────────── */

async function init(): Promise<void> {
  const stored = await chrome.storage.local.get(StorageKeys.CONFIG);
  const config: UnifiedShieldConfig = stored[StorageKeys.CONFIG]
    ? { ...DEFAULT_CONFIG, ...stored[StorageKeys.CONFIG] }
    : { ...DEFAULT_CONFIG };

  populateForm(config);

  els.saveBtn.addEventListener('click', handleSave);
  els.resetBtn.addEventListener('click', handleReset);
}

/* ────────── Form Population ────────── */

function populateForm(config: UnifiedShieldConfig): void {
  els.socksHost.value = config.socksHost;
  els.socksPort.value = config.socksPort.toString();
  els.socksUsername.value = config.socksUsername ?? '';
  els.socksPassword.value = config.socksPassword ?? '';
  els.socksEnabled.checked = config.socksEnabled;
  els.webrtcFallback.checked = config.webrtcFallback;
  els.relaySignalingUrl.value = config.relaySignalingUrl ?? '';
  els.webrtcLocalPort.value = (config.webrtcLocalPort ?? 1081).toString();
  els.dohEnabled.checked = config.dohEnabled;
  els.turnServers.value = (config.turnServers ?? []).join('\n');
  els.turnUsername.value = config.turnUsername ?? '';
  els.turnPassword.value = config.turnPassword ?? '';
  els.dpibypassEnabled.checked = config.dpiBypassEnabled ?? false;
  els.webrtcRelayEnabled.checked = config.webrtcRelayEnabled ?? false;
  els.autoStart.checked = config.autoStart;
  els.nativeAppEnabled.checked = config.nativeAppEnabled;
  els.preferredMode.value = config.preferredMode ?? 'auto';
  els.dohBlocklist.value = (config.dohBlocklist ?? []).join('\n');

  // DoH server checkboxes
  const dohServers = config.dohServers ?? ['alidns', 'dnspod', 'byteplus'];
  dohCheckboxes.forEach((cb) => {
    cb.checked = dohServers.includes(cb.dataset.doh!);
  });
}

/* ────────── Form Extraction ────────── */

function extractConfig(): Partial<UnifiedShieldConfig> {
  const dohServers = Array.from(dohCheckboxes)
    .filter((cb) => cb.checked)
    .map((cb) => cb.dataset.doh!);

  const turnServers = els.turnServers.value
    .split('\n')
    .map((s) => s.trim())
    .filter(Boolean);

  const dohBlocklist = els.dohBlocklist.value
    .split('\n')
    .map((s) => s.trim())
    .filter(Boolean);

  return {
    socksHost: els.socksHost.value || '127.0.0.1',
    socksPort: parseInt(els.socksPort.value, 10) || 1080,
    socksUsername: els.socksUsername.value || undefined,
    socksPassword: els.socksPassword.value || undefined,
    socksEnabled: els.socksEnabled.checked,
    webrtcFallback: els.webrtcFallback.checked,
    relaySignalingUrl: els.relaySignalingUrl.value || undefined,
    webrtcLocalPort: parseInt(els.webrtcLocalPort.value, 10) || 1081,
    dohEnabled: els.dohEnabled.checked,
    dohServers,
    turnServers,
    turnUsername: els.turnUsername.value || undefined,
    turnPassword: els.turnPassword.value || undefined,
    dpiBypassEnabled: els.dpibypassEnabled.checked,
    webrtcRelayEnabled: els.webrtcRelayEnabled.checked,
    autoStart: els.autoStart.checked,
    nativeAppEnabled: els.nativeAppEnabled.checked,
    preferredMode: els.preferredMode.value as any,
    dohBlocklist,
  };
}

/* ────────── Handlers ────────── */

async function handleSave(): Promise<void> {
  els.saveBtn.disabled = true;

  try {
    const partial = extractConfig();
    const config: UnifiedShieldConfig = { ...DEFAULT_CONFIG, ...partial };

    await chrome.storage.local.set({ [StorageKeys.CONFIG]: config });

    // Notify service worker
    chrome.runtime.sendMessage({
      type: 'UPDATE_CONFIG',
      payload: config,
    });

    showStatus('Settings saved successfully', 'success');
  } catch (err) {
    showStatus(`Save failed: ${err}`, 'error');
  }

  els.saveBtn.disabled = false;
}

async function handleReset(): Promise<void> {
  if (!confirm('Reset all settings to defaults?')) return;

  await chrome.storage.local.set({ [StorageKeys.CONFIG]: DEFAULT_CONFIG });
  populateForm(DEFAULT_CONFIG);

  chrome.runtime.sendMessage({
    type: 'UPDATE_CONFIG',
    payload: DEFAULT_CONFIG,
  });

  showStatus('Settings reset to defaults', 'success');
}

/* ────────── Status ────────── */

function showStatus(message: string, type: 'success' | 'error'): void {
  els.statusMessage.textContent = message;
  els.statusBar.className = `status-bar ${type}`;
  els.statusBar.style.display = 'block';

  setTimeout(() => {
    els.statusBar.style.display = 'none';
  }, 4000);
}

/* ────────── Bootstrap ────────── */

document.addEventListener('DOMContentLoaded', init);
