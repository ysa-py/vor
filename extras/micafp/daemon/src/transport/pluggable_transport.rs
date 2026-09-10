use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait PluggableTransport: Send + Sync {
    async fn connect(&mut self, endpoint: &str) -> Result<()>;
    async fn send(&mut self, data: &[u8]) -> Result<()>;
    async fn recv(&mut self) -> Result<Vec<u8>>;
    async fn close(&mut self) -> Result<()>;
    fn name(&self) -> &str;
}

pub struct MeekTransport {
    front_domain: String,
    target_host: String,
    session_cookie: Option<String>,
}

impl MeekTransport {
    pub fn new(front_domain: &str, target_host: &str) -> Self {
        Self {
            front_domain: front_domain.to_string(),
            target_host: target_host.to_string(),
            session_cookie: None,
        }
    }

    pub fn azure_bridge() -> Self {
        Self::new("ajax.aspnetcdn.com", "meek.azureedge.net")
    }

    pub fn amazon_bridge() -> Self {
        Self::new("d1.cloudfront.net", "meek.amazon.aws.amazon.com")
    }

    pub fn cloudflare_bridge() -> Self {
        Self::new("cloudflare.com", "meek.cloudflare.cloudflare.com")
    }
}

#[async_trait]
impl PluggableTransport for MeekTransport {
    async fn connect(&mut self, endpoint: &str) -> Result<()> {
        tracing::info!(
            "Meek transport connecting via front={} host={}",
            self.front_domain,
            self.target_host
        );
        self.session_cookie = Some(format!("meek-session-{}", uuid::Uuid::new_v4()));
        Ok(())
    }

    async fn send(&mut self, data: &[u8]) -> Result<()> {
        let client = reqwest::Client::new();
        let mut req = client
            .post(format!(
                "https://{}/{}",
                self.front_domain, self.target_host
            ))
            .header("Host", &self.target_host)
            .body(data.to_vec());
        if let Some(ref cookie) = self.session_cookie {
            req = req.header("Cookie", format!("session={}", cookie));
        }
        let _resp = req.send().await?;
        Ok(())
    }

    async fn recv(&mut self) -> Result<Vec<u8>> {
        Ok(vec![])
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
    fn name(&self) -> &str {
        "meek"
    }
}

pub struct SnowflakeTransport {
    broker_url: String,
    front_domain: String,
}

impl SnowflakeTransport {
    pub fn new() -> Self {
        Self {
            broker_url: "https://snowflake-broker.torproject.net/".to_string(),
            front_domain: "cdn.snowflake.torproject.org".to_string(),
        }
    }
}

#[async_trait]
impl PluggableTransport for SnowflakeTransport {
    async fn connect(&mut self, _endpoint: &str) -> Result<()> {
        tracing::info!(
            "Snowflake transport connecting via broker={}",
            self.broker_url
        );
        Ok(())
    }

    async fn send(&mut self, data: &[u8]) -> Result<()> {
        tracing::trace!("Snowflake send {} bytes", data.len());
        Ok(())
    }

    async fn recv(&mut self) -> Result<Vec<u8>> {
        Ok(vec![])
    }
    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
    fn name(&self) -> &str {
        "snowflake"
    }
}

pub struct TransportRegistry {
    transports: Vec<Box<dyn PluggableTransport>>,
}

impl TransportRegistry {
    pub fn new() -> Self {
        Self {
            transports: Vec::new(),
        }
    }

    pub fn register(&mut self, transport: Box<dyn PluggableTransport>) {
        tracing::info!("Registered transport: {}", transport.name());
        self.transports.push(transport);
    }

    pub fn select_transport(&self, censorship_level: u8) -> Option<&dyn PluggableTransport> {
        if censorship_level > 7 {
            self.transports.last().map(|t| t.as_ref())
        } else {
            self.transports.first().map(|t| t.as_ref())
        }
    }
}
