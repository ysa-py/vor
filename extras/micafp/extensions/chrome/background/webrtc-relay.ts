/**
 * WebRTC Relay — Data channel relay for when native app is not running
 * Uses WebRTC peer connections to tunnel traffic through relay servers
 * since direct SOCKS5 may not be available.
 */

import type { UnifiedShieldConfig, RelayPeer } from '../../shared/protocol';

interface RTCRelayConfig {
  signalingUrl: string;
  stunServers: string[];
  turnServers: RTCIceServer[];
}

const DEFAULT_ICE_SERVERS: RTCIceServer[] = [
  // Chinese CDN STUN servers (Cloudflare blocked in Iran)
  { urls: 'stun:stun.miwifi.com:3478' },
  { urls: 'stun:stun.chat.bilibili.com:3478' },
  { urls: 'stun:stun.hitv.com:3478' },
  { urls: 'stun:stun.l.google.com:19302' },
];

export class WebRelay {
  private config: UnifiedShieldConfig;
  private peers: Map<string, RelayPeer> = new Map();
  private localServerSocket: TCPServerSocket | null = null;
  private localPort: number = 0;
  private signalingWebSocket: WebSocket | null = null;
  private dataChannels: Map<string, RTCDataChannel> = new Map();
  private pendingConnections: Map<string, {
    resolve: (peer: RelayPeer) => void;
    reject: (err: Error) => void;
  }> = new Map();

  constructor(config: UnifiedShieldConfig) {
    this.config = config;
  }

  updateConfig(config: UnifiedShieldConfig): void {
    this.config = config;
  }

  /**
   * Connect to signaling server and establish WebRTC relay
   */
  async connect(signalingUrl: string): Promise<RelayPeer | null> {
    try {
      // 1. Start local SOCKS5 listener
      await this.startLocalListener();

      // 2. Connect to signaling server
      await this.connectSignaling(signalingUrl);

      // 3. Create WebRTC peer connection
      const peer = await this.createPeerConnection();

      if (peer) {
        this.peers.set(peer.id, peer);
        return peer;
      }

      return null;
    } catch (err) {
      console.error('[WebRTC] Connection failed:', err);
      this.disconnect();
      return null;
    }
  }

  /**
   * Start local TCP server to accept SOCKS5 connections
   * and forward them through WebRTC data channels
   */
  private async startLocalListener(): Promise<void> {
    try {
      // Use chrome.sockets if available, otherwise fallback
      if ('sockets' in chrome) {
        this.localPort = this.config.webrtcLocalPort || 1081;

        // @ts-expect-error chrome.sockets.tcpServer may not be in types
        chrome.sockets.tcpServer.create({}, (createInfo) => {
          // @ts-expect-error
          chrome.sockets.tcpServer.listen(
            createInfo.socketId,
            '127.0.0.1',
            this.localPort,
            (result) => {
              if (result < 0) {
                console.error('[WebRTC] Failed to listen on port', this.localPort);
                return;
              }
              console.log('[WebRTC] Local SOCKS5 listener on port', this.localPort);
            }
          );

          // @ts-expect-error
          chrome.sockets.tcpServer.onAccept.addListener((info) => {
            if (info.socketId === createInfo.socketId) {
              this.handleLocalConnection(info.clientSocketId);
            }
          });
        });
      } else {
        // Fallback: use a port for proxy config
        this.localPort = this.config.webrtcLocalPort || 1081;
        console.log('[WebRTC] Using port', this.localPort, 'for proxy config');
      }
    } catch (err) {
      console.error('[WebRTC] Local listener failed:', err);
      this.localPort = this.config.webrtcLocalPort || 1081;
    }
  }

  /**
   * Handle incoming local connection and forward through WebRTC
   */
  private handleLocalConnection(socketId: number): void {
    // Create a new data channel for this connection
    const channelId = `ch-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;

    for (const [peerId, peer] of this.peers) {
      const pc = peer.connection;
      if (pc.connectionState === 'connected') {
        const dc = pc.createDataChannel(channelId, {
          ordered: false,
          maxRetransmits: 0,
        });

        dc.binaryType = 'arraybuffer';

        dc.onopen = () => {
          this.dataChannels.set(channelId, dc);
        };

        dc.onmessage = (event) => {
          // Forward data back to local socket
          const data = event.data as ArrayBuffer;
          // @ts-expect-error
          chrome.sockets.tcp.send(socketId, data);
        };

        dc.onerror = (err) => {
          console.error('[WebRTC] Data channel error:', err);
          this.dataChannels.delete(channelId);
        };

        dc.onclose = () => {
          this.dataChannels.delete(channelId);
          // @ts-expect-error
          chrome.sockets.tcp.disconnect(socketId);
        };

        break;
      }
    }
  }

  /**
   * Connect to signaling server via WebSocket
   */
  private connectSignaling(url: string): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        this.signalingWebSocket = new WebSocket(url);

        this.signalingWebSocket.onopen = () => {
          console.log('[WebRTC] Signaling connected');
          // Register with signaling server
          this.signalingWebSocket!.send(
            JSON.stringify({
              type: 'register',
              role: 'client',
              version: '2.0',
            })
          );
          resolve();
        };

        this.signalingWebSocket.onmessage = (event) => {
          this.handleSignalingMessage(JSON.parse(event.data));
        };

        this.signalingWebSocket.onerror = (err) => {
          console.error('[WebRTC] Signaling error:', err);
          reject(new Error('Signaling connection failed'));
        };

        this.signalingWebSocket.onclose = () => {
          console.warn('[WebRTC] Signaling disconnected');
          this.signalingWebSocket = null;
        };

        // Timeout
        setTimeout(() => {
          if (this.signalingWebSocket?.readyState !== WebSocket.OPEN) {
            reject(new Error('Signaling timeout'));
          }
        }, 10000);
      } catch (err) {
        reject(err);
      }
    });
  }

  /**
   * Handle signaling messages (SDP offers/answers, ICE candidates)
   */
  private handleSignalingMessage(msg: {
    type: string;
    from?: string;
    sdp?: string;
    candidate?: string;
  }): void {
    switch (msg.type) {
      case 'offer': {
        // Relay server sent an offer — create answer
        this.handleOffer(msg.from!, msg.sdp!);
        break;
      }
      case 'answer': {
        // Our offer was answered
        this.handleAnswer(msg.from!, msg.sdp!);
        break;
      }
      case 'candidate': {
        // ICE candidate from remote
        this.handleIceCandidate(msg.from!, msg.candidate!);
        break;
      }
      case 'peer-ready': {
        const pending = this.pendingConnections.get(msg.from!);
        if (pending) {
          pending.resolve({
            id: msg.from!,
            connection: this.peers.get(msg.from!)!.connection,
            localPort: this.localPort,
            connected: true,
            latency: 0,
          });
          this.pendingConnections.delete(msg.from!);
        }
        break;
      }
    }
  }

  /**
   * Create a WebRTC peer connection to relay server
   */
  private async createPeerConnection(): Promise<RelayPeer | null> {
    const peerId = `relay-${Date.now()}`;

    const iceServers: RTCIceServer[] = [
      ...DEFAULT_ICE_SERVERS,
      ...this.config.turnServers.map((s) => ({
        urls: s,
        username: this.config.turnUsername ?? '',
        credential: this.config.turnPassword ?? '',
      })),
    ];

    const pc = new RTCPeerConnection({
      iceServers,
      iceCandidatePoolSize: 10,
    });

    const peer: RelayPeer = {
      id: peerId,
      connection: pc,
      localPort: this.localPort,
      connected: false,
      latency: 0,
    };

    // ICE candidate handling
    pc.onicecandidate = (event) => {
      if (event.candidate && this.signalingWebSocket?.readyState === WebSocket.OPEN) {
        this.signalingWebSocket.send(
          JSON.stringify({
            type: 'candidate',
            to: peerId,
            candidate: event.candidate.candidate,
          })
        );
      }
    };

    // Connection state
    pc.onconnectionstatechange = () => {
      const state = pc.connectionState;
      console.log(`[WebRTC] Connection state: ${state}`);
      peer.connected = state === 'connected';

      if (state === 'failed' || state === 'disconnected') {
        this.peers.delete(peerId);
        this.attemptReconnect(peerId);
      }
    };

    // Data channel from relay
    pc.ondatachannel = (event) => {
      const dc = event.channel;
      dc.binaryType = 'arraybuffer';

      dc.onmessage = (msgEvent) => {
        // Handle incoming relay data
        this.handleRelayData(dc.label, msgEvent.data as ArrayBuffer);
      };
    };

    // Create offer
    try {
      const offer = await pc.createOffer();
      await pc.setLocalDescription(offer);

      // Send offer to signaling
      if (this.signalingWebSocket?.readyState === WebSocket.OPEN) {
        this.signalingWebSocket.send(
          JSON.stringify({
            type: 'offer',
            from: peerId,
            sdp: offer.sdp,
          })
        );
      }

      this.peers.set(peerId, peer);

      // Wait for connection
      return new Promise((resolve) => {
        const checkInterval = setInterval(() => {
          if (peer.connected) {
            clearInterval(checkInterval);
            resolve(peer);
          }
        }, 500);

        setTimeout(() => {
          clearInterval(checkInterval);
          resolve(null);
        }, 30000);
      });
    } catch (err) {
      console.error('[WebRTC] Offer creation failed:', err);
      return null;
    }
  }

  private async handleOffer(from: string, sdp: string): Promise<void> {
    const pc = this.peers.get(from)?.connection;
    if (!pc) return;

    await pc.setRemoteDescription({ type: 'offer', sdp });
    const answer = await pc.createAnswer();
    await pc.setLocalDescription(answer);

    this.signalingWebSocket?.send(
      JSON.stringify({
        type: 'answer',
        to: from,
        sdp: answer.sdp,
      })
    );
  }

  private async handleAnswer(from: string, sdp: string): Promise<void> {
    const pc = this.peers.get(from)?.connection;
    if (!pc) return;

    await pc.setRemoteDescription({ type: 'answer', sdp });
  }

  private async handleIceCandidate(from: string, candidate: string): Promise<void> {
    const pc = this.peers.get(from)?.connection;
    if (!pc) return;

    await pc.addIceCandidate({ candidate, sdpMid: '0', sdpMLineIndex: 0 });
  }

  /**
   * Handle data received from relay
   */
  private handleRelayData(channelLabel: string, data: ArrayBuffer): void {
    // Forward to local socket or process
    console.log(`[WebRTC] Data received on channel ${channelLabel}: ${data.byteLength} bytes`);
  }

  /**
   * Attempt to reconnect to a lost peer
   */
  private attemptReconnect(peerId: string): void {
    if (!this.signalingWebSocket || this.signalingWebSocket.readyState !== WebSocket.OPEN) {
      return;
    }

    console.log(`[WebRTC] Attempting reconnect for ${peerId}`);
    setTimeout(() => {
      this.createPeerConnection();
    }, 3000);
  }

  /**
   * Disconnect all peers and cleanup
   */
  disconnect(): void {
    for (const [id, peer] of this.peers) {
      try {
        peer.connection.close();
      } catch { /* ignore */ }
    }
    this.peers.clear();
    this.dataChannels.clear();

    if (this.signalingWebSocket) {
      this.signalingWebSocket.close();
      this.signalingWebSocket = null;
    }

    if (this.localServerSocket) {
      try { this.localServerSocket.close(); } catch { /* ignore */ }
      this.localServerSocket = null;
    }

    console.log('[WebRTC] Disconnected');
  }

  /**
   * Get list of connected peers
   */
  getPeers(): RelayPeer[] {
    return Array.from(this.peers.values()).filter((p) => p.connected);
  }

  /**
   * Get local port for proxy configuration
   */
  getLocalPort(): number {
    return this.localPort;
  }
}

// TCPServerSocket type stub for environments without it
interface TCPServerSocket {
  close(): void;
}
