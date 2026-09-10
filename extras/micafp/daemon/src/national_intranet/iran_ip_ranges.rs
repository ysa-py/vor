use ipnet::IpNet;
use std::net::IpAddr;

pub struct IranIpRanges {
    ranges: Vec<IpNet>,
}

impl IranIpRanges {
    pub fn from_config(path: &str) -> Self {
        let mut ranges = Vec::new();
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(arr) = json.get("ranges").and_then(|v| v.as_array()) {
                    for entry in arr {
                        if let Some(cidr) = entry.get("cidr").and_then(|v| v.as_str()) {
                            if let Ok(net) = cidr.parse::<IpNet>() {
                                ranges.push(net);
                            }
                        }
                    }
                }
            }
        }
        if ranges.is_empty() {
            ranges = Self::hardcoded();
        }
        Self { ranges }
    }
    pub fn hardcoded() -> Vec<IpNet> {
        [
            "5.160.0.0/12",
            "31.56.0.0/14",
            "37.32.0.0/14",
            "46.32.0.0/12",
            "62.60.0.0/14",
            "77.36.0.0/14",
            "78.38.0.0/14",
            "80.191.0.0/14",
            "84.47.0.0/14",
            "85.9.0.0/14",
            "86.57.0.0/14",
            "91.92.0.0/14",
            "92.50.0.0/14",
            "93.110.0.0/14",
            "94.101.0.0/14",
            "95.80.0.0/14",
            "109.72.0.0/14",
            "151.232.0.0/14",
            "159.20.0.0/14",
            "164.215.0.0/14",
            "176.65.0.0/14",
            "178.22.0.0/14",
            "185.2.0.0/14",
            "188.121.0.0/14",
            "194.225.0.0/14",
            "213.176.0.0/14",
            "217.11.0.0/14",
            "217.146.0.0/14",
            "5.106.0.0/14",
            "5.112.0.0/12",
            "217.218.0.0/14",
            "78.157.0.0/14",
            "5.208.0.0/12",
            "5.224.0.0/12",
            "37.128.0.0/14",
        ]
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect()
    }
    pub fn is_iranian(&self, ip: IpAddr) -> bool {
        self.ranges.iter().any(|r| r.contains(&ip))
    }
}
