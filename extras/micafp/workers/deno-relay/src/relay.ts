/**
 * Deno Relay — TCP relay logic
 */

interface RelayConfig {
  secret: string;
  maxConnections: number;
  version: string;
}

/**
 * Handle a relay connection — forward data to target
 */
export async function handleRelayConnection(
  clientConn: Deno.Conn,
  initialData: Uint8Array,
  config: RelayConfig
): Promise<void> {
  // Try to connect to a default upstream proxy
  const upstreamHost = Deno.env.get('UPSTREAM_HOST') ?? '127.0.0.1';
  const upstreamPort = parseInt(Deno.env.get('UPSTREAM_PORT') ?? '1080');

  try {
    const upstreamConn = await Deno.connect({
      hostname: upstreamHost,
      port: upstreamPort,
    });

    // Forward initial data
    await upstreamConn.write(initialData);

    // Bidirectional relay
    await Promise.race([
      relayData(clientConn, upstreamConn),
      relayData(upstreamConn, clientConn),
    ]);

    upstreamConn.close();
  } catch (err) {
    console.error('[Relay] Upstream connection failed:', err);
    // Try alternative relay path
    await handleFallbackRelay(clientConn, initialData, config);
  }
}

/**
 * Bidirectional data relay between two connections
 */
async function relayData(source: Deno.Conn, dest: Deno.Conn): Promise<void> {
  const buf = new Uint8Array(32 * 1024);

  try {
    while (true) {
      const n = await source.read(buf);
      if (!n) break;

      await dest.write(buf.subarray(0, n));
    }
  } catch {
    // Connection closed or error
  }
}

/**
 * Fallback relay — use SOCKS5 or HTTP CONNECT
 */
async function handleFallbackRelay(
  clientConn: Deno.Conn,
  initialData: Uint8Array,
  _config: RelayConfig
): Promise<void> {
  // Parse SOCKS5 handshake
  if (initialData[0] === 0x05) {
    await handleSOCKS5(clientConn, initialData);
    return;
  }

  // Parse HTTP CONNECT
  const text = new TextDecoder().decode(initialData);
  if (text.startsWith('CONNECT ')) {
    await handleHTTPConnect(clientConn, text);
    return;
  }

  // Unknown protocol — close
  clientConn.close();
}

/**
 * Handle SOCKS5 proxy request
 */
async function handleSOCKS5(conn: Deno.Conn, initialData: Uint8Array): Promise<void> {
  // SOCKS5 greeting: version(1) nmethods(1) methods(n)
  if (initialData.length < 2 || initialData[0] !== 0x05) {
    conn.close();
    return;
  }

  // Respond with no-auth method (0x00)
  const greeting = new Uint8Array([0x05, 0x00]);
  await conn.write(greeting);

  // Read connect request
  const reqBuf = new Uint8Array(256);
  const n = await conn.read(reqBuf);
  if (!n || reqBuf[0] !== 0x05 || reqBuf[1] !== 0x01) {
    conn.close();
    return;
  }

  let targetHost: string;
  let targetPort: number;

  const addrType = reqBuf[3];
  if (addrType === 0x01) {
    // IPv4
    targetHost = `${reqBuf[4]}.${reqBuf[5]}.${reqBuf[6]}.${reqBuf[7]}`;
    targetPort = (reqBuf[8] << 8) | reqBuf[9];
  } else if (addrType === 0x03) {
    // Domain
    const domainLen = reqBuf[4];
    targetHost = new TextDecoder().decode(reqBuf.subarray(5, 5 + domainLen));
    targetPort = (reqBuf[5 + domainLen] << 8) | reqBuf[6 + domainLen];
  } else {
    // Unsupported
    const reply = new Uint8Array([0x05, 0x08, 0x00, 0x01, 0, 0, 0, 0, 0, 0]);
    await conn.write(reply);
    conn.close();
    return;
  }

  try {
    const targetConn = await Deno.connect({ hostname: targetHost, port: targetPort });

    // Success reply
    const reply = new Uint8Array([0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0]);
    await conn.write(reply);

    // Bidirectional relay
    await Promise.race([
      relayData(conn, targetConn),
      relayData(targetConn, conn),
    ]);

    targetConn.close();
  } catch {
    // Connection refused
    const reply = new Uint8Array([0x05, 0x05, 0x00, 0x01, 0, 0, 0, 0, 0, 0]);
    await conn.write(reply);
    conn.close();
  }
}

/**
 * Handle HTTP CONNECT request
 */
async function handleHTTPConnect(conn: Deno.Conn, initialData: string): Promise<void> {
  const match = initialData.match(/^CONNECT\s+([^\s:]+):(\d+)/);
  if (!match) {
    const response = 'HTTP/1.1 400 Bad Request\r\n\r\n';
    await conn.write(new TextEncoder().encode(response));
    conn.close();
    return;
  }

  const [, host, portStr] = match;
  const port = parseInt(portStr, 10);

  try {
    const targetConn = await Deno.connect({ hostname: host, port });

    const response = 'HTTP/1.1 200 Connection Established\r\n\r\n';
    await conn.write(new TextEncoder().encode(response));

    await Promise.race([
      relayData(conn, targetConn),
      relayData(targetConn, conn),
    ]);

    targetConn.close();
  } catch {
    const response = 'HTTP/1.1 502 Bad Gateway\r\n\r\n';
    await conn.write(new TextEncoder().encode(response));
    conn.close();
  }
}

/**
 * Close a relay connection gracefully
 */
export function closeRelay(conn: Deno.Conn): void {
  try {
    conn.close();
  } catch {
    // Already closed
  }
}
