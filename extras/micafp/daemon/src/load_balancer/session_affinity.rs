// Session Affinity Table — pins long-lived sessions to a specific transport.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

struct AffinityEntry {
    transport: String,
    last_used: Instant,
}

pub struct SessionAffinityTable {
    table: RwLock<HashMap<String, AffinityEntry>>,
    ttl: Duration,
}

impl SessionAffinityTable {
    pub fn new(ttl: Duration) -> Self {
        Self {
            table: RwLock::new(HashMap::new()),
            ttl,
        }
    }

    pub async fn get(&self, session_id: &str) -> Option<String> {
        let table = self.table.read().await;
        table.get(session_id).and_then(|e| {
            if e.last_used.elapsed() < self.ttl {
                Some(e.transport.clone())
            } else {
                None
            }
        })
    }

    pub async fn pin(&self, session_id: String, transport: String) {
        let mut table = self.table.write().await;
        table.insert(
            session_id,
            AffinityEntry {
                transport,
                last_used: Instant::now(),
            },
        );
    }

    pub async fn evict_expired(&self) {
        let mut table = self.table.write().await;
        table.retain(|_, e| e.last_used.elapsed() < self.ttl);
    }
}
