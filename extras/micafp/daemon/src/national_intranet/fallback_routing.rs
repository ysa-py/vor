#[derive(Debug, Clone)]
pub enum FallbackTransport {
    DohTunnel,
    SnowflakeWebRtc,
    PsiphonCdnFronting,
    MqttTunnel,
    IcmpTunnel,
}

pub struct FallbackRouting {
    active_transport: Option<FallbackTransport>,
}

impl FallbackRouting {
    pub fn new() -> Self {
        Self {
            active_transport: None,
        }
    }
    pub async fn activate_fallback_chain(&mut self) -> anyhow::Result<FallbackTransport> {
        let chain = [
            FallbackTransport::DohTunnel,
            FallbackTransport::SnowflakeWebRtc,
            FallbackTransport::PsiphonCdnFronting,
            FallbackTransport::MqttTunnel,
            FallbackTransport::IcmpTunnel,
        ];
        for transport in &chain {
            tracing::info!("Trying fallback: {:?}", transport);
            self.active_transport = Some(transport.clone());
            return Ok(transport.clone());
        }
        Err(anyhow::anyhow!("All fallbacks failed"))
    }
    pub fn active(&self) -> Option<&FallbackTransport> {
        self.active_transport.as_ref()
    }
}
