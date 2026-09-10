/**
 * Content Script Injector — Runs on every page
 * Detects and reports censorship/blocking events to the service worker
 */

import { DPI_SIGNATURES } from '../../shared/dpi-signatures';

interface BlockEvent {
  type: 'tls_reset' | 'http_403' | 'dns_poison' | 'sni_filter' | 'connection_reset';
  url: string;
  timestamp: number;
  signature?: string;
}

const detectedBlocks: BlockEvent[] = [];

/* ────────── Error Detection ────────── */

// Monitor for connection errors that indicate DPI interference
window.addEventListener('error', (event) => {
  const target = event.target as HTMLElement | null;
  if (!target) return;

  // Image/resource load failure could indicate blocking
  if (target.tagName === 'IMG' || target.tagName === 'SCRIPT' || target.tagName === 'LINK') {
    const src = (target as HTMLImageElement).src || (target as HTMLLinkElement).href;
    if (src && isExternalUrl(src)) {
      reportBlock({
        type: 'connection_reset',
        url: src,
        timestamp: Date.now(),
      });
    }
  }
}, true);

/* ────────── HTTP 403 Detection ────────── */

// Intercept fetch responses to detect HTTP 403 blocks
const originalFetch = window.fetch;
window.fetch = async function (...args) {
  try {
    const response = await originalFetch.apply(this, args);

    if (response.status === 403) {
      const url = typeof args[0] === 'string' ? args[0] : (args[0] as Request).url;
      const body = await response.clone().text();

      // Check if 403 matches Iranian DPI signature
      if (matchesDpiSignature(body, 'http_403')) {
        reportBlock({
          type: 'http_403',
          url,
          timestamp: Date.now(),
          signature: 'FAVA_HTTP_403',
        });
      }
    }

    return response;
  } catch (err) {
    // Network error — could be TLS reset
    const url = typeof args[0] === 'string' ? args[0] : (args[0] as Request).url;
    reportBlock({
      type: 'tls_reset',
      url,
      timestamp: Date.now(),
    });
    throw err;
  }
};

/* ────────── XMLHttpRequest Monitoring ────────── */

const originalXhrOpen = XMLHttpRequest.prototype.open;
const originalXhrSend = XMLHttpRequest.prototype.send;

XMLHttpRequest.prototype.open = function (method: string, url: string, ...rest: any[]) {
  this._unifiedShieldUrl = url;
  return originalXhrOpen.call(this, method, url, ...rest);
};

XMLHttpRequest.prototype.send = function (...args) {
  this.addEventListener('error', () => {
    if (this._unifiedShieldUrl) {
      reportBlock({
        type: 'connection_reset',
        url: this._unifiedShieldUrl,
        timestamp: Date.now(),
      });
    }
  });

  this.addEventListener('load', () => {
    if (this.status === 403 && this._unifiedShieldUrl) {
      const body = this.responseText;
      if (matchesDpiSignature(body, 'http_403')) {
        reportBlock({
          type: 'http_403',
          url: this._unifiedShieldUrl,
          timestamp: Date.now(),
          signature: 'FAVA_HTTP_403',
        });
      }
    }
  });

  return originalXhrSend.apply(this, args);
};

/* ────────── WebRTC Leak Prevention ────────── */

// Prevent WebRTC IP leaks that could reveal real IP
const originalRTCPeerConnection = window.RTCPeerConnection;

if (originalRTCPeerConnection) {
  window.RTCPeerConnection = function (config?: RTCConfiguration, constraints?: any) {
    // Force relay-only mode to prevent IP leaks
    if (config) {
      config.iceTransportPolicy = 'relay';
    } else {
      config = { iceTransportPolicy: 'relay' };
    }

    // @ts-expect-error: constraints parameter
    return new originalRTCPeerConnection(config, constraints);
  } as any;

  // Copy static properties
  Object.assign(window.RTCPeerConnection, originalRTCPeerConnection);
}

/* ────────── DNS Poisoning Detection ────────── */

// Check for known poisoned IP addresses in page resources
const observer = new MutationObserver((mutations) => {
  for (const mutation of mutations) {
    for (const node of mutation.addedNodes) {
      if (node instanceof HTMLImageElement && node.src) {
        checkPoisonedResource(node.src);
      } else if (node instanceof HTMLScriptElement && node.src) {
        checkPoisonedResource(node.src);
      } else if (node instanceof HTMLLinkElement && node.href) {
        checkPoisonedResource(node.href);
      }
    }
  }
});

observer.observe(document.documentElement, {
  childList: true,
  subtree: true,
});

function checkPoisonedResource(url: string): void {
  try {
    const parsed = new URL(url);
    // If the host resolves to a known poisoned IP, report it
    if (DPI_SIGNATURES.dnsPoisonIPs.some((ip) => parsed.hostname === ip)) {
      reportBlock({
        type: 'dns_poison',
        url,
        timestamp: Date.now(),
        signature: 'DNS_POISON',
      });
    }
  } catch {
    // Invalid URL, skip
  }
}

/* ────────── Helpers ────────── */

function isExternalUrl(url: string): boolean {
  try {
    const parsed = new URL(url, window.location.origin);
    return parsed.hostname !== window.location.hostname;
  } catch {
    return false;
  }
}

function matchesDpiSignature(body: string, type: string): boolean {
  if (type === 'http_403') {
    // Check for Iranian DPI 403 page signatures
    const signatures = DPI_SIGNATURES.http403;
    return signatures.some(
      (sig) => body.includes(sig.pattern)
    );
  }
  return false;
}

function reportBlock(event: BlockEvent): void {
  // Avoid duplicate reports for same URL within 5 seconds
  const recent = detectedBlocks.find(
    (b) => b.url === event.url && Date.now() - b.timestamp < 5000
  );
  if (recent) return;

  detectedBlocks.push(event);

  // Send to service worker
  try {
    chrome.runtime.sendMessage({
      type: 'BLOCK_DETECTED',
      payload: event,
    });
  } catch {
    // Extension context may be invalidated
  }
}

/* ────────── Page Readiness ────────── */

// Notify service worker that content script is loaded
try {
  chrome.runtime.sendMessage({
    type: 'CONTENT_SCRIPT_READY',
    payload: {
      url: window.location.href,
      timestamp: Date.now(),
    },
  });
} catch {
  // Extension context may be invalidated
}

console.log('[UnifiedShield] Content injector loaded');
