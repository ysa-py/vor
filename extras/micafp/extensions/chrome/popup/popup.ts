/**
 * UnifiedShield Popup — UI logic
 */

import type { ProxyState, UnifiedShieldConfig } from '../../shared/protocol';

/* ────────── DOM Elements ────────── */

const statusCard = document.getElementById('statusCard')!;
const statusDot = document.getElementById('statusDot')!;
const statusText = document.getElementById('statusText')!;
const modeText = document.getElementById('modeText')!;
const ispText = document.getElementById('ispText')!;
const toggleBtn = document.getElementById('toggleBtn') as HTMLButtonElement;
const toggleLabel = document.getElementById('toggleLabel')!;
const blockedCount = document.getElementById('blockedCount')!;
const relayCount = document.getElementById('relayCount')!;
const dnsQueries = document.getElementById('dnsQueries')!;
const dnsTestBtn = document.getElementById('dnsTestBtn') as HTMLButtonElement;
const ispDetectBtn = document.getElementById('ispDetectBtn') as HTMLButtonElement;
const optionsBtn = document.getElementById('optionsBtn')!;
const lastBlockTime = document.getElementById('lastBlockTime')!;
const modeRadios = document.querySelectorAll<HTMLInputElement>('input[name="mode"]');

/* ────────── State ────────── */

let currentState: ProxyState | null = null;
let currentConfig: UnifiedShieldConfig | null = null;

/* ────────── Init ────────── */

async function init(): Promise<void> {
  // Load current state
  currentState = await sendMessage('GET_STATE');
  currentConfig = await sendMessage('GET_CONFIG');

  updateUI();

  // Setup listeners
  toggleBtn.addEventListener('click', handleToggle);
  dnsTestBtn.addEventListener('click', handleDnsTest);
  ispDetectBtn.addEventListener('click', handleIspDetect);
  optionsBtn.addEventListener('click', () => {
    chrome.runtime.openOptionsPage();
  });

  modeRadios.forEach((radio) => {
    radio.addEventListener('change', handleModeChange);
  });

  // Listen for state updates
  chrome.runtime.onMessage.addListener((message) => {
    if (message.type === 'STATE_UPDATE') {
      currentState = message.payload;
      updateUI();
    }
  });

  // Auto-refresh every 3 seconds
  setInterval(async () => {
    currentState = await sendMessage('GET_STATE');
    updateUI();
  }, 3000);
}

/* ────────── UI Updates ────────── */

function updateUI(): void {
  if (!currentState) return;

  const connected = currentState.connected;
  const mode = currentState.mode;

  // Status indicator
  statusDot.className = `status-dot ${connected ? 'connected' : 'disconnected'}`;
  statusText.textContent = connected ? 'Connected' : 'Disconnected';
  statusCard.className = `status-card ${connected ? 'connected' : 'disconnected'}`;

  // Toggle button
  toggleBtn.className = `toggle-btn ${connected ? 'active' : ''}`;
  toggleLabel.textContent = connected ? 'Disconnect' : 'Connect';

  // Mode text
  modeText.textContent = `Mode: ${mode.toUpperCase()}`;

  // ISP
  ispText.textContent = currentState.ispDetected
    ? `ISP: ${currentState.ispDetected.name}`
    : 'ISP: Unknown';

  // Stats
  blockedCount.textContent = currentState.blockedCount.toString();

  // Relay count
  const relays = currentState.webrtcActive ? '1' : '0';
  relayCount.textContent = relays;

  // Last block time
  if (currentState.lastBlockTime) {
    const ago = Math.floor((Date.now() - currentState.lastBlockTime) / 1000);
    lastBlockTime.textContent = `Last block: ${formatTimeAgo(ago)}`;
  }

  // Mode radio
  modeRadios.forEach((radio) => {
    radio.checked = radio.value === (currentConfig?.preferredMode ?? 'auto');
  });
}

/* ────────── Handlers ────────── */

async function handleToggle(): Promise<void> {
  // @ts-ignore
  toggleBtn.disabled = true;
  const state = await sendMessage('TOGGLE_PROXY');
  currentState = state;
  updateUI();
  // @ts-ignore
  toggleBtn.disabled = false;
}

async function handleModeChange(e: Event): Promise<void> {
  const mode = (e.target as HTMLInputElement).value;
  if (currentConfig) {
    currentConfig.preferredMode = mode as any;
    await sendMessage('UPDATE_CONFIG', currentConfig);
  }
}

async function handleDnsTest(): Promise<void> {
  // @ts-ignore
  dnsTestBtn.disabled = true;
  dnsTestBtn.classList.add('loading');

  try {
    const result = await sendMessage('DNS_RESOLVE', {
      hostname: 'www.google.com',
      rrType: 'A',
    });

    if (result.records && result.records.length > 0) {
      const ips = result.records.map((r: any) => r.data).join(', ');
      alert(`DNS Test: google.com → ${ips}${result.poisoned ? ' ⚠️ POISONED' : ' ✓ Clean'}`);
    } else {
      alert(`DNS Test: Failed — ${result.error ?? 'No records'}`);
    }
  } catch (err) {
    alert(`DNS Test: Error — ${err}`);
  }

  // @ts-ignore
  dnsTestBtn.disabled = false;
  dnsTestBtn.classList.remove('loading');
}

async function handleIspDetect(): Promise<void> {
  // @ts-ignore
  ispDetectBtn.disabled = true;
  ispDetectBtn.classList.add('loading');

  const isp = await sendMessage('DETECT_ISP');
  if (currentState) {
    currentState.ispDetected = isp;
  }
  updateUI();

  // @ts-ignore
  ispDetectBtn.disabled = false;
  ispDetectBtn.classList.remove('loading');
}

/* ────────── Helpers ────────── */

function sendMessage(type: string, payload?: any): Promise<any> {
  return new Promise((resolve) => {
    chrome.runtime.sendMessage({ type, payload }, (response) => {
      resolve(response);
    });
  });
}

function formatTimeAgo(seconds: number): string {
  if (seconds < 60) return `${seconds}s ago`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  return `${Math.floor(seconds / 3600)}h ago`;
}

/* ────────── Bootstrap ────────── */

document.addEventListener('DOMContentLoaded', init);
