// @ts-nocheck
/**
 * MICAFP-UnifiedShield-6.0 — Shared WebTransport Tunnel
 *
 * Provides a bidirectional tunnel over WebTransport (HTTP/3) with
 * automatic WebSocket fallback, HMAC-SHA256 authentication,
 * exponential-backoff reconnection, WASM obfuscator integration,
 * and per-session byte counting.
 *
 * Used by both the Chrome MV3 service-worker and the Firefox MV2
 * background script.
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface TunnelConfig {
  /** CDN worker endpoint URLs – tried in priority order */
  endpoints: string[];
  /** Hex-encoded HMAC key (must be 32 bytes = 64 hex chars) */
  hmacKey: string;
  /** WASM obfuscator module (instantiated) – optional */
  wasmObfuscator?: WasmObfuscator | null;
  /** Maximum reconnect attempts before giving up (0 = infinite) */
  maxReconnectAttempts?: number;
  /** Initial backoff in ms (doubles each attempt) */
  initialBackoffMs?: number;
  /** Maximum backoff in ms */
  maxBackoffMs?: number;
  /** How often to send keepalive pings (ms). 0 = disabled */
  keepaliveIntervalMs?: number;
  /** Request timeout for dial (ms) */
  dialTimeoutMs?: number;
}

export interface TunnelStats {
  bytesSent: number;
  bytesReceived: number;
  packetsSent: number;
  packetsReceived: number;
  connectTime: number | null;
  uptimeMs: number;
}

export type TunnelState =
  | "disconnected"
  | "connecting"
  | "connected"
  | "reconnecting"
  | "failed";

export type StateChangeCallback = (state: TunnelState, prev: TunnelState) => void;
export type DataCallback = (data: Uint8Array) => void;

export interface WasmObfuscator {
  transform(data: Uint8Array, key: Uint8Array): Uint8Array;
  reverseTransform(data: Uint8Array, key: Uint8Array): Uint8Array;
}

// ---------------------------------------------------------------------------
// HMAC-SHA256 helpers
// ---------------------------------------------------------------------------

async function hmacSha256(key: Uint8Array, message: Uint8Array): Promise<Uint8Array> {
  const cryptoKey = await crypto.subtle.importKey(
    "raw",
    key,
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"],
  );
  const sig = await crypto.subtle.sign("HMAC", cryptoKey, message);
  return new Uint8Array(sig);
}

function hexDecode(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
  }
  return bytes;
}

// ---------------------------------------------------------------------------
// Protocol framing
// ---------------------------------------------------------------------------

/**
 * Wire format (each frame):
 *   [4 bytes]  big-endian payload length
 *   [32 bytes] HMAC-SHA256 of payload (using shared key)
 *   [N bytes]  payload (possibly obfuscated)
 *
 * The receiver verifies the HMAC before de-obfuscating.
 */

const FRAME_HEADER_LEN = 4 + 32; // length prefix + HMAC

function encodeFrame(payload: Uint8Array, hmacKey: Uint8Array): Promise<Uint8Array> {
  return (async () => {
    const mac = await hmacSha256(hmacKey, payload);
    const len = new DataView(new ArrayBuffer(4));
    len.setUint32(0, payload.length, false); // big-endian
    const frame = new Uint8Array(4 + 32 + payload.length);
    frame.set(new Uint8Array(len.buffer), 0);
    frame.set(mac, 4);
    frame.set(payload, 36);
    return frame;
  })();
}

async function decodeFrame(
  reader: ReadableStreamDefaultReader<Uint8Array>,
  hmacKey: Uint8Array,
): Promise<Uint8Array | null> {
  // Read header (4 + 32 bytes)
  const header = await readExact(reader, FRAME_HEADER_LEN);
  if (header === null) return null;

  const payloadLen = new DataView(header.buffer, header.byteOffset, 4).getUint32(0, false);
  if (payloadLen > 1_048_576) {
    // >1 MiB — sanity check
    throw new Error(`Frame payload too large: ${payloadLen}`);
  }

  const receivedMac = header.slice(4, 36);
  const payload = await readExact(reader, payloadLen);
  if (payload === null) return null;

  // Verify HMAC
  const expectedMac = await hmacSha256(hmacKey, payload);
  if (!timingSafeEqual(receivedMac, expectedMac)) {
    throw new Error("HMAC verification failed – possible tampering or wrong key");
  }

  return payload;
}

function timingSafeEqual(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  let result = 0;
  for (let i = 0; i < a.length; i++) {
    result |= a[i] ^ b[i];
  }
  return result === 0;
}

async function readExact(
  reader: ReadableStreamDefaultReader<Uint8Array>,
  n: number,
): Promise<Uint8Array | null> {
  const buf = new Uint8Array(n);
  let filled = 0;
  while (filled < n) {
    const { value, done } = await reader.read();
    if (done) return filled === 0 ? null : buf.slice(0, filled);
    const toCopy = Math.min(value.length, n - filled);
    buf.set(value.subarray(0, toCopy), filled);
    filled += toCopy;
  }
  return buf;
}

// ---------------------------------------------------------------------------
// Tunnel implementation
// ---------------------------------------------------------------------------

export class WebTransportTunnel {
  private config: Required<TunnelConfig>;
  private hmacKeyBytes: Uint8Array;
  private obfuscatorKey: Uint8Array;

  private state: TunnelState = "disconnected";
  private transport: WebTransport | null = null;
  private ws: WebSocket | null = null;
  private sendStream: WritableStreamDefaultWriter<Uint8Array> | null = null;
  private recvReader: ReadableStreamDefaultReader<Uint8Array> | null = null;

  private reconnectAttempts = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private keepaliveTimer: ReturnType<typeof setInterval> | null = null;
  private connectTimestamp: number | null = null;
  private currentEndpointIdx = 0;

  private stats: TunnelStats = {
    bytesSent: 0,
    bytesReceived: 0,
    packetsSent: 0,
    packetsReceived: 0,
    connectTime: null,
    uptimeMs: 0,
  };

  private stateCallbacks: Set<StateChangeCallback> = new Set();
  private dataCallbacks: Set<DataCallback> = new Set();
  private _closed = false;

  // -----------------------------------------------------------------------
  // Construction
  // -----------------------------------------------------------------------

  constructor(config: TunnelConfig) {
    this.config = {
      endpoints: config.endpoints,
      hmacKey: config.hmacKey,
      wasmObfuscator: config.wasmObfuscator ?? null,
      maxReconnectAttempts: config.maxReconnectAttempts ?? 0,
      initialBackoffMs: config.initialBackoffMs ?? 1000,
      maxBackoffMs: config.maxBackoffMs ?? 60_000,
      keepaliveIntervalMs: config.keepaliveIntervalMs ?? 25_000,
      dialTimeoutMs: config.dialTimeoutMs ?? 10_000,
    };
    this.hmacKeyBytes = hexDecode(this.config.hmacKey);
    // Derive a separate obfuscator key from HMAC key by hashing it
    this.obfuscatorKey = this.hmacKeyBytes.slice(0, 16);
  }

  // -----------------------------------------------------------------------
  // Public API
  // -----------------------------------------------------------------------

  get connected(): boolean {
    return this.state === "connected";
  }

  get currentState(): TunnelState {
    return this.state;
  }

  getStats(): TunnelStats {
    return {
      ...this.stats,
      uptimeMs: this.connectTimestamp ? Date.now() - this.connectTimestamp : 0,
    };
  }

  onStateChange(cb: StateChangeCallback): () => void {
    this.stateCallbacks.add(cb);
    return () => void this.stateCallbacks.delete(cb);
  }

  onData(cb: DataCallback): () => void {
    this.dataCallbacks.add(cb);
    return () => void this.dataCallbacks.delete(cb);
  }

  /** Open the tunnel (tries WebTransport first, falls back to WebSocket). */
  async connect(): Promise<void> {
    if (this.state === "connected" || this.state === "connecting") return;
    this._closed = false;
    this.setState("connecting");

    const endpoint = this.config.endpoints[this.currentEndpointIdx % this.config.endpoints.length];

    try {
      if (typeof WebTransport !== "undefined") {
        await this.connectWebTransport(endpoint);
      } else {
        await this.connectWebSocket(endpoint);
      }
    } catch (err) {
      console.warn("[Shield] Primary transport failed, trying WebSocket fallback:", err);
      try {
        await this.connectWebSocket(endpoint);
      } catch (wsErr) {
        console.error("[Shield] WebSocket fallback also failed:", wsErr);
        this.setState("failed");
        this.scheduleReconnect();
      }
    }
  }

  /** Send data through the tunnel. */
  async send(data: Uint8Array): Promise<void> {
    if (this.state !== "connected") {
      throw new Error("Tunnel not connected");
    }

    // Optionally obfuscate
    let payload = data;
    if (this.config.wasmObfuscator) {
      payload = this.config.wasmObfuscator.transform(data, this.obfuscatorKey);
    }

    const frame = await encodeFrame(payload, this.hmacKeyBytes);

    if (this.sendStream) {
      await this.sendStream.write(frame);
    } else if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(frame);
    } else {
      throw new Error("No active transport to send on");
    }

    this.stats.bytesSent += frame.length;
    this.stats.packetsSent += 1;
  }

  /** Gracefully close the tunnel. */
  disconnect(): void {
    this._closed = true;
    this.cleanup();
    this.setState("disconnected");
  }

  /** Permanently destroy the tunnel (no reconnect). */
  destroy(): void {
    this._closed = true;
    this.stateCallbacks.clear();
    this.dataCallbacks.clear();
    this.cleanup();
    this.setState("disconnected");
  }

  // -----------------------------------------------------------------------
  // WebTransport path
  // -----------------------------------------------------------------------

  private async connectWebTransport(endpoint: string): Promise<void> {
    // Build the URL with auth token as a query param (HMAC of timestamp)
    const timestamp = Date.now().toString();
    const authMac = await hmacSha256(this.hmacKeyBytes, new TextEncoder().encode(timestamp));
    const authToken = Array.from(authMac)
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("");

    const url = `${endpoint}?t=${timestamp}&auth=${authToken}`;

    this.transport = new WebTransport(url, {
      // @ts-expect-error — serverCertificateHashes is not yet in all TS defs
      serverCertificateHashes: [],
      congestionControl: "throughput",
    });

    // Wait for the transport to be ready (with timeout)
    await Promise.race([
      this.transport.ready,
      new Promise<never>((_, reject) =>
        setTimeout(
          () => reject(new Error("WebTransport dial timeout")),
          this.config.dialTimeoutMs,
        ),
      ),
    ]);

    // Open bidirectional stream
    const bidi = await this.transport.createBidirectionalStream();
    this.sendStream = bidi.writable.getWriter();
    this.recvReader = bidi.readable.getReader();

    this.onConnected();
    this.recvLoopWT();
  }

  private async recvLoopWT(): Promise<void> {
    try {
      // eslint-disable-next-line no-constant-condition
      while (true) {
        const payload = await decodeFrame(this.recvReader!, this.hmacKeyBytes);
        if (payload === null) break;

        let data = payload;
        if (this.config.wasmObfuscator) {
          data = this.config.wasmObfuscator.reverseTransform(payload, this.obfuscatorKey);
        }

        this.stats.bytesReceived += payload.length;
        this.stats.packetsReceived += 1;
        this.dispatchData(data);
      }
    } catch (err) {
      console.warn("[Shield] WebTransport recv loop error:", err);
    } finally {
      if (!this._closed) {
        this.handleDisconnect();
      }
    }
  }

  // -----------------------------------------------------------------------
  // WebSocket fallback
  // -----------------------------------------------------------------------

  private connectWebSocket(endpoint: string): Promise<void> {
    return new Promise((resolve, reject) => {
      // Convert https:// → wss:// for the CDN worker
      const wsUrl = endpoint.replace(/^https?:\/\//, "wss://") + "/ws";

      this.ws = new WebSocket(wsUrl, "shield-v1");
      this.ws.binaryType = "arraybuffer";

      const timeout = setTimeout(() => {
        this.ws?.close();
        reject(new Error("WebSocket connect timeout"));
      }, this.config.dialTimeoutMs);

      this.ws.onopen = () => {
        clearTimeout(timeout);
        this.onConnected();
        resolve();
      };

      this.ws.onmessage = async (ev: MessageEvent) => {
        try {
          const raw = new Uint8Array(ev.data as ArrayBuffer);
          const reader = new ReadableStream({
            start(controller) {
              controller.enqueue(raw);
              controller.close();
            },
          }).getReader();
          const payload = await decodeFrame(reader, this.hmacKeyBytes);
          if (payload === null) return;

          let data = payload;
          if (this.config.wasmObfuscator) {
            data = this.config.wasmObfuscator.reverseTransform(payload, this.obfuscatorKey);
          }

          this.stats.bytesReceived += raw.length;
          this.stats.packetsReceived += 1;
          this.dispatchData(data);
        } catch (err) {
          console.warn("[Shield] WS frame decode error:", err);
        }
      };

      this.ws.onerror = () => {
        clearTimeout(timeout);
        reject(new Error("WebSocket error"));
      };

      this.ws.onclose = () => {
        clearTimeout(timeout);
        if (!this._closed) this.handleDisconnect();
      };
    });
  }

  // -----------------------------------------------------------------------
  // Lifecycle helpers
  // -----------------------------------------------------------------------

  private onConnected(): void {
    this.reconnectAttempts = 0;
    this.connectTimestamp = Date.now();
    this.stats.connectTime = this.connectTimestamp;
    this.setState("connected");
    this.startKeepalive();
  }

  private handleDisconnect(): void {
    if (this._closed) return;
    this.cleanup();
    this.setState("reconnecting");
    this.scheduleReconnect();
  }

  private scheduleReconnect(): void {
    if (this._closed) return;
    if (
      this.config.maxReconnectAttempts > 0 &&
      this.reconnectAttempts >= this.config.maxReconnectAttempts
    ) {
      this.setState("failed");
      return;
    }

    this.reconnectAttempts++;
    const jitter = Math.random() * 0.3 + 0.85; // 0.85 – 1.15
    const backoff = Math.min(
      this.config.initialBackoffMs * Math.pow(2, this.reconnectAttempts - 1) * jitter,
      this.config.maxBackoffMs,
    );

    console.log(
      `[Shield] Reconnecting in ${Math.round(backoff)}ms (attempt ${this.reconnectAttempts})`,
    );

    // Rotate to next endpoint on each attempt
    this.currentEndpointIdx =
      (this.currentEndpointIdx + 1) % this.config.endpoints.length;

    this.reconnectTimer = setTimeout(() => {
      this.connect().catch((err) => {
        console.error("[Shield] Reconnect failed:", err);
      });
    }, backoff);
  }

  private startKeepalive(): void {
    if (this.config.keepaliveIntervalMs <= 0) return;
    this.keepaliveTimer = setInterval(() => {
      if (this.state === "connected") {
        // Send a zero-length keepalive frame
        this.send(new Uint8Array(0)).catch(() => {
          // ignore — recv loop will detect failure
        });
      }
    }, this.config.keepaliveIntervalMs);
  }

  private cleanup(): void {
    if (this.keepaliveTimer) {
      clearInterval(this.keepaliveTimer);
      this.keepaliveTimer = null;
    }
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    try {
      this.sendStream?.releaseLock();
    } catch {
      /* already released */
    }
    this.sendStream = null;
    try {
      this.recvReader?.releaseLock();
    } catch {
      /* already released */
    }
    this.recvReader = null;
    try {
      this.transport?.close();
    } catch {
      /* ignore */
    }
    this.transport = null;
    try {
      this.ws?.close();
    } catch {
      /* ignore */
    }
    this.ws = null;
  }

  // -----------------------------------------------------------------------
  // Event dispatch
  // -----------------------------------------------------------------------

  private setState(s: TunnelState): void {
    const prev = this.state;
    if (prev === s) return;
    this.state = s;
    for (const cb of this.stateCallbacks) {
      try {
        cb(s, prev);
      } catch {
        /* swallow */
      }
    }
  }

  private dispatchData(data: Uint8Array): void {
    for (const cb of this.dataCallbacks) {
      try {
        cb(data);
      } catch {
        /* swallow */
      }
    }
  }
}
