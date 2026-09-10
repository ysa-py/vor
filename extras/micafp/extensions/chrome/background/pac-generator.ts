/**
 * PAC Script Generator — Iranian IP direct, others via SOCKS5 or WebRTC
 * Generates Proxy Auto-Configuration scripts for chrome.proxy API
 */

import type { UnifiedShieldConfig, ISPInfo } from '../../shared/protocol';
import { IRAN_IP_RANGES } from '../../shared/iran-ip-ranges';

export class PacGenerator {
  private config: UnifiedShieldConfig;
  private isp: ISPInfo | null = null;

  constructor(config: UnifiedShieldConfig) {
    this.config = config;
  }

  updateConfig(config: UnifiedShieldConfig): void {
    this.config = config;
  }

  updateISP(isp: ISPInfo | null): void {
    this.isp = isp;
  }

  /**
   * Generate a PAC script that routes:
   * - Iranian IPs → DIRECT
   * - Configured domains → DIRECT (whitelisted local services)
   * - Everything else → SOCKS5 127.0.0.1:port or WebRTC relay
   */
  generate(proxyHost: string, proxyPort: number, mode: 'socks5' | 'webrtc'): string {
    const proxyString = `SOCKS5 ${proxyHost}:${proxyPort}; SOCKS ${proxyHost}:${proxyPort}`;

    // Build Iranian IP CIDR check functions
    const ipRangeChecks = this.buildIPRangeChecks();

    // Build direct domains list (Iranian services that should bypass proxy)
    const directDomains = this.buildDirectDomains();

    // Build blocked domains (always proxy these even if IP appears Iranian)
    const forcedProxyDomains = this.buildForcedProxyDomains();

    const pac = `
// UnifiedShield NextGen PAC Script
// Generated: ${new Date().toISOString()}
// ISP: ${this.isp?.name ?? 'unknown'}
// Mode: ${mode}

var PROXY = "${proxyString}";
var DIRECT = "DIRECT";

${ipRangeChecks}

${directDomains}

${forcedProxyDomains}

// DNS poisoning detection IPs
var POISON_IPS = [
  "10.10.34.34",
  "10.10.34.35",
  "0.0.0.0",
  "127.0.0.1"
];

function isInSubnet(ip, cidr) {
  var parts = cidr.split('/');
  var subnet = parts[0];
  var mask = parseInt(parts[1] || '32', 10);
  var ipInt = ipToInt(ip);
  var subnetInt = ipToInt(subnet);
  var maskInt = mask === 0 ? 0 : (~0 << (32 - mask)) >>> 0;
  return (ipInt & maskInt) === (subnetInt & maskInt);
}

function ipToInt(ip) {
  var parts = ip.split('.');
  return ((parts[0] << 24) | (parts[1] << 16) | (parts[2] << 8) | parts[3]) >>> 0;
}

function isIranianIP(ip) {
  if (!ip) return false;
  return isIPInIranRange(ip);
}

function isPoisonedIP(ip) {
  for (var i = 0; i < POISON_IPS.length; i++) {
    if (ip === POISON_IPS[i]) return true;
  }
  return false;
}

function FindProxyForURL(url, host) {
  // Remove trailing dots
  host = host.replace(/\\.+$/, '');

  // Forced proxy domains — always route through proxy
  if (isForcedProxyDomain(host)) {
    return PROXY;
  }

  // Direct domains — Iranian services that must bypass proxy
  if (isDirectDomain(host)) {
    return DIRECT;
  }

  // Resolve DNS via DoH if enabled
  // (Note: PAC cannot do async, but we flag poisoned IPs)

  // For known blocked categories, always proxy
  var lowerUrl = url.toLowerCase();
  if (matchesBlockedPattern(lowerUrl)) {
    return PROXY;
  }

  // Default: proxy everything outside Iran
  // Iranian IPs detected at DNS level go direct
  return PROXY;
}

function matchesBlockedPattern(url) {
  var blockedPatterns = [
    "youtube.com",
    "twitter.com",
    "x.com",
    "facebook.com",
    "instagram.com",
    "telegram.org",
    "whatsapp.com",
    "google.com/search",
    "wikipedia.org",
    "bbc.com",
    "voanews.com",
    "dw.com",
    "reddit.com",
    "linkedin.com",
    "medium.com",
    "netflix.com",
    "spotify.com",
    "soundcloud.com",
    "twitch.tv",
    "discord.com",
    "github.com",
    "gitlab.com",
    "stackoverflow.com",
    "amazonaws.com",
    "cloudflare.com",
    "android.com",
    "chrome.com",
    "play.google.com"
  ];
  for (var i = 0; i < blockedPatterns.length; i++) {
    if (url.indexOf(blockedPatterns[i]) !== -1) return true;
  }
  return false;
}
`.trim();

    return pac;
  }

  /**
   * Build JavaScript IP range check functions from the Iranian IP database
   */
  private buildIPRangeChecks(): string {
    const ranges: string[] = [];
    for (const entry of IRAN_IP_RANGES) {
      ranges.push(`  "${entry.cidr}"`);
    }

    return `
var IRAN_RANGES = [
${ranges.join(',\n')}
];

function isIPInIranRange(ip) {
  for (var i = 0; i < IRAN_RANGES.length; i++) {
    if (isInSubnet(ip, IRAN_RANGES[i])) return true;
  }
  return false;
}`.trim();
  }

  /**
   * Build direct domain list — Iranian services that work without proxy
   */
  private buildDirectDomains(): string {
    const domains = [
      // Iranian government & services
      '.ir', '.ایران',
      // Iranian banks
      'bankmellat.ir', 'sb24.ir', 'bpi.ir', 'enbank.ir',
      'bsi.ir', 'bank-maskan.ir', 'postbank.ir', 'bank-refah.ir',
      'ttbank.ir', 'banksepah.ir', 'edbi.ir', 'bank-saderat.ir',
      // Iranian e-commerce
      'digikala.com', 'snapp.ir', 'divar.ir', 'esam.ir',
      // Iranian ISPs self-service
      'shatel.ir', 'mci.ir', 'irancell.ir', 'rightel.ir',
      // Iranian academic
      'ac.ir', 'edu.ir', 'sbu.ac.ir', 'ut.ac.ir', 'sharif.ir',
      'iust.ac.ir', 'amol.ac.ir', 'umz.ac.ir',
      // CDN servers inside Iran
      'cdn.ir', 'static.ir',
    ];

    const lines = domains.map((d) => `  "${d}"`).join(',\n');

    return `
var DIRECT_DOMAINS = [
${lines}
];

function isDirectDomain(host) {
  for (var i = 0; i < DIRECT_DOMAINS.length; i++) {
    var d = DIRECT_DOMAINS[i];
    if (d.startsWith('.') && host.endsWith(d)) return true;
    if (host === d) return true;
    if (host.endsWith('.' + d)) return true;
  }
  // All .ir domains go direct
  if (host.endsWith('.ir')) return true;
  return false;
}`.trim();
  }

  /**
   * Build forced proxy domain list — domains that must always use proxy
   */
  private buildForcedProxyDomains(): string {
    const domains = [
      'youtube.com', 'youtu.be', 'googlevideo.com',
      'twitter.com', 'x.com', 't.co', 'twimg.com',
      'facebook.com', 'fbcdn.net', 'instagram.com', 'cdninstagram.com',
      'telegram.org', 't.me', 'telegram.me',
      'whatsapp.com', 'whatsapp.net',
      'reddit.com', 'redditstatic.com',
      'linkedin.com', 'licdn.com',
      'github.com', 'githubusercontent.com',
      'wikipedia.org', 'wikimedia.org',
      'bbc.com', 'bbci.co.uk',
      'medium.com',
      'netflix.com', 'nflxvideo.net', 'nflxso.net',
      'spotify.com',
      'discord.com', 'discordapp.com', 'discord.gg',
      'twitch.tv', 'ttvnw.net', 'jtvnw.net',
      'soundcloud.com',
      'stackoverflow.com',
      'cloudflare.com', 'cloudflare-dns.com',
      'android.com',
    ];

    const lines = domains.map((d) => `  "${d}"`).join(',\n');

    return `
var FORCED_PROXY_DOMAINS = [
${lines}
];

function isForcedProxyDomain(host) {
  for (var i = 0; i < FORCED_PROXY_DOMAINS.length; i++) {
    var d = FORCED_PROXY_DOMAINS[i];
    if (host === d || host.endsWith('.' + d)) return true;
  }
  return false;
}`.trim();
  }
}
