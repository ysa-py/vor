use anyhow::Result;

pub struct DefyxVpnAdapter {
    binary_path: String,
    running: bool,
}

impl DefyxVpnAdapter {
    pub fn new(binary_path: &str) -> Self {
        Self {
            binary_path: binary_path.to_string(),
            running: false,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("Starting DefyxVPN from path");
        self.running = true;
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        tracing::info!("Stopping DefyxVPN");
        self.running = false;
        Ok(())
    }

    pub async fn health_check(&self) -> Result<bool> {
        Ok(self.running)
    }

    pub fn is_running(&self) -> bool {
        self.running
    }
}
