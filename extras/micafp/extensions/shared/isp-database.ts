/**
 * ISP Database — Iranian ISPs with throttling profiles
 */

export interface ISPEntry {
  asn: number;
  name: string;
  nameFa: string;
  type: 'mobile' | 'fixed' | 'backbone' | 'government';
  throttlingProfile: string;
  knownBlockingPatterns: string[];
  estimatedUsers: number;
}

export const ISP_DATABASE: ISPEntry[] = [
  {
    asn: 12880,
    name: 'DCI',
    nameFa: 'شرکت ارتباطات زیرساخت',
    type: 'backbone',
    throttlingProfile: 'aggressive',
    knownBlockingPatterns: ['fava_tls_rst', 'http_403', 'dns_poison', 'sni_filter'],
    estimatedUsers: 0, // backbone, not end-user
  },
  {
    asn: 16322,
    name: 'Pars Online',
    nameFa: 'پارس آنلاین',
    type: 'fixed',
    throttlingProfile: 'moderate',
    knownBlockingPatterns: ['fava_tls_rst', 'dns_poison'],
    estimatedUsers: 2_000_000,
  },
  {
    asn: 24631,
    name: 'MCI',
    nameFa: 'همراه اول',
    type: 'mobile',
    throttlingProfile: 'aggressive',
    knownBlockingPatterns: ['fava_tls_rst', 'http_403', 'dns_poison', 'sni_filter', 'protocol_detect'],
    estimatedUsers: 70_000_000,
  },
  {
    asn: 25184,
    name: 'Irancell',
    nameFa: 'ایرانسل',
    type: 'mobile',
    throttlingProfile: 'aggressive',
    knownBlockingPatterns: ['fava_tls_rst', 'dns_poison', 'sni_filter'],
    estimatedUsers: 45_000_000,
  },
  {
    asn: 31549,
    name: 'Arian Telecommunication',
    nameFa: 'آریان تلکام',
    type: 'fixed',
    throttlingProfile: 'moderate',
    knownBlockingPatterns: ['fava_tls_rst', 'dns_poison'],
    estimatedUsers: 500_000,
  },
  {
    asn: 34918,
    name: 'Shatel',
    nameFa: 'شاتل',
    type: 'fixed',
    throttlingProfile: 'moderate',
    knownBlockingPatterns: ['fava_tls_rst', 'dns_poison'],
    estimatedUsers: 3_000_000,
  },
  {
    asn: 39074,
    name: 'Rightel',
    nameFa: 'رایتل',
    type: 'mobile',
    throttlingProfile: 'moderate',
    knownBlockingPatterns: ['fava_tls_rst', 'dns_poison'],
    estimatedUsers: 5_000_000,
  },
  {
    asn: 41689,
    name: 'TIC',
    nameFa: 'شرکت فناوری اطلاعات ارتباطات',
    type: 'government',
    throttlingProfile: 'aggressive',
    knownBlockingPatterns: ['fava_tls_rst', 'http_403', 'dns_poison', 'sni_filter', 'protocol_detect'],
    estimatedUsers: 1_000_000,
  },
  {
    asn: 44244,
    name: 'Iran Post',
    nameFa: 'پست ایران',
    type: 'government',
    throttlingProfile: 'moderate',
    knownBlockingPatterns: ['fava_tls_rst', 'dns_poison'],
    estimatedUsers: 200_000,
  },
  {
    asn: 48434,
    name: 'Sabanet',
    nameFa: 'صبانت',
    type: 'fixed',
    throttlingProfile: 'moderate',
    knownBlockingPatterns: ['fava_tls_rst', 'dns_poison'],
    estimatedUsers: 800_000,
  },
  {
    asn: 56402,
    name: 'Asiatech',
    nameFa: 'آسیاتک',
    type: 'fixed',
    throttlingProfile: 'moderate',
    knownBlockingPatterns: ['fava_tls_rst', 'dns_poison'],
    estimatedUsers: 1_500_000,
  },
  {
    asn: 197207,
    name: 'Pishgaman',
    nameFa: 'پیشگامان',
    type: 'fixed',
    throttlingProfile: 'light',
    knownBlockingPatterns: ['dns_poison'],
    estimatedUsers: 1_000_000,
  },
  // Additional ISPs
  {
    asn: 42337,
    name: 'Respina',
    nameFa: 'رسپینا',
    type: 'fixed',
    throttlingProfile: 'moderate',
    knownBlockingPatterns: ['fava_tls_rst', 'dns_poison'],
    estimatedUsers: 600_000,
  },
  {
    asn: 49581,
    name: 'Farabord',
    nameFa: 'فرابرد',
    type: 'fixed',
    throttlingProfile: 'light',
    knownBlockingPatterns: ['dns_poison'],
    estimatedUsers: 300_000,
  },
  {
    asn: 50752,
    name: 'Parsun',
    nameFa: 'پارسان',
    type: 'fixed',
    throttlingProfile: 'light',
    knownBlockingPatterns: ['dns_poison'],
    estimatedUsers: 100_000,
  },
  {
    asn: 51035,
    name: 'Zitel',
    nameFa: 'زیتل',
    type: 'fixed',
    throttlingProfile: 'moderate',
    knownBlockingPatterns: ['fava_tls_rst', 'dns_poison'],
    estimatedUsers: 200_000,
  },
];

/**
 * Get ISP entry by ASN
 */
export function getISPByASN(asn: number): ISPEntry | undefined {
  return ISP_DATABASE.find((e) => e.asn === asn);
}

/**
 * Get throttling profile for an ASN
 */
export function getThrottlingProfile(asn: number): string {
  return getISPByASN(asn)?.throttlingProfile ?? 'default';
}

/**
 * Get all known blocking patterns for an ASN
 */
export function getBlockingPatterns(asn: number): string[] {
  return getISPByASN(asn)?.knownBlockingPatterns ?? [];
}
