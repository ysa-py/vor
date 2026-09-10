/**
 * UnifiedShield TypeScript Types
 *
 * Core types for the 9 anti-censorship cores, ISP rules,
 * threat intelligence, and national intranet mode.
 */

// ============================================================================
// Core Types (9 Anti-Censorship Cores)
// ============================================================================

export type CoreId =
  | "xtls-reality"
  | "hysteria2"
  | "tuicv5"
  | "shadowsocks"
  | "vless"
  | "wireguard"
  | "trojan"
  | "naiveproxy"
  | "p2p-relay";

export type CoreProtocol =
  | "xtls"
  | "quic"
  | "socks5"
  | "wireguard"
  | "tls"
  | "http2"
  | "p2p";

export type CoreStatus =
  | "idle"
  | "connecting"
  | "connected"
  | "failed"
  | "blocked"
  | "testing";

export interface CoreState {
  id: CoreId;
  name: string;
  iconEmoji: string;
  protocol: CoreProtocol;
  status: CoreStatus;
  latency: number;       // ms
  bandwidth: number;     // KB/s
  successCount: number;  // For UCB1
  failureCount: number;  // For UCB1
  successRate: number;   // 0-1
  ucb1Score: number;     // Current UCB1 score
  description: string;
  isAvailable: boolean;
  serverAddr?: string;
  port: number;
}

// ============================================================================
// Connection Statistics
// ============================================================================

export interface ConnectionStats {
  speedDown: number;      // bytes/sec
  speedUp: number;        // bytes/sec
  latency: number;        // ms
  jitter: number;         // ms
  packetLoss: number;     // percentage
  totalBytesDown: number;
  totalBytesUp: number;
  uptime: number;         // seconds
  activeCore: string;
  connectedServer: string;
  connectedCountry: string;
  protocol: string;
  speedHistory: number[];
  latencyHistory: number[];
}

// ============================================================================
// P2P Types
// ============================================================================

export interface P2PPeer {
  id: string;
  country: string;
  isRelay: boolean;
  latency: number;
  bandwidth: number;
  isOnline: boolean;
  address?: string;
  lastSeen?: string;
}

export interface P2PRelayStats {
  activeRelays: number;
  totalBytesRelayed: number;
  totalMessagesRelayed: number;
  avgRelayLatency: number;
}

export interface P2PNetworkHealth {
  healthy: boolean;
  reachablePeers: number;
  avgLatency: number;
  dhtSize: number;
}

// ============================================================================
// Intranet Mode Types
// ============================================================================

export type IntranetMode = "disabled" | "essential" | "smart" | "full";

export interface IntranetConfig {
  mode: IntranetMode;
  allowedDomains: string[];
  blockAllExternal: boolean;
  p2pFallback: boolean;
  p2pCriticalServices: string[];
}

export interface NationalServiceCategory {
  id: string;
  nameEn: string;
  nameFa: string;
  domains: string[];
  icon: string;
}

// ============================================================================
// Security & Threat Intelligence Types
// ============================================================================

export type ThreatLevel = "low" | "elevated" | "high" | "critical";

export interface ThreatIntel {
  dpiBlocked: number;
  ipBlocked: number;
  dnsBlocked: number;
  allowed: number;
  level: ThreatLevel;
  lastUpdated: string;
}

export interface ThreatReport {
  id: string;
  type: "dpi" | "dns_poisoning" | "active_probing" | "ip_blocking" | "throttling";
  severity: "low" | "medium" | "high" | "critical";
  description: string;
  affectedProtocols: CoreProtocol[];
  mitigation: string;
  timestamp: string;
}

export interface DpiTestResult {
  isResistant: boolean;
  testsPassed: number;
  totalTests: number;
  detectedTechniques: string[];
  coreId: CoreId;
}

export interface SecurityAuditResult {
  score: number;
  grade: string;
  checks: SecurityCheck[];
  recommendations: string[];
}

export interface SecurityCheck {
  name: string;
  passed: boolean;
  detail?: string;
}

// ============================================================================
// ISP Rules (Iran-Specific)
// ============================================================================

export interface IspRule {
  id: string;
  name: string;
  nameFa: string;
  asn: string;
  routing: string;
  avoid: string[];
  throttlingDetected: boolean;
  throttledProtocols: CoreProtocol[];
  safeProtocols: CoreProtocol[];
}

export const IRAN_ISP_RULES: IspRule[] = [
  {
    id: "mci",
    name: "MCI (Hamrah-e-Aval)",
    nameFa: "همراه اول",
    asn: "AS197207",
    routing: "turkey-uae",
    avoid: ["tk-telekom"],
    throttlingDetected: true,
    throttledProtocols: ["wireguard", "socks5"],
    safeProtocols: ["xtls", "quic", "tls"],
  },
  {
    id: "irancell",
    name: "Irancell",
    nameFa: "ایرانسل",
    asn: "AS44244",
    routing: "turkey-germany",
    avoid: ["iran-backbone"],
    throttlingDetected: true,
    throttledProtocols: ["wireguard"],
    safeProtocols: ["xtls", "quic", "http2"],
  },
  {
    id: "rightel",
    name: "Rightel",
    nameFa: "رایتل",
    asn: "AS49581",
    routing: "uae-india",
    avoid: ["irancell-gw"],
    throttlingDetected: false,
    throttledProtocols: [],
    safeProtocols: ["xtls", "quic", "tls", "http2", "wireguard"],
  },
  {
    id: "shatel",
    name: "Shatel",
    nameFa: "شاتل",
    asn: "AS31549",
    routing: "germany-netherlands",
    avoid: [],
    throttlingDetected: true,
    throttledProtocols: ["wireguard", "socks5"],
    safeProtocols: ["xtls", "quic", "tls"],
  },
  {
    id: "parsonline",
    name: "ParsOnline",
    nameFa: "پارس آنلاین",
    asn: "AS16322",
    routing: "turkey-uae",
    avoid: ["mci-gw"],
    throttlingDetected: false,
    throttledProtocols: [],
    safeProtocols: ["xtls", "quic", "tls", "http2", "wireguard", "p2p"],
  },
  {
    id: "mokhaberat",
    name: "Mokhaberat",
    nameFa: "مخابرات",
    asn: "AS58224",
    routing: "any-available",
    avoid: [],
    throttlingDetected: true,
    throttledProtocols: ["wireguard", "socks5", "p2p"],
    safeProtocols: ["xtls", "quic", "tls"],
  },
];

// ============================================================================
// CDN Mirror Types (Chinese CDNs PRIMARY - Cloudflare BLOCKED in Iran)
// ============================================================================

export interface CdnMirror {
  id: string;
  url: string;
  priority: number;
  region: string;
  accessible: boolean;
  note: string;
}

export const CDN_MIRRORS: CdnMirror[] = [
  {
    id: "alibaba-sh",
    url: "https://unifiedshield.oss-cn-shanghai.aliyuncs.com",
    priority: 1,
    region: "china-east",
    accessible: true,
    note: "PRIMARY - Alibaba Cloud Shanghai",
  },
  {
    id: "alibaba-hk",
    url: "https://unifiedshield.oss-cn-hongkong.aliyuncs.com",
    priority: 2,
    region: "china-hk",
    accessible: true,
    note: "PRIMARY - Alibaba Cloud Hong Kong",
  },
  {
    id: "tencent-hk",
    url: "https://unifiedshield-1258344699.cos.ap-hongkong.myqcloud.com",
    priority: 3,
    region: "china-hk",
    accessible: true,
    note: "PRIMARY - Tencent COS Hong Kong",
  },
  {
    id: "github",
    url: "https://github.com/unifiedshield/unifiedshield-nextgen/releases/download",
    priority: 99,
    region: "global",
    accessible: true,
    note: "SECONDARY - May be slow/throttled in Iran",
  },
];

// ============================================================================
// Obfuscation Types
// ============================================================================

export type ObfuscationTechnique =
  | "tls-camouflage"
  | "domain-fronting"
  | "protocol-padding"
  | "timing-randomization"
  | "packet-segmentation"
  | "traffic-shaping";

export interface ObfuscationConfig {
  technique: ObfuscationTechnique;
  enabled: boolean;
  config: Record<string, unknown>;
}

// ============================================================================
// OTA Update Types
// ============================================================================

export interface OtaUpdateInfo {
  daemonUpdateAvailable: boolean;
  daemonVersion: string;
  appUpdateAvailable: boolean;
  appVersion: string;
  releaseNotes: string;
  releaseNotesFa: string;
  downloadSize: number;
  isCritical: boolean;
}

// ============================================================================
// Exit Node Types
// ============================================================================

export interface ExitNode {
  id: string;
  city: string;
  country: string;
  latency: number;
  bandwidth: number;
  load?: number;
}
