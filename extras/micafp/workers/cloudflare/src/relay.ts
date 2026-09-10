/**
 * Relay Logic — Forward traffic to target host
 */

export interface RelayRequest {
  targetHost: string;
  targetPort: number;
  data: Uint8Array;
}

export interface RelayResponse {
  data: Uint8Array;
  status: number;
  latency: number;
}

/**
 * Perform X25519 ECDH key exchange
 */
export async function performECDH(
  clientPublicKeyBase64: string
): Promise<{ serverPublicKey: string; sharedSecret: string }> {
  // Generate server key pair
  const serverKeyPair = await crypto.subtle.generateKey(
    { name: 'ECDH', namedCurve: 'P-256' },
    true,
    ['deriveBits']
  );

  // Import client public key
  const clientPublicKeyBuffer = base64ToUint8Array(clientPublicKeyBase64);
  const clientPublicKey = await crypto.subtle.importKey(
    'raw',
    clientPublicKeyBuffer,
    { name: 'ECDH', namedCurve: 'P-256' },
    false,
    []
  );

  // Derive shared secret
  const sharedBits = await crypto.subtle.deriveBits(
    { name: 'ECDH', public: clientPublicKey },
    serverKeyPair.privateKey,
    256
  );

  // Export server public key
  const serverPublicKeyBuffer = await crypto.subtle.exportKey(
    'raw',
    serverKeyPair.publicKey
  );

  return {
    serverPublicKey: uint8ArrayToBase64(new Uint8Array(serverPublicKeyBuffer)),
    sharedSecret: uint8ArrayToBase64(new Uint8Array(sharedBits)),
  };
}

/**
 * Validate handshake parameters
 */
export function validateHandshake(params: {
  timestamp: number;
  nonce: string;
  maxSkewSeconds?: number;
}): boolean {
  const maxSkew = params.maxSkewSeconds ?? 300;
  const now = Date.now() / 1000;
  return Math.abs(now - params.timestamp) <= maxSkew;
}

/**
 * Handle relay request — connect to target and forward data
 */
export async function handleRelay(request: RelayRequest): Promise<Uint8Array> {
  const startTime = Date.now();

  try {
    // For HTTP targets, use fetch
    if (request.targetPort === 80 || request.targetPort === 443) {
      const protocol = request.targetPort === 443 ? 'https' : 'http';
      const url = `${protocol}://${request.targetHost}`;

      const response = await fetch(url, {
        method: 'POST',
        body: request.data,
        headers: {
          'Content-Type': 'application/octet-stream',
          'X-Relayed-By': 'UnifiedShield/2.0',
        },
      });

      const responseData = new Uint8Array(await response.arrayBuffer());
      return responseData;
    }

    // For non-HTTP targets, return error (Cloudflare Workers can't do raw TCP)
    return new TextEncoder().encode(
      JSON.stringify({
        error: 'Raw TCP relay not supported on Cloudflare Workers',
        suggestion: 'Use WebSocket relay or Deno relay for TCP connections',
      })
    );
  } catch (err) {
    return new TextEncoder().encode(
      JSON.stringify({
        error: 'Relay connection failed',
        details: String(err),
      })
    );
  }
}

/* ────────── Utility ────────── */

function base64ToUint8Array(base64: string): Uint8Array {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

function uint8ArrayToBase64(bytes: Uint8Array): string {
  let binary = '';
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}
