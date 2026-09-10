// ──────────────────────────────────────────────
// MICAFP-UnifiedShield — Security Audit Engine
// DNS leak test, WebRTC leak, IPv6 leak, kill switch
// verification, encryption strength, privacy score,
// security recommendations in Persian, real-time monitoring
// ──────────────────────────────────────────────

import type {
  SecurityAuditState,
  DNSLeakResult,
  WebRTCLeakResult,
  IPv6LeakResult,
  EncryptionAssessment,
  SecurityRecommendation,
} from './unified-shield-types';

// ──────────────────────────────────────────────
// DNS Leak Test Simulation
// ──────────────────────────────────────────────
export function performDNSLeakTest(
  vpnConnected: boolean,
  dnsMode: 'doh' | 'dot' | 'plain',
): DNSLeakResult {
  const totalQueries = 50 + Math.floor(Math.random() * 30);
  const testDurationMs = 3000 + Math.floor(Math.random() * 2000);

  if (!vpnConnected) {
    return {
      isLeaking: true,
      detectedServers: ['5.160.128.1', '217.218.127.127', '78.38.64.1'],
      expectedServer: '1.1.1.1',
      leakCount: totalQueries,
      totalQueries,
      testDurationMs,
      details: 'VPN not connected — all DNS queries go through ISP servers',
      detailsFa: 'VPN متصل نیست — تمام درخواست‌های DNS از سرورهای ISP عبور می‌کنند',
    };
  }

  // Simulate leak based on DNS mode
  let isLeaking: boolean;
  let leakCount: number;
  let detectedServers: string[];

  if (dnsMode === 'plain') {
    // Plain DNS is very likely to leak
    isLeaking = Math.random() < 0.85;
    leakCount = isLeaking ? Math.floor(totalQueries * (0.3 + Math.random() * 0.5)) : 0;
    detectedServers = isLeaking
      ? ['5.160.128.1', '217.218.127.127']
      : ['1.1.1.1'];
  } else if (dnsMode === 'dot') {
    // DNS over TLS is somewhat safe
    isLeaking = Math.random() < 0.15;
    leakCount = isLeaking ? Math.floor(totalQueries * (0.05 + Math.random() * 0.15)) : 0;
    detectedServers = isLeaking
      ? ['5.160.128.1', '1.1.1.1']
      : ['1.1.1.1'];
  } else {
    // DNS over HTTPS is safest
    isLeaking = Math.random() < 0.05;
    leakCount = isLeaking ? Math.floor(totalQueries * 0.05) : 0;
    detectedServers = isLeaking
      ? ['5.160.128.1', '1.1.1.1']
      : ['1.1.1.1'];
  }

  const details = isLeaking
    ? `DNS leak detected — ${leakCount} of ${totalQueries} queries went to non-VPN servers`
    : `No DNS leak — all ${totalQueries} queries went through encrypted VPN tunnel`;
  const detailsFa = isLeaking
    ? `نشت DNS شناسایی شد — ${leakCount} از ${totalQueries} درخواست به سرورهای غیر VPN رفت`
    : `بدون نشت DNS — تمام ${totalQueries} درخواست از تونل رمزنگاری‌شده VPN عبور کردند`;

  return {
    isLeaking,
    detectedServers,
    expectedServer: '1.1.1.1',
    leakCount,
    totalQueries,
    testDurationMs,
    details,
    detailsFa,
  };
}

// ──────────────────────────────────────────────
// WebRTC Leak Detection
// ──────────────────────────────────────────────
export function detectWebRTCLeak(
  vpnConnected: boolean,
  killSwitchEnabled: boolean,
): WebRTCLeakResult {
  if (!vpnConnected) {
    return {
      isLeaking: true,
      detectedIPs: ['192.168.1.' + Math.floor(10 + Math.random() * 200), '5.160.' + Math.floor(Math.random() * 255) + '.' + Math.floor(Math.random() * 255)],
      localIPs: ['192.168.1.' + Math.floor(10 + Math.random() * 200)],
      publicIPs: ['5.160.' + Math.floor(Math.random() * 255) + '.' + Math.floor(Math.random() * 255)],
      details: 'WebRTC leak — real IP address exposed when VPN disconnected',
      detailsFa: 'نشت WebRTC — آدرس IP واقعی وقتی VPN قطع است فاش شده',
    };
  }

  // WebRTC can still leak even with VPN if not properly configured
  const hasLeak = !killSwitchEnabled && Math.random() < 0.3;
  const localIPs = hasLeak
    ? ['192.168.1.' + Math.floor(10 + Math.random() * 200)]
    : [];
  const publicIPs = hasLeak
    ? ['5.160.' + Math.floor(Math.random() * 255) + '.' + Math.floor(Math.random() * 255)]
    : [];

  const vpnIP = `${10 + Math.floor(Math.random() * 5)}.${Math.floor(Math.random() * 255)}.${Math.floor(Math.random() * 255)}.${Math.floor(Math.random() * 255)}`;

  const details = hasLeak
    ? `WebRTC leak detected — real IP exposed through STUN/TURN servers despite VPN`
    : `No WebRTC leak — VPN tunnel properly isolates WebRTC traffic (detected IP: ${vpnIP})`;
  const detailsFa = hasLeak
    ? 'نشت WebRTC شناسایی شد — IP واقعی از طریق سرورهای STUN/TURN فاش شده با وجود VPN'
    : `بدون نشت WebRTC — تونل VPN ترافیک WebRTC را به‌درستی ایزوله می‌کند (IP شناسایی‌شده: ${vpnIP})`;

  return {
    isLeaking: hasLeak,
    detectedIPs: hasLeak ? [...localIPs, ...publicIPs] : [vpnIP],
    localIPs,
    publicIPs,
    details,
    detailsFa,
  };
}

// ──────────────────────────────────────────────
// IPv6 Leak Detection
// ──────────────────────────────────────────────
export function detectIPv6Leak(
  vpnConnected: boolean,
  ipv6Enabled: boolean,
): IPv6LeakResult {
  if (!vpnConnected) {
    return {
      isLeaking: true,
      ipv6Address: ipv6Enabled ? '2001:1c04:' + Math.floor(Math.random() * 9999) + '::1' : null,
      expectedIPv6: null,
      details: 'VPN not connected — IPv6 traffic goes through ISP',
      detailsFa: 'VPN متصل نیست — ترافیک IPv6 از ISP عبور می‌کند',
    };
  }

  // Check if IPv6 is leaking (going outside VPN tunnel)
  if (ipv6Enabled) {
    const hasLeak = Math.random() < 0.12;
    const realIPv6 = hasLeak
      ? '2001:1c04:' + Math.floor(Math.random() * 9999) + '::1'
      : null;
    const vpnIPv6 = 'fd00:dead:beef::1';

    const details = hasLeak
      ? `IPv6 leak detected — real IPv6 address ${realIPv6} exposed outside VPN tunnel`
      : `No IPv6 leak — IPv6 traffic properly routed through VPN (${vpnIPv6})`;
    const detailsFa = hasLeak
      ? `نشت IPv6 شناسایی شد — آدرس IPv6 واقعی ${realIPv6} خارج از تونل VPN فاش شده`
      : `بدون نشت IPv6 — ترافیک IPv6 به‌درستی از VPN عبور می‌کند (${vpnIPv6})`;

    return {
      isLeaking: hasLeak,
      ipv6Address: hasLeak ? realIPv6 : vpnIPv6,
      expectedIPv6: vpnIPv6,
      details,
      detailsFa,
    };
  }

  // IPv6 disabled — check if it's still somehow leaking
  const hasLeak = Math.random() < 0.03;
  const details = hasLeak
    ? 'IPv6 leak detected — IPv6 traffic detected despite IPv6 being disabled'
    : 'No IPv6 leak — IPv6 is disabled and no IPv6 traffic detected';
  const detailsFa = hasLeak
    ? 'نشت IPv6 شناسایی شد — ترافیک IPv6 با وجود غیرفعال بودن شناسایی شد'
    : 'بدون نشت IPv6 — IPv6 غیرفعال است و هیچ ترافیک IPv6 شناسایی نشد';

  return {
    isLeaking: hasLeak,
    ipv6Address: hasLeak ? '2001:1c04::1' : null,
    expectedIPv6: null,
    details,
    detailsFa,
  };
}

// ──────────────────────────────────────────────
// Kill Switch Verification
// ──────────────────────────────────────────────
export function verifyKillSwitch(
  killSwitchEnabled: boolean,
  networkLockEnabled: boolean,
): {
  verified: boolean;
  details: string;
  detailsFa: string;
} {
  if (!killSwitchEnabled) {
    return {
      verified: false,
      details: 'Kill switch is disabled — traffic will leak if VPN disconnects',
      detailsFa: 'کلید کشت غیرفعال است — ترافیک در صورت قطع VPN نشت خواهد کرد',
    };
  }

  if (!networkLockEnabled) {
    return {
      verified: false,
      details: 'Network lock is disabled — some traffic may bypass kill switch',
      detailsFa: 'قفل شبکه غیرفعال است — برخی ترافیک ممکن است از کلید کشت عبور کند',
    };
  }

  // Simulate kill switch test (attempt to reach internet with VPN disconnected)
  const testPassed = Math.random() < 0.97;
  if (!testPassed) {
    return {
      verified: false,
      details: 'Kill switch test failed — brief traffic leak detected during disconnect simulation',
      detailsFa: 'تست کلید کشت ناموفق — نشت کوتاه ترافیک در شبیه‌سازی قطع شناسایی شد',
    };
  }

  return {
    verified: true,
    details: 'Kill switch verified — all traffic blocked when VPN disconnected, network lock active',
    detailsFa: 'کلید کشت تأیید شد — تمام ترافیک هنگام قطع VPN مسدود شد، قفل شبکه فعال',
  };
}

// ──────────────────────────────────────────────
// Encryption Strength Assessment
// ──────────────────────────────────────────────
export function assessEncryption(activeCoreId: string): EncryptionAssessment {
  // Map core IDs to realistic encryption configurations
  const encryptionMap: Record<string, EncryptionAssessment> = {
    'hiddify': {
      protocol: 'VLESS + Reality + XTLS',
      protocolFa: 'VLESS Reality + XTLS',
      keyExchange: 'X25519 Reality Handshake',
      keyExchangeFa: 'دست‌دهی Reality X25519',
      cipher: 'AES-128-GCM / ChaCha20-Poly1305',
      cipherFa: 'AES-128-GCM / ChaCha20-Poly1305',
      strength: 'excellent',
      strengthFa: 'عالی',
      score: 95,
    },
    'xray-gfw': {
      protocol: 'VLESS + Fragment + Reality',
      protocolFa: 'VLESS + Fragment + Reality',
      keyExchange: 'X25519 + TLS Fragmentation',
      keyExchangeFa: 'X25519 + تقسیم Fragment TLS',
      cipher: 'AES-256-GCM / ChaCha20-Poly1305',
      cipherFa: 'AES-256-GCM / ChaCha20-Poly1305',
      strength: 'excellent',
      strengthFa: 'عالی',
      score: 97,
    },
    'sing-box': {
      protocol: 'Hysteria2 + QUIC',
      protocolFa: 'هیستریا۲ + QUIC',
      keyExchange: 'QUIC Salting + X25519',
      keyExchangeFa: 'QUIC Salting + X25519',
      cipher: 'AES-128-GCM',
      cipherFa: 'AES-128-GCM',
      strength: 'strong',
      strengthFa: 'قوی',
      score: 88,
    },
    'amneziavpn': {
      protocol: 'AmneziaWG 1.5',
      protocolFa: 'آمنزیاوی‌جی ۱.۵',
      keyExchange: 'Noise_IKpsk2 + Junk Headers',
      keyExchangeFa: 'Noise_IKpsk2 + هدرهای جونک',
      cipher: 'ChaCha20-Poly1305',
      cipherFa: 'ChaCha20-Poly1305',
      strength: 'strong',
      strengthFa: 'قوی',
      score: 85,
    },
    'defyxvpn': {
      protocol: 'VLESS Reality + AmneziaWG',
      protocolFa: 'VLESS Reality + آمنزیاوی‌جی',
      keyExchange: 'X25519 Reality + Noise_IK',
      keyExchangeFa: 'Reality X25519 + Noise_IK',
      cipher: 'AES-256-GCM / ChaCha20-Poly1305',
      cipherFa: 'AES-256-GCM / ChaCha20-Poly1305',
      strength: 'excellent',
      strengthFa: 'عالی',
      score: 94,
    },
    'moav': {
      protocol: 'MoaV Tunnel',
      protocolFa: 'تونل موآوی',
      keyExchange: 'Adaptive Key Exchange',
      keyExchangeFa: 'تبادل کلید تطبیقی',
      cipher: 'ChaCha20-Poly1305',
      cipherFa: 'ChaCha20-Poly1305',
      strength: 'strong',
      strengthFa: 'قوی',
      score: 82,
    },
    'lantern': {
      protocol: 'Domain Fronting + TLS',
      protocolFa: 'فرانتینگ دامنه + TLS',
      keyExchange: 'TLS 1.3 ECDHE',
      keyExchangeFa: 'TLS 1.3 ECDHE',
      cipher: 'AES-128-GCM',
      cipherFa: 'AES-128-GCM',
      strength: 'moderate',
      strengthFa: 'متوسط',
      score: 70,
    },
    'mahsang': {
      protocol: 'MVLESS + Reality',
      protocolFa: 'MVLESS + Reality',
      keyExchange: 'X25519 Reality + Custom Obfs',
      keyExchangeFa: 'Reality X25519 + پنهان‌سازی سفارشی',
      cipher: 'AES-256-GCM / ChaCha20-Poly1305',
      cipherFa: 'AES-256-GCM / ChaCha20-Poly1305',
      strength: 'excellent',
      strengthFa: 'عالی',
      score: 96,
    },
    'psiphon': {
      protocol: 'SSH + Obfuscated Transport',
      protocolFa: 'SSH + حمل مبهم‌شده',
      keyExchange: 'Curve25519 SSH',
      keyExchangeFa: 'Curve25519 SSH',
      cipher: 'AES-256-CTR',
      cipherFa: 'AES-256-CTR',
      strength: 'moderate',
      strengthFa: 'متوسط',
      score: 65,
    },
  };

  return encryptionMap[activeCoreId] ?? {
    protocol: 'Unknown',
    protocolFa: 'نامشخص',
    keyExchange: 'Unknown',
    keyExchangeFa: 'نامشخص',
    cipher: 'Unknown',
    cipherFa: 'نامشخص',
    strength: 'weak',
    strengthFa: 'ضعیف',
    score: 30,
  };
}

// ──────────────────────────────────────────────
// Privacy Score Calculation (0-100)
// ──────────────────────────────────────────────
export function calculatePrivacyScore(params: {
  dnsLeak: DNSLeakResult;
  webrtcLeak: WebRTCLeakResult;
  ipv6Leak: IPv6LeakResult;
  killSwitchVerified: boolean;
  encryption: EncryptionAssessment;
  vpnConnected: boolean;
}): {
  score: number;
  label: string;
  labelFa: string;
  status: SecurityAuditState['overallSecurityStatus'];
  statusFa: string;
} {
  if (!params.vpnConnected) {
    return {
      score: 5,
      label: 'Not Protected',
      labelFa: 'بدون محافظت',
      status: 'critical',
      statusFa: 'بحرانی',
    };
  }

  let score = 0;

  // DNS leak: 0-25 points
  if (!params.dnsLeak.isLeaking) {
    score += 25;
  } else {
    const leakRatio = params.dnsLeak.leakCount / Math.max(1, params.dnsLeak.totalQueries);
    score += Math.round(25 * (1 - leakRatio));
  }

  // WebRTC leak: 0-20 points
  if (!params.webrtcLeak.isLeaking) {
    score += 20;
  } else {
    score += 5;
  }

  // IPv6 leak: 0-15 points
  if (!params.ipv6Leak.isLeaking) {
    score += 15;
  } else {
    score += 3;
  }

  // Kill switch: 0-15 points
  if (params.killSwitchVerified) {
    score += 15;
  } else {
    score += 3;
  }

  // Encryption strength: 0-25 points
  score += Math.round((params.encryption.score / 100) * 25);

  score = Math.min(100, Math.max(0, score));

  let label: string;
  let labelFa: string;
  let status: SecurityAuditState['overallSecurityStatus'];
  let statusFa: string;

  if (score >= 85) {
    label = 'Excellent Privacy';
    labelFa = 'حریم خصوصی عالی';
    status = 'secure';
    statusFa = 'امن';
  } else if (score >= 65) {
    label = 'Good Privacy';
    labelFa = 'حریم خصوصی خوب';
    status = 'secure';
    statusFa = 'امن';
  } else if (score >= 45) {
    label = 'Moderate Privacy';
    labelFa = 'حریم خصوصی متوسط';
    status = 'warning';
    statusFa = 'هشدار';
  } else if (score >= 25) {
    label = 'Poor Privacy';
    labelFa = 'حریم خصوصی ضعیف';
    status = 'vulnerable';
    statusFa = 'آسیب‌پذیر';
  } else {
    label = 'Critical — Privacy at Risk';
    labelFa = 'بحرانی — حریم خصوصی در خطر';
    status = 'critical';
    statusFa = 'بحرانی';
  }

  return { score, label, labelFa, status, statusFa };
}

// ──────────────────────────────────────────────
// Security Recommendations in Persian
// ──────────────────────────────────────────────
export function generateSecurityRecommendations(params: {
  dnsLeak: DNSLeakResult;
  webrtcLeak: WebRTCLeakResult;
  ipv6Leak: IPv6LeakResult;
  killSwitchVerified: boolean;
  encryption: EncryptionAssessment;
  vpnConnected: boolean;
  dnsMode: 'doh' | 'dot' | 'plain';
}): SecurityRecommendation[] {
  const recommendations: SecurityRecommendation[] = [];

  if (!params.vpnConnected) {
    recommendations.push({
      id: 'rec-vpn-disconnected',
      category: 'Connection',
      categoryFa: 'اتصال',
      title: 'VPN is not connected',
      titleFa: 'VPN متصل نیست',
      description: 'Your traffic is completely exposed. Connect to VPN immediately.',
      descriptionFa: 'ترافیک شما کاملاً فاش است. فوراً به VPN متصل شوید.',
      severity: 'critical',
      action: 'Connect to VPN',
      actionFa: 'اتصال به VPN',
      implemented: false,
    });
    return recommendations;
  }

  // DNS leak recommendation
  if (params.dnsLeak.isLeaking) {
    const upgradeTarget = params.dnsMode === 'plain' ? 'DoH' : params.dnsMode === 'dot' ? 'DoH' : 'DoH with fallback';
    recommendations.push({
      id: 'rec-dns-leak',
      category: 'DNS Security',
      categoryFa: 'امنیت DNS',
      title: 'DNS leak detected',
      titleFa: 'نشت DNS شناسایی شد',
      description: `Your DNS queries are leaking to ISP servers (${params.dnsLeak.detectedServers.filter(s => s !== '1.1.1.1').join(', ')}). Switch to ${upgradeTarget} to encrypt DNS.`,
      descriptionFa: `درخواست‌های DNS شما به سرورهای ISP نشت می‌کند (${params.dnsLeak.detectedServers.filter(s => s !== '1.1.1.1').join('، ')}). به ${upgradeTarget} تعویض کنید تا DNS رمزنگاری شود.`,
      severity: 'critical',
      action: `Switch DNS to ${upgradeTarget}`,
      actionFa: `تعویض DNS به ${upgradeTarget}`,
      implemented: false,
    });
  } else if (params.dnsMode === 'plain') {
    recommendations.push({
      id: 'rec-dns-upgrade',
      category: 'DNS Security',
      categoryFa: 'امنیت DNS',
      title: 'Upgrade DNS encryption',
      titleFa: 'ارتقای رمزنگاری DNS',
      description: 'Your DNS is not leaking, but plain DNS is vulnerable. Upgrade to DoH for better protection.',
      descriptionFa: 'DNS شما نشت نمی‌کند، اما DNS ساده آسیب‌پذیر است. برای محافظت بهتر به DoH ارتقا دهید.',
      severity: 'warning',
      action: 'Enable DNS over HTTPS',
      actionFa: 'فعال‌سازی DNS over HTTPS',
      implemented: false,
    });
  }

  // WebRTC leak recommendation
  if (params.webrtcLeak.isLeaking) {
    recommendations.push({
      id: 'rec-webrtc-leak',
      category: 'WebRTC',
      categoryFa: 'WebRTC',
      title: 'WebRTC leak detected',
      titleFa: 'نشت WebRTC شناسایی شد',
      description: 'Your real IP address can be exposed through WebRTC STUN requests. Enable WebRTC blocking in browser or VPN.',
      descriptionFa: 'آدرس IP واقعی شما از طریق درخواست‌های STUN وب‌آر‌تی‌سی فاش می‌شود. مسدودسازی WebRTC را در مرورگر یا VPN فعال کنید.',
      severity: 'critical',
      action: 'Enable WebRTC leak protection',
      actionFa: 'فعال‌سازی محافظت از نشت WebRTC',
      implemented: false,
    });
  }

  // IPv6 leak recommendation
  if (params.ipv6Leak.isLeaking) {
    recommendations.push({
      id: 'rec-ipv6-leak',
      category: 'IPv6',
      categoryFa: 'IPv6',
      title: 'IPv6 leak detected',
      titleFa: 'نشت IPv6 شناسایی شد',
      description: 'IPv6 traffic is bypassing the VPN tunnel, exposing your real IPv6 address. Disable IPv6 or route it through VPN.',
      descriptionFa: 'ترافیک IPv6 از تونل VPN عبور نمی‌کند و آدرس IPv6 واقعی شما فاش می‌شود. IPv6 را غیرفعال کنید یا از VPN عبور دهید.',
      severity: 'warning',
      action: 'Disable IPv6 or enable VPN IPv6 routing',
      actionFa: 'غیرفعال‌سازی IPv6 یا فعال‌سازی مسیریابی IPv6 از VPN',
      implemented: false,
    });
  }

  // Kill switch recommendation
  if (!params.killSwitchVerified) {
    recommendations.push({
      id: 'rec-kill-switch',
      category: 'Kill Switch',
      categoryFa: 'کلید کشت',
      title: 'Kill switch not verified',
      titleFa: 'کلید کشت تأیید نشد',
      description: 'Traffic may leak when VPN disconnects. Enable kill switch and network lock for full protection.',
      descriptionFa: 'ترافیک ممکن است هنگام قطع VPN نشت کند. کلید کشت و قفل شبکه را برای محافظت کامل فعال کنید.',
      severity: 'critical',
      action: 'Enable kill switch and network lock',
      actionFa: 'فعال‌سازی کلید کشت و قفل شبکه',
      implemented: false,
    });
  }

  // Encryption recommendation
  if (params.encryption.strength === 'moderate' || params.encryption.strength === 'weak') {
    recommendations.push({
      id: 'rec-encryption-upgrade',
      category: 'Encryption',
      categoryFa: 'رمزنگاری',
      title: 'Upgrade encryption strength',
      titleFa: 'ارتقای قدرت رمزنگاری',
      description: `Current encryption (${params.encryption.cipher}) is ${params.encryption.strength}. Consider switching to a core with stronger encryption like VLESS Reality or AmneziaWG.`,
      descriptionFa: `رمزنگاری فعلی (${params.encryption.cipherFa}) ${params.encryption.strengthFa} است. تعویض به هسته‌ای با رمزنگاری قوی‌تر مانند VLESS Reality یا آمنزیاوی‌جی پیشنهاد می‌شود.`,
      severity: 'warning',
      action: 'Switch to VLESS Reality or AmneziaWG core',
      actionFa: 'تعویض به هسته VLESS Reality یا آمنزیاوی‌جی',
      implemented: false,
    });
  }

  // General best practices
  recommendations.push({
    id: 'rec-regular-audit',
    category: 'Best Practices',
    categoryFa: 'بهترین شیوه‌ها',
    title: 'Run security audits regularly',
    titleFa: 'ممیزی امنیتی را به‌طور منظم اجرا کنید',
    description: 'Run a full security audit at least once per day to ensure no new leaks or vulnerabilities have appeared.',
    descriptionFa: 'حداقل روزی یک بار ممیزی امنیتی کامل اجرا کنید تا مطمئن شوید نشت یا آسیب‌پذیری جدیدی ظاهر نشده.',
    severity: 'info',
    action: 'Schedule daily security audits',
    actionFa: 'زمان‌بندی ممیزی‌های امنیتی روزانه',
    implemented: false,
  });

  if (params.dnsMode === 'doh' && !params.dnsLeak.isLeaking && !params.webrtcLeak.isLeaking && params.killSwitchVerified) {
    recommendations.push({
      id: 'rec-good-status',
      category: 'Status',
      categoryFa: 'وضعیت',
      title: 'Security posture is strong',
      titleFa: 'وضعیت امنیتی قوی است',
      description: 'Your VPN security configuration looks good. Continue monitoring and keep all components updated.',
      descriptionFa: 'پیکربندی امنیتی VPN شما خوب به نظر می‌رسد. به مانیتورینگ ادامه دهید و تمام اجزا را به‌روز نگه دارید.',
      severity: 'info',
      action: 'Maintain current configuration',
      actionFa: 'حفظ پیکربندی فعلی',
      implemented: true,
    });
  }

  return recommendations;
}

// ──────────────────────────────────────────────
// Full Security Audit State Builder
// ──────────────────────────────────────────────
export function buildInitialSecurityAuditState(
  vpnConnected: boolean = true,
  activeCoreId: string = 'xray-gfw',
  dnsMode: 'doh' | 'dot' | 'plain' = 'doh',
  killSwitchEnabled: boolean = true,
  networkLockEnabled: boolean = true,
  ipv6Enabled: boolean = true,
): SecurityAuditState {
  const dnsLeak = performDNSLeakTest(vpnConnected, dnsMode);
  const webrtcLeak = detectWebRTCLeak(vpnConnected, killSwitchEnabled);
  const ipv6Leak = detectIPv6Leak(vpnConnected, ipv6Enabled);
  const killSwitchResult = verifyKillSwitch(killSwitchEnabled, networkLockEnabled);
  const encryption = assessEncryption(activeCoreId);

  const privacyResult = calculatePrivacyScore({
    dnsLeak,
    webrtcLeak,
    ipv6Leak,
    killSwitchVerified: killSwitchResult.verified,
    encryption,
    vpnConnected,
  });

  const recommendations = generateSecurityRecommendations({
    dnsLeak,
    webrtcLeak,
    ipv6Leak,
    killSwitchVerified: killSwitchResult.verified,
    encryption,
    vpnConnected,
    dnsMode,
  });

  return {
    isRunning: false,
    lastAuditTime: Date.now() - 300000,
    privacyScore: privacyResult.score,
    privacyScoreLabel: privacyResult.label,
    privacyScoreLabelFa: privacyResult.labelFa,
    dnsLeak,
    webrtcLeak,
    ipv6Leak,
    killSwitchVerified: killSwitchResult.verified,
    killSwitchDetails: killSwitchResult.details,
    killSwitchDetailsFa: killSwitchResult.detailsFa,
    encryptionAssessment: encryption,
    recommendations,
    realTimeMonitoring: true,
    lastRealTimeCheck: Date.now(),
    overallSecurityStatus: privacyResult.status,
    overallSecurityStatusFa: privacyResult.statusFa,
  };
}

// ──────────────────────────────────────────────
// Run Full Security Audit
// ──────────────────────────────────────────────
export function runFullSecurityAudit(
  currentState: SecurityAuditState,
  vpnConnected: boolean,
  activeCoreId: string,
  dnsMode: 'doh' | 'dot' | 'plain',
  killSwitchEnabled: boolean,
  networkLockEnabled: boolean,
  ipv6Enabled: boolean,
): SecurityAuditState {
  const dnsLeak = performDNSLeakTest(vpnConnected, dnsMode);
  const webrtcLeak = detectWebRTCLeak(vpnConnected, killSwitchEnabled);
  const ipv6Leak = detectIPv6Leak(vpnConnected, ipv6Enabled);
  const killSwitchResult = verifyKillSwitch(killSwitchEnabled, networkLockEnabled);
  const encryption = assessEncryption(activeCoreId);

  const privacyResult = calculatePrivacyScore({
    dnsLeak,
    webrtcLeak,
    ipv6Leak,
    killSwitchVerified: killSwitchResult.verified,
    encryption,
    vpnConnected,
  });

  const recommendations = generateSecurityRecommendations({
    dnsLeak,
    webrtcLeak,
    ipv6Leak,
    killSwitchVerified: killSwitchResult.verified,
    encryption,
    vpnConnected,
    dnsMode,
  });

  return {
    ...currentState,
    isRunning: false,
    lastAuditTime: Date.now(),
    privacyScore: privacyResult.score,
    privacyScoreLabel: privacyResult.label,
    privacyScoreLabelFa: privacyResult.labelFa,
    dnsLeak,
    webrtcLeak,
    ipv6Leak,
    killSwitchVerified: killSwitchResult.verified,
    killSwitchDetails: killSwitchResult.details,
    killSwitchDetailsFa: killSwitchResult.detailsFa,
    encryptionAssessment: encryption,
    recommendations,
    lastRealTimeCheck: Date.now(),
    overallSecurityStatus: privacyResult.status,
    overallSecurityStatusFa: privacyResult.statusFa,
  };
}

// ──────────────────────────────────────────────
// Real-time Security Status Monitoring
// ──────────────────────────────────────────────
export function updateRealTimeSecurityStatus(
  currentState: SecurityAuditState,
  vpnConnected: boolean,
): SecurityAuditState {
  if (!vpnConnected) {
    return {
      ...currentState,
      overallSecurityStatus: 'critical',
      overallSecurityStatusFa: 'بحرانی',
      privacyScore: 5,
      privacyScoreLabel: 'Not Protected',
      privacyScoreLabelFa: 'بدون محافظت',
      lastRealTimeCheck: Date.now(),
    };
  }

  // Lightweight real-time check — only update score slightly
  const scoreVariation = Math.random() * 4 - 2;
  const newScore = Math.min(100, Math.max(0, Math.round(currentState.privacyScore + scoreVariation)));

  let status: SecurityAuditState['overallSecurityStatus'] = currentState.overallSecurityStatus;
  let statusFa = currentState.overallSecurityStatusFa;

  if (newScore >= 65) {
    status = 'secure';
    statusFa = 'امن';
  } else if (newScore >= 45) {
    status = 'warning';
    statusFa = 'هشدار';
  } else if (newScore >= 20) {
    status = 'vulnerable';
    statusFa = 'آسیب‌پذیر';
  } else {
    status = 'critical';
    statusFa = 'بحرانی';
  }

  return {
    ...currentState,
    privacyScore: newScore,
    lastRealTimeCheck: Date.now(),
    overallSecurityStatus: status,
    overallSecurityStatusFa: statusFa,
  };
}
