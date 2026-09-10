use std::time::Duration;
pub struct SystemWatchdog {
    pub heartbeat_interval: Duration,
}
impl SystemWatchdog {
    pub fn new(interval_secs: u64) -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(interval_secs),
        }
    }
    pub async fn run(&self) {
        loop {
            tokio::time::sleep(self.heartbeat_interval).await;
        }
    }
}
