// @ts-nocheck
/**
 * Firefox PAC Generator — Same logic as Chrome, adapted for Firefox proxy API
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

  generate(proxyHost: string, proxyPort: number, _mode: 'socks5'): string {
    const proxyString = `SOCKS5 ${proxyHost}:${proxyPort}; SOCKS ${proxyHost}:${proxyPort}`;

    const ranges = IRAN_IP_RANGES.map((e) => `  "${e.cidr}"`).join(',\n');

    const directDomains = [
      '.ir', 'bankmellat.ir', 'sb24.ir', 'bpi.ir', 'enbank.ir',
      'bsi.ir', 'digikala.com', 'snapp.ir', 'divar.ir',
      'shatel.ir', 'mci.ir', 'irancell.ir', 'rightel.ir',
      'ac.ir', 'edu.ir', 'sbu.ac.ir', 'ut.ac.ir', 'sharif.ir',
    ].map((d) => `  "${d}"`).join(',\n');

    const forcedDomains = [
      'youtube.com', 'googlevideo.com', 'twitter.com', 'x.com',
      'facebook.com', 'instagram.com', 'telegram.org', 't.me',
      'whatsapp.com', 'reddit.com', 'github.com', 'wikipedia.org',
      'bbc.com', 'linkedin.com', 'medium.com', 'netflix.com',
      'discord.com', 'twitch.tv', 'spotify.com', 'soundcloud.com',
    ].map((d) => `  "${d}"`).join(',\n');

    return `
// UnifiedShield NextGen PAC Script (Firefox)
// Generated: ${new Date().toISOString()}
// ISP: ${this.isp?.name ?? 'unknown'}

var PROXY = "${proxyString}";
var DIRECT = "DIRECT";

var IRAN_RANGES = [
${ranges}
];

var DIRECT_DOMAINS = [
${directDomains}
];

var FORCED_PROXY_DOMAINS = [
${forcedDomains}
];

var POISON_IPS = ["10.10.34.34", "10.10.34.35", "0.0.0.0"];

function ipToInt(ip) {
  var p = ip.split('.');
  return ((p[0] << 24) | (p[1] << 16) | (p[2] << 8) | p[3]) >>> 0;
}

function isInSubnet(ip, cidr) {
  var parts = cidr.split('/');
  var subnet = parts[0];
  var mask = parseInt(parts[1] || '32', 10);
  var ipInt = ipToInt(ip);
  var subnetInt = ipToInt(subnet);
  var maskInt = mask === 0 ? 0 : (~0 << (32 - mask)) >>> 0;
  return (ipInt & maskInt) === (subnetInt & maskInt);
}

function isIPInIranRange(ip) {
  for (var i = 0; i < IRAN_RANGES.length; i++) {
    if (isInSubnet(ip, IRAN_RANGES[i])) return true;
  }
  return false;
}

function isDirectDomain(host) {
  for (var i = 0; i < DIRECT_DOMAINS.length; i++) {
    var d = DIRECT_DOMAINS[i];
    if (d.startsWith('.') && host.endsWith(d)) return true;
    if (host === d || host.endsWith('.' + d)) return true;
  }
  if (host.endsWith('.ir')) return true;
  return false;
}

function isForcedProxyDomain(host) {
  for (var i = 0; i < FORCED_PROXY_DOMAINS.length; i++) {
    var d = FORCED_PROXY_DOMAINS[i];
    if (host === d || host.endsWith('.' + d)) return true;
  }
  return false;
}

function FindProxyForURL(url, host) {
  host = host.replace(/\\.+$/, '');

  if (isForcedProxyDomain(host)) return PROXY;
  if (isDirectDomain(host)) return DIRECT;

  return PROXY;
}
`.trim();
  }
}
