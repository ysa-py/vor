use std::time::{Duration, Instant};
use rand::Rng;
use rand::RngCore;

pub struct Http3Masquerade {
    connection_id: [u8; 20],
    spin_bit: bool,
    spin_counter: u8,
    last_cid_rotation: Instant,
}

impl Http3Masquerade {
    pub fn new() -> Self {
        let mut cid = [0u8; 20];
        rand::thread_rng().fill_bytes(&mut cid);
        Self {
            connection_id: cid,
            spin_bit: false,
            spin_counter: 0,
            last_cid_rotation: Instant::now(),
        }
    }

    pub fn rotate_connection_id(&mut self) -> [u8; 20] {
        if self.last_cid_rotation.elapsed() > Duration::from_secs(30) {
            rand::thread_rng().fill_bytes(&mut self.connection_id);
            self.last_cid_rotation = Instant::now();
            tracing::debug!("Rotated QUIC connection ID");
        }
        self.connection_id
    }

    pub fn get_spin_bit(&mut self) -> bool {
        self.spin_counter += 1;
        if self.spin_counter >= 8 {
            self.spin_bit = !self.spin_bit;
            self.spin_counter = 0;
        }
        self.spin_bit
    }

    pub fn build_qpack_header(&self, headers: &[(&str, &str)]) -> Vec<u8> {
        let mut encoded = Vec::new();
        encoded.push(0x00); // QPACK prefix: base index 0
        for &(name, value) in headers {
            encoded.push(0x50); // Literal header field without indexing
            encoded.push(name.len() as u8);
            encoded.extend_from_slice(name.as_bytes());
            encoded.push(value.len() as u8);
            encoded.extend_from_slice(value.as_bytes());
        }
        encoded
    }

    pub fn inject_server_push(&self, stream_id: u32) -> Vec<u8> {
        let mut frame = Vec::new();
        frame.push(0x05); // PUSH_PROMISE frame type
        frame.extend_from_slice(&stream_id.to_be_bytes()[4..8]);
        frame.push(0x00); frame.push(0x00); // No payload
        frame
    }

    pub fn build_priority_update(&self, urgency: u8, incremental: bool) -> Vec<u8> {
        let mut frame = Vec::new();
        frame.push(0x07); // PRIORITY_UPDATE
        let priority = format!("u={},{},i", urgency.min(7), if incremental { "i" } else { "?" });
        frame.push((priority.len() >> 8) as u8);
        frame.push((priority.len() & 0xFF) as u8);
        frame.extend_from_slice(priority.as_bytes());
        frame
    }
}
