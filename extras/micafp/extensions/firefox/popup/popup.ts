// @ts-nocheck
/**
 * UnifiedShield Firefox Popup — UI logic
 */

const api = typeof browser !== 'undefined' ? browser : chrome;

const statusDot = document.getElementById('statusDot')!;
const statusText = document.getElementById('statusText')!;
const modeText = document.getElementById('modeText')!;
const ispText = document.getElementById('ispText')!;
const toggleBtn = document.getElementById('toggleBtn')!;
const toggleLabel = document.getElementById('toggleLabel')!;
const blockedCount = document.getElementById('blockedCount')!;
const dnsQueries = document.getElementById('dnsQueries')!;
const dnsTestBtn = document.getElementById('dnsTestBtn')!;
const ispDetectBtn = document.getElementById('ispDetectBtn')!;
const optionsBtn = document.getElementById('optionsBtn')!;
const lastBlockTime = document.getElementById('lastBlockTime')!;
const modeRadios = document.querySelectorAll<HTMLInputElement>('input[name="mode"]');

let state: any = null;

async function init(): Promise<void> {
  state = await sendMessage('GET_STATE');
  updateUI();

  toggleBtn.addEventListener('click', handleToggle);
  dnsTestBtn.addEventListener('click', handleDnsTest);
  ispDetectBtn.addEventListener('click', handleIspDetect);
  optionsBtn.addEventListener('click', () => api.runtime.openOptionsPage());

  modeRadios.forEach((r) => r.addEventListener('change', handleModeChange));

  setInterval(async () => {
    state = await sendMessage('GET_STATE');
    updateUI();
  }, 3000);
}

function updateUI(): void {
  if (!state) return;

  const connected = state.connected;
  statusDot.className = `status-dot ${connected ? 'connected' : 'disconnected'}`;
  statusText.textContent = connected ? 'Connected' : 'Disconnected';
  toggleBtn.className = `toggle-btn ${connected ? 'active' : ''}`;
  toggleLabel.textContent = connected ? 'Disconnect' : 'Connect';
  modeText.textContent = `Mode: ${(state.mode ?? 'direct').toUpperCase()}`;
  ispText.textContent = state.ispDetected ? `ISP: ${state.ispDetected.name}` : 'ISP: Unknown';
  blockedCount.textContent = (state.blockedCount ?? 0).toString();

  if (state.lastBlockTime) {
    const ago = Math.floor((Date.now() - state.lastBlockTime) / 1000);
    lastBlockTime.textContent = `Last block: ${formatAgo(ago)}`;
  }
}

async function handleToggle(): Promise<void> {
  toggleBtn.disabled = true;
  state = await sendMessage('TOGGLE_PROXY');
  updateUI();
  toggleBtn.disabled = false;
}

function handleModeChange(e: Event): void {
  const mode = (e.target as HTMLInputElement).value;
  sendMessage('UPDATE_CONFIG', { preferredMode: mode });
}

async function handleDnsTest(): Promise<void> {
  dnsTestBtn.disabled = true;
  try {
    const result = await sendMessage('DNS_RESOLVE', { hostname: 'www.google.com', rrType: 'A' });
    if (result?.records?.length) {
      alert(`DNS: google.com → ${result.records.map((r: any) => r.data).join(', ')}`);
    } else {
      alert(`DNS Test: Failed — ${result?.error ?? 'No records'}`);
    }
  } catch (err) {
    alert(`DNS Test: Error — ${err}`);
  }
  dnsTestBtn.disabled = false;
}

async function handleIspDetect(): Promise<void> {
  ispDetectBtn.disabled = true;
  const isp = await sendMessage('DETECT_ISP');
  if (state) state.ispDetected = isp;
  updateUI();
  ispDetectBtn.disabled = false;
}

function sendMessage(type: string, payload?: any): Promise<any> {
  return new Promise((resolve) => {
    api.runtime.sendMessage({ type, payload }, resolve);
  });
}

function formatAgo(s: number): string {
  if (s < 60) return `${s}s ago`;
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  return `${Math.floor(s / 3600)}h ago`;
}

document.addEventListener('DOMContentLoaded', init);
