/**
 * UnifiedShield NextGen — MV3 Service Worker
 * Manages proxy, PAC script generation, WebRTC relay, DoH, and ISP detection.
 */

import { ProxyManager } from './proxy-manager';
import { PacGenerator } from './pac-generator';
import { WebRelay } from './webrtc-relay';
import { DohResolver } from './doh-resolver';
import { ISPDetector } from './isp-detector';
import {
  DEFAULT_CONFIG,
  StorageKeys,
  type UnifiedShieldConfig,
  type ProxyState,
  type RelayPeer,
} from '../../shared/protocol';

/* ────────────────────── state ────────────────────── */

let config: UnifiedShieldConfig = { ...DEFAULT_CONFIG };
let proxyManager: ProxyManager;
let pacGenerator: PacGenerator;
let webRelay: WebRelay;
let dohResolver: DohResolver;
let ispDetector: ISPDetector;

const state: ProxyState = {
  connected: false,
  mode: 'auto',
  socksPort: 1080,
  webrtcActive: false,
  ispDetected: null,
  blockedCount: 0,
  lastBlockTime: null,
};

/* ────────────────────── lifecycle ────────────────────── */

chrome.runtime.onInstalled.addListener(async (details) => {
  console.log('[UnifiedShield] Installed:', details.reason);
  await loadConfig();
  await initModules();
  if (details.reason === 'install') {
    await chrome.runtime.openOptionsPage();
  }
});

chrome.runtime.onStartup.addListener(async () => {
  await loadConfig();
  await initModules();
});

/* ────────────────────── config ────────────────────── */

async function loadConfig(): Promise<void> {
  const stored = await chrome.storage.local.get(StorageKeys.CONFIG);
  if (stored[StorageKeys.CONFIG]) {
    config = { ...DEFAULT_CONFIG, ...stored[StorageKeys.CONFIG] };
  }
  const stateStored = await chrome.storage.local.get(StorageKeys.STATE);
  if (stateStored[StorageKeys.STATE]) {
    Object.assign(state, stateStored[StorageKeys.STATE]);
  }
}

async function saveConfig(): Promise<void> {
  await chrome.storage.local.set({ [StorageKeys.CONFIG]: config });
  await chrome.storage.local.set({ [StorageKeys.STATE]: state });
}

/* ────────────────────── init ────────────────────── */

async function initModules(): Promise<void> {
  ispDetector = new ISPDetector(config);
  pacGenerator = new PacGenerator(config);
  proxyManager = new ProxyManager(config, pacGenerator);
  webRelay = new WebRelay(config);
  dohResolver = new DohResolver(config);

  // Detect ISP
  const isp = await ispDetector.detect();
  state.ispDetected = isp;
  console.log('[UnifiedShield] ISP detected:', isp?.name ?? 'unknown');

  // Auto-start proxy if configured
  if (config.autoStart) {
    await startProxy();
  }

  // Setup periodic ISP re-check
  chrome.alarms.create('isp-check', { periodInMinutes: 30 });
  chrome.alarms.create('config-sync', { periodInMinutes: 5 });
  chrome.alarms.onAlarm.addListener(handleAlarm);

  // Listen for web requests to detect blocking
  chrome.webRequest.onBeforeRequest.addListener(
    onRequestBefore,
    { urls: ['<all_urls>'] },
    ['blocking']
  );

  chrome.webRequest.onErrorOccurred.addListener(
    onRequestError,
    { urls: ['<all_urls>'] }
  );
}

/* ────────────────────── proxy control ────────────────────── */

async function startProxy(): Promise<void> {
  try {
    // Try native SOCKS5 first
    if (config.socksEnabled) {
      await proxyManager.setProxy(config.socksHost, config.socksPort);
      state.connected = true;
      state.mode = 'socks5';
      console.log('[UnifiedShield] SOCKS5 proxy active');
    }

    // Fallback to WebRTC relay if native not available
    if (!state.connected && config.webrtcFallback) {
      const peer = await webRelay.connect(config.relaySignalingUrl);
      if (peer) {
        state.connected = true;
        state.mode = 'webrtc';
        state.webrtcActive = true;
        await proxyManager.setWebRTCProxy(peer);
        console.log('[UnifiedShield] WebRTC relay active');
      }
    }

    await saveConfig();
  } catch (err) {
    console.error('[UnifiedShield] Proxy start failed:', err);
    state.connected = false;
    await saveConfig();
  }
}

async function stopProxy(): Promise<void> {
  await proxyManager.clearProxy();
  if (state.webrtcActive) {
    webRelay.disconnect();
    state.webrtcActive = false;
  }
  state.connected = false;
  state.mode = 'direct';
  await saveConfig();
  console.log('[UnifiedShield] Proxy stopped');
}

/* ────────────────────── request monitoring ────────────────────── */

function onRequestBefore(
  details: chrome.webRequest.WebRequestBodyDetails
): chrome.webRequest.BlockingResponse | void {
  if (!state.connected) return;

  // Detect DNS poisoning
  if (details.url && config.dohEnabled) {
    const url = new URL(details.url);
    // Intercept DNS requests to use DoH
    if (url.hostname && config.dohBlocklist.includes(url.hostname)) {
      state.blockedCount++;
      state.lastBlockTime = Date.now();
      return { cancel: true };
    }
  }
}

function onRequestError(
  details: chrome.webRequest.WebResponseErrorDetails
): void {
  if (!state.connected) return;

  const err = details.error;
  // Detect DPI-induced errors
  if (
    err === 'net::ERR_CONNECTION_RESET' ||
    err === 'net::ERR_CONNECTION_REFUSED' ||
    err === 'net::ERR_SSL_PROTOCOL_ERROR'
  ) {
    state.blockedCount++;
    state.lastBlockTime = Date.now();
    console.warn('[UnifiedShield] Possible DPI block:', details.url, err);

    // Auto-switch relay if too many blocks
    if (state.blockedCount > 5 && config.webrtcFallback && !state.webrtcActive) {
      console.log('[UnifiedShield] High block count, switching to WebRTC relay');
      startProxy();
    }
  }
}

/* ────────────────────── alarms ────────────────────── */

async function handleAlarm(alarm: chrome.alarms.Alarm): Promise<void> {
  switch (alarm.name) {
    case 'isp-check': {
      const isp = await ispDetector.detect();
      if (isp?.name !== state.ispDetected?.name) {
        console.log('[UnifiedShield] ISP changed:', isp?.name);
        state.ispDetected = isp;
        // Re-generate PAC with new ISP info
        pacGenerator.updateISP(isp);
        if (state.connected) {
          await proxyManager.setProxy(config.socksHost, config.socksPort);
        }
        await saveConfig();
      }
      break;
    }
    case 'config-sync': {
      await loadConfig();
      break;
    }
  }
}

/* ────────────────────── messaging ────────────────────── */

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  (async () => {
    switch (message.type) {
      case 'GET_STATE':
        sendResponse(state);
        break;

      case 'GET_CONFIG':
        sendResponse(config);
        break;

      case 'UPDATE_CONFIG':
        config = { ...config, ...message.payload };
        await saveConfig();
        // Re-init with new config
        pacGenerator.updateConfig(config);
        proxyManager.updateConfig(config);
        dohResolver.updateConfig(config);
        if (state.connected) {
          await startProxy();
        }
        sendResponse({ ok: true });
        break;

      case 'START_PROXY':
        await startProxy();
        sendResponse(state);
        break;

      case 'STOP_PROXY':
        await stopProxy();
        sendResponse(state);
        break;

      case 'TOGGLE_PROXY':
        if (state.connected) {
          await stopProxy();
        } else {
          await startProxy();
        }
        sendResponse(state);
        break;

      case 'DETECT_ISP':
        const isp = await ispDetector.detect();
        state.ispDetected = isp;
        await saveConfig();
        sendResponse(isp);
        break;

      case 'DNS_RESOLVE':
        if (!dohResolver) {
          sendResponse({ error: 'DoH not initialized' });
          return;
        }
        const result = await dohResolver.resolve(message.hostname, message.rrType);
        sendResponse(result);
        break;

      case 'GET_RELAY_PEERS':
        sendResponse(webRelay?.getPeers() ?? []);
        break;

      default:
        sendResponse({ error: 'Unknown message type' });
    }
  })();
  return true; // keep channel open for async
});

/* ────────────────────── native messaging ────────────────────── */

let nativePort: chrome.runtime.Port | null = null;

function connectNative(): void {
  try {
    nativePort = chrome.runtime.connectNative('com.unifiedshield.native');
    nativePort.onMessage.addListener((msg) => {
      console.log('[UnifiedShield] Native message:', msg);
      if (msg.type === 'SOCKS_READY') {
        state.connected = true;
        state.mode = 'socks5';
        saveConfig();
      }
    });
    nativePort.onDisconnect.addListener(() => {
      console.warn('[UnifiedShield] Native app disconnected');
      nativePort = null;
      if (state.mode === 'socks5') {
        state.connected = false;
        // Attempt WebRTC fallback
        if (config.webrtcFallback) {
          startProxy();
        }
      }
    });
  } catch {
    console.warn('[UnifiedShield] Native app not available');
  }
}

// Auto-detect native app on startup
if (config.nativeAppEnabled) {
  connectNative();
}

console.log('[UnifiedShield] Service worker loaded');
