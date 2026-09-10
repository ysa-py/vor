use std::net::TcpStream;
use std::time::Duration;

pub struct IntranetDetector {
    is_intranet_mode: bool,
}

impl IntranetDetector {
    pub fn new() -> Self {
        Self {
            is_intranet_mode: false,
        }
    }

    pub async fn check(&mut self) -> bool {
        let international = [
            "google.com:443",
            "8.8.8.8:53",
            "1.1.1.1:53",
            "github.com:443",
            "cloudflare.com:443",
            "api.telegram.org:443",
            "youtube.com:443",
            "amazon.com:443",
            "twitter.com:443",
            "facebook.com:443",
            "microsoft.com:443",
            "9.9.9.9:53",
        ];
        let domestic = [
            "10.202.10.10:53",
            "10.202.10.11:53",
            "78.157.42.100:53",
            "shaparak.ir:443",
            "bpi.ir:443",
            "irna.ir:443",
        ];
        let mut intl_ok = 0u8;
        let mut dom_ok = 0u8;
        for target in &international {
            if Self::probe_tcp(target).await {
                intl_ok += 1;
            }
        }
        for target in &domestic {
            if Self::probe_tcp(target).await {
                dom_ok += 1;
            }
        }
        let was = self.is_intranet_mode;
        self.is_intranet_mode = intl_ok < 2 && dom_ok >= 3;
        if self.is_intranet_mode && !was {
            tracing::warn!(
                "NATIONAL INTRANET MODE ACTIVATED - intl:{}/12 dom:{}/6",
                intl_ok,
                dom_ok
            );
        } else if !self.is_intranet_mode && was {
            tracing::info!(
                "National intranet mode DEACTIVATED - intl:{}/12 dom:{}/6",
                intl_ok,
                dom_ok
            );
        }
        self.is_intranet_mode
    }

    async fn probe_tcp(target: &str) -> bool {
        tokio::time::timeout(Duration::from_secs(3), async {
            TcpStream::connect(target).is_ok()
        })
        .await
        .is_ok()
    }

    pub fn is_intranet_mode(&self) -> bool {
        self.is_intranet_mode
    }
}
