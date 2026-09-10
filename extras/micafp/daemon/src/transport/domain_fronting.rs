use anyhow::Result;
use rand::Rng;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CdnProvider {
    AlibabaCloud,
    TencentCloud,
    JdCloud,
    AmazonCloudFront,
    Fastly,
}

impl CdnProvider {
    pub fn sni_domain(&self) -> &str {
        match self {
            Self::AlibabaCloud => "alibaba.com",
            Self::TencentCloud => "cloud.tencent.com",
            Self::JdCloud => "jdcloud.com",
            Self::AmazonCloudFront => "d1.cloudfront.net",
            Self::Fastly => "global.fastly.net",
        }
    }

    pub fn works_in_iran(&self) -> bool {
        matches!(
            self,
            Self::AlibabaCloud | Self::TencentCloud | Self::JdCloud
        )
    }

    pub fn all_providers() -> Vec<CdnProvider> {
        vec![
            Self::AlibabaCloud,
            Self::TencentCloud,
            Self::JdCloud,
            Self::AmazonCloudFront,
            Self::Fastly,
        ]
    }
}

pub struct DomainFrontingTransport {
    weights: HashMap<CdnProvider, f64>,
}

impl DomainFrontingTransport {
    pub fn new() -> Self {
        let mut weights = HashMap::new();
        for p in CdnProvider::all_providers() {
            weights.insert(p, if p.works_in_iran() { 3.0 } else { 1.0 });
        }
        Self { weights }
    }

    pub fn select_provider(&self, in_iran: bool) -> CdnProvider {
        let providers: Vec<CdnProvider> = CdnProvider::all_providers();
        if in_iran {
            let chinese: Vec<_> = providers.iter().filter(|p| p.works_in_iran()).collect();
            if !chinese.is_empty() {
                return *chinese[rand::thread_rng().gen_range(0..chinese.len())];
            }
        }
        let total: f64 = self.weights.values().sum();
        let mut r = rand::thread_rng().gen::<f64>() * total;
        for &p in &providers {
            r -= self.weights[&p];
            if r <= 0.0 {
                return p;
            }
        }
        providers[0]
    }

    pub async fn fronted_connect(&self, target_host: &str, in_iran: bool) -> Result<()> {
        let provider = self.select_provider(in_iran);
        tracing::info!(
            "Domain fronting: SNI={}, Host={} (Iran={})",
            provider.sni_domain(),
            target_host,
            in_iran
        );
        Ok(())
    }
}
