/**
 * UnifiedShield Tencent Cloud Relay Logic
 *
 * Handles frame multiplexing, connection state management,
 * and upstream proxy connections for Tencent Cloud SCF.
 */

export enum FrameType {
  DATA = 0x01,
  KEEPALIVE = 0x02,
  RENEGOTIATE = 0x03,
  CLOSE = 0x04,
  PADDING = 0x05,
}

export enum ConnectionState {
  INIT = 'init',
  HANDSHAKE = 'handshake',
  CONNECTED = 'connected',
  RENEGOTIATING = 'renegotiating',
  CLOSING = 'closing',
  CLOSED = 'closed',
}

export interface Connection {
  id: string;
  state: ConnectionState;
  clientIP: string;
  targetHost: string;
  targetPort: number;
  sessionKey: Uint8Array | null;
  createdAt: number;
  lastActivity: number;
  bytesIn: number;
  bytesOut: number;
}

export interface Frame {
  type: FrameType;
  payload: Uint8Array;
  timestamp: number;
}

const MAX_FRAME_SIZE = 65536;
const CONNECTION_TIMEOUT = 300000;
const MAX_CONNECTIONS_PER_CLIENT = 5;

interface RateLimitEntry {
  count: number;
  windowStart: number;
}

const MAX_RATE_LIMIT = 1000;
const RATE_WINDOW_SECONDS = 3600;

import * as http from 'http';
import * as https from 'https';

export class TencentRelayHandler {
  private privateKey: Uint8Array;
  private connections: Map<string, Connection> = new Map();
  private rateLimitEntries: Map<string, RateLimitEntry> = new Map();
  private cleanupInterval: NodeJS.Timeout | null = null;

  constructor(privateKey: Uint8Array) {
    this.privateKey = privateKey;
    this.startCleanup();
  }

  private startCleanup(): void {
    this.cleanupInterval = setInterval(() => this.cleanupStaleConnections(), 60000);
  }

  private cleanupStaleConnections(): void {
    const now = Date.now();
    for (const [id, conn] of this.connections) {
      if (now - conn.lastActivity > CONNECTION_TIMEOUT) {
        this.closeConnection(id, 'timeout');
      }
    }
    // Cleanup rate limit entries
    const rateNow = Math.floor(Date.now() / 1000);
    for (const [ip, entry] of this.rateLimitEntries) {
      if (rateNow - entry.windowStart >= RATE_WINDOW_SECONDS * 2) {
        this.rateLimitEntries.delete(ip);
      }
    }
  }

  checkMemoryRateLimit(ip: string): boolean {
    const now = Math.floor(Date.now() / 1000);
    const entry = this.rateLimitEntries.get(ip);
    if (!entry || now - entry.windowStart >= RATE_WINDOW_SECONDS) {
      this.rateLimitEntries.set(ip, { count: 1, windowStart: now });
      return true;
    }
    if (entry.count >= MAX_RATE_LIMIT) {
      return false;
    }
    entry.count++;
    return true;
  }

  createConnection(clientIP: string, targetHost: string, targetPort: number): Connection {
    const clientConns = Array.from(this.connections.values()).filter(
      (c) => c.clientIP === clientIP && c.state !== ConnectionState.CLOSED
    );
    if (clientConns.length >= MAX_CONNECTIONS_PER_CLIENT) {
      throw new Error('Max connections per client exceeded');
    }

    const id = crypto.randomUUID();
    const conn: Connection = {
      id,
      state: ConnectionState.INIT,
      clientIP,
      targetHost,
      targetPort,
      sessionKey: null,
      createdAt: Date.now(),
      lastActivity: Date.now(),
      bytesIn: 0,
      bytesOut: 0,
    };
    this.connections.set(id, conn);
    return conn;
  }

  getConnection(id: string): Connection | undefined {
    return this.connections.get(id);
  }

  updateConnectionState(id: string, state: ConnectionState): void {
    const conn = this.connections.get(id);
    if (conn) {
      conn.state = state;
      conn.lastActivity = Date.now();
    }
  }

  setSessionKey(id: string, key: Uint8Array): void {
    const conn = this.connections.get(id);
    if (conn) {
      conn.sessionKey = key;
      conn.state = ConnectionState.CONNECTED;
      conn.lastActivity = Date.now();
    }
  }

  parseFrame(data: Uint8Array): Frame {
    if (data.length < 1) {
      throw new Error('Frame too short');
    }
    if (data.length > MAX_FRAME_SIZE) {
      throw new Error('Frame exceeds maximum size');
    }
    const type = data[0] as FrameType;
    if (!Object.values(FrameType).includes(type)) {
      throw new Error(`Invalid frame type: ${type}`);
    }
    return {
      type,
      payload: data.slice(1),
      timestamp: Date.now(),
    };
  }

  buildFrame(type: FrameType, payload?: Uint8Array): Uint8Array {
    const frame = new Uint8Array(1 + (payload?.length || 0));
    frame[0] = type;
    if (payload) {
      frame.set(payload, 1);
    }
    return frame;
  }

  handleFrame(connId: string, frame: Frame): Frame | null {
    const conn = this.connections.get(connId);
    if (!conn) {
      return this.buildFrame(FrameType.CLOSE, new TextEncoder().encode('Unknown connection'));
    }

    conn.lastActivity = Date.now();
    conn.bytesIn += 1 + frame.payload.length;

    switch (frame.type) {
      case FrameType.DATA:
        if (conn.state !== ConnectionState.CONNECTED && conn.state !== ConnectionState.RENEGOTIATING) {
          return this.buildFrame(FrameType.CLOSE, new TextEncoder().encode('Not connected'));
        }
        return null;

      case FrameType.KEEPALIVE:
        return this.buildFrame(FrameType.KEEPALIVE);

      case FrameType.RENEGOTIATE:
        if (conn.state !== ConnectionState.CONNECTED) {
          return this.buildFrame(FrameType.CLOSE, new TextEncoder().encode('Cannot renegotiate'));
        }
        conn.state = ConnectionState.RENEGOTIATING;
        return this.buildFrame(FrameType.RENEGOTIATE, frame.payload);

      case FrameType.CLOSE:
        return this.buildFrame(FrameType.CLOSE);

      case FrameType.PADDING:
        return null;

      default:
        return this.buildFrame(FrameType.CLOSE, new TextEncoder().encode('Unknown frame type'));
    }
  }

  async relayToUpstream(
    targetHost: string,
    targetPort: number,
    payload: Uint8Array,
    clientIP: string
  ): Promise<Uint8Array> {
    return new Promise((resolve, reject) => {
      const isSecure = targetPort === 443;
      const lib = isSecure ? https : http;

      const options = {
        hostname: targetHost,
        port: targetPort,
        path: '/',
        method: 'POST',
        headers: {
          'Content-Type': 'application/octet-stream',
          'Content-Length': payload.length.toString(),
          'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36',
        },
        timeout: 30000,
        ...(isSecure ? { rejectUnauthorized: false } : {}),
      };

      const req = lib.request(options, (res) => {
        const chunks: Buffer[] = [];
        res.on('data', (chunk: Buffer) => chunks.push(chunk));
        res.on('end', () => {
          resolve(new Uint8Array(Buffer.concat(chunks)));
        });
        res.on('error', (err) => reject(err));
      });

      req.on('error', (err) => reject(err));
      req.on('timeout', () => {
        req.destroy();
        reject(new Error('Upstream timeout'));
      });

      req.write(Buffer.from(payload));
      req.end();
    });
  }

  closeConnection(id: string, reason: string = 'normal'): void {
    const conn = this.connections.get(id);
    if (conn) {
      conn.state = ConnectionState.CLOSED;
      this.connections.delete(id);
    }
  }

  getActiveConnectionCount(): number {
    let count = 0;
    for (const conn of this.connections.values()) {
      if (conn.state !== ConnectionState.CLOSED) count++;
    }
    return count;
  }

  getStats(): { total: number; active: number; byState: Record<string, number> } {
    const byState: Record<string, number> = {};
    for (const conn of this.connections.values()) {
      byState[conn.state] = (byState[conn.state] || 0) + 1;
    }
    return {
      total: this.connections.size,
      active: this.getActiveConnectionCount(),
      byState,
    };
  }

  destroy(): void {
    if (this.cleanupInterval) {
      clearInterval(this.cleanupInterval);
    }
    for (const id of this.connections.keys()) {
      this.closeConnection(id, 'shutdown');
    }
  }
}
