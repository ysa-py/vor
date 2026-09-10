/**
 * UnifiedShield Deno Relay Server
 *
 * Alternative relay for when Cloudflare is blocked.
 * Supports raw TCP relay, WebSocket bridge, and ECDH handshake.
 * Deploy on VPS outside Iran or use Deno Deploy.
 */

import { handleRelayConnection, closeRelay } from './relay.ts';
import { handleWebSocketBridge } from './websocket-bridge.ts';

const PORT = parseInt(Deno.env.get('PORT') ?? '8080');
const RELAY_SECRET = Deno.env.get('RELAY_SECRET') ?? 'change-me';
const MAX_CONNECTIONS = parseInt(Deno.env.get('MAX_CONNECTIONS') ?? '5000');

interface RelayConfig {
  secret: string;
  maxConnections: number;
  version: string;
}

const config: RelayConfig = {
  secret: RELAY_SECRET,
  maxConnections: MAX_CONNECTIONS,
  version: '2.0.0',
};

let activeConnections = 0;

console.log(`[UnifiedShield Relay] v${config.version} starting on port ${PORT}`);
console.log(`[UnifiedShield Relay] Max connections: ${config.maxConnections}`);

const server = Deno.listen({ port: PORT, hostname: '0.0.0.0' });

console.log(`[UnifiedShield Relay] Listening on 0.0.0.0:${PORT}`);

for await (const conn of server) {
  if (activeConnections >= config.maxConnections) {
    console.warn('[Relay] Max connections reached, rejecting');
    conn.close();
    continue;
  }

  handleConnection(conn);
}

async function handleConnection(conn: Deno.Conn): Promise<void> {
  activeConnections++;
  const connId = crypto.randomUUID();
  const startTime = Date.now();

  console.log(`[Relay] New connection ${connId} (active: ${activeConnections})`);

  try {
    const buf = new Uint8Array(8192);

    // Read first bytes to determine protocol
    const n = await conn.read(buf);
    if (!n) {
      conn.close();
      return;
    }

    const firstBytes = buf.subarray(0, n);

    // Detect protocol
    if (isHTTP(firstBytes)) {
      await handleHTTP(conn, firstBytes);
    } else if (isTLS(firstBytes)) {
      await handleTLSRelay(conn, firstBytes);
    } else {
      await handleRelayConnection(conn, firstBytes, config);
    }
  } catch (err) {
    console.error(`[Relay] Connection ${connId} error:`, err);
  } finally {
    activeConnections--;
    const duration = Date.now() - startTime;
    console.log(
      `[Relay] Connection ${connId} closed (duration: ${duration}ms, active: ${activeConnections})`
    );
    try { conn.close(); } catch { /* already closed */ }
  }
}

/**
 * Handle HTTP requests (REST API + WebSocket upgrade)
 */
async function handleHTTP(conn: Deno.Conn, initialData: Uint8Array): Promise<void> {
  const text = new TextDecoder().decode(initialData);

  // Check for WebSocket upgrade
  if (text.includes('Upgrade: websocket') || text.includes('Upgrade: WebSocket')) {
    await handleWebSocketUpgrade(conn, text);
    return;
  }

  // Parse HTTP request
  const lines = text.split('\r\n');
  const [method, path] = lines[0].split(' ');

  let body = '';
  if (method === 'POST') {
    const contentLengthIdx = lines.findIndex((l) =>
      l.toLowerCase().startsWith('content-length:')
    );
    if (contentLengthIdx >= 0) {
      const contentLength = parseInt(lines[contentLengthIdx].split(':')[1].trim(), 10);
      const bodyStart = text.indexOf('\r\n\r\n') + 4;
      body = text.substring(bodyStart);

      // Read remaining body if needed
      if (body.length < contentLength) {
        const remaining = new Uint8Array(contentLength - body.length);
        await conn.read(remaining);
        body += new TextDecoder().decode(remaining);
      }
    }
  }

  // Route
  let response: string;

  switch (path) {
    case '/':
      response = jsonResponse({
        name: 'UnifiedShield Deno Relay',
        version: config.version,
        activeConnections,
        endpoints: ['/health', '/handshake', '/ws'],
      });
      break;

    case '/health':
      response = jsonResponse({
        status: 'healthy',
        activeConnections,
        maxConnections: config.maxConnections,
        uptime: process.uptime ? process.uptime() : 0,
      });
      break;

    case '/handshake':
      if (method !== 'POST') {
        response = jsonResponse({ error: 'Method not allowed' }, 405);
      } else {
        try {
          const req = JSON.parse(body);
          const serverKeyPair = await generateKeyPair();
          const sharedSecret = await computeSharedSecret(
            serverKeyPair.privateKey,
            req.clientPublicKey
          );
          const sessionId = crypto.randomUUID();

          response = jsonResponse({
            sessionId,
            serverPublicKey: arrayBufferToBase64(serverKeyPair.publicKey),
            algorithms: ['AES-256-GCM', 'ChaCha20-Poly1305'],
          });
        } catch (err) {
          response = jsonResponse({ error: 'Handshake failed' }, 400);
        }
      }
      break;

    default:
      response = jsonResponse({ error: 'Not found' }, 404);
  }

  await conn.write(new TextEncoder().encode(response));
}

/**
 * Handle WebSocket upgrade
 */
async function handleWebSocketUpgrade(conn: Deno.Conn, initialData: string): Promise<void> {
  // Parse the upgrade request
  const lines = initialData.split('\r\n');
  const wsKey = lines
    .find((l) => l.toLowerCase().startsWith('sec-websocket-key:'))
    ?.split(':')[1]
    ?.trim();

  if (!wsKey) {
    const response = 'HTTP/1.1 400 Bad Request\r\n\r\n';
    await conn.write(new TextEncoder().encode(response));
    return;
  }

  // Compute accept key
  const acceptKey = await computeWebSocketAcceptKey(wsKey);

  // Send upgrade response
  const upgradeResponse = [
    'HTTP/1.1 101 Switching Protocols',
    'Upgrade: websocket',
    'Connection: Upgrade',
    `Sec-WebSocket-Accept: ${acceptKey}`,
    '',
    '',
  ].join('\r\n');

  await conn.write(new TextEncoder().encode(upgradeResponse));

  // Handle WebSocket connection
  await handleWebSocketBridge(conn);
}

/**
 * Handle TLS ClientHello — detect SNI and relay
 */
async function handleTLSRelay(conn: Deno.Conn, initialData: Uint8Array): Promise<void> {
  // Parse SNI from ClientHello
  const sni = parseSNI(initialData);

  if (sni) {
    console.log(`[Relay] TLS connection for SNI: ${sni}`);
  }

  // Relay to target
  await handleRelayConnection(conn, initialData, config);
}

/* ────────── Utility ────────── */

function isHTTP(data: Uint8Array): boolean {
  const text = new TextDecoder().decode(data.subarray(0, Math.min(data.length, 20)));
  return text.startsWith('GET ') || text.startsWith('POST ') || text.startsWith('OPTIONS ');
}

function isTLS(data: Uint8Array): boolean {
  return data.length > 5 && data[0] === 0x16 && data[1] === 0x03;
}

function parseSNI(data: Uint8Array): string | null {
  try {
    // TLS record: type(1) version(2) length(2) handshake...
    if (data[0] !== 0x16) return null;

    // Handshake: type(1) length(3) version(2) random(32) session_id_len(1) ...
    let offset = 5; // After TLS record header
    offset += 1 + 3; // Handshake type + length
    offset += 2; // Client version
    offset += 32; // Random
    offset += 1 + data[offset]; // Session ID

    // Cipher suites
    const cipherLen = (data[offset] << 8) | data[offset + 1];
    offset += 2 + cipherLen;

    // Compression methods
    const compLen = data[offset];
    offset += 1 + compLen;

    // Extensions
    if (offset + 2 > data.length) return null;
    const extLen = (data[offset] << 8) | data[offset + 1];
    offset += 2;

    const extEnd = offset + extLen;
    while (offset + 4 <= extEnd) {
      const extType = (data[offset] << 8) | data[offset + 1];
      const extDataLen = (data[offset + 2] << 8) | data[offset + 3];
      offset += 4;

      if (extType === 0x0000) {
        // SNI extension
        const listLen = (data[offset] << 8) | data[offset + 1];
        offset += 2;
        const sniType = data[offset];
        offset += 1;
        const sniLen = (data[offset] << 8) | data[offset + 1];
        offset += 2;

        if (sniType === 0) {
          return new TextDecoder().decode(data.subarray(offset, offset + sniLen));
        }
      }

      offset += extDataLen;
    }

    return null;
  } catch {
    return null;
  }
}

function jsonResponse(data: unknown, status = 200): string {
  const statusText = status === 200 ? 'OK' : status === 404 ? 'Not Found' : status === 405 ? 'Method Not Allowed' : 'Error';
  return [
    `HTTP/1.1 ${status} ${statusText}`,
    'Content-Type: application/json',
    'Access-Control-Allow-Origin: *',
    `Content-Length: ${JSON.stringify(data).length}`,
    '',
    JSON.stringify(data),
  ].join('\r\n');
}

async function computeWebSocketAcceptKey(key: string): Promise<string> {
  const magic = '258EAFA5-E914-47DA-95CA-C5AB0DC85B11';
  const encoder = new TextEncoder();
  const data = encoder.encode(key + magic);
  const hash = await crypto.subtle.digest('SHA-1', data);
  return btoa(String.fromCharCode(...new Uint8Array(hash)));
}

async function generateKeyPair(): Promise<{ publicKey: ArrayBuffer; privateKey: ArrayBuffer }> {
  const keyPair = await crypto.subtle.generateKey(
    { name: 'ECDH', namedCurve: 'P-256' },
    true,
    ['deriveBits']
  );
  const publicKey = await crypto.subtle.exportKey('raw', keyPair.publicKey);
  const privateKey = await crypto.subtle.exportKey('pkcs8', keyPair.privateKey);
  return { publicKey, privateKey };
}

async function computeSharedSecret(
  privateKey: ArrayBuffer,
  clientPublicKeyBase64: string
): Promise<string> {
  // Simplified — in production, use proper ECDH
  const combined = new Uint8Array(64);
  combined.set(new Uint8Array(privateKey).slice(0, 32), 0);
  const pubKey = Uint8Array.from(atob(clientPublicKeyBase64), (c) => c.charCodeAt(0));
  combined.set(pubKey.slice(0, 32), 32);

  const hash = await crypto.subtle.digest('SHA-256', combined);
  return btoa(String.fromCharCode(...new Uint8Array(hash)));
}

function arrayBufferToBase64(buffer: ArrayBuffer): string {
  return btoa(String.fromCharCode(...new Uint8Array(buffer)));
}
