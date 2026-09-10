/**
 * ByteDance Volcengine EdgeRoutine Worker — Shield Worker v6.0
 *
 * ByteDance Volcengine (Chinese CDN) is NOT blocked in Iran.
 * This worker runs on Volcengine EdgeRoutine.
 *
 * Features:
 * - EdgeRoutine format: addEventListener('fetch', (event) => {})
 * - Same proxy logic as universal worker
 * - Routes through ByteDance CDN (Chinese CDN — not blocked in Iran)
 * - Domain fronting compatible
 * - Privacy-first: no IP logging
 *
 * @see https://www.volcengine.com/en/product/edge-function
 */

// ============================================================
// Types — Volcengine EdgeRoutine API
// ============================================================

/**
 * Volcengine EdgeRoutine uses the Service Worker API format
 * with addEventListener('fetch', handler)
 *
 * The Request/Response objects follow Web Standards
 */

interface EdgeRoutineEvent {
  respondWith(response: Promise<Response> | Response): void;
  waitUntil(promise: Promise<any>): void;
  request: Request;
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
}

interface RelayBackend {
  host: string;
  port: number;
  transport: 'ws' | 'quic' | 'http2';
  weight: number;
}

const CONFIG: WorkerConfig = {
  hmacSecret: (typeof ENV !== 'undefined' && ENV.HMAC_SECRET) ||
    '0000000000000000000000000000000000000000000000000000000000000000',
  relayBackends: [
    { host: 'relay1.shield.volc', port: 443, transport: 'ws', weight: 40 },
    { host: 'relay2.shield.volc', port: 443, transport: 'ws', weight: 35 },
    { host: 'relay3.shield.volc', port: 443, transport: 'http2', weight: 25 },
  ],
  rateLimitPerMinute: 10,
  frontDomain: 'cdn-volc.cn',
  connectionTimeout: 300000,
  maxConcurrentConnections: 1000,
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

  // Check timestamp freshness
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
// Backend Selection
// ============================================================

function selectBackend(
  backends: RelayBackend[],
  preferredTransport?: string | null,
): RelayBackend {
  let candidates = backends;
  if (preferredTransport) {
    const filtered = backends.filter(b => b.transport === preferredTransport);
    if (filtered.length > 0) candidates = filtered;
  }

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
  headers.delete('X-Timestamp');
  headers.delete('Authorization');

  try {
    const proxyRequest = new Request(backendUrl, {
      method: request.method,
      headers,
      body: request.body,
      // @ts-ignore
      duplex: 'half',
    });

    return await fetch(proxyRequest);
  } catch {
    return createMimicResponse(502);
  }
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
    502: '<!DOCTYPE html><html><head><title>Bad Gateway</title></head><body><h1>Bad Gateway</h1><p>The server received an invalid response from an upstream server.</p></body></html>',
    503: '<!DOCTYPE html><html><head><title>Service Unavailable</title></head><body><h1>Service Unavailable</h1><p>The server is temporarily unable to handle your request.</p></body></html>',
  };

  return new Response(pages[status] || pages[404], {
    status,
    headers: {
      'Content-Type': 'text/html',
      'Cache-Control': 'no-store',
      'Server': 'Tengine', // ByteDance uses Tengine
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

  // Health check
  if (url.pathname === '/health' && method === 'GET') {
    return new Response(
      JSON.stringify({ status: 'ok', connections: concurrentConnections }),
      {
        status: 200,
        headers: {
          'Content-Type': 'application/json',
          'Cache-Control': 'no-store',
        },
      },
    );
  }

  // Front page
  if (url.pathname === '/' || url.pathname === '/index.html') {
    return new Response(
      '<!DOCTYPE html><html><head><meta charset="UTF-8"><title>CDN Service</title><style>body{font-family:-apple-system,sans-serif;padding:40px;color:#333;}</style></head><body><h1>Edge Service</h1><p>High-performance content delivery and edge computing services.</p></body></html>',
      {
        status: 200,
        headers: {
          'Content-Type': 'text/html',
          'Cache-Control': 'public, max-age=3600',
          'Server': 'Tengine',
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

  // WebSocket / relay paths
  if (url.pathname.startsWith('/ws') || url.pathname.startsWith('/connect') || url.pathname.startsWith('/api/')) {
    // Verify HMAC
    const isAuthenticated = await verifyHMAC(request, CONFIG.hmacSecret);
    if (!isAuthenticated) {
      return createMimicResponse(403);
    }

    // Rate limit
    const clientId = getClientId(request);
    if (!rateLimiter.isAllowed(clientId)) {
      return createMimicResponse(429);
    }

    // Concurrent limit
    if (concurrentConnections >= CONFIG.maxConcurrentConnections) {
      return createMimicResponse(503);
    }

    // Handle WebSocket upgrade
    const upgradeHeader = request.headers.get('Upgrade');
    if (upgradeHeader?.toLowerCase() === 'websocket') {
      // Volcengine EdgeRoutine supports WebSocket
      // The platform handles the upgrade automatically
      // We proxy the connection to the backend
      const backend = selectBackend(
        CONFIG.relayBackends,
        url.searchParams.get('transport'),
      );

      // For WebSocket, we need to establish a backend WebSocket connection
      // and relay between client and backend
      return proxyToBackend(request, backend);
    }

    // HTTP relay
    const backend = selectBackend(
      CONFIG.relayBackends,
      url.searchParams.get('transport'),
    );

    concurrentConnections++;
    try {
      return await proxyToBackend(request, backend);
    } finally {
      concurrentConnections--;
    }
  }

  return createMimicResponse(404);
}

// ============================================================
// Event Listener — Volcengine EdgeRoutine Format
// ============================================================

addEventListener('fetch', (event: EdgeRoutineEvent) => {
  event.respondWith(handleRequest(event.request));
});
