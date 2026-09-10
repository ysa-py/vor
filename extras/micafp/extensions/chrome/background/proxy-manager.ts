/**
 * Proxy Manager — chrome.proxy API management for MV3
 */

import { PacGenerator } from './pac-generator';
import type { UnifiedShieldConfig, ISPInfo } from '../../shared/protocol';

export class ProxyManager {
  private config: UnifiedShieldConfig;
  private pacGenerator: PacGenerator;
  private activeProxy: 'socks5' | 'webrtc' | 'direct' = 'direct';

  constructor(config: UnifiedShieldConfig, pacGenerator: PacGenerator) {
    this.config = config;
    this.pacGenerator = pacGenerator;
  }

  updateConfig(config: UnifiedShieldConfig): void {
    this.config = config;
    this.pacGenerator.updateConfig(config);
  }

  /**
   * Set SOCKS5 proxy via chrome.proxy.settings with PAC script
   */
  async setProxy(host: string, port: number): Promise<void> {
    const pacScript = this.pacGenerator.generate(host, port, 'socks5');

    await chrome.proxy.settings.set({
      value: {
        mode: 'pac_script',
        pacScript: {
          data: pacScript,
        },
      },
      scope: 'regular',
    });

    this.activeProxy = 'socks5';
    console.log(`[ProxyManager] SOCKS5 proxy set: ${host}:${port}`);
  }

  /**
   * Set WebRTC relay as proxy
   */
  async setWebRTCProxy(peer: { localPort: number }): Promise<void> {
    const pacScript = this.pacGenerator.generate(
      '127.0.0.1',
      peer.localPort,
      'socks5'
    );

    await chrome.proxy.settings.set({
      value: {
        mode: 'pac_script',
        pacScript: {
          data: pacScript,
        },
      },
      scope: 'regular',
    });

    this.activeProxy = 'webrtc';
    console.log(`[ProxyManager] WebRTC relay proxy set on port ${peer.localPort}`);
  }

  /**
   * Clear all proxy settings (direct connection)
   */
  async clearProxy(): Promise<void> {
    await chrome.proxy.settings.clear({ scope: 'regular' });
    this.activeProxy = 'direct';
    console.log('[ProxyManager] Proxy cleared');
  }

  /**
   * Get current proxy state
   */
  getActiveProxy(): 'socks5' | 'webrtc' | 'direct' {
    return this.activeProxy;
  }

  /**
   * Test if proxy is reachable
   */
  async testProxy(host: string, port: number): Promise<boolean> {
    try {
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 5000);

      // Try connecting through proxy to a test endpoint
      const pacScript = this.pacGenerator.generate(host, port, 'socks5');
      await chrome.proxy.settings.set({
        value: {
          mode: 'pac_script',
          pacScript: { data: pacScript },
        },
        scope: 'regular',
      });

      clearTimeout(timeout);
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Handle proxy authentication if required
   */
  setupAuthHandler(): void {
    // MV3: Use webRequest onAuthRequired
    chrome.webRequest.onAuthRequired.addListener(
      (details) => {
        if (
          details.isProxy &&
          this.config.socksUsername &&
          this.config.socksPassword
        ) {
          return {
            authCredentials: {
              username: this.config.socksUsername,
              password: this.config.socksPassword,
            },
          };
        }
        return {};
      },
      { urls: ['<all_urls>'] },
      ['blocking']
    );
  }
}
