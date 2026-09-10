/**
 * UnifiedShield Rate Limiter
 *
 * In-memory LRU rate limiter with optional Alibaba Cloud Table Store backend.
 * Limits: max 1000 connections per IP per hour.
 */

interface RateLimitEntry {
  count: number;
  windowStart: number;
  lastAccess: number;
}

const DEFAULT_MAX_REQUESTS = 1000;
const DEFAULT_WINDOW_SECONDS = 3600;
const MAX_ENTRIES = 100000;
const CLEANUP_INTERVAL_MS = 120000;
const EVICT_BATCH_SIZE = 1000;

export class RateLimiter {
  private maxRequests: number;
  private windowSeconds: number;
  private entries: Map<string, RateLimitEntry> = new Map();
  private cleanupTimer: NodeJS.Timeout | null = null;
  private tableStoreClient: any | null = null;

  constructor(maxRequests: number = DEFAULT_MAX_REQUESTS, windowSeconds: number = DEFAULT_WINDOW_SECONDS) {
    this.maxRequests = maxRequests;
    this.windowSeconds = windowSeconds;
    this.startCleanup();
  }

  /**
   * Initialize Alibaba Cloud Table Store as persistent rate limit backend.
   */
  async initWithTableStore(endpoint: string, instanceName: string, tableName: string): Promise<void> {
    try {
      const TableStore = require('@alicloud/tablestore');
      this.tableStoreClient = new TableStore.Client({
        accessKeyId: process.env.ALIBABA_ACCESS_KEY_ID || '',
        accessKeySecret: process.env.ALIBABA_ACCESS_KEY_SECRET || '',
        endpoint,
        instancename: instanceName,
      });

      const params = {
        tableMeta: {
          tableName,
          primaryKey: [{ name: 'ip', type: 'STRING' }],
        },
        reservedThroughput: {
          capacityUnit: { read: 0, write: 0 },
        },
        tableOptions: {
          timeToLive: this.windowSeconds,
          maxVersions: 1,
        },
        streamSpecification: {
          enableStream: false,
        },
      };

      try {
        await this.tableStoreClient.createTable(params);
      } catch (err: any) {
        if (!err.message?.includes('already exists')) {
          console.warn('Table Store table creation warning:', err.message);
        }
      }
    } catch (err: any) {
      console.warn('Table Store not available, using in-memory fallback:', err.message);
      this.tableStoreClient = null;
    }
  }

  /**
   * Check if a request from the given IP is allowed.
   */
  check(ip: string): boolean {
    const now = Math.floor(Date.now() / 1000);
    const entry = this.entries.get(ip);

    if (!entry || now - entry.windowStart >= this.windowSeconds) {
      this.evictIfNeeded();
      this.entries.set(ip, {
        count: 1,
        windowStart: now,
        lastAccess: now,
      });
      return true;
    }

    entry.lastAccess = now;
    if (entry.count >= this.maxRequests) {
      return false;
    }

    entry.count++;
    return true;
  }

  /**
   * Get remaining requests for an IP in the current window.
   */
  getRemaining(ip: string): number {
    const now = Math.floor(Date.now() / 1000);
    const entry = this.entries.get(ip);
    if (!entry || now - entry.windowStart >= this.windowSeconds) {
      return this.maxRequests;
    }
    return Math.max(0, this.maxRequests - entry.count);
  }

  /**
   * Get the time until the rate limit window resets for an IP.
   */
  getResetTime(ip: string): number {
    const now = Math.floor(Date.now() / 1000);
    const entry = this.entries.get(ip);
    if (!entry || now - entry.windowStart >= this.windowSeconds) {
      return 0;
    }
    return this.windowSeconds - (now - entry.windowStart);
  }

  /**
   * Increment count for distributed Table Store backend.
   */
  async incrementDistributed(ip: string): Promise<boolean> {
    if (!this.tableStoreClient) {
      return this.check(ip);
    }

    try {
      const now = Date.now();
      const windowStart = now - (this.windowSeconds * 1000);
      const long = require('@alicloud/tablestore').Long;

      const getParams = {
        tableName: 'unifiedshield_rate_limits',
        primaryKey: [{ ip }],
      };

      let currentCount = 0;
      try {
        const result = await this.tableStoreClient.getRow(getParams);
        const row = result.row;
        if (row && row.attributes) {
          const countAttr = row.attributes.find((a: any) => a.name === 'count');
          const windowAttr = row.attributes.find((a: any) => a.name === 'windowStart');
          if (countAttr && windowAttr) {
            const ws = parseInt(windowAttr.value, 10);
            if (now - ws < this.windowSeconds * 1000) {
              currentCount = parseInt(countAttr.value, 10);
            } else {
              currentCount = 0;
            }
          }
        }
      } catch {
        currentCount = 0;
      }

      if (currentCount >= this.maxRequests) {
        return false;
      }

      const updateParams = {
        tableName: 'unifiedshield_rate_limits',
        condition: {
          rowExistence: 'IGNORE',
        },
        primaryKey: [{ ip }],
        attributeColumns: [
          { count: String(currentCount + 1) },
          { windowStart: currentCount === 0 ? String(now) : undefined },
          { lastAccess: String(now) },
        ].filter((c) => c[String(Object.keys(c)[0])] !== undefined) as any,
      };

      await this.tableStoreClient.updateRow(updateParams);
      return true;
    } catch (err: any) {
      console.warn('Table Store rate limit failed, falling back to memory:', err.message);
      return this.check(ip);
    }
  }

  /**
   * Reset the rate limit for a specific IP.
   */
  reset(ip: string): void {
    this.entries.delete(ip);
  }

  /**
   * Get the current number of tracked IPs.
   */
  size(): number {
    return this.entries.size;
  }

  private startCleanup(): void {
    this.cleanupTimer = setInterval(() => {
      this.cleanup();
    }, CLEANUP_INTERVAL_MS);
  }

  private cleanup(): void {
    const now = Math.floor(Date.now() / 1000);
    for (const [ip, entry] of this.entries) {
      if (now - entry.windowStart >= this.windowSeconds * 2) {
        this.entries.delete(ip);
      }
    }
  }

  private evictIfNeeded(): void {
    if (this.entries.size < MAX_ENTRIES) return;

    const entries = Array.from(this.entries.entries());
    entries.sort((a, b) => a[1].lastAccess - b[1].lastAccess);

    for (let i = 0; i < EVICT_BATCH_SIZE && i < entries.length; i++) {
      this.entries.delete(entries[i][0]);
    }
  }

  destroy(): void {
    if (this.cleanupTimer) {
      clearInterval(this.cleanupTimer);
      this.cleanupTimer = null;
    }
    this.entries.clear();
    this.tableStoreClient = null;
  }
}
