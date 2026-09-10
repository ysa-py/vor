// @ts-nocheck
/**
 * Firefox ISP Detector — Same logic as Chrome
 */

import type { UnifiedShieldConfig, ISPInfo } from '../../shared/protocol';
import { ISP_DATABASE } from '../../shared/isp-database';

const IP_DETECT_ENDPOINTS = [
  'https://myip.ipip.net',
  'https://ipinfo.io/json',
  'https://api.ip.sb/geoip',
];

export class ISPDetector {
  private config: UnifiedShieldConfig;
  private lastDetection: ISPInfo | null = null;
  private lastDetectionTime: number = 0;
  private cacheTtl = 30 * 60 * 1000;

  constructor(config: UnifiedShieldConfig) {
    this.config = config;
  }

  async detect(): Promise<ISPInfo | null> {
    if (this.lastDetection && Date.now() - this.lastDetectionTime < this.cacheTtl) {
      return this.lastDetection;
    }

    let ipInfo: { ip: string; asn: string; country: string; isp?: string; city?: string } | null = null;

    for (const endpoint of IP_DETECT_ENDPOINTS) {
      try {
        const controller = new AbortController();
        const timeout = setTimeout(() => controller.abort(), 5000);

        const response = await fetch(endpoint, { signal: controller.signal });
        clearTimeout(timeout);

        if (!response.ok) continue;

        const text = await response.text();
        try {
          const data = JSON.parse(text);
          ipInfo = {
            ip: data.ip ?? '',
            asn: data.org?.replace('AS', '').split(' ')[0] ?? data.asn?.toString() ?? '',
            country: data.country ?? data.country_code ?? '',
            isp: data.org ?? data.isp ?? data.organization ?? '',
            city: data.city,
          };
        } catch {
          const ipMatch = text.match(/(\d+\.\d+\.\d+\.\d+)/);
          ipInfo = { ip: ipMatch?.[1] ?? '', asn: '', country: '', isp: '' };
        }

        if (ipInfo) break;
      } catch {
        continue;
      }
    }

    if (!ipInfo) return this.lastDetection;

    const asnNum = parseInt(ipInfo.asn, 10);
    const ispEntry = ISP_DATABASE.find((e) => e.asn === asnNum);

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
    return result;
  }

  async isIranianISP(): Promise<boolean> {
    const info = await this.detect();
    return info?.isIranian ?? false;
  }

  async forceDetect(): Promise<ISPInfo | null> {
    this.lastDetection = null;
    this.lastDetectionTime = 0;
    return this.detect();
  }
}
