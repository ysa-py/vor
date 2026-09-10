/**
 * UnifiedShield Tencent Cloud WebSocket Bridge
 *
 * Manages WebSocket connections through Tencent Cloud API Gateway.
 * The API Gateway sends connect/disconnect/send events as SCF invocations.
 * This bridge maintains connection state across invocations.
 */

import { TencentRelayHandler, FrameType, ConnectionState, Connection } from './relay';

const HKDF_INFO = 'unifiedshield-session-v1';
const NONCE_LENGTH = 12;
const TAG_LENGTH = 16;

interface WSConnection extends Connection {
  connectionId: string;
  sessionToken: string;
}

export class WebSocketBridge {
  private relayHandler: TencentRelayHandler;
  private wsConnections: Map<string, WSConnection> = new Map();

  // Tencent Cloud API Gateway callback URL for sending messages back
  private callbackHost: string;
  private apiGatewayUrl: string;

  constructor(relayHandler: TencentRelayHandler) {
    this.relayHandler = relayHandler;
    this.callbackHost = process.env.TENCENT_APIGATEWAY_HOST || '';
    this.apiGatewayUrl = process.env.TENCENT_APIGATEWAY_URL || '';
  }

  /**
   * Handle a WebSocket connect event from Tencent API Gateway.
   */
  async handleConnect(
    connectionId: string,
    clientIP: string,
    sessionToken: string,
    headers: Record<string, string>
  ): Promise<void> {
    const targetHost = headers['x-target-host'] || '';
    const targetPort = parseInt(headers['x-target-port'] || '443', 10);

    const conn = this.relayHandler.createConnection(clientIP, targetHost, targetPort);

    const wsConn: WSConnection = {
      ...conn,
      connectionId,
      sessionToken,
    };

    this.wsConnections.set(connectionId, wsConn);

    try {
      const clientPubKey = this.hexToBytes(sessionToken);
      const privateKeyHex = process.env.WORKER_PRIVATE_KEY!;
      const serverPrivateKey = await crypto.subtle.importKey(
        'raw',
        this.hexToBytes(privateKeyHex),
        { name: 'X25519' },
        false,
        ['deriveBits']
      );
      const clientPubKeyObj = await crypto.subtle.importKey(
        'raw',
        clientPubKey,
        { name: 'X25519' },
        true,
        []
      );

      const sharedBits = await crypto.subtle.deriveBits(
        { name: 'X25519', public: clientPubKeyObj },
        serverPrivateKey,
        256
      );
      const salt = new Uint8Array(32);
      const infoBytes = new TextEncoder().encode(HKDF_INFO);
      const hkdfKey = await crypto.subtle.importKey('raw', sharedBits, { name: 'HKDF' }, false, [
        'deriveBits',
      ]);
      const derivedBits = await crypto.subtle.deriveBits(
        { name: 'HKDF', hash: 'SHA-256', salt, info: infoBytes },
        hkdfKey,
        256
      );
      const sessionKey = new Uint8Array(derivedBits);

      this.relayHandler.setSessionKey(conn.id, sessionKey);
      wsConn.sessionKey = sessionKey;

      // Send server public key back to client
      const serverKeyPair = await crypto.subtle.generateKey({ name: 'X25519' }, true, ['deriveBits']);
      const serverPubKeyRaw = new Uint8Array(
        await crypto.subtle.exportKey('raw', serverKeyPair.publicKey as CryptoKey)
      );

      const response = this.relayHandler.buildFrame(FrameType.KEEPALIVE, serverPubKeyRaw);
      await this.sendToConnection(connectionId, response);
    } catch (err: any) {
      console.error(`Handshake failed for connection ${connectionId}:`, err.message);
      this.wsConnections.delete(connectionId);
      this.relayHandler.closeConnection(conn.id, 'handshake_failed');
    }
  }

  /**
   * Handle a WebSocket message event from Tencent API Gateway.
   */
  async handleMessage(connectionId: string, data: Uint8Array): Promise<void> {
    const wsConn = this.wsConnections.get(connectionId);
    if (!wsConn) {
      console.warn(`Unknown WebSocket connection: ${connectionId}`);
      return;
    }

    wsConn.lastActivity = Date.now();

    try {
      let frameData = data;
      if (wsConn.sessionKey) {
        frameData = await this.decryptFrame(data, wsConn.sessionKey);
      }

      const frame = this.relayHandler.parseFrame(frameData);
      const responseFrame = this.relayHandler.handleFrame(wsConn.id, frame);

      if (frame.type === FrameType.DATA && wsConn.sessionKey) {
        try {
          const upstreamResponse = await this.relayHandler.relayToUpstream(
            wsConn.targetHost,
            wsConn.targetPort,
            frame.payload,
            wsConn.clientIP
          );

          const dataFrame = this.relayHandler.buildFrame(FrameType.DATA, upstreamResponse);
          const encrypted = await this.encryptFrame(dataFrame, wsConn.sessionKey);
          await this.sendToConnection(connectionId, encrypted);
        } catch (err: any) {
          const errorFrame = this.relayHandler.buildFrame(
            FrameType.CLOSE,
            new TextEncoder().encode('Upstream error')
          );
          await this.sendToConnection(connectionId, errorFrame);
        }
      } else if (responseFrame) {
        let sendData = responseFrame;
        if (wsConn.sessionKey) {
          sendData = await this.encryptFrame(responseFrame, wsConn.sessionKey);
        }
        await this.sendToConnection(connectionId, sendData);
      }
    } catch (err: any) {
      console.error(`Message handling failed for ${connectionId}:`, err.message);
    }
  }

  /**
   * Handle a WebSocket disconnect event from Tencent API Gateway.
   */
  async handleDisconnect(connectionId: string): Promise<void> {
    const wsConn = this.wsConnections.get(connectionId);
    if (wsConn) {
      this.relayHandler.closeConnection(wsConn.id, 'client_disconnect');
      this.wsConnections.delete(connectionId);
    }
  }

  /**
   * Send data to a WebSocket connection via Tencent API Gateway callback.
   */
  private async sendToConnection(connectionId: string, data: Uint8Array): Promise<void> {
    if (!this.callbackHost && !this.apiGatewayUrl) {
      // In local development, just log
      console.log(`[WS Bridge] Send to ${connectionId}: ${data.length} bytes`);
      return;
    }

    const baseUrl = this.apiGatewayUrl || `https://${this.callbackHost}`;
    const url = `${baseUrl}/${connectionId}`;

    try {
      const response = await fetch(url, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/octet-stream',
          'X-Connection-Id': connectionId,
        },
        body: Buffer.from(data).toString('base64'),
      });

      if (!response.ok) {
        console.warn(`Failed to send to connection ${connectionId}: ${response.status}`);
      }
    } catch (err: any) {
      console.error(`Callback failed for ${connectionId}:`, err.message);
    }
  }

  private async encryptFrame(data: Uint8Array, key: Uint8Array): Promise<Uint8Array> {
    const nonce = crypto.getRandomValues(new Uint8Array(NONCE_LENGTH));
    const algoKey = await crypto.subtle.importKey('raw', key, { name: 'ChaCha20-Poly1305' }, false, [
      'encrypt',
    ]);
    const encrypted = await crypto.subtle.encrypt(
      { name: 'ChaCha20-Poly1305', iv: nonce, additionalData: new Uint8Array(0) },
      algoKey,
      data
    );
    const result = new Uint8Array(encrypted);
    const combined = new Uint8Array(NONCE_LENGTH + result.length);
    combined.set(nonce);
    combined.set(result, NONCE_LENGTH);
    return combined;
  }

  private async decryptFrame(data: Uint8Array, key: Uint8Array): Promise<Uint8Array> {
    if (data.length < NONCE_LENGTH + TAG_LENGTH + 1) {
      throw new Error('Frame too short to decrypt');
    }
    const nonce = data.slice(0, NONCE_LENGTH);
    const ciphertext = data.slice(NONCE_LENGTH);
    const algoKey = await crypto.subtle.importKey('raw', key, { name: 'ChaCha20-Poly1305' }, false, [
      'decrypt',
    ]);
    const decrypted = await crypto.subtle.decrypt(
      { name: 'ChaCha20-Poly1305', iv: nonce, additionalData: new Uint8Array(0) },
      algoKey,
      ciphertext
    );
    return new Uint8Array(decrypted);
  }

  private hexToBytes(hex: string): Uint8Array {
    const bytes = new Uint8Array(hex.length / 2);
    for (let i = 0; i < hex.length; i += 2) {
      bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
    }
    return bytes;
  }

  getActiveConnectionCount(): number {
    return this.wsConnections.size;
  }

  getConnection(connectionId: string): WSConnection | undefined {
    return this.wsConnections.get(connectionId);
  }
}
