// @ts-nocheck
/**
 * Firefox DoH Resolver — Same as Chrome version
 * Uses Chinese CDN endpoints (Cloudflare blocked in Iran)
 */

import type { UnifiedShieldConfig, DohResponse, DNSRecord } from '../../shared/protocol';

const DOH_SERVERS = {
  alidns: { url: 'https://dns.alidns.com/dns-query', hostname: 'dns.alidns.com' },
  dnspod: { url: 'https://doh.pub/dns-query', hostname: 'doh.pub' },
  byteplus: { url: 'https://dns.byteplus.com/dns-query', hostname: 'dns.byteplus.com' },
  google: { url: 'https://dns.google/dns-query', hostname: 'dns.google' },
  quad9: { url: 'https://dns.quad9.net/dns-query', hostname: 'dns.quad9.net' },
  mullvad: { url: 'https://dns.mullvad.net/dns-query', hostname: 'dns.mullvad.net' },
} as const;

const POISONED_IPS = new Set(['10.10.34.34', '10.10.34.35', '0.0.0.0', '127.0.0.1']);

const RR_TYPE: Record<string, number> = {
  A: 1, AAAA: 28, CNAME: 5, MX: 15, TXT: 16, NS: 2,
};

export class DohResolver {
  private config: UnifiedShieldConfig;
  private cache: Map<string, { records: DNSRecord[]; expires: number }> = new Map();
  private serverOrder: string[] = ['alidns', 'dnspod', 'byteplus', 'quad9', 'mullvad'];

  constructor(config: UnifiedShieldConfig) {
    this.config = config;
    this.updateConfig(config);
  }

  updateConfig(config: UnifiedShieldConfig): void {
    this.config = config;
    if (config.dohServers?.length) {
      this.serverOrder = config.dohServers;
    }
  }

  async resolve(hostname: string, rrType: string | number = 'A'): Promise<DohResponse> {
    const type = typeof rrType === 'string' ? (RR_TYPE[rrType] ?? 1) : rrType;
    const cacheKey = `${hostname}:${type}`;

    const cached = this.cache.get(cacheKey);
    if (cached && cached.expires > Date.now()) {
      return { hostname, type: String(rrType), records: cached.records, fromCache: true, poisoned: false };
    }

    for (const key of this.serverOrder) {
      const server = DOH_SERVERS[key as keyof typeof DOH_SERVERS];
      if (!server) continue;

      try {
        const records = await this.queryServer(server, hostname, type);
        const poisoned = records.some((r) => r.type === 1 && POISONED_IPS.has(r.data));
        if (poisoned) continue;

        const minTtl = Math.min(...records.map((r) => r.ttl || 300), 3600);
        this.cache.set(cacheKey, { records, expires: Date.now() + minTtl * 1000 });

        return { hostname, type: String(rrType), records, fromCache: false, poisoned: false };
      } catch {
        continue;
      }
    }

    return { hostname, type: String(rrType), records: [], fromCache: false, poisoned: false, error: 'All DoH servers failed' };
  }

  private async queryServer(
    server: { url: string },
    hostname: string,
    type: number
  ): Promise<DNSRecord[]> {
    const url = new URL(server.url);
    url.searchParams.set('name', hostname);
    url.searchParams.set('type', type.toString());

    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 8000);

    try {
      const response = await fetch(url.toString(), {
        headers: { Accept: 'application/dns-json' },
        signal: controller.signal,
      });
      clearTimeout(timeout);

      if (!response.ok) throw new Error(`DoH HTTP ${response.status}`);

      const data = await response.json();
      const records: DNSRecord[] = [];

      if (data.Answer) {
        for (const a of data.Answer) {
          if (a.type === 1 && POISONED_IPS.has(a.data)) continue;
          records.push({ name: a.name, type: a.type, ttl: a.TTL ?? 300, data: a.data });
        }
      }

      return records;
    } finally {
      clearTimeout(timeout);
    }
  }

  clearCache(): void {
    this.cache.clear();
  }
}
