/**
 * Rate Limiter — Token bucket rate limiting for the relay worker
 */

interface Bucket {
  tokens: number;
  lastRefill: number;
  maxTokens: number;
}

export class RateLimiter {
  private buckets: Map<string, Bucket> = new Map();
  private cleanupInterval = 60_000; // 1 minute
  private lastCleanup = Date.now();

  /**
   * Check if a request is allowed under the rate limit.
   * Uses token bucket algorithm.
   *
   * @param key - Identifier (usually IP address)
   * @param maxRPM - Maximum requests per minute
   * @returns true if allowed, false if rate limited
   */
  check(key: string, maxRPM: number): boolean {
    this.maybeCleanup();

    const now = Date.now();
    let bucket = this.buckets.get(key);

    if (!bucket) {
      bucket = {
        tokens: maxRPM - 1, // Consume one token for this request
        lastRefill: now,
        maxTokens: maxRPM,
      };
      this.buckets.set(key, bucket);
      return true;
    }

    // Refill tokens based on time elapsed
    const elapsed = now - bucket.lastRefill;
    const tokensToAdd = (elapsed / 60_000) * maxRPM;
    bucket.tokens = Math.min(bucket.maxTokens, bucket.tokens + tokensToAdd);
    bucket.lastRefill = now;

    // Check if we have a token available
    if (bucket.tokens >= 1) {
      bucket.tokens -= 1;
      return true;
    }

    return false;
  }

  /**
   * Clean up old buckets
   */
  private maybeCleanup(): void {
    const now = Date.now();
    if (now - this.lastCleanup < this.cleanupInterval) return;

    const cutoff = now - 5 * 60_000; // Remove buckets inactive for 5 minutes
    for (const [key, bucket] of this.buckets) {
      if (bucket.lastRefill < cutoff) {
        this.buckets.delete(key);
      }
    }

    this.lastCleanup = now;
  }

  /**
   * Get rate limiter stats
   */
  getStats(): { activeBuckets: number; totalTokens: number } {
    let totalTokens = 0;
    for (const bucket of this.buckets.values()) {
      totalTokens += Math.floor(bucket.tokens);
    }
    return {
      activeBuckets: this.buckets.size,
      totalTokens,
    };
  }

  /**
   * Reset all rate limits
   */
  reset(): void {
    this.buckets.clear();
  }
}
