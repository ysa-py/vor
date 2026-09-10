// @ts-nocheck
/**
 * DPI Signatures — Iranian Deep Packet Inspection patterns
 *
 * Known DPI behaviors:
 * - FAVA TLS RST: Resets TLS connections with 95-320ms timing
 * - HTTP 403: Returns HTTP 403 for blocked URLs
 * - DNS Poison: Returns 10.10.34.34 / 10.10.34.35 for blocked domains
 * - SNI Filter: Filters based on Server Name Indication in TLS ClientHello
 * - Protocol Detection: Detects and blocks VPN/proxy protocols
 */

import type { DPISignature } from './protocol';

/* ────────── FAVA TLS RST Signatures ────────── */
export const FAVA_TLS_RST: DPISignature[] = [
  {
    id: 'fava_tls_rst_1',
    name: 'FAVA TLS Reset (Quick)',
    type: 'tls_rst',
    description: 'TLS connection reset within 95-150ms — indicates DPI reset after reading ClientHello SNI',
    pattern: 'RST after ClientHello',
    timingRangeMs: [95, 150],
    severity: 'high',
  },
  {
    id: 'fava_tls_rst_2',
    name: 'FAVA TLS Reset (Standard)',
    type: 'tls_rst',
    description: 'TLS connection reset within 150-320ms — DPI with deeper inspection or keyword matching',
    pattern: 'RST after ServerHello',
    timingRangeMs: [150, 320],
    severity: 'high',
  },
  {
    id: 'fava_tls_rst_3',
    name: 'FAVA TLS Reset (Delayed)',
    type: 'tls_rst',
    description: 'TLS connection reset after 320ms — DPI with full handshake inspection or HTTP content analysis',
    pattern: 'RST after application data',
    timingRangeMs: [320, 5000],
    severity: 'medium',
  },
];

/* ────────── HTTP 403 Signatures ────────── */
export const HTTP_403_SIGNATURES: DPISignature[] = [
  {
    id: 'http_403_isp',
    name: 'ISP HTTP 403 Block',
    type: 'http_403',
    description: 'HTTP 403 response from DPI system, typically includes "Access Denied" or Farsi message',
    pattern: 'access.denied|access.to.this.site.is.restricted|ممنوع|فیلتر',
    timingRangeMs: [0, 100],
    severity: 'medium',
  },
  {
    id: 'http_403_redirect',
    name: 'ISP HTTP 302 Redirect Block',
    type: 'http_403',
    description: 'Redirect to filtering notice page',
    pattern: 'redirect.to.filtering.notice|internet.filtration',
    timingRangeMs: [0, 200],
    severity: 'medium',
  },
];

/* ────────── DNS Poison Signatures ────────── */
export const DNS_POISON_SIGNATURES: DPISignature[] = [
  {
    id: 'dns_poison_1',
    name: 'DNS Poison (10.10.34.34)',
    type: 'dns_poison',
    description: 'Iranian DPI returns 10.10.34.34 for blocked domain DNS queries',
    pattern: '10.10.34.34',
    timingRangeMs: [0, 50],
    severity: 'critical',
  },
  {
    id: 'dns_poison_2',
    name: 'DNS Poison (10.10.34.35)',
    type: 'dns_poison',
    description: 'Iranian DPI returns 10.10.34.35 for blocked domain DNS queries',
    pattern: '10.10.34.35',
    timingRangeMs: [0, 50],
    severity: 'critical',
  },
  {
    id: 'dns_poison_3',
    name: 'DNS Poison (NXDOMAIN)',
    type: 'dns_poison',
    description: 'Fake NXDOMAIN response from DPI for blocked domains',
    pattern: 'NXDOMAIN.for.known.valid.domain',
    timingRangeMs: [0, 30],
    severity: 'high',
  },
];

/* ────────── SNI Filter Signatures ────────── */
export const SNI_FILTER_SIGNATURES: DPISignature[] = [
  {
    id: 'sni_filter_exact',
    name: 'SNI Exact Match Filter',
    type: 'sni_filter',
    description: 'DPI blocks connections with exact SNI match to blocklist (e.g., youtube.com)',
    pattern: 'exact.sni.match',
    timingRangeMs: [95, 150],
    severity: 'high',
  },
  {
    id: 'sni_filter_wildcard',
    name: 'SNI Wildcard Match Filter',
    type: 'sni_filter',
    description: 'DPI blocks connections with wildcard SNI match (e.g., *.youtube.com)',
    pattern: 'wildcard.sni.match',
    timingRangeMs: [95, 200],
    severity: 'high',
  },
  {
    id: 'sni_filter_keyword',
    name: 'SNI Keyword Filter',
    type: 'sni_filter',
    description: 'DPI blocks connections with keyword in SNI (e.g., "proxy", "vpn", "tor")',
    pattern: 'keyword.sni.match',
    timingRangeMs: [150, 320],
    severity: 'medium',
  },
];

/* ────────── Protocol Detection Signatures ────────── */
export const PROTOCOL_DETECT_SIGNATURES: DPISignature[] = [
  {
    id: 'proto_openvpn',
    name: 'OpenVPN Detection',
    type: 'protocol_detect',
    description: 'DPI detects OpenVPN handshake patterns and resets connection',
    pattern: 'openvpn.handshake|\\x00\\x0c\\x4e\\x6f\\x76\\x61',
    timingRangeMs: [0, 500],
    severity: 'high',
  },
  {
    id: 'proto_wireguard',
    name: 'WireGuard Detection',
    type: 'protocol_detect',
    description: 'DPI detects WireGuard handshake initiation packets',
    pattern: 'wireguard.handshake.init|\\x01\\x00\\x00\\x00',
    timingRangeMs: [0, 300],
    severity: 'high',
  },
  {
    id: 'proto_shadowsocks',
    name: 'Shadowsocks Detection',
    type: 'protocol_detect',
    description: 'DPI detects Shadowsocks traffic via entropy analysis or length patterns',
    pattern: 'high.entropy.stream|ss.handshake',
    timingRangeMs: [500, 5000],
    severity: 'medium',
  },
  {
    id: 'proto_tor',
    name: 'Tor Detection',
    type: 'protocol_detect',
    description: 'DPI detects Tor directory and relay connections',
    pattern: 'tor.cell|\\x00\\x00.*(relay|create|destroy)',
    timingRangeMs: [0, 300],
    severity: 'high',
  },
  {
    id: 'proto_v2ray',
    name: 'VMess/VLESS Detection',
    type: 'protocol_detect',
    description: 'DPI detects VMess/VLESS protocols via header patterns',
    pattern: 'vmess.header|vless.header',
    timingRangeMs: [300, 2000],
    severity: 'medium',
  },
];

/* ────────── Aggregated Exports ────────── */

export const DPI_SIGNATURES = {
  tlsRst: FAVA_TLS_RST,
  http403: HTTP_403_SIGNATURES,
  dnsPoison: DNS_POISON_SIGNATURES,
  sniFilter: SNI_FILTER_SIGNATURES,
  protocolDetect: PROTOCOL_DETECT_SIGNATURES,

  /** Known DNS poisoning IPs */
  dnsPoisonIPs: ['10.10.34.34', '10.10.34.35'],

  /** Known blocked SNI keywords */
  blockedSNIKeywords: [
    'youtube', 'googlevideo', 'youtu.be',
    'twitter', 'facebook', 'instagram',
    'telegram', 'whatsapp',
    'reddit', 'linkedin',
    'tor', 'vpn', 'proxy',
    'filternet', 'baaref', 'v2ray',
  ],

  /** All signatures flat */
  all: [
    ...FAVA_TLS_RST,
    ...HTTP_403_SIGNATURES,
    ...DNS_POISON_SIGNATURES,
    ...SNI_FILTER_SIGNATURES,
    ...PROTOCOL_DETECT_SIGNATURES,
  ],
} as const;

/**
 * Check if a TLS reset timing matches FAVA DPI signature
 */
export function isFAVA_RST(rttMs: number): boolean {
  return FAVA_TLS_RST.some(
    (sig) => rttMs >= sig.timingRangeMs[0] && rttMs <= sig.timingRangeMs[1]
  );
}

/**
 * Check if a DNS response is poisoned
 */
export function isPoisonedDNS(ip: string): boolean {
  return DPI_SIGNATURES.dnsPoisonIPs.includes(ip);
}

/**
 * Check if an SNI is likely to be filtered
 */
export function isFilteredSNI(sni: string): boolean {
  const lower = sni.toLowerCase();
  return DPI_SIGNATURES.blockedSNIKeywords.some((kw) => lower.includes(kw));
}
