/**
 * Alibaba Cloud Function Compute Worker — Shield Worker v6.0
 *
 * Alibaba Cloud (Chinese CDN) is NOT blocked in Iran.
 * This worker runs on Alibaba Cloud Function Compute (FC).
 *
 * Features:
 * - Supports HTTP trigger and API Gateway trigger
 * - Alibaba FC format: exports.handler = async (event, context, callback) => {}
 * - Routes through Alibaba CDN (Chinese CDN — not blocked in Iran)
 * - Same proxy logic as universal worker
 * - Domain fronting compatible
 *
 * @see https://www.alibabacloud.com/en/product/function-compute
 */

// ============================================================
// Types — Alibaba Cloud Function Compute API
// ============================================================

/**
 * HTTP Trigger event format
 * Used when Function Compute is invoked via HTTP trigger
 */
interface FCHttpEvent {
  method: string;
  path: string;
  query: Record<string, string>;
  headers: Record<string, string>;
  body: string;
  isBase64Encoded: boolean;
  clientIP: string; // We deliberately do NOT log this
}

/**
 * API Gateway event format
 * Used when invoked via API Gateway
 */
interface FCApiGatewayEvent {
  path: string;
  httpMethod: string;
  headers: Record<string, string>;
  queryParameters: Record<string, string>;
  pathParameters: Record<string, string>;
  body: string;
  isBase64Encoded: boolean;
  requestContext: {
    accountId: string;
    stage: string;
    requestId: string;
    // We do NOT use identity.sourceIp
  };
}

interface FCContext {
  requestId: string;
  credentials: {
    accessKeyId: string;
    accessKeySecret: string;
    securityToken: string;
  };
  function: {
    name: string;
    handler: string;
    memorySize: number;
    timeout: number;
    initializer: string;
  };
  service: {
    name: string;
    logProject: string;
    logStore: string;
    qualifier: string;
    versionId: string;
  };
  region: string;
  accountId: string;
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
    { host: 'relay1.shield.alibaba', port: 443, transport: 'ws', weight: 40 },
    { host: 'relay2.shield.alibaba', port: 443, transport: 'ws', weight: 35 },
    { host: 'relay3.shield.alibaba', port: 443, transport: 'http2', weight: 25 },
  ],
  rateLimitPerMinute: 10,
  frontDomain: 'cdn-alibaba.cn',
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

const crypto = require('crypto');

function verifyHMAC(
  method: string,
  path: string,
  headers: Record<string, string>,
  body: string,
  isBase64Encoded: boolean,
  secret: string,
): boolean {
  const authHeader = headers['authorization'] || headers['Authorization'];
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    return false;
  }

  const providedHmac = authHeader.slice(7);
  const timestamp = headers['x-timestamp'] || headers['X-Timestamp'];
  if (!timestamp) return false;

  // Check timestamp freshness
  const requestTime = parseInt(timestamp, 10);
  const now = Math.floor(Date.now() / 1000);
  if (isNaN(requestTime) || Math.abs(now - requestTime) > 300) {
    return false;
  }

  // Compute body hash
  const bodyData = isBase64Encoded ? Buffer.from(body, 'base64') : body;
  const bodyHash = crypto.createHash('sha256').update(bodyData).digest('hex');

  // Compute expected HMAC
  const message = `${timestamp}:${method}:${path}:${bodyHash}`;
  const expectedHmac = crypto
    .createHmac('sha256', Buffer.from(secret, 'hex'))
    .update(message)
    .digest('hex');

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
// Proxy
// ============================================================

const https = require('https');
const http = require('http');

let concurrentConnections = 0;
const rateLimiter = new RateLimiter(CONFIG.rateLimitPerMinute);
let lastCleanup = Date.now();

interface ProxyResult {
  statusCode: number;
  headers: Record<string, string>;
  body: string;
  isBase64Encoded: boolean;
}

async function proxyToBackend(
  method: string,
  path: string,
  query: Record<string, string>,
  headers: Record<string, string>,
  body: string,
  isBase64Encoded: boolean,
  backend: RelayBackend,
): Promise<ProxyResult> {
  return new Promise((resolve) => {
    const proxyHeaders: Record<string, string> = {
      ...headers,
      'Host': CONFIG.frontDomain,
      'X-Forwarded-Host': backend.host,
      'X-Real-Port': backend.port.toString(),
    };
    delete proxyHeaders['x-timestamp'];
    delete proxyHeaders['X-Timestamp'];
    delete proxyHeaders['authorization'];
    delete proxyHeaders['Authorization'];

    const queryString = Object.keys(query).length > 0
      ? '?' + new URLSearchParams(query).toString()
      : '';

    const options = {
      hostname: CONFIG.frontDomain,
      port: 443,
      path: path + queryString,
      method,
      headers: proxyHeaders,
      timeout: CONFIG.connectionTimeout,
    };

    const req = https.request(options, (res: any) => {
      const chunks: Buffer[] = [];
      res.on('data', (chunk: Buffer) => chunks.push(chunk));
      res.on('end', () => {
        concurrentConnections--;
        const responseBody = Buffer.concat(chunks);

        // Check if response is binary
        const contentType = res.headers['content-type'] || '';
        const isBinary = !contentType.includes('text') &&
          !contentType.includes('json') &&
          !contentType.includes('xml');

        resolve({
          statusCode: res.statusCode || 502,
          headers: {
            'Content-Type': contentType || 'application/octet-stream',
            'Cache-Control': 'no-store',
          },
          body: isBinary
            ? responseBody.toString('base64')
            : responseBody.toString('utf-8'),
          isBase64Encoded: isBinary,
        });
      });
    });

    req.on('error', () => {
      concurrentConnections--;
      resolve(createMimicResponse(502));
    });

    req.on('timeout', () => {
      concurrentConnections--;
      req.destroy();
      resolve(createMimicResponse(504));
    });

    if (body) {
      const bodyData = isBase64Encoded
        ? Buffer.from(body, 'base64')
        : body;
      req.write(bodyData);
    }

    req.end();
    concurrentConnections++;
  });
}

// ============================================================
// Mimic Responses
// ============================================================

function createMimicResponse(status: number): ProxyResult {
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
      'Server': 'Tengine', // Alibaba's nginx fork
    },
    body: pages[status] || pages[404],
    isBase64Encoded: false,
  };
}

// ============================================================
// Route Handler
// ============================================================

function routeRequest(
  method: string,
  path: string,
  query: Record<string, string>,
  headers: Record<string, string>,
  body: string,
  isBase64Encoded: boolean,
): Promise<ProxyResult> | ProxyResult {
  // Periodic cleanup
  const now = Date.now();
  if (now - lastCleanup > 60000) {
    rateLimiter.cleanup();
    lastCleanup = now;
  }

  // Health check
  if (path === '/health' && method === 'GET') {
    return {
      statusCode: 200,
      headers: { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' },
      body: JSON.stringify({ status: 'ok', connections: concurrentConnections }),
      isBase64Encoded: false,
    };
  }

  // Front page
  if (path === '/' || path === '/index.html') {
    return {
      statusCode: 200,
      headers: {
        'Content-Type': 'text/html',
        'Cache-Control': 'public, max-age=3600',
        'Server': 'Tengine',
      },
      body: '<!DOCTYPE html><html><head><meta charset="UTF-8"><title>CDN Service</title><style>body{font-family:-apple-system,sans-serif;padding:40px;color:#333;}</style></head><body><h1>Cloud Service</h1><p>High-performance content delivery and edge computing services.</p></body></html>',
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

  // Relay paths
  if (path.startsWith('/ws') || path.startsWith('/connect') || path.startsWith('/api/')) {
    // Verify HMAC
    if (!verifyHMAC(method, path, headers, body, isBase64Encoded, CONFIG.hmacSecret)) {
      return createMimicResponse(403);
    }

    // Rate limit
    const clientId = (headers['authorization'] || '').slice(7, 20) || 'anon';
    if (!rateLimiter.isAllowed(clientId)) {
      return createMimicResponse(429);
    }

    // Concurrent limit
    if (concurrentConnections >= CONFIG.maxConcurrentConnections) {
      return createMimicResponse(503);
    }

    // Select backend and proxy
    const transport = query['transport'];
    const backend = selectBackend(CONFIG.relayBackends, transport);

    return proxyToBackend(method, path, query, headers, body, isBase64Encoded, backend);
  }

  return createMimicResponse(404);
}

// ============================================================
// Exports — Alibaba Cloud Function Compute Format
// ============================================================

/**
 * HTTP Trigger handler
 *
 * Used when Function Compute is configured with an HTTP trigger.
 * The event object contains the HTTP request details.
 */
exports.handler = async function (
  event: FCHttpEvent | FCApiGatewayEvent | Buffer,
  context: FCContext,
  callback?: Function,
): Promise<any> {
  try {
    // Handle different event formats

    // HTTP Trigger format (JSON event)
    if (event && typeof event === 'object' && 'method' in event) {
      const httpEvent = event as FCHttpEvent;
      const result = await routeRequest(
        httpEvent.method,
        httpEvent.path,
        httpEvent.query || {},
        httpEvent.headers || {},
        httpEvent.body || '',
        httpEvent.isBase64Encoded || false,
      );
      return result;
    }

    // API Gateway format
    if (event && typeof event === 'object' && 'httpMethod' in event) {
      const apiEvent = event as FCApiGatewayEvent;
      const result = await routeRequest(
        apiEvent.httpMethod,
        apiEvent.path,
        apiEvent.queryParameters || {},
        apiEvent.headers || {},
        apiEvent.body || '',
        apiEvent.isBase64Encoded || false,
      );
      return result;
    }

    // Raw buffer (HTTP trigger with raw body)
    if (Buffer.isBuffer(event)) {
      return createMimicResponse(400);
    }

    return createMimicResponse(400);
  } catch (error) {
    // Never expose internal errors
    return createMimicResponse(500);
  }
};

/**
 * Initializer function (called once when function instance starts)
 */
exports.initializer = async function (
  context: FCContext,
  callback: Function,
): Promise<void> {
  // Pre-warm connections, initialize caches, etc.
  callback(null, '');
};
