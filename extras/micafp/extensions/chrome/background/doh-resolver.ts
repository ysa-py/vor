/**
 * DNS-over-HTTPS Resolver
 * Resolves DNS queries over HTTPS to bypass ISP DNS poisoning.
 * Uses Chinese CDNs as primary (Cloudflare blocked in Iran).
 */

import type { UnifiedShieldConfig, DohResponse, DNSRecord } from '../../shared/protocol';

/** DoH server configurations — Chinese CDNs first, others as fallback */
const DOH_SERVERS = {
  // Chinese CDN servers (primary — not blocked in Iran)
  alidns: {
    url: 'https://dns.alidns.com/dns-query',
    hostname: 'dns.alidns.com',
  },
  dnspod: {
    url: 'https://doh.pub/dns-query',
    hostname: 'doh.pub',
  },
  byteplus: {
    url: 'https://dns.byteplus.com/dns-query',
    hostname: 'dns.byteplus.com',
  },
  // Fallback servers
  google: {
    url: 'https://dns.google/dns-query',
    hostname: 'dns.google',
  },
  quad9: {
    url: 'https://dns.quad9.net/dns-query',
    hostname: 'dns.quad9.net',
  },
  mullvad: {
    url: 'https://dns.mullvad.net/dns-query',
    hostname: 'dns.mullvad.net',
  },
} as const;

/** Known poisoned DNS responses (Iranian DNS poisoning IPs) */
const POISONED_IPS = new Set([
  '10.10.34.34',
  '10.10.34.35',
  '0.0.0.0',
  '127.0.0.1',
  '::1',
  '::',
]);

/** DNS RR types */
const RR_TYPE = {
  A: 1,
  AAAA: 28,
  CNAME: 5,
  MX: 15,
  TXT: 16,
  NS: 2,
} as const;

type RRTypeName = keyof typeof RR_TYPE;

export class DohResolver {
  private config: UnifiedShieldConfig;
  private cache: Map<string, { records: DNSRecord[]; expires: number }> = new Map();
  private serverOrder: string[] = ['alidns', 'dnspod', 'byteplus', 'quad9', 'mullvad', 'google'];
  private requestCount: number = 0;

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

  /**
   * Resolve a hostname using DNS-over-HTTPS
   */
  async resolve(
    hostname: string,
    rrType: RRTypeName | number = 'A'
  ): Promise<DohResponse> {
    const type = typeof rrType === 'string' ? RR_TYPE[rrType] : rrType;
    const cacheKey = `${hostname}:${type}`;

    // Check cache
    const cached = this.cache.get(cacheKey);
    if (cached && cached.expires > Date.now()) {
      return {
        hostname,
        type: rrType as string,
        records: cached.records,
        fromCache: true,
        poisoned: false,
      };
    }

    // Try each DoH server in order
    let lastError: Error | null = null;

    for (const serverKey of this.serverOrder) {
      const server = DOH_SERVERS[serverKey as keyof typeof DOH_SERVERS];
      if (!server) continue;

      try {
        const records = await this.queryServer(server, hostname, type);

        // Check for DNS poisoning
        const poisoned = records.some(
          (r) => r.type === 1 && POISONED_IPS.has(r.data)
        );

        if (poisoned) {
          console.warn(`[DoH] Poisoned response from ${serverKey} for ${hostname}`);
          continue; // Try next server
        }

        // Cache the result
        const minTtl = Math.min(...records.map((r) => r.ttl || 300), 3600);
        this.cache.set(cacheKey, {
          records,
          expires: Date.now() + minTtl * 1000,
        });

        return {
          hostname,
          type: rrType as string,
          records,
          fromCache: false,
          poisoned: false,
        };
      } catch (err) {
        lastError = err as Error;
        console.warn(`[DoH] Server ${serverKey} failed:`, err);
        continue;
      }
    }

    // All servers failed
    return {
      hostname,
      type: rrType as string,
      records: [],
      fromCache: false,
      poisoned: false,
      error: lastError?.message ?? 'All DoH servers failed',
    };
  }

  /**
   * Query a specific DoH server using JSON API (RFC 8427)
   */
  private async queryServer(
    server: { url: string; hostname: string },
    hostname: string,
    type: number
  ): Promise<DNSRecord[]> {
    const url = new URL(server.url);
    url.searchParams.set('name', hostname);
    url.searchParams.set('type', type.toString());
    url.searchParams.set('do', 'false'); // Don't want DNSSEC
    url.searchParams.set('cd', 'false'); // Don't disable checking

    this.requestCount++;

    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 8000);

    try {
      const response = await fetch(url.toString(), {
        method: 'GET',
        headers: {
          Accept: 'application/dns-json',
        },
        signal: controller.signal,
      });

      clearTimeout(timeout);

      if (!response.ok) {
        throw new Error(`DoH HTTP ${response.status}`);
      }

      const data = await response.json();
      return this.parseDohResponse(data);
    } finally {
      clearTimeout(timeout);
    }
  }

  /**
   * Parse DNS-over-HTTPS JSON response (RFC 8427 format)
   */
  private parseDohResponse(data: any): DNSRecord[] {
    const records: DNSRecord[] = [];

    if (!data.Answer) {
      return records;
    }

    for (const answer of data.Answer) {
      const record: DNSRecord = {
        name: answer.name,
        type: answer.type,
        ttl: answer.TTL ?? 300,
        data: answer.data,
      };

      // Validate: skip poisoned IPs
      if (record.type === 1 && POISONED_IPS.has(record.data)) {
        console.warn(`[DoH] Skipping poisoned record: ${record.name} → ${record.data}`);
        continue;
      }

      records.push(record);
    }

    return records;
  }

  /**
   * Bulk resolve multiple hostnames
   */
  async resolveBatch(
    hostnames: string[],
    rrType: RRTypeName = 'A'
  ): Promise<Map<string, DohResponse>> {
    const results = new Map<string, DohResponse>();

    const promises = hostnames.map(async (hostname) => {
      const result = await this.resolve(hostname, rrType);
      results.set(hostname, result);
    });

    await Promise.allSettled(promises);
    return results;
  }

  /**
   * Check if a hostname is likely DNS-poisoned by comparing responses
   * from multiple servers
   */
  async detectPoisoning(hostname: string): Promise<{
    poisoned: boolean;
    consensus: string[];
    poisonedServers: string[];
  }> {
    const responses = new Map<string, string[]>();
    const poisonedServers: string[] = [];

    for (const serverKey of this.serverOrder) {
      const server = DOH_SERVERS[serverKey as keyof typeof DOH_SERVERS];
      if (!server) continue;

      try {
        const records = await this.queryServer(server, hostname, RR_TYPE.A);
        const ips = records
          .filter((r) => r.type === 1)
          .map((r) => r.data);

        responses.set(serverKey, ips);

        // Directly poisoned
        if (ips.some((ip) => POISONED_IPS.has(ip))) {
          poisonedServers.push(serverKey);
        }
      } catch {
        // Server failed, skip
      }
    }

    // Find consensus (most common IP set)
    const ipSets = Array.from(responses.values());
    const consensus = this.findConsensus(ipSets);

    return {
      poisoned: poisonedServers.length > 0,
      consensus,
      poisonedServers,
    };
  }

  private findConsensus(ipSets: string[][]): string[] {
    if (ipSets.length === 0) return [];

    const freq = new Map<string, number>();
    for (const ips of ipSets) {
      for (const ip of ips) {
        freq.set(ip, (freq.get(ip) || 0) + 1);
      }
    }

    const threshold = Math.ceil(ipSets.length / 2);
    return Array.from(freq.entries())
      .filter(([_, count]) => count >= threshold)
      .map(([ip]) => ip);
  }

  /**
   * Clear the DNS cache
   */
  clearCache(): void {
    this.cache.clear();
  }

  /**
   * Get cache stats
   */
  getCacheStats(): { size: number; hitRate: number } {
    return {
      size: this.cache.size,
      hitRate: this.requestCount > 0 ? this.cache.size / this.requestCount : 0,
    };
  }
}
