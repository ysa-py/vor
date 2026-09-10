// @ts-nocheck
// @ts-nocheck
/**
 * Firefox Proxy Manager — browser.proxy API
 * Firefox uses browser.proxy.onRequest instead of chrome.proxy.settings
 */

import { PacGenerator } from './pac-generator';
import type { UnifiedShieldConfig } from '../../shared/protocol';

const api = typeof browser !== 'undefined' ? browser : chrome;

export class ProxyManager {
  private config: UnifiedShieldConfig;
  private pacGenerator: PacGenerator;
  private activeProxy: 'socks5' | 'direct' = 'direct';
  private isListening = false;

  constructor(config: UnifiedShieldConfig, pacGenerator: PacGenerator) {
    this.config = config;
    this.pacGenerator = pacGenerator;
  }

  updateConfig(config: UnifiedShieldConfig): void {
    this.config = config;
    this.pacGenerator.updateConfig(config);
  }

  /**
   * Set proxy — Firefox uses browser.proxy.settings or onRequest
   */
  async setProxy(host: string, port: number): Promise<void> {
    // Method 1: Use browser.proxy.settings (Firefox 91+)
    if (api.proxy?.settings) {
      const pacScript = this.pacGenerator.generate(host, port, 'socks5');

      await api.proxy.settings.set({
        value: {
          proxyType: 'pacScript',
          pacScript,
        },
      });

      this.activeProxy = 'socks5';
      console.log(`[ProxyManager-FF] PAC proxy set: ${host}:${port}`);
      return;
    }

    // Method 2: Use browser.proxy.onRequest (Firefox MV2)
    if (api.proxy?.onRequest && !this.isListening) {
      this.setupOnRequestListener(host, port);
      this.isListening = true;
      this.activeProxy = 'socks5';
      console.log(`[ProxyManager-FF] onRequest proxy set: ${host}:${port}`);
      return;
    }

    // Method 3: Fallback to chrome.proxy.settings
    if (api.proxy?.settings) {
      const pacScript = this.pacGenerator.generate(host, port, 'socks5');
      await (api.proxy.settings as any).set({
        value: {
          mode: 'pac_script',
          pacScript: { data: pacScript },
        },
        scope: 'regular',
      });

      this.activeProxy = 'socks5';
    }
  }

  /**
   * Setup browser.proxy.onRequest listener for per-request proxy
   */
  private setupOnRequestListener(host: string, port: number): void {
    if (!api.proxy?.onRequest) return;

    api.proxy.onRequest.addListener(
      (details: any) => {
        // Check if URL should bypass proxy
        const url = new URL(details.url);

        // .ir domains go direct
        if (url.hostname.endsWith('.ir')) {
          return { type: 'direct' };
        }

        // Everything else via SOCKS5
        return {
          type: 'socks',
          host,
          port,
          proxyDNS: true,
        };
      },
      { urls: ['<all_urls>'] }
    );
  }

  /**
   * Clear proxy settings
   */
  async clearProxy(): Promise<void> {
    if (api.proxy?.settings) {
      try {
        await api.proxy.settings.clear({});
      } catch {
        // Fallback: set to direct
        await api.proxy.settings.set({
          value: { proxyType: 'direct' },
        });
      }
    }

    this.activeProxy = 'direct';
    this.isListening = false;
    console.log('[ProxyManager-FF] Proxy cleared');
  }

  getActiveProxy(): 'socks5' | 'direct' {
    return this.activeProxy;
  }
}
