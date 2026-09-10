/**
 * Obfuscation Helpers — Frame obfuscation and padding
 */

/**
 * Obfuscate a data frame with XOR-based transformation using the shared secret
 */
export function obfuscateFrame(data: Uint8Array, sharedSecretBase64: string): string {
  const secret = base64ToUint8Array(sharedSecretBase64);
  const result = new Uint8Array(data.length);

  for (let i = 0; i < data.length; i++) {
    result[i] = data[i] ^ secret[i % secret.length];
  }

  // Prepend a random IV (12 bytes)
  const iv = new Uint8Array(12);
  crypto.getRandomValues(iv);

  const combined = new Uint8Array(iv.length + result.length);
  combined.set(iv, 0);
  combined.set(result, iv.length);

  return uint8ArrayToBase64(combined);
}

/**
 * Deobfuscate a data frame
 */
export function deobfuscateFrame(dataBase64: string, sharedSecretBase64: string): Uint8Array {
  const combined = base64ToUint8Array(dataBase64);
  const secret = base64ToUint8Array(sharedSecretBase64);

  // Skip the 12-byte IV
  const data = combined.slice(12);
  const result = new Uint8Array(data.length);

  for (let i = 0; i < data.length; i++) {
    result[i] = data[i] ^ secret[i % secret.length];
  }

  return result;
}

/**
 * Add a PADDING frame to data to prevent length-based traffic analysis.
 * Follows the TLS PADDING extension pattern.
 *
 * Frame format:
 *   [1 byte: type=0x00] [2 bytes: padding length] [N bytes: zero padding]
 */
export function addPaddingFrame(data: Uint8Array, maxPaddingBytes: number): Uint8Array {
  if (maxPaddingBytes <= 0) return data;

  // Random padding length between 1 and maxPaddingBytes
  const paddingLength = 1 + Math.floor(Math.random() * Math.min(maxPaddingBytes, 1024));

  const result = new Uint8Array(data.length + 3 + paddingLength);
  result.set(data, 0);

  // Padding frame header
  result[data.length] = 0x00; // Type: padding
  result[data.length + 1] = (paddingLength >> 8) & 0xFF;
  result[data.length + 2] = paddingLength & 0xFF;

  // Zero padding bytes
  for (let i = 0; i < paddingLength; i++) {
    result[data.length + 3 + i] = 0x00;
  }

  return result;
}

/**
 * Strip PADDING frame from data
 */
export function stripPaddingFrame(data: Uint8Array): Uint8Array {
  // Find padding frame marker (0x00 type byte near end)
  for (let i = data.length - 1; i >= 3; i--) {
    if (data[i - 3] === 0x00) {
      const paddingLength = (data[i - 2] << 8) | data[i - 1];
      if (i - 3 + 3 + paddingLength === data.length) {
        return data.slice(0, i - 3);
      }
    }
  }

  // No padding frame found
  return data;
}

/**
 * Generate a random-looking HTTP header to make relay traffic
 * appear as normal HTTPS traffic
 */
export function generateCoverHeaders(): Record<string, string> {
  const userAgents = [
    'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36',
    'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36',
    'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36',
    'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:126.0) Gecko/20100101 Firefox/126.0',
    'Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:126.0) Gecko/20100101 Firefox/126.0',
  ];

  return {
    'User-Agent': userAgents[Math.floor(Math.random() * userAgents.length)],
    'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8',
    'Accept-Language': 'en-US,en;q=0.5',
    'Accept-Encoding': 'gzip, deflate, br',
    'Connection': 'keep-alive',
    'Cache-Control': 'no-cache',
  };
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
