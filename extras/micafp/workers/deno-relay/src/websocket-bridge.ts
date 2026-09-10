/**
 * WebSocket Bridge — Handles WebSocket relay connections
 *
 * Provides full-duplex relay between WebSocket clients and TCP targets.
 */

/**
 * Handle a WebSocket connection after upgrade.
 * Implements a simple framing protocol:
 *
 * Frame types:
 *   0x01 - CONNECT  { host: string, port: number }
 *   0x02 - DATA     { data: base64 }
 *   0x03 - CLOSE
 *   0x04 - PING
 *   0x05 - PONG
 *   0x10 - HANDSHAKE  { clientPublicKey: base64 }
 *   0x11 - HANDSHAKE_RESPONSE { serverPublicKey: base64, sessionId: string }
 */
export async function handleWebSocketBridge(conn: Deno.Conn): Promise<void> {
  let targetConn: Deno.Conn | null = null;
  let sessionId: string | null = null;
  let sharedSecret: string | null = null;

  const buf = new Uint8Array(65536);

  try {
    while (true) {
      // Read WebSocket frame
      const n = await conn.read(buf);
      if (!n) break;

      const frame = parseWebSocketFrame(buf.subarray(0, n));
      if (!frame) continue;

      switch (frame.opcode) {
        case 0x01: { // Text frame
          const message = JSON.parse(new TextDecoder().decode(frame.payload));

          switch (message.type) {
            case 'connect': {
              try {
                targetConn = await Deno.connect({
                  hostname: message.host,
                  port: message.port,
                });

                // Start reading from target
                readFromTarget(targetConn, conn);

                sendWebSocketText(conn, {
                  type: 'connected',
                  host: message.host,
                  port: message.port,
                });
              } catch (err) {
                sendWebSocketText(conn, {
                  type: 'error',
                  error: `Connection failed: ${err}`,
                });
              }
              break;
            }

            case 'data': {
              if (!targetConn) {
                sendWebSocketText(conn, { type: 'error', error: 'Not connected' });
                break;
              }

              const data = Uint8Array.from(atob(message.data), (c) => c.charCodeAt(0));
              await targetConn.write(data);
              break;
            }

            case 'close': {
              if (targetConn) {
                targetConn.close();
                targetConn = null;
              }
              break;
            }

            case 'handshake': {
              const keyPair = await crypto.subtle.generateKey(
                { name: 'ECDH', namedCurve: 'P-256' },
                true,
                ['deriveBits']
              );

              const serverPublicKey = await crypto.subtle.exportKey('raw', keyPair.publicKey);
              sessionId = crypto.randomUUID();

              sendWebSocketText(conn, {
                type: 'handshake-response',
                sessionId,
                serverPublicKey: btoa(String.fromCharCode(...new Uint8Array(serverPublicKey))),
              });
              break;
            }

            case 'ping': {
              sendWebSocketText(conn, { type: 'pong', timestamp: Date.now() });
              break;
            }
          }
          break;
        }

        case 0x02: { // Binary frame
          if (targetConn) {
            await targetConn.write(frame.payload);
          }
          break;
        }

        case 0x08: { // Close
          if (targetConn) targetConn.close();
          return;
        }

        case 0x09: { // Ping
          sendWebSocketPong(conn, frame.payload);
          break;
        }
      }
    }
  } catch (err) {
    console.error('[WS Bridge] Error:', err);
  } finally {
    if (targetConn) {
      try { targetConn.close(); } catch { /* ignore */ }
    }
  }
}

/**
 * Read from target connection and forward to WebSocket client
 */
async function readFromTarget(target: Deno.Conn, wsConn: Deno.Conn): Promise<void> {
  const buf = new Uint8Array(32 * 1024);

  try {
    while (true) {
      const n = await target.read(buf);
      if (!n) break;

      const data = buf.subarray(0, n);
      const base64 = btoa(String.fromCharCode(...data));

      sendWebSocketText(wsConn, {
        type: 'data',
        data: base64,
        length: n,
      });
    }
  } catch {
    // Connection closed
  }

  sendWebSocketText(wsConn, { type: 'disconnected' });
}

/* ────────── WebSocket Frame Handling ────────── */

interface WSFrame {
  fin: boolean;
  opcode: number;
  payload: Uint8Array;
}

function parseWebSocketFrame(data: Uint8Array): WSFrame | null {
  if (data.length < 2) return null;

  const byte0 = data[0];
  const byte1 = data[1];

  const fin = (byte0 & 0x80) !== 0;
  const opcode = byte0 & 0x0F;
  const masked = (byte1 & 0x80) !== 0;
  let payloadLength = byte1 & 0x7F;

  let offset = 2;

  if (payloadLength === 126) {
    payloadLength = (data[offset] << 8) | data[offset + 1];
    offset += 2;
  } else if (payloadLength === 127) {
    payloadLength = 0;
    for (let i = 0; i < 8; i++) {
      payloadLength = (payloadLength << 8) | data[offset + i];
    }
    offset += 8;
  }

  let mask: Uint8Array | null = null;
  if (masked) {
    mask = data.subarray(offset, offset + 4);
    offset += 4;
  }

  const payload = data.subarray(offset, offset + payloadLength);

  // Unmask
  if (mask) {
    const unmasked = new Uint8Array(payload.length);
    for (let i = 0; i < payload.length; i++) {
      unmasked[i] = payload[i] ^ mask[i % 4];
    }
    return { fin, opcode, payload: unmasked };
  }

  return { fin, opcode, payload };
}

function sendWebSocketText(conn: Deno.Conn, data: unknown): void {
  const text = JSON.stringify(data);
  const payload = new TextEncoder().encode(text);
  const frame = createWebSocketFrame(0x01, payload);
  try {
    conn.write(frame);
  } catch {
    // Connection may be closed
  }
}

function sendWebSocketPong(conn: Deno.Conn, payload: Uint8Array): void {
  const frame = createWebSocketFrame(0x0A, payload);
  try {
    conn.write(frame);
  } catch {
    // Connection may be closed
  }
}

function createWebSocketFrame(opcode: number, payload: Uint8Array): Uint8Array {
  const header: number[] = [];

  header.push(0x80 | opcode); // FIN + opcode

  if (payload.length < 126) {
    header.push(payload.length);
  } else if (payload.length < 65536) {
    header.push(126);
    header.push((payload.length >> 8) & 0xFF);
    header.push(payload.length & 0xFF);
  } else {
    header.push(127);
    for (let i = 7; i >= 0; i--) {
      header.push((payload.length >> (i * 8)) & 0xFF);
    }
  }

  const frame = new Uint8Array(header.length + payload.length);
  frame.set(header, 0);
  frame.set(payload, header.length);

  return frame;
}
