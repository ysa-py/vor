use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Instant;

pub struct LocalDnsResolver {
    iranian_dns: Vec<String>,
    cache: HashMap<String, (Vec<IpAddr>, Instant, u32)>,
}

impl LocalDnsResolver {
    pub fn new() -> Self {
        Self {
            iranian_dns: vec![
                "10.202.10.10".into(),
                "10.202.10.11".into(),
                "78.157.42.100".into(),
            ],
            cache: HashMap::new(),
        }
    }
    pub async fn resolve(&mut self, domain: &str) -> Option<Vec<IpAddr>> {
        if let Some((ips, cached_at, ttl)) = self.cache.get(domain) {
            if cached_at.elapsed().as_secs() < *ttl as u64 {
                return Some(ips.clone());
            }
        }
        None
    }
    pub fn cache_resolution(&mut self, domain: &str, ips: Vec<IpAddr>, ttl: u32) {
        self.cache
            .insert(domain.to_string(), (ips, Instant::now(), ttl));
    }
}
