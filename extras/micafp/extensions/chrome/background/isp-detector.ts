/**
 * ISP Detector — Detect Iranian ISP from public IP
 * Uses Chinese CDN endpoints (Cloudflare blocked in Iran)
 */

import type { UnifiedShieldConfig, ISPInfo } from '../../shared/protocol';
import { ISP_DATABASE, type ISPEntry } from '../../shared/isp-database';

/** IP detection endpoints — Chinese CDNs primary */
const IP_DETECT_ENDPOINTS = [
  'https://myip.ipip.net',
  'https://ipinfo.io/json',
  'https://api.ip.sb/geoip',
  'https://ip.api.miwifi.com/ip',
  'https://speed.cloudflare.com/__down?bytes=0', // May be blocked in Iran
];

interface IPInfo {
  ip: string;
  asn: string;
  org: string;
  country: string;
  city?: string;
  isp?: string;
}

export class ISPDetector {
  private config: UnifiedShieldConfig;
  private lastDetection: ISPInfo | null = null;
  private lastDetectionTime: number = 0;
  private cacheTtl = 30 * 60 * 1000; // 30 minutes

  constructor(config: UnifiedShieldConfig) {
    this.config = config;
  }

  /**
   * Detect current ISP by querying external IP info services
   */
  async detect(): Promise<ISPInfo | null> {
    // Return cached result if fresh
    if (
      this.lastDetection &&
      Date.now() - this.lastDetectionTime < this.cacheTtl
    ) {
      return this.lastDetection;
    }

    let ipInfo: IPInfo | null = null;

    for (const endpoint of IP_DETECT_ENDPOINTS) {
      try {
        ipInfo = await this.queryEndpoint(endpoint);
        if (ipInfo) break;
      } catch {
        continue;
      }
    }

    if (!ipInfo) {
      console.warn('[ISP] Could not detect ISP');
      return this.lastDetection;
    }

    // Match against ISP database
    const ispEntry = this.matchISP(ipInfo);

    const result: ISPInfo = {
      name: ispEntry?.name ?? ipInfo.isp ?? 'Unknown',
      asn: ipInfo.asn,
      ip: ipInfo.ip,
      country: ipInfo.country,
      city: ipInfo.city,
      isIranian: ipInfo.country === 'IR' || ipInfo.country === 'Iran',
      throttlingProfile: ispEntry?.throttlingProfile ?? 'default',
      knownBlockingPatterns: ispEntry?.knownBlockingPatterns ?? [],
    };

    this.lastDetection = result;
    this.lastDetectionTime = Date.now();

    console.log(
      `[ISP] Detected: ${result.name} (AS${result.asn}) in ${result.country}`
    );

    return result;
  }

  /**
   * Query an IP detection endpoint
   */
  private async queryEndpoint(endpoint: string): Promise<IPInfo | null> {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 5000);

    try {
      const response = await fetch(endpoint, {
        signal: controller.signal,
      });
      clearTimeout(timeout);

      if (!response.ok) return null;

      const text = await response.text();

      // Parse based on endpoint format
      if (endpoint.includes('ipip.net')) {
        // Format: "当前 IP: x.x.x.x  来自于: 中国 北京 北京 电信"
        return this.parseIPIP(text);
      }

      // Try JSON parsing
      try {
        const data = JSON.parse(text);
        return this.parseJSON(data);
      } catch {
        return this.parseIPIP(text);
      }
    } catch {
      clearTimeout(timeout);
      return null;
    }
  }

  private parseIPIP(text: string): IPInfo {
    const ipMatch = text.match(/(\d+\.\d+\.\d+\.\d+)/);
    const info = text.match(/来自于:\s*(.+)/);

    return {
      ip: ipMatch?.[1] ?? '',
      asn: '',
      org: info?.[1] ?? '',
      country: info?.[1]?.includes('伊朗') || info?.[1]?.includes('Iran') ? 'IR' : '',
      isp: info?.[1] ?? '',
    };
  }

  private parseJSON(data: any): IPInfo {
    // ipinfo.io format
    if (data.ip) {
      return {
        ip: data.ip,
        asn: data.org?.replace('AS', '').split(' ')[0] ?? '',
        org: data.org ?? '',
        country: data.country ?? '',
        city: data.city,
        isp: data.org ?? data.isp ?? '',
      };
    }

    // ip.sb format
    if (data.ip && data.asn) {
      return {
        ip: data.ip,
        asn: data.asn?.toString() ?? '',
        org: data.organization ?? '',
        country: data.country_code ?? data.country ?? '',
        city: data.city,
        isp: data.isp ?? data.organization ?? '',
      };
    }

    return null!;
  }

  /**
   * Match detected IP/ASN against the ISP database
   */
  private matchISP(ipInfo: IPInfo): ISPEntry | null {
    const asnNum = parseInt(ipInfo.asn, 10);

    for (const entry of ISP_DATABASE) {
      if (entry.asn === asnNum) {
        return entry;
      }
    }

    // Fuzzy match by name
    const nameLower = (ipInfo.isp || '').toLowerCase();
    for (const entry of ISP_DATABASE) {
      if (nameLower.includes(entry.name.toLowerCase())) {
        return entry;
      }
    }

    return null;
  }

  /**
   * Check if current connection is behind an Iranian ISP
   */
  async isIranianISP(): Promise<boolean> {
    const info = await this.detect();
    return info?.isIranian ?? false;
  }

  /**
   * Get the throttling profile for the current ISP
   */
  async getThrottlingProfile(): Promise<string> {
    const info = await this.detect();
    return info?.throttlingProfile ?? 'default';
  }

  /**
   * Force re-detection (clear cache)
   */
  async forceDetect(): Promise<ISPInfo | null> {
    this.lastDetection = null;
    this.lastDetectionTime = 0;
    return this.detect();
  }
}
