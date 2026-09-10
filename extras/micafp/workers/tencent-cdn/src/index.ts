/**
 * Tencent EdgeOne CDN Worker — Shield Worker v6.0
 *
 * Tencent EdgeOne (Chinese CDN) is NOT blocked in Iran.
 * This worker runs on Tencent Cloud EdgeOne EdgeFunction.
 *
 * Features:
 * - EdgeOne Function format: addEventListener('fetch', (event) => {})
 * - Same proxy logic as universal worker
 * - Routes through Tencent CDN (Chinese CDN — not blocked in Iran)
 * - Domain fronting compatible
 * - Privacy-first: no IP logging
 * - Supports WebSocket upgrade
 *
 * @see https://cloud.tencent.com/product/teo
 */

// ============================================================
// Types — Tencent EdgeOne EdgeFunction API
// ============================================================

/**
 * Tencent EdgeOne uses the Service Worker API format
 * similar to Cloudflare Workers but without CF-specific APIs.
 *
 * Available globals:
 * - addEventListener
 * - Request / Response / Headers (Web Standards)
 * - fetch (subrequest API)
 * - crypto (Web Crypto API)
 * - WebSocket / WebSocketPair (for WebSocket upgrade)
 *
 * Tencent-specific:
 * - ENV object for environment variables
 */

interface EdgeOneEvent {
  respondWith(response: Promise<Response> | Response): void;
  waitUntil(promise: Promise<any>): void;
  request: Request;
  passThroughOnException(): void;
}

// ============================================================
// Configuration
// ============================================================

interface WorkerConfig {
  hmacSecret: string;
  relayBackends: RelayBackend[];
  rateLimitPerMinute: number;
  frontDomain: string;
  connectionTimeout: number;
  maxConcurrentConnections: number;
  /** Tencent-specific: region preference for backend selection */
  preferredRegion: string;
}

interface RelayBackend {
  host: string;
  port: number;
  transport: 'ws' | 'quic' | 'http2';
  weight: number;
  region: string;
}

const CONFIG: WorkerConfig = {
  hmacSecret: (typeof ENV !== 'undefined' && ENV.HMAC_SECRET) ||
    '0000000000000000000000000000000000000000000000000000000000000000',
  relayBackends: [
    { host: 'relay1.shield.tencent', port: 443, transport: 'ws', weight: 40, region: 'ap-guangzhou' },
    { host: 'relay2.shield.tencent', port: 443, transport: 'ws', weight: 35, region: 'ap-shanghai' },
    { host: 'relay3.shield.tencent', port: 443, transport: 'http2', weight: 25, region: 'ap-beijing' },
  ],
  rateLimitPerMinute: 10,
  frontDomain: 'cdn-tencent.cn',
  connectionTimeout: 300000,
  maxConcurrentConnections: 1200,
  preferredRegion: 'ap-guangzhou',
};

// ============================================================
// Rate Limiter
// ============================================================

class RateLimiter {
  private clients: Map<string, { count: number; windowStart: number }> = new Map();
  private readonly maxRequests: number;
  private readonly windowMs: number;

  constructor(maxRequests: number, windowMs: number = 60000) {
    this.maxRequests = maxRequests;
    this.windowMs = windowMs;
  }

  isAllowed(clientId: string): boolean {
    const now = Date.now();
    const entry = this.clients.get(clientId);
    if (!entry || (now - entry.windowStart) > this.windowMs) {
      this.clients.set(clientId, { count: 1, windowStart: now });
      return true;
    }
    entry.count++;
    return entry.count <= this.maxRequests;
  }

  cleanup(): void {
    const now = Date.now();
    for (const [id, entry] of this.clients) {
      if ((now - entry.windowStart) > this.windowMs) {
        this.clients.delete(id);
      }
    }
  }
}

// ============================================================
// HMAC Authentication
// ============================================================

async function verifyHMAC(request: Request, secret: string): Promise<boolean> {
  const authHeader = request.headers.get('Authorization');
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    return false;
  }

  const providedHmac = authHeader.slice(7);
  const timestamp = request.headers.get('X-Timestamp');
  if (!timestamp) return false;

  // Check timestamp freshness (±300 seconds / 5 minutes)
  const requestTime = parseInt(timestamp, 10);
  const now = Math.floor(Date.now() / 1000);
  if (isNaN(requestTime) || Math.abs(now - requestTime) > 300) {
    return false;
  }

  // Compute body hash
  let bodyHash = '';
  if (request.body) {
    const body = await request.clone().arrayBuffer();
    const hashBuffer = await crypto.subtle.digest('SHA-256', body);
    bodyHash = bufferToHex(hashBuffer);
  }

  // Compute expected HMAC
  const url = new URL(request.url);
  const message = `${timestamp}:${request.method}:${url.pathname}:${bodyHash}`;

  const encoder = new TextEncoder();
  const key = await crypto.subtle.importKey(
    'raw',
    hexToBuffer(secret),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign'],
  );

  const signature = await crypto.subtle.sign('HMAC', key, encoder.encode(message));
  const expectedHmac = bufferToHex(signature);

  return constantTimeEqual(providedHmac, expectedHmac);
}

// ============================================================
// Backend Selection (Region-Aware)
// ============================================================

function selectBackend(
  backends: RelayBackend[],
  preferredTransport?: string | null,
  preferredRegion?: string,
): RelayBackend {
  let candidates = backends;

  // Prefer same-region backends for lower latency
  if (preferredRegion) {
    const regionalBackends = backends.filter(b => b.region === preferredRegion);
    if (regionalBackends.length > 0) {
      candidates = regionalBackends;
    }
  }

  // Filter by transport
  if (preferredTransport) {
    const filtered = candidates.filter(b => b.transport === preferredTransport);
    if (filtered.length > 0) candidates = filtered;
  }

  // Weighted random selection
  const totalWeight = candidates.reduce((sum, b) => sum + b.weight, 0);
  let random = Math.random() * totalWeight;

  for (const backend of candidates) {
    random -= backend.weight;
    if (random <= 0) return backend;
  }
  return candidates[candidates.length - 1];
}

// ============================================================
// Domain Fronting Proxy
// ============================================================

let concurrentConnections = 0;
const rateLimiter = new RateLimiter(CONFIG.rateLimitPerMinute);
let lastCleanup = Date.now();

async function proxyToBackend(
  request: Request,
  backend: RelayBackend,
): Promise<Response> {
  const url = new URL(request.url);
  const backendUrl = `https://${CONFIG.frontDomain}${url.pathname}${url.search}`;

  const headers = new Headers(request.headers);
  headers.set('Host', CONFIG.frontDomain);
  headers.set('X-Forwarded-Host', backend.host);
  headers.set('X-Real-Port', backend.port.toString());
  headers.set('X-Backend-Region', backend.region);
  headers.delete('X-Timestamp');
  headers.delete('Authorization');

  try {
    const proxyRequest = new Request(backendUrl, {
      method: request.method,
      headers,
      body: request.body,
      // @ts-ignore - duplex needed for streaming
      duplex: 'half',
    });

    return await fetch(proxyRequest, {
      // Tencent EdgeOne: use specified origin
      // @ts-ignore
      origin: CONFIG.frontDomain,
    });
  } catch {
    return createMimicResponse(502);
  }
}

// ============================================================
// WebSocket Handler
// ============================================================

async function handleWebSocket(
  request: Request,
  backend: RelayBackend,
): Promise<Response> {
  const url = new URL(request.url);

  // Tencent EdgeOne supports WebSocketPair for WebSocket upgrade
  // @ts-ignore - WebSocketPair is available in EdgeOne runtime
  const pair = new WebSocketPair();
  const [client, server] = [pair[0], pair[1]];

  // Accept the WebSocket on the server side
  server.accept();

  // Connect to backend WebSocket
  const backendWsUrl = `wss://${CONFIG.frontDomain}${url.pathname}?X-Forwarded-Host=${backend.host}&X-Real-Port=${backend.port}`;
  const backendWs = new WebSocket(backendWsUrl);

  const connectionId = crypto.randomUUID();
  concurrentConnections++;

  // Bidirectional relay
  server.addEventListener('message', (event: MessageEvent) => {
    if (backendWs.readyState === WebSocket.OPEN) {
      backendWs.send(event.data);
    }
  });

  backendWs.addEventListener('open', () => {
    // Backend connected — start relaying
  });

  backendWs.addEventListener('message', (event: MessageEvent) => {
    if (server.readyState === WebSocket.OPEN) {
      server.send(event.data);
    }
  });

  // Cleanup on close
  const cleanup = () => {
    concurrentConnections--;
    if (server.readyState === WebSocket.OPEN) server.close();
    if (backendWs.readyState === WebSocket.OPEN) backendWs.close();
  };

  server.addEventListener('close', cleanup);
  server.addEventListener('error', cleanup);
  backendWs.addEventListener('close', cleanup);
  backendWs.addEventListener('error', cleanup);

  // Connection timeout
  setTimeout(() => {
    cleanup();
  }, CONFIG.connectionTimeout);

  return new Response(null, {
    status: 101,
    // @ts-ignore - webSocket is EdgeOne-specific
    webSocket: client,
  });
}

// ============================================================
// Mimic Responses
// ============================================================

function createMimicResponse(status: number): Response {
  const pages: Record<number, string> = {
    400: '<!DOCTYPE html><html><head><title>Bad Request</title></head><body><h1>Bad Request</h1><p>Your browser sent a request that this server could not understand.</p></body></html>',
    403: '<!DOCTYPE html><html><head><title>Forbidden</title></head><body><h1>Forbidden</h1><p>You don\'t have permission to access this resource.</p></body></html>',
    404: '<!DOCTYPE html><html><head><title>Not Found</title></head><body><h1>Not Found</h1><p>The requested URL was not found on this server.</p></body></html>',
    429: '<!DOCTYPE html><html><head><title>Too Many Requests</title></head><body><h1>Too Many Requests</h1><p>Please slow down and try again later.</p></body></html>',
    500: '<!DOCTYPE html><html><head><title>Internal Server Error</title></head><body><h1>Internal Server Error</h1><p>The server encountered an internal error.</p></body></html>',
    502: '<!DOCTYPE html><html><head><title>Bad Gateway</title></head><body><h1>Bad Gateway</h1><p>The server received an invalid response from an upstream server.</p></body></html>',
    503: '<!DOCTYPE html><html><head><title>Service Unavailable</title></head><body><h1>Service Unavailable</h1><p>The server is temporarily unable to handle your request.</p></body></html>',
  };

  return new Response(pages[status] || pages[404], {
    status,
    headers: {
      'Content-Type': 'text/html',
      'Cache-Control': 'no-store',
      'Server': 'Tencent-COS', // Mimic Tencent Object Storage
    },
  });
}

// ============================================================
// Utility
// ============================================================

function bufferToHex(buffer: ArrayBuffer): string {
  return Array.from(new Uint8Array(buffer))
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
}

function hexToBuffer(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substr(i, 2), 16);
  }
  return bytes;
}

function constantTimeEqual(a: string, b: string): boolean {
  if (a.length !== b.length) return false;
  let result = 0;
  for (let i = 0; i < a.length; i++) {
    result |= a.charCodeAt(i) ^ b.charCodeAt(i);
  }
  return result === 0;
}

function getClientId(request: Request): string {
  const auth = request.headers.get('Authorization') || '';
  return auth.slice(7, 20) || 'anonymous';
}

// ============================================================
// Request Handler
// ============================================================

async function handleRequest(request: Request): Promise<Response> {
  // Periodic cleanup
  const now = Date.now();
  if (now - lastCleanup > 60000) {
    rateLimiter.cleanup();
    lastCleanup = now;
  }

  const url = new URL(request.url);
  const method = request.method;

  // ---- Public endpoints ----

  // Health check
  if (url.pathname === '/health' && method === 'GET') {
    return new Response(
      JSON.stringify({
        status: 'ok',
        connections: concurrentConnections,
        region: CONFIG.preferredRegion,
      }),
      {
        status: 200,
        headers: {
          'Content-Type': 'application/json',
          'Cache-Control': 'no-store',
        },
      },
    );
  }

  // Front page — mimics a legitimate Chinese CDN service
  if (url.pathname === '/' || url.pathname === '/index.html') {
    return new Response(
      `<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>CDN 加速服务</title>
  <style>
    body { font-family: -apple-system, BlinkMacSystemFont, 'PingFang SC', sans-serif; margin: 0; padding: 40px; color: #333; }
    h1 { font-size: 24px; color: #1a1a1a; }
    p { line-height: 1.6; color: #666; }
    .container { max-width: 800px; margin: 0 auto; }
  </style>
</head>
<body>
  <div class="container">
    <h1>CDN 加速服务</h1>
    <p>高性能内容分发与边缘计算服务</p>
    <p>如需支持，请联系您的客户经理</p>
  </div>
</body>
</html>`,
      {
        status: 200,
        headers: {
          'Content-Type': 'text/html; charset=utf-8',
          'Cache-Control': 'public, max-age=3600',
          'Server': 'Tencent-COS',
        },
      },
    );
  }

  // Favicon
  if (url.pathname === '/favicon.ico') {
    return new Response(null, { status: 204 });
  }

  // Robots.txt
  if (url.pathname === '/robots.txt') {
    return new Response('User-agent: *\nDisallow: /private/\n', {
      status: 200,
      headers: { 'Content-Type': 'text/plain' },
    });
  }

  // Well-known paths (mimic normal website)
  if (url.pathname === '/.well-known/security.txt') {
    return new Response('Contact: security@example.com\n', {
      status: 200,
      headers: { 'Content-Type': 'text/plain' },
    });
  }

  // ---- Authenticated relay endpoints ----

  if (url.pathname.startsWith('/ws') || url.pathname.startsWith('/connect') || url.pathname.startsWith('/api/')) {
    // Verify HMAC authentication
    const isAuthenticated = await verifyHMAC(request, CONFIG.hmacSecret);
    if (!isAuthenticated) {
      return createMimicResponse(403);
    }

    // Rate limit
    const clientId = getClientId(request);
    if (!rateLimiter.isAllowed(clientId)) {
      return createMimicResponse(429);
    }

    // Concurrent connection limit
    if (concurrentConnections >= CONFIG.maxConcurrentConnections) {
      return createMimicResponse(503);
    }

    // Select backend
    const transport = url.searchParams.get('transport');
    const backend = selectBackend(
      CONFIG.relayBackends,
      transport,
      CONFIG.preferredRegion,
    );

    // WebSocket upgrade
    const upgradeHeader = request.headers.get('Upgrade');
    if (upgradeHeader?.toLowerCase() === 'websocket') {
      return handleWebSocket(request, backend);
    }

    // HTTP relay
    concurrentConnections++;
    try {
      return await proxyToBackend(request, backend);
    } finally {
      concurrentConnections--;
    }
  }

  // All other paths: 404
  return createMimicResponse(404);
}

// ============================================================
// Event Listener — Tencent EdgeOne EdgeFunction Format
// ============================================================

addEventListener('fetch', (event: EdgeOneEvent) => {
  // Pass through on exception to prevent information leakage
  event.passThroughOnException();

  event.respondWith(
    handleRequest(event.request).catch(() => {
      // Never expose internal errors — return a generic 503
      return createMimicResponse(503);
    }),
  );
});
