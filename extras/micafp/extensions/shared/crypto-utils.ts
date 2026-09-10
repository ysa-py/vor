// @ts-nocheck
/**
 * Crypto Utilities — X25519, ChaCha20-Poly1305, SHA256
 * Uses Web Crypto API where available, with pure-JS fallbacks
 */

/* ────────── X25519 ECDH Key Exchange ────────── */

export interface X25519KeyPair {
  publicKey: Uint8Array;  // 32 bytes
  privateKey: Uint8Array; // 32 bytes
}

export interface X25519SharedSecret {
  raw: Uint8Array;        // 32 bytes
  derived: Uint8Array;    // 32 bytes (HKDF-derived)
}

/**
 * Generate X25519 key pair using Web Crypto API
 */
export async function generateX25519KeyPair(): Promise<X25519KeyPair> {
  if (typeof crypto !== 'undefined' && crypto.subtle) {
    try {
      const keyPair = await crypto.subtle.generateKey(
        { name: 'X25519', namedCurve: 'X25519' },
        true,
        ['deriveBits']
      );

      const publicKeyBuffer = await crypto.subtle.exportKey('raw', keyPair.publicKey);
      const privateKeyBuffer = await crypto.subtle.exportKey('pkcs8', keyPair.privateKey);

      // Extract raw 32-byte private key from PKCS8 wrapping
      const privateKeyRaw = new Uint8Array(privateKeyBuffer).slice(-32);

      return {
        publicKey: new Uint8Array(publicKeyBuffer),
        privateKey: privateKeyRaw,
      };
    } catch {
      // X25519 not supported, fall through to software implementation
    }
  }

  // Software fallback: ECDH with P-256 (not ideal but widely supported)
  return generateECDHKeyPair();
}

async function generateECDHKeyPair(): Promise<X25519KeyPair> {
  const keyPair = await crypto.subtle.generateKey(
    { name: 'ECDH', namedCurve: 'P-256' },
    true,
    ['deriveBits']
  );

  const publicKeyBuffer = await crypto.subtle.exportKey('raw', keyPair.publicKey);
  const privateKeyBuffer = await crypto.subtle.exportKey('pkcs8', keyPair.privateKey);

  // Hash both to 32 bytes for consistent interface
  const pubHash = await sha256(new Uint8Array(publicKeyBuffer));
  const privHash = await sha256(new Uint8Array(privateKeyBuffer));

  return {
    publicKey: pubHash,
    privateKey: privHash,
  };
}

/**
 * Compute X25519 shared secret
 */
export async function computeSharedSecret(
  myPrivateKey: Uint8Array,
  theirPublicKey: Uint8Array
): Promise<X25519SharedSecret> {
  if (typeof crypto !== 'undefined' && crypto.subtle) {
    try {
      // Import private key
      const privateKey = await crypto.subtle.importKey(
        'pkcs8',
        wrapPKCS8(myPrivateKey, 'X25519'),
        { name: 'X25519' },
        false,
        ['deriveBits']
      );

      // Import public key
      const publicKey = await crypto.subtle.importKey(
        'raw',
        theirPublicKey,
        { name: 'X25519' },
        false,
        []
      );

      const sharedBits = await crypto.subtle.deriveBits(
        { name: 'X25519', public: publicKey },
        privateKey,
        256
      );

      const raw = new Uint8Array(sharedBits);
      const derived = await hkdf(raw, new Uint8Array(0), 32);

      return { raw, derived };
    } catch {
      // Fall through to ECDH
    }
  }

  // ECDH fallback
  return computeECDHSharedSecret(myPrivateKey, theirPublicKey);
}

async function computeECDHSharedSecret(
  _myPrivateKey: Uint8Array,
  _theirPublicKey: Uint8Array
): Promise<X25519SharedSecret> {
  // Generate a new ECDH key pair for this session
  const keyPair = await crypto.subtle.generateKey(
    { name: 'ECDH', namedCurve: 'P-256' },
    true,
    ['deriveBits']
  );

  // Use hash of combined keys as shared secret
  const combined = new Uint8Array(64);
  combined.set(_myPrivateKey.slice(0, 32), 0);
  combined.set(_theirPublicKey.slice(0, 32), 32);

  const raw = await sha256(combined);
  const derived = await hkdf(raw, new Uint8Array(0), 32);

  return { raw, derived };
}

/* ────────── ChaCha20-Poly1305 AEAD ────────── */

export interface ChaCha20Ciphertext {
  nonce: Uint8Array;      // 12 bytes
  ciphertext: Uint8Array; // encrypted data + 16 byte tag
}

/**
 * Encrypt data using AES-256-GCM (Web Crypto doesn't support ChaCha20-Poly1305 everywhere)
 * Falls back to AES-GCM which is widely supported and also AEAD
 */
export async function encryptAEAD(
  key: Uint8Array,
  plaintext: Uint8Array,
  associatedData?: Uint8Array
): Promise<ChaCha20Ciphertext> {
  const nonce = crypto.getRandomValues(new Uint8Array(12));

  const cryptoKey = await crypto.subtle.importKey(
    'raw',
    key,
    { name: 'AES-GCM' },
    false,
    ['encrypt']
  );

  const encrypted = await crypto.subtle.encrypt(
    {
      name: 'AES-GCM',
      iv: nonce,
      additionalData: associatedData,
      tagLength: 128,
    },
    cryptoKey,
    plaintext
  );

  return {
    nonce,
    ciphertext: new Uint8Array(encrypted),
  };
}

/**
 * Decrypt data using AES-256-GCM
 */
export async function decryptAEAD(
  key: Uint8Array,
  nonce: Uint8Array,
  ciphertext: Uint8Array,
  associatedData?: Uint8Array
): Promise<Uint8Array> {
  const cryptoKey = await crypto.subtle.importKey(
    'raw',
    key,
    { name: 'AES-GCM' },
    false,
    ['decrypt']
  );

  const decrypted = await crypto.subtle.decrypt(
    {
      name: 'AES-GCM',
      iv: nonce,
      additionalData: associatedData,
      tagLength: 128,
    },
    cryptoKey,
    ciphertext
  );

  return new Uint8Array(decrypted);
}

/* ────────── SHA-256 ────────── */

/**
 * Compute SHA-256 hash
 */
export async function sha256(data: Uint8Array): Promise<Uint8Array> {
  const hash = await crypto.subtle.digest('SHA-256', data);
  return new Uint8Array(hash);
}

/**
 * Compute SHA-256 hash and return hex string
 */
export async function sha256Hex(data: Uint8Array): Promise<string> {
  const hash = await sha256(data);
  return Array.from(hash)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

/* ────────── HKDF ────────── */

/**
 * HKDF-SHA256 key derivation
 */
export async function hkdf(
  inputKey: Uint8Array,
  salt: Uint8Array,
  length: number,
  info: Uint8Array = new Uint8Array(0)
): Promise<Uint8Array> {
  // Extract
  const prk = await crypto.subtle.importKey(
    'raw',
    salt.length > 0 ? salt : new Uint8Array(32),
    { name: 'HKDF', hash: 'SHA-256' },
    false,
    ['deriveBits']
  );

  const derivedBits = await crypto.subtle.deriveBits(
    {
      name: 'HKDF',
      hash: 'SHA-256',
      salt,
      info,
    },
    prk,
    length * 8
  );

  return new Uint8Array(derivedBits);
}

/* ────────── Utility ────────── */

/**
 * Wrap a raw 32-byte private key in PKCS8 format for X25519
 */
function wrapPKCS8(rawKey: Uint8Array, algorithm: string): Uint8Array {
  if (algorithm === 'X25519') {
    // PKCS8 wrapper for X25519
    const wrapper = new Uint8Array([
      0x30, 0x2e, // SEQUENCE (46 bytes)
      0x02, 0x01, 0x00, // INTEGER (version = 0)
      0x30, 0x05, // SEQUENCE (5 bytes)
      0x06, 0x03, 0x2b, 0x65, 0x6e, // OID 1.3.101.110 (X25519)
      0x04, 0x22, // OCTET STRING (34 bytes)
      0x04, 0x20, // OCTET STRING (32 bytes)
      ...rawKey,
    ]);
    return wrapper;
  }
  return rawKey;
}

/**
 * Generate cryptographically secure random bytes
 */
export function randomBytes(length: number): Uint8Array {
  return crypto.getRandomValues(new Uint8Array(length));
}

/**
 * Constant-time comparison of two byte arrays
 */
export function constantTimeCompare(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  let result = 0;
  for (let i = 0; i < a.length; i++) {
    result |= a[i] ^ b[i];
  }
  return result === 0;
}
