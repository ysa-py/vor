use super::chinese_cdn::ChineseCdnTransport;
use super::domain_fronting::{CdnProvider, DomainFrontingTransport};
use anyhow::Result;

pub struct CdnTunnel {
    fronting: DomainFrontingTransport,
    chinese_cdn: ChineseCdnTransport,
    in_iran: bool,
}

impl CdnTunnel {
    pub fn new(in_iran: bool) -> Self {
        Self {
            fronting: DomainFrontingTransport::new(),
            chinese_cdn: ChineseCdnTransport::new(),
            in_iran,
        }
    }

    pub async fn establish_tunnel(&mut self, target_host: &str) -> Result<()> {
        if self.in_iran {
            tracing::info!("In Iran: using Chinese CDN tunnel (Alibaba/Tencent/JD/Qiniu)");
            let provider = self.chinese_cdn.connect_best_provider().await?;
            tracing::info!("Chinese CDN tunnel established via {:?}", provider);
        } else {
            tracing::info!("Outside Iran: using CloudFront/Fastly CDN tunnel");
            self.fronting.fronted_connect(target_host, false).await?;
        }
        Ok(())
    }

    pub async fn tunnel_data(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        if self.in_iran && self.chinese_cdn.is_connected() {
            self.chinese_cdn.send_via_cdn(data).await?;
            self.chinese_cdn.recv_via_cdn(data).await
        } else {
            Ok(data.to_vec())
        }
    }
}
