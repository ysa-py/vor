/**
 * UnifiedShield Cloudflare Worker — Main Entry
 *
 * WebSocket relay, X25519 ECDH handshake, rate limiting, PADDING frame handling.
 *
 * IMPORTANT: Cloudflare is BLOCKED in Iran.
 * This worker serves as a relay endpoint OUTSIDE Iran only.
 * Users inside Iran connect via WebRTC or alternative CDN paths.
 */

import { handleRelay, performECDH, validateHandshake } from './relay';
import { obfuscateFrame, deobfuscateFrame, addPaddingFrame, stripPaddingFrame } from './obfuscator';
import { RateLimiter } from './rate-limiter';

export interface Env {
  RELAY_SECRET: string;
  MAX_CONNECTIONS: number;
  RATE_LIMIT_RPM: number;
  PADDING_MAX_BYTES: number;
  SESSIONS: KVNamespace;
}

const rateLimiter = new RateLimiter();
const activeConnections = new Map<string, WebSocket>();

export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    const url = new URL(request.url);

    // CORS headers for API endpoints
    const corsHeaders = {
      'Access-Control-Allow-Origin': '*',
      'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
      'Access-Control-Allow-Headers': 'Content-Type, Authorization',
    };

    if (request.method === 'OPTIONS') {
      return new Response(null, { headers: corsHeaders });
    }

    // Route requests
    switch (url.pathname) {
      case '/':
        return handleInfo();

      case '/health':
        return handleHealth(env);

      case '/handshake':
        return handleHandshake(request, env, corsHeaders);

      case '/relay':
        return handleRelayEndpoint(request, env, ctx);

      case '/ws':
        return handleWebSocket(request, env, ctx);

      case '/stats':
        return handleStats(corsHeaders);

      default:
        return new Response('Not Found', { status: 404 });
    }
  },
};

/* ────────── Info Endpoint ────────── */

function handleInfo(): Response {
  return Response.json({
    name: 'UnifiedShield Relay',
    version: '2.0.0',
    endpoints: {
      handshake: 'POST /handshake — X25519 key exchange',
      relay: 'POST /relay — Encrypted relay',
      ws: 'GET /ws — WebSocket relay',
      health: 'GET /health — Health check',
      stats: 'GET /stats — Connection stats',
    },
    warning: 'Cloudflare is blocked in Iran. Use alternative relay paths.',
  });
}

/* ────────── Health Check ────────── */

function handleHealth(env: Env): Response {
  const connections = activeConnections.size;
  const maxConn = env.MAX_CONNECTIONS || 1000;

  return Response.json({
    status: connections < maxConn ? 'healthy' : 'overloaded',
    connections,
    maxConnections: maxConn,
    uptime: Date.now(),
  });
}

/* ────────── X25519 ECDH Handshake ────────── */

async function handleHandshake(
  request: Request,
  env: Env,
  corsHeaders: Record<string, string>
): Promise<Response> {
  if (request.method !== 'POST') {
    return Response.json({ error: 'Method not allowed' }, { status: 405, headers: corsHeaders });
  }

  try {
    const body = await request.json() as {
      clientPublicKey: string; // base64-encoded 32 bytes
      timestamp: number;
      nonce: string;
    };

    // Validate timestamp (must be within 5 minutes)
    const now = Date.now() / 1000;
    if (Math.abs(now - body.timestamp) > 300) {
      return Response.json(
        { error: 'Timestamp out of range' },
        { status: 400, headers: corsHeaders }
      );
    }

    // Perform ECDH
    const { serverPublicKey, sharedSecret } = await performECDH(body.clientPublicKey);

    // Store session
    const sessionId = crypto.randomUUID();
    await env.SESSIONS.put(
      `session:${sessionId}`,
      JSON.stringify({
        sharedSecret,
        clientPublicKey: body.clientPublicKey,
        createdAt: Date.now(),
        lastActivity: Date.now(),
      }),
      { expirationTtl: 3600 }
    );

    return Response.json(
      {
        sessionId,
        serverPublicKey,
        algorithms: ['AES-256-GCM', 'ChaCha20-Poly1305'],
        padding: true,
      },
      { headers: corsHeaders }
    );
  } catch (err) {
    return Response.json(
      { error: 'Handshake failed', details: String(err) },
      { status: 400, headers: corsHeaders }
    );
  }
}

/* ────────── Relay Endpoint ────────── */

async function handleRelayEndpoint(
  request: Request,
  env: Env,
  ctx: ExecutionContext
): Promise<Response> {
  if (request.method !== 'POST') {
    return Response.json({ error: 'Method not allowed' }, { status: 405 });
  }

  // Rate limiting
  const clientIP = request.headers.get('CF-Connecting-IP') ?? 'unknown';
  if (!rateLimiter.check(clientIP, env.RATE_LIMIT_RPM || 600)) {
    return Response.json({ error: 'Rate limit exceeded' }, { status: 429 });
  }

  try {
    const body = await request.json() as {
      sessionId: string;
      nonce: string;
      ciphertext: string;
      targetHost: string;
      targetPort: number;
    };

    // Validate session
    const sessionData = await env.SESSIONS.get(`session:${body.sessionId}`);
    if (!sessionData) {
      return Response.json({ error: 'Invalid session' }, { status: 401 });
    }

    const session = JSON.parse(sessionData);

    // Update last activity
    await env.SESSIONS.put(
      `session:${body.sessionId}`,
      JSON.stringify({ ...session, lastActivity: Date.now() }),
      { expirationTtl: 3600 }
    );

    // Deobfuscate the frame
    const frameData = deobfuscateFrame(body.ciphertext, session.sharedSecret);

    // Forward to target (via relay logic)
    const response = await handleRelay({
      targetHost: body.targetHost,
      targetPort: body.targetPort,
      data: frameData,
    });

    // Obfuscate response
    const responseData = obfuscateFrame(response, session.sharedSecret);

    // Add padding
    const paddedResponse = addPaddingFrame(responseData, env.PADDING_MAX_BYTES || 512);

    return Response.json({
      nonce: crypto.randomUUID(),
      data: paddedResponse,
    });
  } catch (err) {
    return Response.json(
      { error: 'Relay failed', details: String(err) },
      { status: 500 }
    );
  }
}

/* ────────── WebSocket ────────── */

async function handleWebSocket(
  request: Request,
  env: Env,
  ctx: ExecutionContext
): Promise<Response> {
  const upgradeHeader = request.headers.get('Upgrade');
  if (upgradeHeader !== 'websocket') {
    return Response.json({ error: 'Expected WebSocket upgrade' }, { status: 400 });
  }

  // Rate limiting
  const clientIP = request.headers.get('CF-Connecting-IP') ?? 'unknown';
  if (!rateLimiter.check(clientIP, env.RATE_LIMIT_RPM || 600)) {
    return Response.json({ error: 'Rate limit exceeded' }, { status: 429 });
  }

  // Check max connections
  if (activeConnections.size >= (env.MAX_CONNECTIONS || 1000)) {
    return Response.json({ error: 'Max connections reached' }, { status: 503 });
  }

  const [client, server] = Object.values(new WebSocketPair()) as [WebSocket, WebSocket];

  const connId = crypto.randomUUID();
  activeConnections.set(connId, server);

  server.accept();

  server.addEventListener('message', async (event) => {
    try {
      const data = typeof event.data === 'string' ? JSON.parse(event.data) : event.data;

      // Handle different frame types
      switch (data.type) {
        case 'handshake': {
          const { serverPublicKey, sharedSecret } = await performECDH(data.clientPublicKey);
          const sessionId = crypto.randomUUID();

          await env.SESSIONS.put(
            `session:${sessionId}`,
            JSON.stringify({
              sharedSecret,
              clientPublicKey: data.clientPublicKey,
              createdAt: Date.now(),
              lastActivity: Date.now(),
            }),
            { expirationTtl: 3600 }
          );

          server.send(JSON.stringify({
            type: 'handshake-response',
            sessionId,
            serverPublicKey,
          }));
          break;
        }

        case 'relay': {
          const sessionData = await env.SESSIONS.get(`session:${data.sessionId}`);
          if (!sessionData) {
            server.send(JSON.stringify({ type: 'error', error: 'Invalid session' }));
            return;
          }

          const session = JSON.parse(sessionData);

          // Deobfuscate → relay → obfuscate
          const frameData = deobfuscateFrame(data.ciphertext, session.sharedSecret);
          const response = await handleRelay({
            targetHost: data.targetHost,
            targetPort: data.targetPort,
            data: frameData,
          });

          const responseData = obfuscateFrame(response, session.sharedSecret);
          const paddedResponse = addPaddingFrame(responseData, env.PADDING_MAX_BYTES || 512);

          server.send(JSON.stringify({
            type: 'relay-response',
            data: paddedResponse,
          }));
          break;
        }

        case 'ping': {
          server.send(JSON.stringify({ type: 'pong', timestamp: Date.now() }));
          break;
        }

        default:
          server.send(JSON.stringify({ type: 'error', error: 'Unknown frame type' }));
      }
    } catch (err) {
      server.send(JSON.stringify({ type: 'error', error: String(err) }));
    }
  });

  server.addEventListener('close', () => {
    activeConnections.delete(connId);
  });

  server.addEventListener('error', () => {
    activeConnections.delete(connId);
  });

  return new Response(null, {
    status: 101,
    webSocket: client,
  });
}

/* ────────── Stats ────────── */

function handleStats(corsHeaders: Record<string, string>): Response {
  return Response.json(
    {
      activeConnections: activeConnections.size,
      rateLimiter: rateLimiter.getStats(),
    },
    { headers: corsHeaders }
  );
}
