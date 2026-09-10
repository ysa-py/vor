/**
 * MICAFP-UnifiedShield Huawei Cloud CDN Worker
 * Huawei Cloud FunctionGraph compatible handler
 * 
 * Features:
 * - Huawei FunctionGraph API format
 * - Huawei-specific event and context handling
 * - Same proxy functionality
 * - Huawei Cloud edge nodes (global + China)
 */

// ── Huawei FunctionGraph Configuration ─────────────────────────────────────

const HUAWEI_CONFIG = {
  upstreamHost: (globalThis as any).FG_UPSTREAM_HOST || "shield-huawei.internal",
  upstreamPort: parseInt((globalThis as any).FG_UPSTREAM_PORT || "443", 10),
  upstreamScheme: (globalThis as any).FG_UPSTREAM_SCHEME || "https",
  hmacSecret: (globalThis as any).HMAC_SECRET || "huawei-secret-change-me",
  tokenRotationSeconds: 300,
  maxRequestBodyBytes: 10 * 1024 * 1024,
  requestTimeoutMs: 30000,
  defaultRegion: (globalThis as any).FG_REGION || "ap-southeast-1",
  // Huawei Cloud FunctionGraph routing
  functionNodes: {
    "cn-north-4": "huawei-bj-node.internal",
    "cn-east-3": "huawei-sh-node.internal",
    "cn-south-1": "huawei-gz-node.internal",
    "ap-southeast-1": "huawei-sg-node.internal",
    "ap-southeast-2": "huawei-syd-node.internal",
    "af-south-1": "huawei-jhb-node.internal",
  } as Record<string, string>,
};

// ── Metrics ────────────────────────────────────────────────────────────────

const metrics = {
  requestsTotal: 0,
  requestsProxied: 0,
  requestsRejected: 0,
  wsUpgrades: 0,
  errorsTotal: 0,
  bytesUpstream: 0,
  bytesDownstream: 0,
  startTime: Date.now(),
  fgInvocations: 0,
  functionErrors: 0,
};

// ── HMAC ───────────────────────────────────────────────────────────────────

async function hmacSign(key: string, message: string): Promise<string> {
  const encoder = new TextEncoder();
  const cryptoKey = await crypto.subtle.importKey(
    "raw", encoder.encode(key),
    { name: "HMAC", hash: "SHA-256" }, false, ["sign"]
  );
  const sig = await crypto.subtle.sign("HMAC", cryptoKey, encoder.encode(message));
  return Array.from(new Uint8Array(sig)).map((b) => b.toString(16).padStart(2, "0")).join("");
}

async function validateToken(authHeader: string | null): Promise<boolean> {
  if (!authHeader || !authHeader.startsWith("Bearer ")) return false;
  const token = authHeader.slice(7);
  const parts = token.split(":");
  if (parts.length !== 2) return false;
  const [bucketStr, providedHmac] = parts;
  const bucket = parseInt(bucketStr, 10);
  if (isNaN(bucket)) return false;
  const nowBucket = Math.floor(Date.now() / 1000 / HUAWEI_CONFIG.tokenRotationSeconds);
  if (bucket !== nowBucket && bucket !== nowBucket - 1) return false;
  const expected = await hmacSign(HUAWEI_CONFIG.hmacSecret, bucketStr);
  return expected === providedHmac;
}

// ── CORS ───────────────────────────────────────────────────────────────────

function corsHeaders(request: Request): Record<string, string> {
  const origin = request.headers.get("Origin") || "";
  const allowed = ["chrome-extension://", "moz-extension://", "https://unifiedshield.local"];
  const isAllowed = allowed.some((o) => origin.startsWith(o));
  return {
    "Access-Control-Allow-Origin": isAllowed ? origin : "null",
    "Access-Control-Allow-Methods": "GET, POST, PUT, DELETE, OPTIONS, PATCH",
    "Access-Control-Allow-Headers": "Authorization, Content-Type, X-Shield-Token",
    "Access-Control-Allow-Credentials": "true",
    "Access-Control-Max-Age": "86400",
  };
}

// ── Error Response ─────────────────────────────────────────────────────────

function errorResponse(
  status: number, code: string, message: string,
  request: Request, cors: Record<string, string>
): Response {
  metrics.errorsTotal++;
  metrics.functionErrors++;
  return new Response(JSON.stringify({
    error: { code, message, requestId: request.headers.get("X-Request-ID") || crypto.randomUUID() },
    upstream: "shield-huawei",
  }), { status, headers: { "Content-Type": "application/json", ...cors } });
}

// ── Node Resolution ────────────────────────────────────────────────────────

function resolveFunctionNode(region?: string): string {
  const target = region || HUAWEI_CONFIG.defaultRegion;
  return HUAWEI_CONFIG.functionNodes[target] || HUAWEI_CONFIG.functionNodes["ap-southeast-1"];
}

// ── Huawei FunctionGraph HTTP Handler ──────────────────────────────────────

/**
 * Huawei Cloud FunctionGraph supports two invocation modes:
 * 1. HTTP trigger: receives standard Request/Response
 * 2. Event trigger: receives APIG event format
 * 
 * For HTTP trigger, the handler receives (context, req) where:
 * - context: FunctionGraph context with request ID, region, etc.
 * - req: standard Request object
 */
async function huaweiHandler(request: Request, context?: HuaweiFunctionContext): Promise<Response> {
  const cors = corsHeaders(request);
  metrics.requestsTotal++;
  metrics.fgInvocations++;

  const url = new URL(request.url);
  const fgRegion = context?.region || request.headers.get("x-fg-region") || HUAWEI_CONFIG.defaultRegion;
  const fgRequestId = context?.requestId || request.headers.get("x-fg-request-id") || crypto.randomUUID();

  // Health check
  if (url.pathname === "/health") {
    return new Response(JSON.stringify({
      status: "healthy",
      region: fgRegion,
      requestId: fgRequestId,
      uptime: Date.now() - metrics.startTime,
      version: "6.0.0-huawei",
    }), { headers: { "Content-Type": "application/json", ...cors } });
  }

  // Metrics
  if (url.pathname === "/metrics") {
    const mToken = url.searchParams.get("token");
    if (mToken !== "internal-metrics") {
      return errorResponse(403, "FORBIDDEN", "Invalid metrics token", request, cors);
    }
    return new Response(JSON.stringify({ ...metrics, uptimeSeconds: (Date.now() - metrics.startTime) / 1000 }), {
      headers: { "Content-Type": "application/json", ...cors },
    });
  }

  // CORS preflight
  if (request.method === "OPTIONS") {
    return new Response(null, { status: 204, headers: cors });
  }

  // Auth
  if (!(await validateToken(request.headers.get("Authorization")))) {
    metrics.requestsRejected++;
    return errorResponse(401, "UNAUTHORIZED", "Invalid auth token", request, cors);
  }

  // WebSocket upgrade
  if (request.headers.get("Upgrade") === "websocket") {
    metrics.wsUpgrades++;
    return handleHuaweiWebSocket(request, cors);
  }

  // Proxy
  return handleHuaweiProxy(request, url, cors, fgRegion);
}

// ── WebSocket ──────────────────────────────────────────────────────────────

function handleHuaweiWebSocket(request: Request, cors: Record<string, string>): Response {
  try {
    const pair = (globalThis as any).WebSocketPair
      ? new (globalThis as any).WebSocketPair()
      : null;
    if (pair) {
      const [client, server] = Object.values(pair) as [any, any];
      server.accept();
      server.addEventListener("message", (e: MessageEvent) => {
        metrics.bytesDownstream += (e.data as ArrayBuffer)?.byteLength || 0;
      });
      return new Response(null, { status: 101, webSocket: client });
    }
    return errorResponse(501, "NOT_SUPPORTED", "WebSocket not available in this FunctionGraph region", request, cors);
  } catch {
    metrics.errorsTotal++;
    return errorResponse(500, "WS_ERROR", "WebSocket upgrade failed", request, cors);
  }
}

// ── HTTP Proxy ─────────────────────────────────────────────────────────────

async function handleHuaweiProxy(
  request: Request, url: URL, cors: Record<string, string>, fgRegion: string
): Promise<Response> {
  const node = resolveFunctionNode(fgRegion);
  const targetPath = url.pathname + url.search;
  const upstreamUrl = `${HUAWEI_CONFIG.upstreamScheme}://${node}:${HUAWEI_CONFIG.upstreamPort}${targetPath}`;

  const headers = new Headers();
  const hopByHop = ["connection", "keep-alive", "transfer-encoding", "te", "trailer", "upgrade", "host"];
  for (const [key, value] of request.headers.entries()) {
    if (!hopByHop.includes(key.toLowerCase())) headers.set(key, value);
  }
  headers.set("Host", HUAWEI_CONFIG.upstreamHost);
  headers.set("X-Forwarded-For", request.headers.get("X-Forwarded-For") || "127.0.0.1");
  headers.set("X-Shield-Proxy", "v6.0.0-huawei");
  headers.set("X-FG-Region", fgRegion);

  let body: ReadableStream<Uint8Array> | null = null;
  if (request.method !== "GET" && request.method !== "HEAD") {
    const cl = parseInt(request.headers.get("Content-Length") || "0", 10);
    if (cl > HUAWEI_CONFIG.maxRequestBodyBytes) {
      return errorResponse(413, "PAYLOAD_TOO_LARGE", "Body exceeds limit", request, cors);
    }
    if (request.body) body = request.body;
  }

  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), HUAWEI_CONFIG.requestTimeoutMs);

  let upstreamResponse: Response;
  try {
    upstreamResponse = await fetch(upstreamUrl, {
      method: request.method, headers, body,
      signal: controller.signal, redirect: "manual",
    });
  } catch {
    metrics.errorsTotal++;
    return errorResponse(502, "UPSTREAM_ERROR", "Upstream unreachable", request, cors);
  } finally {
    clearTimeout(timeoutId);
  }

  metrics.requestsProxied++;
  metrics.bytesUpstream += parseInt(upstreamResponse.headers.get("Content-Length") || "0", 10);

  const responseHeaders = new Headers();
  for (const [key, value] of upstreamResponse.headers.entries()) {
    if (!hopByHop.includes(key.toLowerCase())) responseHeaders.set(key, value);
  }
  responseHeaders.delete("x-powered-by");
  responseHeaders.delete("server");
  responseHeaders.set("X-Shield-Version", "6.0.0-huawei");
  responseHeaders.set("Server", "HuaweiFG");
  for (const [k, v] of Object.entries(cors)) responseHeaders.set(k, v);

  return new Response(upstreamResponse.body, {
    status: upstreamResponse.status,
    statusText: upstreamResponse.statusText,
    headers: responseHeaders,
  });
}

// ── Huawei FunctionGraph Context Interface ─────────────────────────────────

interface HuaweiFunctionContext {
  requestId: string;
  region: string;
  projectId: string;
  userId: string;
  functionName: string;
  functionVersion: string;
  memorySize: number;
  getCodeDestination(): string;
  getToken(): string;
  getLogger(): any;
}

// ── Huawei FunctionGraph Event Handler (APIG format) ───────────────────────

interface HuaweiAPIGEvent {
  body: string;
  headers: Record<string, string>;
  httpMethod: string;
  isBase64Encoded: boolean;
  path: string;
  pathParameters: Record<string, string>;
  queryString: Record<string, string>;
  requestContext: {
    requestId: string;
    stage: string;
    apiId: string;
    region: string;
  };
}

async function huaweiEventHandler(event: HuaweiAPIGEvent, context: HuaweiFunctionContext): Promise<any> {
  const method = event.httpMethod || "GET";
  const path = event.path || "/";
  const url = `https://${event.headers.host || "localhost"}${path}`;

  const body = event.isBase64Encoded
    ? atob(event.body)
    : event.body;

  const req = new Request(url, {
    method,
    headers: new Headers(event.headers),
    body: method !== "GET" && method !== "HEAD" ? body : undefined,
  });

  const response = await huaweiHandler(req, context);
  const responseBody = await response.text();

  return {
    statusCode: response.status,
    headers: Object.fromEntries(response.headers.entries()),
    body: responseBody,
    isBase64Encoded: false,
  };
}

// ── Huawei FunctionGraph Exports ───────────────────────────────────────────

export const handler = huaweiHandler;
export default huaweiHandler;
export { huaweiEventHandler };
