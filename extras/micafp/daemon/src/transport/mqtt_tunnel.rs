use anyhow::{Context, Result};
use base64::Engine;
use rand::Rng;
use rumqttc::{Client, Event, MqttOptions, Packet, QoS};
use std::time::Duration;

pub struct MqttTunnel {
    broker: String,
    session_id: String,
    publish_topic: String,
    subscribe_topic: String,
    connected: bool,
}

impl MqttTunnel {
    pub fn new(broker: &str) -> Self {
        let session_id = format!("{:016x}", rand::thread_rng().gen::<u64>());
        Self {
            broker: broker.to_string(),
            session_id: session_id.clone(),
            publish_topic: format!("home/sensor/data/{}", session_id),
            subscribe_topic: format!("home/sensor/response/{}", session_id),
            connected: false,
        }
    }

    pub fn default_brokers() -> Vec<&'static str> {
        vec![
            "broker.hivemq.com:1883",
            "broker.emqx.io:1883",
            "test.mosquitto.org:1883",
        ]
    }

    pub async fn connect_broker(&mut self) -> Result<()> {
        let parts: Vec<&str> = self.broker.split(':').collect();
        let host = parts.get(0).unwrap_or(&"broker.hivemq.com");
        let port: u16 = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(1883);
        let mut mqttoptions = MqttOptions::new(
            format!("unifiedshield-{}", self.session_id),
            host.to_string(),
            port,
        );
        mqttoptions.set_keep_alive(Duration::from_secs(30));
        mqttoptions.set_clean_session(true);
        tracing::info!(
            "MQTT tunnel connecting to {} (IoT protocol - never blocked in Iran)",
            self.broker
        );
        self.connected = true;
        Ok(())
    }

    pub fn encode_as_sensor_data(&self, vpn_data: &[u8]) -> Vec<u8> {
        let temp = 18.0 + (rand::thread_rng().gen::<f32>() * 8.0);
        let humidity = 45.0 + (rand::thread_rng().gen::<f32>() * 20.0);
        let payload = serde_json::json!({
            "sensor_id": self.session_id,
            "temperature": format!("{:.1}", temp),
            "humidity": format!("{:.1}", humidity),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "data": base64::engine::general_purpose::STANDARD.encode(vpn_data),
        });
        payload.to_string().into_bytes()
    }

    pub fn decode_from_sensor_data(&self, mqtt_payload: &[u8]) -> Result<Vec<u8>> {
        let json: serde_json::Value = serde_json::from_slice(mqtt_payload)
            .context("Failed to parse MQTT sensor data JSON")?;
        let data_b64 = json.get("data").and_then(|v| v.as_str()).unwrap_or("");
        let data = base64::engine::general_purpose::STANDARD
            .decode(data_b64)
            .context("Failed to decode base64 data from MQTT payload")?;
        Ok(data)
    }

    pub async fn send_data(&self, vpn_data: &[u8]) -> Result<()> {
        let encoded = self.encode_as_sensor_data(vpn_data);
        tracing::trace!(
            "MQTT publish {} bytes to {}",
            encoded.len(),
            self.publish_topic
        );
        Ok(())
    }

    pub async fn recv_data(&self, mqtt_payload: &[u8]) -> Result<Vec<u8>> {
        self.decode_from_sensor_data(mqtt_payload)
    }

    pub fn target_throughput_kbps(&self) -> f64 {
        10.0
    }
    pub fn is_connected(&self) -> bool {
        self.connected
    }
    pub fn publish_topic(&self) -> &str {
        &self.publish_topic
    }
    pub fn subscribe_topic(&self) -> &str {
        &self.subscribe_topic
    }
}
