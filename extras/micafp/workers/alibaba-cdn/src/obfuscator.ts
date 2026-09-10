/**
 * UnifiedShield Traffic Obfuscator
 *
 * Provides TLS fingerprint simulation, traffic padding,
 * and timing jitter to evade DPI detection.
 */

const TLS_FINGERPRINTS: Record<number, { name: string; cipherSuites: number[]; extensions: number[]; ua: string }> = {
  0: {
    name: 'chrome_120',
    cipherSuites: [0x1301, 0x1302, 0x1303, 0xc02b, 0xc02f, 0xc02c, 0xc030, 0x009e, 0x009c, 0xcca9, 0xcca8],
    extensions: [0x0000, 0x0005, 0x000a, 0x000b, 0x000d, 0x0012, 0x0015, 0x0017, 0x001b, 0x0023, 0x0029, 0x002b, 0x002d, 0x0033, 0xfe0d],
    ua: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
  },
  1: {
    name: 'firefox_121',
    cipherSuites: [0x1301, 0x1302, 0x1303, 0xc02b, 0xc02f, 0xc024, 0xc028, 0xc00a, 0xc009, 0xc013, 0xc014, 0x009e, 0x009c, 0x0039, 0x0033],
    extensions: [0x0000, 0x0005, 0x000a, 0x000b, 0x000d, 0x0015, 0x0017, 0x001b, 0x0023, 0x002b, 0x002d, 0x0033, 0xfe0d],
    ua: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0',
  },
  2: {
    name: 'safari_17',
    cipherSuites: [0x1301, 0x1302, 0x1303, 0xc02c, 0xc030, 0xc02b, 0xc02f, 0x009e, 0x009c, 0xc024, 0xc028, 0xc00a, 0xc009, 0x0039, 0x0033],
    extensions: [0x0000, 0x0005, 0x000a, 0x000b, 0x000d, 0x0012, 0x0015, 0x0017, 0x001b, 0x0023, 0x002b, 0x002d, 0x0033, 0xfe0d],
    ua: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 14_2) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15',
  },
  3: {
    name: 'edge_120',
    cipherSuites: [0x1301, 0x1302, 0x1303, 0xc02b, 0xc02f, 0xc02c, 0xc030, 0x009e, 0x009c, 0xcca9, 0xcca8],
    extensions: [0x0000, 0x0005, 0x000a, 0x000b, 0x000d, 0x0012, 0x0015, 0x0017, 0x001b, 0x0023, 0x0029, 0x002b, 0x002d, 0x0033, 0xfe0d],
    ua: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0',
  },
};

const PADDING_DELIMITER = 0x00;
const MIN_PADDING_SIZE = 8;
const MAX_PADDING_SIZE = 64;
const PADDING_BLOCK_SIZE = 16;

export class Obfuscator {
  private selectedProfile: number;
  private jitterRange: [number, number];

  constructor(profile?: number) {
    this.selectedProfile = profile ?? Math.floor(Math.random() * Object.keys(TLS_FINGERPRINTS).length);
    this.jitterRange = [5, 50];
  }

  /**
   * Simulate a TLS ClientHello fingerprint for the selected browser profile.
   * Returns a crafted ClientHello-like byte sequence.
   */
  simulateTLSFingerprint(): Uint8Array {
    const profile = TLS_FINGERPRINTS[this.selectedProfile];
    if (!profile) {
      throw new Error(`Unknown fingerprint profile: ${this.selectedProfile}`);
    }

    const recordHeader = new Uint8Array([0x16, 0x03, 0x01, 0x00, 0x00]);
    const handshakeHeader = new Uint8Array([0x01, 0x00, 0x00, 0x00]);
    const clientVersion = new Uint8Array([0x03, 0x03]);
    const clientRandom = crypto.getRandomValues(new Uint8Array(32));
    const sessionIdLen = new Uint8Array([0x20]);
    const sessionId = crypto.getRandomValues(new Uint8Array(32));

    let offset = 0;
    const cipherLen = profile.cipherSuites.length * 2;
    const cipherLenBytes = new Uint8Array([(cipherLen >> 8) & 0xff, cipherLen & 0xff]);
    const cipherData = new Uint8Array(cipherLen);
    for (let i = 0; i < profile.cipherSuites.length; i++) {
      cipherData[i * 2] = (profile.cipherSuites[i] >> 8) & 0xff;
      cipherData[i * 2 + 1] = profile.cipherSuites[i] & 0xff;
    }

    const compression = new Uint8Array([0x01, 0x00]);

    const extLen = profile.extensions.length * 4 + profile.extensions.length * 6;
    const extLenBytes = new Uint8Array([(extLen >> 8) & 0xff, extLen & 0xff]);
    const extData = new Uint8Array(extLen);
    offset = 0;
    for (const extId of profile.extensions) {
      extData[offset++] = (extId >> 8) & 0xff;
      extData[offset++] = extId & 0xff;
      const extPayloadLen = 4;
      extData[offset++] = (extPayloadLen >> 8) & 0xff;
      extData[offset++] = extPayloadLen & 0xff;
      const extPayload = crypto.getRandomValues(new Uint8Array(extPayloadLen));
      extData.set(extPayload, offset);
      offset += extPayloadLen;
    }

    const totalLen =
      handshakeHeader.length +
      clientVersion.length +
      clientRandom.length +
      sessionIdLen.length +
      sessionId.length +
      cipherLenBytes.length +
      cipherData.length +
      compression.length +
      extLenBytes.length +
      extData.length;

    const result = new Uint8Array(recordHeader.length + totalLen);
    let pos = 0;
    recordHeader[3] = (totalLen >> 8) & 0xff;
    recordHeader[4] = totalLen & 0xff;
    result.set(recordHeader, pos); pos += recordHeader.length;
    result.set(handshakeHeader, pos); pos += handshakeHeader.length;
    handshakeHeader[1] = ((totalLen - 4) >> 16) & 0xff;
    handshakeHeader[2] = ((totalLen - 4) >> 8) & 0xff;
    handshakeHeader[3] = (totalLen - 4) & 0xff;
    result.set(handshakeHeader, recordHeader.length);
    result.set(clientVersion, pos); pos += clientVersion.length;
    result.set(clientRandom, pos); pos += clientRandom.length;
    result.set(sessionIdLen, pos); pos += sessionIdLen.length;
    result.set(sessionId, pos); pos += sessionId.length;
    result.set(cipherLenBytes, pos); pos += cipherLenBytes.length;
    result.set(cipherData, pos); pos += cipherData.length;
    result.set(compression, pos); pos += compression.length;
    result.set(extLenBytes, pos); pos += extLenBytes.length;
    result.set(extData, pos);

    return result;
  }

  /**
   * Add padding to a packet to reach target_size or random size.
   * Format: [payload][0x00 delimiter][padding_length: u16][padding bytes]
   */
  addPadding(data: Uint8Array, targetSize?: number): Uint8Array {
    const paddingLen = targetSize
      ? Math.max(0, targetSize - data.length - 3)
      : this.randomPaddingLength();
    const actualPadding = Math.max(MIN_PADDING_SIZE, paddingLen);
    const paddingBytes = crypto.getRandomValues(new Uint8Array(actualPadding));
    const result = new Uint8Array(data.length + 1 + 2 + actualPadding);
    let offset = 0;
    result.set(data, offset); offset += data.length;
    result[offset++] = PADDING_DELIMITER;
    result[offset++] = (actualPadding >> 8) & 0xff;
    result[offset++] = actualPadding & 0xff;
    result.set(paddingBytes, offset);
    return result;
  }

  /**
   * Remove padding from a packet.
   */
  removePadding(data: Uint8Array): Uint8Array {
    for (let i = data.length - 1; i >= 2; i--) {
      if (data[i - 2] === PADDING_DELIMITER) {
        const paddingLen = (data[i - 1] << 8) | data[i];
        if (i + 1 + paddingLen === data.length || i - 2 === data.length - paddingLen - 3) {
          return data.slice(0, i - 2);
        }
      }
    }
    const lastDelimiter = data.lastIndexOf(PADDING_DELIMITER);
    if (lastDelimiter > 0 && lastDelimiter < data.length - 3) {
      return data.slice(0, lastDelimiter);
    }
    return data;
  }

  /**
   * Pad a response to a uniform size to avoid size-based fingerprinting.
   */
  padResponse(data: Uint8Array, minSize: number = 256): Uint8Array {
    const targetSize = Math.max(minSize, Math.ceil((data.length + 3) / PADDING_BLOCK_SIZE) * PADDING_BLOCK_SIZE + 64);
    return this.addPadding(data, targetSize);
  }

  /**
   * Calculate timing jitter for a given base delay.
   * Returns jittered delay in milliseconds.
   */
  addTimingJitter(delayMs: number): number {
    const [minJitter, maxJitter] = this.jitterRange;
    const jitter = Math.floor(Math.random() * (maxJitter - minJitter + 1)) + minJitter;
    const sign = Math.random() < 0.5 ? -1 : 1;
    return Math.max(0, delayMs + sign * jitter);
  }

  /**
   * Get a random User-Agent string based on the selected profile.
   */
  getRandomUserAgent(): string {
    const profile = TLS_FINGERPRINTS[this.selectedProfile];
    return profile?.ua || TLS_FINGERPRINTS[0].ua;
  }

  /**
   * Generate a fake X-Forwarded-For header with a random Chinese IP.
   */
  generateFakeXFF(): string {
    const chineseIPRanges = [
      [110, 111], [112, 117], [120, 123], [124, 125],
      [171, 175], [180, 183], [202, 223], [36, 61],
    ];
    const range = chineseIPRanges[Math.floor(Math.random() * chineseIPRanges.length)];
    const first = range[0] + Math.floor(Math.random() * (range[1] - range[0] + 1));
    const ip = `${first}.${Math.floor(Math.random() * 256)}.${Math.floor(Math.random() * 256)}.${Math.floor(Math.random() * 256)}`;
    return ip;
  }

  /**
   * Apply burst shaping: introduce small delays between consecutive sends
   * to simulate natural web browsing behavior.
   */
  async applyBurstShaping(sendCallback: () => Promise<void>, burstSize: number): Promise<void> {
    for (let i = 0; i < burstSize; i++) {
      const delay = this.addTimingJitter(i === 0 ? 0 : 15);
      if (delay > 0) {
        await new Promise((resolve) => setTimeout(resolve, delay));
      }
      await sendCallback();
    }
  }

  /**
   * Generate dummy traffic to maintain a baseline traffic rate.
   */
  generateDummyTraffic(): Uint8Array {
    const size = 64 + Math.floor(Math.random() * 192);
    const dummy = crypto.getRandomValues(new Uint8Array(size));
    dummy[0] = 0x05; // PADDING frame type
    return dummy;
  }

  private randomPaddingLength(): number {
    const range = MAX_PADDING_SIZE - MIN_PADDING_SIZE;
    const raw = Math.floor(Math.random() * range) + MIN_PADDING_SIZE;
    return Math.ceil(raw / PADDING_BLOCK_SIZE) * PADDING_BLOCK_SIZE;
  }
}
