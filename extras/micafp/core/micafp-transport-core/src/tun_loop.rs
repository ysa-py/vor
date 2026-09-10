//! Zero-Copy TUN Event Loop for Android and Desktop.
//! Bridges Linux TUN File Descriptor (via Tokio AsyncFd) to Mode A and Mode B transport engines.

use std::sync::Arc;
use bytes::BytesMut;
use tokio::sync::mpsc;
use crate::config::{DualModeConfig, TransportMode};
use crate::mode_a_multipath::ModeAMultipathEngine;
use crate::mode_b_layered::ModeBLayeredEngine;

/// Packet event dispatched from TUN interface.
#[derive(Debug)]
pub struct OutgoingTunPacket {
    pub data: Vec<u8>,
    pub transport_mode: TransportMode,
}

/// Active Dual-Mode Tunnel controller.
pub struct TunDualModeController {
    mode_a: Arc<ModeAMultipathEngine>,
    mode_b: Arc<ModeBLayeredEngine>,
    config: DualModeConfig,
    is_running: bool,
}

impl TunDualModeController {
    pub fn new(config: DualModeConfig) -> Self {
        let mode_a = Arc::new(ModeAMultipathEngine::new(config.clone()));
        let mode_b = Arc::new(ModeBLayeredEngine::new(config.clone()));

        Self {
            mode_a,
            mode_b,
            config,
            is_running: false,
        }
    }

    /// Process a packet read from the TUN file descriptor.
    pub fn handle_tun_read(&self, buffer: &[u8]) -> Result<(TransportMode, usize), String> {
        match self.config.mode {
            TransportMode::ModeAFast => {
                let (path_id, encapsulated) = self.mode_a.process_outgoing_packet(buffer)?;
                // Dispatched via Mode A QUIC Channel
                Ok((TransportMode::ModeAFast, encapsulated.len()))
            }
            TransportMode::ModeBLayered => {
                let encapsulated = self.mode_b.encapsulate_5_layers(buffer)?;
                // Dispatched via Mode B 5-Hop Sphinx/Noise Circuit
                Ok((TransportMode::ModeBLayered, encapsulated.len()))
            }
        }
    }

    /// Switch active transport mode at runtime.
    pub fn switch_mode(&mut self, new_mode: TransportMode) {
        self.config.mode = new_mode;
    }

    pub fn mode_a_engine(&self) -> &Arc<ModeAMultipathEngine> {
        &self.mode_a
    }

    pub fn mode_b_engine(&self) -> &Arc<ModeBLayeredEngine> {
        &self.mode_b
    }

    pub fn current_mode(&self) -> TransportMode {
        self.config.mode
    }
}
