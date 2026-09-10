// @ts-nocheck
// @ts-nocheck
/**
 * UnifiedShield NextGen — Firefox Background Script (MV2)
 * Uses browser.* APIs with fallback to chrome.*
 */

import { ProxyManager } from './proxy-manager';
import { PacGenerator } from './pac-generator';
import { DohResolver } from './doh-resolver';
import { ISPDetector } from './isp-detector';
import {
  DEFAULT_CONFIG,
  StorageKeys,
  type UnifiedShieldConfig,
  type ProxyState,
} from '../../shared/protocol';

// Use browser API (Firefox) with chrome fallback
const api = typeof browser !== 'undefined' ? browser : chrome;

/* ────────────────────── state ────────────────────── */

let config: UnifiedShieldConfig = { ...DEFAULT_CONFIG };
let proxyManager: ProxyManager;
let pacGenerator: PacGenerator;
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

api.runtime.onInstalled.addListener(async (details) => {
  console.log('[UnifiedShield-FF] Installed:', details.reason);
  await loadConfig();
  await initModules();
});

api.runtime.onStartup.addListener(async () => {
  await loadConfig();
  await initModules();
});

/* ────────────────────── config ────────────────────── */

async function loadConfig(): Promise<void> {
  const stored = await api.storage.local.get(StorageKeys.CONFIG);
  if (stored[StorageKeys.CONFIG]) {
    config = { ...DEFAULT_CONFIG, ...stored[StorageKeys.CONFIG] };
  }
  const stateStored = await api.storage.local.get(StorageKeys.STATE);
  if (stateStored[StorageKeys.STATE]) {
    Object.assign(state, stateStored[StorageKeys.STATE]);
  }
}

async function saveConfig(): Promise<void> {
  await api.storage.local.set({ [StorageKeys.CONFIG]: config });
  await api.storage.local.set({ [StorageKeys.STATE]: state });
}

/* ────────────────────── init ────────────────────── */

async function initModules(): Promise<void> {
  ispDetector = new ISPDetector(config);
  pacGenerator = new PacGenerator(config);
  proxyManager = new ProxyManager(config, pacGenerator);
  dohResolver = new DohResolver(config);

  // Detect ISP
  const isp = await ispDetector.detect();
  state.ispDetected = isp;

  // Auto-start
  if (config.autoStart) {
    await startProxy();
  }

  // Periodic checks
  api.alarms.create('isp-check', { periodInMinutes: 30 });
  api.alarms.onAlarm.addListener(async (alarm) => {
    if (alarm.name === 'isp-check') {
      const newIsp = await ispDetector.detect();
      if (newIsp?.name !== state.ispDetected?.name) {
        state.ispDetected = newIsp;
        pacGenerator.updateISP(newIsp);
        if (state.connected) {
          await proxyManager.setProxy(config.socksHost, config.socksPort);
        }
        await saveConfig();
      }
    }
  });

  // Request monitoring — Firefox MV2 supports blocking
  api.webRequest.onBeforeRequest.addListener(
    (details: any) => {
      if (!state.connected) return;
      // Block known DNS-poisoned requests
      if (config.dohEnabled && details.url) {
        try {
          const url = new URL(details.url);
          if (config.dohBlocklist.includes(url.hostname)) {
            state.blockedCount++;
            state.lastBlockTime = Date.now();
            return { cancel: true };
          }
        } catch { /* ignore */ }
      }
    },
    { urls: ['<all_urls>'] },
    ['blocking']
  );

  api.webRequest.onErrorOccurred.addListener(
    (details: any) => {
      const err = details.error;
      if (
        err === 'NS_ERROR_CONNECTION_REFUSED' ||
        err === 'NS_ERROR_NET_RESET' ||
        err === 'NS_ERROR_SSL_PROTOCOL_ERROR'
      ) {
        state.blockedCount++;
        state.lastBlockTime = Date.now();
      }
    },
    { urls: ['<all_urls>'] }
  );
}

/* ────────────────────── proxy control ────────────────────── */

async function startProxy(): Promise<void> {
  try {
    await proxyManager.setProxy(config.socksHost, config.socksPort);
    state.connected = true;
    state.mode = 'socks5';
    await saveConfig();
  } catch (err) {
    console.error('[UnifiedShield-FF] Proxy start failed:', err);
    state.connected = false;
    await saveConfig();
  }
}

async function stopProxy(): Promise<void> {
  await proxyManager.clearProxy();
  state.connected = false;
  state.mode = 'direct';
  await saveConfig();
}

/* ────────────────────── messaging ────────────────────── */

api.runtime.onMessage.addListener((message, _sender, sendResponse) => {
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
        const result = await dohResolver.resolve(message.hostname, message.rrType);
        sendResponse(result);
        break;

      default:
        sendResponse({ error: 'Unknown message type' });
    }
  })();
  return true;
});

/* ────────────────────── native messaging ────────────────────── */

let nativePort: any = null;

function connectNative(): void {
  try {
    nativePort = api.runtime.connectNative('com.unifiedshield.native');
    nativePort.onMessage.addListener((msg: any) => {
      if (msg.type === 'SOCKS_READY') {
        state.connected = true;
        state.mode = 'socks5';
        saveConfig();
      }
    });
    nativePort.onDisconnect.addListener(() => {
      nativePort = null;
      if (state.mode === 'socks5') {
        state.connected = false;
        saveConfig();
      }
    });
  } catch {
    console.warn('[UnifiedShield-FF] Native app not available');
  }
}

if (config.nativeAppEnabled) {
  connectNative();
}

console.log('[UnifiedShield-FF] Background script loaded');
