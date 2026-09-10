use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use rand::Rng;
use rand::RngCore;

pub struct WsTunnel {
    url: String,
    origin: String,
    write: Option<futures::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>>,
    read: Option<futures::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>>,
}

impl WsTunnel {
    pub fn new(url: &str, origin: &str) -> Self {
        Self { url: url.to_string(), origin: origin.to_string(), write: None, read: None }
    }

    pub async fn connect(&mut self) -> Result<()> {
        let mut request = self.url.clone().into_client_request().context("Invalid WebSocket URL")?;
        let headers = request.headers_mut();
        let mut key_bytes = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut key_bytes);
        headers.insert("Sec-WebSocket-Key", base64::encode(key_bytes).parse().unwrap());
        headers.insert("Sec-WebSocket-Version", "13".parse().unwrap());
        headers.insert("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".parse().unwrap());
        headers.insert("Origin", self.origin.parse().unwrap());
        let (ws_stream, _) = connect_async(request).await.context("WebSocket connect failed")?;
        let (write, read) = ws_stream.split();
        self.write = Some(write);
        self.read = Some(read);
        tracing::info!("WebSocket tunnel connected to {}", self.url);
        Ok(())
    }

    pub async fn send_data(&mut self, data: &[u8]) -> Result<()> {
        let masked = self.mask_frame(data);
        if let Some(ref mut write) = self.write {
            write.send(Message::Binary(masked)).await.context("WebSocket send failed")?;
        }
        Ok(())
    }

    pub async fn recv_data(&mut self) -> Result<Vec<u8>> {
        if let Some(ref mut read) = self.read {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Binary(data)) => return Ok(data),
                    Ok(Message::Ping(data)) => { /* pong handled by tungstenite */ }
                    Ok(Message::Close(_)) => return Err(anyhow::anyhow!("WebSocket closed")),
                    Err(e) => return Err(anyhow::anyhow!("WebSocket recv error: {}", e)),
                    _ => continue,
                }
            }
        }
        Err(anyhow::anyhow!("No data available"))
    }

    fn mask_frame(&self, data: &[u8]) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let mut masking_key = [0u8; 4];
        rng.fill_bytes(&mut masking_key);
        let mut output = Vec::with_capacity(data.len());
        for (i, &byte) in data.iter().enumerate() {
            output.push(byte ^ masking_key[i % 4]);
        }
        output
    }
}
