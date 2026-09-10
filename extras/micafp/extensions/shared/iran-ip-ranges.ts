/**
 * Iranian IP Ranges — Complete CIDR array
 * Covers major ASNs: AS12880, AS16322, AS24631, AS25184, AS31549,
 * AS34918, AS39074, AS41689, AS44244, AS48434, AS56402, AS197207
 */

export interface IPRangeEntry {
  cidr: string;
  asn: number;
  name: string;
  type: 'datacenter' | 'residential' | 'mobile' | 'backbone' | 'government';
}

export const IRAN_IP_RANGES: IPRangeEntry[] = [
  // ─── AS12880 — DCI (Data Communication Iran — Backbone) ───
  { cidr: '5.106.0.0/16', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '5.107.0.0/16', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '5.108.0.0/14', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '5.112.0.0/12', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '5.128.0.0/14', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '31.7.64.0/18', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '31.56.0.0/14', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '46.32.0.0/19', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '46.36.96.0/19', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '62.60.128.0/17', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '62.102.64.0/18', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '77.36.128.0/17', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '78.38.0.0/15', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '78.110.112.0/20', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '78.154.32.0/19', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '79.127.0.0/17', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '79.175.128.0/17', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '80.191.0.0/16', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '81.12.0.0/17', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '81.28.32.0/19', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '81.31.160.0/19', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '82.99.192.0/18', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '83.147.192.0/18', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '84.47.192.0/18', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '85.9.0.0/18', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '85.15.0.0/18', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '85.133.128.0/17', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '85.185.0.0/16', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '86.55.0.0/16', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '86.57.0.0/16', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '87.107.0.0/16', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '87.247.160.0/19', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '89.144.128.0/18', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '89.165.0.0/17', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '91.92.96.0/19', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '91.98.0.0/15', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '91.186.192.0/18', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '92.42.48.0/21', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '92.61.176.0/20', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '92.114.0.0/16', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '93.110.0.0/16', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '93.126.0.0/18', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '94.74.128.0/18', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '94.101.128.0/20', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '94.139.160.0/19', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '95.38.0.0/16', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '95.64.0.0/18', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '95.80.128.0/18', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '95.81.96.0/19', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '95.130.56.0/21', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '151.232.0.0/14', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '151.238.0.0/15', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '151.240.0.0/13', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.2.12.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.3.124.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.4.0.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.5.156.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.8.172.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.10.72.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.11.68.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.12.60.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.13.228.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.14.80.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.14.160.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.15.28.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.16.232.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.18.212.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.20.60.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.21.68.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.22.28.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.23.128.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.24.136.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.24.252.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.25.172.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.26.32.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.29.220.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.30.4.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.30.72.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.34.160.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.36.160.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.37.48.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.39.136.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.40.224.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.41.0.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.42.24.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.42.212.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.44.36.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.46.248.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.49.84.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.49.96.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.50.36.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.51.200.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.53.140.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.55.224.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.56.92.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.57.132.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '185.59.16.0/22', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '217.11.16.0/20', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '217.25.48.0/20', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '217.146.208.0/20', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '217.170.240.0/20', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '217.172.96.0/19', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '217.174.16.0/20', asn: 12880, name: 'DCI', type: 'backbone' },
  { cidr: '217.218.0.0/15', asn: 12880, name: 'DCI', type: 'backbone' },

  // ─── AS16322 — Pars Online ───
  { cidr: '5.250.0.0/17', asn: 16322, name: 'ParsOnline', type: 'residential' },
  { cidr: '31.47.32.0/19', asn: 16322, name: 'ParsOnline', type: 'residential' },
  { cidr: '46.34.96.0/19', asn: 16322, name: 'ParsOnline', type: 'residential' },
  { cidr: '46.224.0.0/17', asn: 16322, name: 'ParsOnline', type: 'residential' },
  { cidr: '91.99.0.0/16', asn: 16322, name: 'ParsOnline', type: 'residential' },
  { cidr: '94.182.0.0/15', asn: 16322, name: 'ParsOnline', type: 'residential' },
  { cidr: '185.88.176.0/22', asn: 16322, name: 'ParsOnline', type: 'residential' },
  { cidr: '185.143.232.0/22', asn: 16322, name: 'ParsOnline', type: 'residential' },
  { cidr: '185.149.76.0/22', asn: 16322, name: 'ParsOnline', type: 'residential' },
  { cidr: '185.162.232.0/22', asn: 16322, name: 'ParsOnline', type: 'residential' },
  { cidr: '188.34.0.0/16', asn: 16322, name: 'ParsOnline', type: 'residential' },
  { cidr: '188.121.96.0/19', asn: 16322, name: 'ParsOnline', type: 'residential' },
  { cidr: '195.146.32.0/19', asn: 16322, name: 'ParsOnline', type: 'residential' },
  { cidr: '212.16.64.0/19', asn: 16322, name: 'ParsOnline', type: 'residential' },
  { cidr: '217.24.144.0/20', asn: 16322, name: 'ParsOnline', type: 'residential' },

  // ─── AS24631 — MCI (Mobile Communication Company of Iran) ───
  { cidr: '5.104.0.0/17', asn: 24631, name: 'MCI', type: 'mobile' },
  { cidr: '5.117.0.0/16', asn: 24631, name: 'MCI', type: 'mobile' },
  { cidr: '5.134.128.0/18', asn: 24631, name: 'MCI', type: 'mobile' },
  { cidr: '5.200.0.0/14', asn: 24631, name: 'MCI', type: 'mobile' },
  { cidr: '5.208.0.0/13', asn: 24631, name: 'MCI', type: 'mobile' },
  { cidr: '31.56.0.0/14', asn: 24631, name: 'MCI', type: 'mobile' },
  { cidr: '46.209.0.0/16', asn: 24631, name: 'MCI', type: 'mobile' },
  { cidr: '46.224.128.0/17', asn: 24631, name: 'MCI', type: 'mobile' },
  { cidr: '46.245.0.0/17', asn: 24631, name: 'MCI', type: 'mobile' },
  { cidr: '89.165.128.0/17', asn: 24631, name: 'MCI', type: 'mobile' },
  { cidr: '151.236.0.0/14', asn: 24631, name: 'MCI', type: 'mobile' },
  { cidr: '151.240.0.0/14', asn: 24631, name: 'MCI', type: 'mobile' },
  { cidr: '178.131.0.0/16', asn: 24631, name: 'MCI', type: 'mobile' },
  { cidr: '188.212.0.0/15', asn: 24631, name: 'MCI', type: 'mobile' },
  { cidr: '185.129.80.0/22', asn: 24631, name: 'MCI', type: 'mobile' },
  { cidr: '185.131.60.0/22', asn: 24631, name: 'MCI', type: 'mobile' },
  { cidr: '185.134.28.0/22', asn: 24631, name: 'MCI', type: 'mobile' },
  { cidr: '185.138.228.0/22', asn: 24631, name: 'MCI', type: 'mobile' },
  { cidr: '185.143.136.0/22', asn: 24631, name: 'MCI', type: 'mobile' },
  { cidr: '185.150.28.0/22', asn: 24631, name: 'MCI', type: 'mobile' },

  // ─── AS25184 — Irancell ───
  { cidr: '5.250.128.0/17', asn: 25184, name: 'Irancell', type: 'mobile' },
  { cidr: '2.144.0.0/12', asn: 25184, name: 'Irancell', type: 'mobile' },
  { cidr: '2.176.0.0/12', asn: 25184, name: 'Irancell', type: 'mobile' },
  { cidr: '46.32.0.0/19', asn: 25184, name: 'Irancell', type: 'mobile' },
  { cidr: '46.148.32.0/19', asn: 25184, name: 'Irancell', type: 'mobile' },
  { cidr: '77.104.64.0/18', asn: 25184, name: 'Irancell', type: 'mobile' },
  { cidr: '91.186.192.0/18', asn: 25184, name: 'Irancell', type: 'mobile' },
  { cidr: '185.125.0.0/18', asn: 25184, name: 'Irancell', type: 'mobile' },
  { cidr: '185.128.80.0/22', asn: 25184, name: 'Irancell', type: 'mobile' },
  { cidr: '185.136.228.0/22', asn: 25184, name: 'Irancell', type: 'mobile' },
  { cidr: '185.148.152.0/22', asn: 25184, name: 'Irancell', type: 'mobile' },
  { cidr: '185.153.152.0/22', asn: 25184, name: 'Irancell', type: 'mobile' },
  { cidr: '213.195.0.0/17', asn: 25184, name: 'Irancell', type: 'mobile' },

  // ─── AS31549 — Arian Telecommunication ───
  { cidr: '5.160.0.0/15', asn: 31549, name: 'ArianTel', type: 'residential' },
  { cidr: '185.44.96.0/22', asn: 31549, name: 'ArianTel', type: 'residential' },
  { cidr: '185.55.224.0/22', asn: 31549, name: 'ArianTel', type: 'residential' },
  { cidr: '185.56.92.0/22', asn: 31549, name: 'ArianTel', type: 'residential' },

  // ─── AS34918 — Shatel ───
  { cidr: '5.134.0.0/18', asn: 34918, name: 'Shatel', type: 'residential' },
  { cidr: '5.202.0.0/16', asn: 34918, name: 'Shatel', type: 'residential' },
  { cidr: '31.24.32.0/19', asn: 34918, name: 'Shatel', type: 'residential' },
  { cidr: '37.32.0.0/19', asn: 34918, name: 'Shatel', type: 'residential' },
  { cidr: '37.98.0.0/17', asn: 34918, name: 'Shatel', type: 'residential' },
  { cidr: '77.81.192.0/18', asn: 34918, name: 'Shatel', type: 'residential' },
  { cidr: '78.109.192.0/20', asn: 34918, name: 'Shatel', type: 'residential' },
  { cidr: '185.4.0.0/22', asn: 34918, name: 'Shatel', type: 'residential' },
  { cidr: '185.10.72.0/22', asn: 34918, name: 'Shatel', type: 'residential' },
  { cidr: '185.20.60.0/22', asn: 34918, name: 'Shatel', type: 'residential' },
  { cidr: '185.24.136.0/22', asn: 34918, name: 'Shatel', type: 'residential' },
  { cidr: '185.25.172.0/22', asn: 34918, name: 'Shatel', type: 'residential' },
  { cidr: '185.29.220.0/22', asn: 34918, name: 'Shatel', type: 'residential' },
  { cidr: '185.53.140.0/22', asn: 34918, name: 'Shatel', type: 'residential' },
  { cidr: '188.213.160.0/19', asn: 34918, name: 'Shatel', type: 'residential' },
  { cidr: '212.33.192.0/19', asn: 34918, name: 'Shatel', type: 'residential' },

  // ─── AS39074 — Rightel ───
  { cidr: '5.112.0.0/14', asn: 39074, name: 'Rightel', type: 'mobile' },
  { cidr: '185.248.0.0/22', asn: 39074, name: 'Rightel', type: 'mobile' },
  { cidr: '188.208.0.0/16', asn: 39074, name: 'Rightel', type: 'mobile' },
  { cidr: '188.209.0.0/16', asn: 39074, name: 'Rightel', type: 'mobile' },

  // ─── AS41689 — TIC (Information Technology Company) ───
  { cidr: '78.38.0.0/15', asn: 41689, name: 'TIC', type: 'government' },
  { cidr: '80.191.0.0/16', asn: 41689, name: 'TIC', type: 'government' },
  { cidr: '85.185.0.0/16', asn: 41689, name: 'TIC', type: 'government' },
  { cidr: '86.57.0.0/16', asn: 41689, name: 'TIC', type: 'government' },
  { cidr: '217.218.0.0/15', asn: 41689, name: 'TIC', type: 'government' },

  // ─── AS44244 — Iran Post ───
  { cidr: '185.21.68.0/22', asn: 44244, name: 'IranPost', type: 'government' },
  { cidr: '185.37.48.0/22', asn: 44244, name: 'IranPost', type: 'government' },
  { cidr: '185.42.24.0/22', asn: 44244, name: 'IranPost', type: 'government' },
  { cidr: '185.49.84.0/22', asn: 44244, name: 'IranPost', type: 'government' },

  // ─── AS48434 — Sabanet ───
  { cidr: '5.215.0.0/16', asn: 48434, name: 'Sabanet', type: 'residential' },
  { cidr: '5.216.0.0/15', asn: 48434, name: 'Sabanet', type: 'residential' },
  { cidr: '185.8.172.0/22', asn: 48434, name: 'Sabanet', type: 'residential' },
  { cidr: '185.12.60.0/22', asn: 48434, name: 'Sabanet', type: 'residential' },
  { cidr: '185.14.80.0/22', asn: 48434, name: 'Sabanet', type: 'residential' },
  { cidr: '185.14.160.0/22', asn: 48434, name: 'Sabanet', type: 'residential' },
  { cidr: '185.15.28.0/22', asn: 48434, name: 'Sabanet', type: 'residential' },
  { cidr: '185.16.232.0/22', asn: 48434, name: 'Sabanet', type: 'residential' },
  { cidr: '185.22.28.0/22', asn: 48434, name: 'Sabanet', type: 'residential' },
  { cidr: '185.23.128.0/22', asn: 48434, name: 'Sabanet', type: 'residential' },

  // ─── AS56402 — Asiatech ───
  { cidr: '5.113.0.0/16', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '5.114.0.0/16', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '31.7.0.0/18', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '37.156.0.0/16', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '77.81.0.0/18', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.2.12.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.3.124.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.5.156.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.11.68.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.13.228.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.18.212.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.21.68.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.24.252.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.26.32.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.30.4.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.30.72.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.34.160.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.36.160.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.39.136.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.40.224.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.41.0.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.42.212.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.44.36.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.46.248.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.49.96.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.50.36.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.51.200.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.55.224.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.57.132.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },
  { cidr: '185.59.16.0/22', asn: 56402, name: 'Asiatech', type: 'residential' },

  // ─── AS197207 — Pishgaman ───
  { cidr: '5.126.0.0/16', asn: 197207, name: 'Pishgaman', type: 'residential' },
  { cidr: '5.127.0.0/16', asn: 197207, name: 'Pishgaman', type: 'residential' },
  { cidr: '37.130.128.0/17', asn: 197207, name: 'Pishgaman', type: 'residential' },
  { cidr: '37.191.0.0/17', asn: 197207, name: 'Pishgaman', type: 'residential' },
  { cidr: '77.104.0.0/18', asn: 197207, name: 'Pishgaman', type: 'residential' },
  { cidr: '185.10.72.0/22', asn: 197207, name: 'Pishgaman', type: 'residential' },
  { cidr: '185.23.128.0/22', asn: 197207, name: 'Pishgaman', type: 'residential' },
  { cidr: '185.29.220.0/22', asn: 197207, name: 'Pishgaman', type: 'residential' },
  { cidr: '185.36.160.0/22', asn: 197207, name: 'Pishgaman', type: 'residential' },
  { cidr: '185.41.0.0/22', asn: 197207, name: 'Pishgaman', type: 'residential' },
  { cidr: '185.44.36.0/22', asn: 197207, name: 'Pishgaman', type: 'residential' },
];

/**
 * Check if an IP address falls within any Iranian IP range
 */
export function isIranianIP(ip: string): boolean {
  const ipInt = ipToInt(ip);
  if (ipInt === 0) return false;

  for (const entry of IRAN_IP_RANGES) {
    if (isInSubnet(ipInt, entry.cidr)) return true;
  }
  return false;
}

function ipToInt(ip: string): number {
  const parts = ip.split('.');
  if (parts.length !== 4) return 0;
  return (
    ((parseInt(parts[0], 10) << 24) |
      (parseInt(parts[1], 10) << 16) |
      (parseInt(parts[2], 10) << 8) |
      parseInt(parts[3], 10)) >>>
    0
  );
}

function isInSubnet(ipInt: number, cidr: string): boolean {
  const [subnet, maskStr] = cidr.split('/');
  const mask = parseInt(maskStr, 10);
  const subnetInt = ipToInt(subnet);
  const maskInt = mask === 0 ? 0 : (~0 << (32 - mask)) >>> 0;
  return (ipInt & maskInt) === (subnetInt & maskInt);
}

/**
 * Get all CIDR ranges for a specific ASN
 */
export function getRangesByASN(asn: number): string[] {
  return IRAN_IP_RANGES.filter((e) => e.asn === asn).map((e) => e.cidr);
}
