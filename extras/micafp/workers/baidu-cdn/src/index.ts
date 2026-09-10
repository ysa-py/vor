/**
 * UnifiedShield Baidu Cloud Function Relay Worker
 *
 * Handles HTTP trigger events with X25519 ECDH key negotiation
 * and ChaCha20-Poly1305 encryption. Adapted for Baidu CFC API format.
 */

import { BaiduRelayHandler, FrameType, ConnectionState } from './relay';
import Redis from 'ioredis';

const HKDF_INFO = 'unifiedshield-session-v1';
const NONCE_LENGTH = 12;
const TAG_LENGTH = 16;
const MAX_RATE_LIMIT = 1000;
const RATE_WINDOW_SECONDS = 3600;

let relayHandler: BaiduRelayHandler;
let redis: Redis | null = null;

function hexToBytes(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
  }
  return bytes;
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

async function deriveSessionKey(
  privateKeyBytes: Uint8Array,
  publicKeyBytes: Uint8Array
): Promise<Uint8Array> {
  const privateKey = await crypto.subtle.importKey(
    'raw', privateKeyBytes, { name: 'X25519' }, false, ['deriveBits']
  );
  const publicKey = await crypto.subtle.importKey(
    'raw', publicKeyBytes, { name: 'X25519' }, true, []
  );

  const sharedBits = await crypto.subtle.deriveBits(
    { name: 'X25519', public: publicKey }, privateKey, 256
  );

  const salt = new Uint8Array(32);
  const infoBytes = new TextEncoder().encode(HKDF_INFO);
  const hkdfKey = await crypto.subtle.importKey(
    'raw', sharedBits, { name: 'HKDF' }, false, ['deriveBits']
  );
  const derivedBits = await crypto.subtle.deriveBits(
    { name: 'HKDF', hash: 'SHA-256', salt, info: infoBytes }, hkdfKey, 256
  );
  return new Uint8Array(derivedBits);
}

async function chaCha20Poly1305Encrypt(
  key: Uint8Array,
  plaintext: Uint8Array,
  associatedData?: Uint8Array
): Promise<{ ciphertext: Uint8Array; nonce: Uint8Array; tag: Uint8Array }> {
  const nonce = crypto.getRandomValues(new Uint8Array(NONCE_LENGTH));
  const algoKey = await crypto.subtle.importKey(
    'raw', key, { name: 'ChaCha20-Poly1305' }, false, ['encrypt']
  );
  const encrypted = await crypto.subtle.encrypt(
    { name: 'ChaCha20-Poly1305', iv: nonce, additionalData: associatedData || new Uint8Array(0) },
    algoKey, plaintext
  );
  const fullResult = new Uint8Array(encrypted);
  const ciphertext = fullResult.slice(0, fullResult.length - TAG_LENGTH);
  const tag = fullResult.slice(fullResult.length - TAG_LENGTH);
  return { ciphertext, nonce, tag };
}

async function chaCha20Poly1305Decrypt(
  key: Uint8Array,
  ciphertext: Uint8Array,
  nonce: Uint8Array,
  tag: Uint8Array,
  associatedData?: Uint8Array
): Promise<Uint8Array> {
  const algoKey = await crypto.subtle.importKey(
    'raw', key, { name: 'ChaCha20-Poly1305' }, false, ['decrypt']
  );
  const combined = new Uint8Array(ciphertext.length + tag.length);
  combined.set(ciphertext);
  combined.set(tag, ciphertext.length);
  const decrypted = await crypto.subtle.decrypt(
    { name: 'ChaCha20-Poly1305', iv: nonce, additionalData: associatedData || new Uint8Array(0) },
    algoKey, combined
  );
  return new Uint8Array(decrypted);
}

function buildEncryptedResponse(key: Uint8Array, data: Uint8Array, serverPubKey: Uint8Array): Promise<CFCResponse> {
  return chaCha20Poly1305Encrypt(key, data).then((enc) => {
    const combined = new Uint8Array(enc.ciphertext.length + NONCE_LENGTH + TAG_LENGTH);
    combined.set(enc.ciphertext);
    combined.set(enc.nonce, enc.ciphertext.length);
    combined.set(enc.tag, enc.ciphertext.length + NONCE_LENGTH);
    return {
      isBase64Encoded: true,
      statusCode: 200,
      headers: {
        'Content-Type': 'application/octet-stream',
        'X-Session-Public': bytesToHex(serverPubKey),
      },
      body: btoa(String.fromCharCode(...combined)),
    };
  });
}

interface CFCEvent {
  httpMethod: string;
  path: string;
  headers: Record<string, string>;
  queryParameters: Record<string, string>;
  body: string;
  isBase64Encoded: boolean;
  pathParameters: Record<string, string>;
  requestContext: {
    requestId: string;
    stage: string;
    sourceIp: string;
    apiId: string;
    httpMethod: string;
  };
}

interface CFCResponse {
  isBase64Encoded: boolean;
  statusCode: number;
  headers: Record<string, string>;
  body: string;
}

function initRedis(): Redis | null {
  const redisUrl = process.env.BAIDU_REDIS_URL;
  if (!redisUrl) return null;
  try {
    const client = new Redis(redisUrl, {
      maxRetriesPerRequest: 3,
      retryStrategy(times) {
        return Math.min(times * 100, 3000);
      },
    });
    client.on('error', (err) => console.warn('Baidu Redis error:', err.message));
    return client;
  } catch {
    return null;
  }
}

async function checkRateLimit(ip: string): Promise<boolean> {
  if (!redis) {
    redis = initRedis();
  }

  if (!redis) {
    return relayHandler.checkMemoryRateLimit(ip);
  }

  try {
    const key = `unifiedshield:ratelimit:${ip}`;
    const now = Math.floor(Date.now() / 1000);
    const windowStart = now - RATE_WINDOW_SECONDS;

    const multi = redis.multi();
    multi.zremrangebyscore(key, 0, windowStart);
    multi.zadd(key, now, `${now}:${Math.random().toString(36).slice(2)}`);
    multi.zcard(key);
    multi.expire(key, RATE_WINDOW_SECONDS);
    const results = await multi.exec();

    const count = results?.[2]?.[1] as number;
    return count <= MAX_RATE_LIMIT;
  } catch {
    return relayHandler.checkMemoryRateLimit(ip);
  }
}

function parseAllowedTargets(): string[] {
  try {
    return JSON.parse(process.env.ALLOWED_TARGETS || '[]');
  } catch {
    return [];
  }
}

function validateTarget(targetHost: string): boolean {
  const allowed = parseAllowedTargets();
  if (allowed.length === 0) return true;
  return allowed.some((pattern) => {
    if (pattern.startsWith('*.')) {
      return targetHost.endsWith(pattern.slice(1)) || targetHost === pattern.slice(2);
    }
    return targetHost === pattern;
  });
}

export const handler = async (event: CFCEvent, context: any): Promise<CFCResponse> => {
  if (!relayHandler) {
    const privateKeyHex = process.env.WORKER_PRIVATE_KEY;
    if (!privateKeyHex) {
      return {
        isBase64Encoded: false,
        statusCode: 500,
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ error: 'Server misconfigured' }),
      };
    }
    relayHandler = new BaiduRelayHandler(hexToBytes(privateKeyHex));
  }

  const clientIP =
    event.requestContext?.sourceIp ||
    event.headers['x-forwarded-for']?.split(',')[0]?.trim() ||
    event.headers['x-bce-client-ip'] ||
    '0.0.0.0';

  if (!(await checkRateLimit(clientIP))) {
    return {
      isBase64Encoded: false,
      statusCode: 429,
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ error: 'Rate limit exceeded' }),
    };
  }

  if (event.httpMethod === 'GET' && event.path === '/health') {
    return {
      isBase64Encoded: false,
      statusCode: 200,
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ status: 'ok', version: '1.0.0', provider: 'baidu' }),
    };
  }

  const sessionToken = event.headers['x-session-token'];
  if (!sessionToken) {
    return {
      isBase64Encoded: false,
      statusCode: 400,
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ error: 'Missing session token' }),
    };
  }

  try {
    const targetHost = event.headers['x-target-host'] || '';
    const targetPort = parseInt(event.headers['x-target-port'] || '443', 10);

    if (!validateTarget(targetHost)) {
      return {
        isBase64Encoded: false,
        statusCode: 403,
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ error: 'Target not allowed' }),
      };
    }

    const clientPubKey = hexToBytes(sessionToken);
    const serverPrivateKeyBytes = hexToBytes(process.env.WORKER_PRIVATE_KEY!);
    const sessionKey = await deriveSessionKey(serverPrivateKeyBytes, clientPubKey);

    const serverKeyPair = await crypto.subtle.generateKey({ name: 'X25519' }, true, ['deriveBits']);
    const serverPubKeyRaw = new Uint8Array(
      await crypto.subtle.exportKey('raw', serverKeyPair.publicKey as CryptoKey)
    );

    const bodyBuffer = event.isBase64Encoded
      ? Uint8Array.from(atob(event.body), (c) => c.charCodeAt(0))
      : new TextEncoder().encode(event.body);

    if (bodyBuffer.length < NONCE_LENGTH + TAG_LENGTH + 1) {
      return {
        isBase64Encoded: false,
        statusCode: 400,
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ error: 'Payload too short' }),
      };
    }

    // Layout: [ciphertext][nonce(12)][tag(16)]
    const tagOffset = bodyBuffer.length - TAG_LENGTH;
    const nonceOffset = tagOffset - NONCE_LENGTH;

    const ct = bodyBuffer.slice(0, nonceOffset);
    const nonce = bodyBuffer.slice(nonceOffset, tagOffset);
    const tag = bodyBuffer.slice(tagOffset);

    const decryptedFrame = await chaCha20Poly1305Decrypt(sessionKey, ct, nonce, tag, new Uint8Array(0));

    const frameType = decryptedFrame[0] as FrameType;

    if (frameType === FrameType.PADDING) {
      return {
        isBase64Encoded: true,
        statusCode: 200,
        headers: { 'Content-Type': 'application/octet-stream' },
        body: btoa(String.fromCharCode(...crypto.getRandomValues(new Uint8Array(64)))),
      };
    }

    if (frameType === FrameType.KEEPALIVE) {
      return buildEncryptedResponse(
        sessionKey,
        new Uint8Array([FrameType.KEEPALIVE]),
        serverPubKeyRaw
      );
    }

    if (frameType === FrameType.DATA) {
      const payload = decryptedFrame.slice(1);
      const upstreamResponse = await relayHandler.relayToUpstream(
        targetHost, targetPort, payload, clientIP
      );

      const responseData = new Uint8Array([FrameType.DATA, ...upstreamResponse]);
      return buildEncryptedResponse(sessionKey, responseData, serverPubKeyRaw);
    }

    if (frameType === FrameType.CLOSE) {
      return {
        isBase64Encoded: false,
        statusCode: 200,
        headers: { 'Content-Type': 'application/octet-stream' },
        body: '',
      };
    }

    if (frameType === FrameType.RENEGOTIATE) {
      const newClientPub = decryptedFrame.slice(1);
      const newSessionKey = await deriveSessionKey(serverPrivateKeyBytes, newClientPub);
      const renegotiateData = new Uint8Array([FrameType.RENEGOTIATE, ...serverPubKeyRaw]);
      return buildEncryptedResponse(newSessionKey, renegotiateData, serverPubKeyRaw);
    }

    return {
      isBase64Encoded: false,
      statusCode: 400,
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ error: 'Unknown frame type' }),
    };
  } catch (err: any) {
    return {
      isBase64Encoded: false,
      statusCode: 500,
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ error: 'Processing failed' }),
    };
  }
};

export { deriveSessionKey, chaCha20Poly1305Encrypt, chaCha20Poly1305Decrypt, hexToBytes, bytesToHex };
