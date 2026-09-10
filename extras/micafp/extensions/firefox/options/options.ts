// @ts-nocheck
/**
 * UnifiedShield Firefox Options — Settings logic
 */

const api = typeof browser !== 'undefined' ? browser : chrome;

const DEFAULT_CONFIG = {
  socksHost: '127.0.0.1',
  socksPort: 1080,
  socksEnabled: true,
  webrtcFallback: true,
  dohEnabled: true,
  dohServers: ['alidns', 'dnspod', 'byteplus'],
  autoStart: false,
  nativeAppEnabled: false,
  preferredMode: 'auto',
  turnServers: [],
  dohBlocklist: [],
};

const els = {
  socksHost: document.getElementById('socksHost') as HTMLInputElement,
  socksPort: document.getElementById('socksPort') as HTMLInputElement,
  socksEnabled: document.getElementById('socksEnabled') as HTMLInputElement,
  dohEnabled: document.getElementById('dohEnabled') as HTMLInputElement,
  autoStart: document.getElementById('autoStart') as HTMLInputElement,
  preferredMode: document.getElementById('preferredMode') as HTMLSelectElement,
  saveBtn: document.getElementById('saveBtn') as HTMLButtonElement,
  resetBtn: document.getElementById('resetBtn') as HTMLButtonElement,
  statusBar: document.getElementById('statusBar') as HTMLDivElement,
  statusMessage: document.getElementById('statusMessage') as HTMLSpanElement,
};

const dohCheckboxes = document.querySelectorAll<HTMLInputElement>('input[data-doh]');

async function init(): Promise<void> {
  const stored = await api.storage.local.get('unifiedshield_config');
  const config = stored.unifiedshield_config
    ? { ...DEFAULT_CONFIG, ...stored.unifiedshield_config }
    : { ...DEFAULT_CONFIG };

  els.socksHost.value = config.socksHost;
  els.socksPort.value = config.socksPort;
  els.socksEnabled.checked = config.socksEnabled;
  els.dohEnabled.checked = config.dohEnabled;
  els.autoStart.checked = config.autoStart;
  els.preferredMode.value = config.preferredMode;

  const dohServers = config.dohServers ?? [];
  dohCheckboxes.forEach((cb) => {
    cb.checked = dohServers.includes(cb.dataset.doh!);
  });

  els.saveBtn.addEventListener('click', handleSave);
  els.resetBtn.addEventListener('click', handleReset);
}

async function handleSave(): Promise<void> {
  els.saveBtn.disabled = true;

  const dohServers = Array.from(dohCheckboxes)
    .filter((cb) => cb.checked)
    .map((cb) => cb.dataset.doh!);

  const config = {
    ...DEFAULT_CONFIG,
    socksHost: els.socksHost.value || '127.0.0.1',
    socksPort: parseInt(els.socksPort.value, 10) || 1080,
    socksEnabled: els.socksEnabled.checked,
    dohEnabled: els.dohEnabled.checked,
    dohServers,
    autoStart: els.autoStart.checked,
    preferredMode: els.preferredMode.value,
  };

  await api.storage.local.set({ unifiedshield_config: config });
  api.runtime.sendMessage({ type: 'UPDATE_CONFIG', payload: config });

  showStatus('Settings saved', 'success');
  els.saveBtn.disabled = false;
}

async function handleReset(): Promise<void> {
  if (!confirm('Reset all settings to defaults?')) return;
  await api.storage.local.set({ unifiedshield_config: DEFAULT_CONFIG });
  await init();
  showStatus('Settings reset', 'success');
}

function showStatus(msg: string, type: 'success' | 'error'): void {
  els.statusMessage.textContent = msg;
  els.statusBar.className = `status-bar ${type}`;
  els.statusBar.style.display = 'block';
  setTimeout(() => { els.statusBar.style.display = 'none'; }, 4000);
}

document.addEventListener('DOMContentLoaded', init);
