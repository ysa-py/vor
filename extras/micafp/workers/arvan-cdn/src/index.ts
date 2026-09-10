/**
 * Arvan Cloud FaaS Worker — Shield Worker v6.0
 *
 * Arvan Cloud is an Iranian CDN that is NEVER blocked in Iran.
 * This worker runs on Arvan Cloud Functions (FaaS).
 *
 * Deployment requirements:
 * - Iranian phone number required for account verification
 * - Diaspora operators handle deployment
 * - Routes through Arvan's Iranian CDN nodes
 * - Optimized for domestic Iranian traffic
 *
 * API Format: handler(event, context)
 * - event: Arvan Cloud Function event object
 * - context: Arvan Cloud Function context
 *
 * @see https://www.arvancloud.com/en/products/cloud-faas
 */

// ============================================================
// Types — Arvan Cloud Functions API
// ============================================================

interface ArvanEvent {
  /** HTTP method */
  httpMethod: string;
  /** Request path */
  path: string;
  /** Query string parameters */
  queryStringParameters: Record<string, string>;
  /** Request headers */
  headers: Record<string, string>;
  /** Request body (base64 encoded if binary) */
  body: string;
  /** Whether body is base64-encoded */
  isBase64Encoded: boolean;
  /** Source IP (we ignore this for privacy) */
  requestContext: {
    requestId: string;
    stage: string;
    // We deliberately do NOT log or use sourceIp
  };
}

interface ArvanContext {
  functionName: string;
  functionVersion: string;
  invokedFunctionArn: string;
  memoryLimitInMB: number;
  awsRequestId: string;
  getRemainingTimeInMillis: () => number;
}

interface ArvanResponse {
  statusCode: number;
  headers: Record<string, string>;
  body: string;
  isBase64Encoded: boolean;
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
  hmacSecret: process.env?.HMAC_SECRET ||
    '0000000000000000000000000000000000000000000000000000000000000000',
  relayBackends: [
    { host: 'relay1.shield.arvan', port: 443, transport: 'ws', weight: 40 },
    { host: 'relay2.shield.arvan', port: 443, transport: 'ws', weight: 35 },
    { host: 'relay3.shield.arvan', port: 443, transport: 'http2', weight: 25 },
  ],
  rateLimitPerMinute: 10,
  frontDomain: 'cdn-arvan.ir',
  connectionTimeout: 300000,
  maxConcurrentConnections: 800,
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

const crypto = require('crypto');

function verifyHMAC(event: ArvanEvent, secret: string): boolean {
  const authHeader = event.headers['authorization'] ||
    event.headers['Authorization'];
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    return false;
  }

  const providedHmac = authHeader.slice(7);
  const timestamp = event.headers['x-timestamp'] ||
    event.headers['X-Timestamp'];
  if (!timestamp) return false;

  // Check timestamp freshness (±300 seconds)
  const requestTime = parseInt(timestamp, 10);
  const now = Math.floor(Date.now() / 1000);
  if (isNaN(requestTime) || Math.abs(now - requestTime) > 300) {
    return false;
  }

  // Compute expected HMAC
  const method = event.httpMethod;
  const path = event.path;
  const bodyHash = event.body
    ? crypto.createHash('sha256').update(event.body).digest('hex')
    : '';

  const message = `${timestamp}:${method}:${path}:${bodyHash}`;
  const expectedHmac = crypto
    .createHmac('sha256', Buffer.from(secret, 'hex'))
    .update(message)
    .digest('hex');

  // Constant-time comparison
  return constantTimeEqual(providedHmac, expectedHmac);
}

function constantTimeEqual(a: string, b: string): boolean {
  if (a.length !== b.length) return false;
  let result = 0;
  for (let i = 0; i < a.length; i++) {
    result |= a.charCodeAt(i) ^ b.charCodeAt(i);
  }
  return result === 0;
}

// ============================================================
// Backend Selection
// ============================================================

function selectBackend(
  backends: RelayBackend[],
  preferredTransport?: string,
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
// Proxy Logic
// ============================================================

const http = require('http');
const https = require('https');

let concurrentConnections = 0;
const rateLimiter = new RateLimiter(CONFIG.rateLimitPerMinute);
let lastCleanup = Date.now();

/**
 * Proxy request to backend relay server
 *
 * Uses domain fronting:
 * - Connects to front domain (allowed by Iranian CDN)
 * - Routes to actual backend via custom headers
 */
async function proxyToBackend(
  event: ArvanEvent,
  backend: RelayBackend,
): Promise<ArvanResponse> {
  return new Promise((resolve) => {
    const isSecure = true; // Always use HTTPS for CDN
    const transport = isSecure ? https : http;

    const proxyHeaders: Record<string, string> = {
      ...event.headers,
      'Host': CONFIG.frontDomain,
      'X-Forwarded-Host': backend.host,
      'X-Real-Port': backend.port.toString(),
    };
    delete proxyHeaders['x-timestamp'];
    delete proxyHeaders['X-Timestamp'];
    delete proxyHeaders['authorization'];
    delete proxyHeaders['Authorization'];

    const path = event.path +
      (event.queryStringParameters && Object.keys(event.queryStringParameters).length > 0
        ? '?' + new URLSearchParams(event.queryStringParameters).toString()
        : '');

    const options = {
      hostname: CONFIG.frontDomain,
      port: 443,
      path,
      method: event.httpMethod,
      headers: proxyHeaders,
      timeout: CONFIG.connectionTimeout,
    };

    const req = transport.request(options, (res: any) => {
      let body = '';
      res.on('data', (chunk: Buffer) => { body += chunk.toString(); });
      res.on('end', () => {
        concurrentConnections--;
        resolve({
          statusCode: res.statusCode || 502,
          headers: {
            'Content-Type': res.headers['content-type'] || 'application/octet-stream',
            'Cache-Control': 'no-store',
          },
          body,
          isBase64Encoded: false,
        });
      });
    });

    req.on('error', (error: Error) => {
      concurrentConnections--;
      resolve(createMimicResponse(502));
    });

    req.on('timeout', () => {
      concurrentConnections--;
      req.destroy();
      resolve(createMimicResponse(504));
    });

    if (event.body) {
      const bodyData = event.isBase64Encoded
        ? Buffer.from(event.body, 'base64')
        : event.body;
      req.write(bodyData);
    }

    req.end();
    concurrentConnections++;
  });
}

// ============================================================
// Mimic Responses
// ============================================================

function createMimicResponse(status: number): ArvanResponse {
  const pages: Record<number, string> = {
    400: '<!DOCTYPE html><html><head><title>Bad Request</title></head><body><h1>Bad Request</h1><p>Your browser sent a request that this server could not understand.</p></body></html>',
    403: '<!DOCTYPE html><html><head><title>Forbidden</title></head><body><h1>Forbidden</h1><p>You don\'t have permission to access this resource.</p></body></html>',
    404: '<!DOCTYPE html><html><head><title>Not Found</title></head><body><h1>Not Found</h1><p>The requested URL was not found on this server.</p></body></html>',
    429: '<!DOCTYPE html><html><head><title>Too Many Requests</title></head><body><h1>Too Many Requests</h1><p>Please slow down and try again later.</p></body></html>',
    502: '<!DOCTYPE html><html><head><title>Bad Gateway</title></head><body><h1>Bad Gateway</h1><p>The server received an invalid response from an upstream server.</p></body></html>',
    503: '<!DOCTYPE html><html><head><title>Service Unavailable</title></head><body><h1>Service Unavailable</h1><p>The server is temporarily unable to handle your request.</p></body></html>',
    504: '<!DOCTYPE html><html><head><title>Gateway Timeout</title></head><body><h1>Gateway Timeout</h1><p>The server did not receive a timely response.</p></body></html>',
  };

  return {
    statusCode: status,
    headers: {
      'Content-Type': 'text/html',
      'Cache-Control': 'no-store',
      'Server': 'ArvanCloud', // Mimic Arvan Cloud default
    },
    body: pages[status] || pages[404],
    isBase64Encoded: false,
  };
}

// ============================================================
// Request Handler — Arvan Cloud Functions Format
// ============================================================

/**
 * Main handler for Arvan Cloud Functions
 *
 * @param event - Arvan Cloud Function event
 * @param context - Arvan Cloud Function context
 * @returns Arvan Cloud Function response
 */
exports.handler = async function handler(
  event: ArvanEvent,
  context: ArvanContext,
): Promise<ArvanResponse> {
  // Periodic cleanup
  const now = Date.now();
  if (now - lastCleanup > 60000) {
    rateLimiter.cleanup();
    lastCleanup = now;
  }

  const method = event.httpMethod;
  const path = event.path;

  // Health check
  if (path === '/health' && method === 'GET') {
    return {
      statusCode: 200,
      headers: {
        'Content-Type': 'application/json',
        'Cache-Control': 'no-store',
      },
      body: JSON.stringify({
        status: 'ok',
        connections: concurrentConnections,
      }),
      isBase64Encoded: false,
    };
  }

  // Front page (mimics normal Iranian CDN website)
  if (path === '/' || path === '/index.html') {
    return {
      statusCode: 200,
      headers: {
        'Content-Type': 'text/html',
        'Cache-Control': 'public, max-age=3600',
        'Server': 'ArvanCloud',
      },
      body: '<!DOCTYPE html><html lang="fa"><head><meta charset="UTF-8"><title>سرویس ابری</title><style>body{font-family:Tahoma,sans-serif;padding:40px;color:#333;direction:rtl;}</style></head><body><h1>سرویس ابری آروان</h1><p>سرویس توزیع محتوا و محاسبات لبه</p></body></html>',
      isBase64Encoded: false,
    };
  }

  // Favicon
  if (path === '/favicon.ico') {
    return { statusCode: 204, headers: {}, body: '', isBase64Encoded: false };
  }

  // Robots.txt
  if (path === '/robots.txt') {
    return {
      statusCode: 200,
      headers: { 'Content-Type': 'text/plain' },
      body: 'User-agent: *\nDisallow: /private/\n',
      isBase64Encoded: false,
    };
  }

  // WebSocket / relay paths
  if (path.startsWith('/ws') || path.startsWith('/connect') || path.startsWith('/api/')) {
    // Verify HMAC authentication
    if (!verifyHMAC(event, CONFIG.hmacSecret)) {
      return createMimicResponse(403);
    }

    // Rate limit check
    const clientId = (event.headers['authorization'] || '').slice(7, 20) || 'anon';
    if (!rateLimiter.isAllowed(clientId)) {
      return createMimicResponse(429);
    }

    // Check concurrent connection limit
    if (concurrentConnections >= CONFIG.maxConcurrentConnections) {
      return createMimicResponse(503);
    }

    // Select backend and proxy
    const transport = event.queryStringParameters?.transport;
    const backend = selectBackend(CONFIG.relayBackends, transport);

    // Note: Arvan Cloud Functions don't support WebSocket upgrade natively
    // WebSocket connections must use HTTP long-polling or SSE fallback
    // The actual WebSocket relay happens via HTTP/2 streaming

    return proxyToBackend(event, backend);
  }

  // Default: 404
  return createMimicResponse(404);
};
