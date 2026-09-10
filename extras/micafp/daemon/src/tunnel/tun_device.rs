use anyhow::{Context, Result};
use tokio::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};

pub struct TunDevice {
    writer: Box<dyn AsyncWrite + Send + Unpin>,
    reader: Box<dyn AsyncRead + Send + Unpin>,
    mtu: u16,
}

impl TunDevice {
    pub async fn create(mtu: u16, address: &str) -> Result<Self> {
        let mut config = tun::Configuration::default();
        config
            .address(address)
            .netmask("255.255.255.0")
            .mtu(mtu as i32)
            .up();
        #[cfg(target_os = "linux")]
        config.platform(|p| {
            p.packet_information(false);
        });
        let device = tun::create_as_async(&config).context("Failed to create TUN device")?;
        let (reader, writer) = tokio::io::split(device);
        tracing::info!("TUN device created with MTU={}, address={}", mtu, address);
        Ok(Self {
            writer: Box::new(writer),
            reader: Box::new(reader),
            mtu,
        })
    }
    pub async fn read_packet(&mut self) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; self.mtu as usize + 100];
        let n = self
            .reader
            .read(&mut buf)
            .await
            .context("TUN read failed")?;
        buf.truncate(n);
        Ok(buf)
    }
    pub async fn write_packet(&mut self, packet: Vec<u8>) -> Result<()> {
        self.writer
            .write_all(&packet)
            .await
            .context("TUN write failed")?;
        self.writer.flush().await.context("TUN flush failed")?;
        Ok(())
    }
}
