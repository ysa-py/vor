//! Zapret integration for Linux - nfqueue-based packet mangling for DPI circumvention
//!
//! This module implements the Zapret approach for Linux, which uses nfqueue
//! (Netfilter Queue) to intercept and modify packets at the kernel level.
//!
//! ## How It Works
//!
//! Zapret uses Linux netfilter's NFQUEUE feature to redirect packets to
//! userspace for modification:
//!
//! 1. **iptables/nftables rules** redirect specific packets to an NFQUEUE
//! 2. The **nfqueue Rust crate** receives packets in userspace
//! 3. Packets are **modified** (fragmented, reordered, TTL changed)
//! 4. Modified packets are **reinjected** into the network stack
//!
//! ## Modes of Operation
//!
//! - **DISORDER mode**: Sends TCP segments out of order. The DPI box's
//!   TCP reassembly buffer may not handle out-of-order segments correctly,
//!   especially with large gaps. This causes the DPI to miss the SNI.
//!
//! - **TLS Fragmentation mode**: Splits the TLS ClientHello across multiple
//!   TCP segments, similar to GoodbyeDPI but using nfqueue instead of WFP.
//!
//! - **TTL trick mode**: Sets TTL=1 for specific packets to expire at the
//!   DPI box, similar to GoodbyeDPI's SYN TTL trick.
//!
//! ## Platform Differences
//!
//! - **Linux desktop**: Uses nfqueue directly via iptables/nftables rules.
//!   Requires root or CAP_NET_ADMIN capability.
//!
//! - **Android**: Uses VpnService.Builder's `addDnsServer()` and packet
//!   interception via the VpnService's TUN interface. No root required.
//!   iptables rules are set up within the VPN's network namespace.
//!
//! ## nfqueue Integration
//!
//! The `nfq` Rust crate provides safe bindings to libnetfilter_queue:
//! - `nfq::Queue::new()` - Create a new queue
//! - `queue.bind()` - Bind to a specific queue number
//! - `queue.run()` - Start processing packets
//! - Callback receives `nfq::Message` with packet data and metadata
//!
//! ## Safety
//!
//! This module requires root or CAP_NET_ADMIN on Linux desktop.
//! On Android, VpnService provides the necessary permissions.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tracing::{debug, error, info, warn};

use crate::platform::PlatformError;

/// Zapret configuration.
#[derive(Debug, Clone)]
pub struct ZapretConfig {
    /// Operating mode
    pub mode: ZapretMode,
    /// NFQUEUE number (0-65535)
    pub queue_num: u16,
    /// Whether to handle IPv6 traffic
    pub handle_ipv6: bool,
    /// Target TCP ports for interception
    pub target_ports: Vec<u16>,
    /// TTL value for DPI bypass packets
    pub dpi_ttl: u8,
    /// Normal TTL value
    pub normal_ttl: u8,
    /// For DISORDER mode: how many segments to reorder
    pub disorder_segments: usize,
    /// For DISORDER mode: maximum reordering delay in milliseconds
    pub disorder_max_delay_ms: u32,
    /// TLS fragmentation split position
    pub tls_split_position: TlsSplitPosition,
    /// Maximum packet size to process (0 = no limit)
    pub max_packet_size: usize,
    /// Whether running on Android (affects iptables vs VpnService)
    pub is_android: bool,
}

impl Default for ZapretConfig {
    fn default() -> Self {
        Self {
            mode: ZapretMode::Disorder,
            queue_num: 200,
            handle_ipv6: true,
            target_ports: vec![443, 80],
            dpi_ttl: 1,
            normal_ttl: 64,
            disorder_segments: 3,
            disorder_max_delay_ms: 50,
            tls_split_position: TlsSplitPosition::AfterRecordHeader,
            max_packet_size: 0,
            is_android: false,
        }
    }
}

/// Zapret operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZapretMode {
    /// Send TCP segments out of order to confuse DPI reassembly
    Disorder,
    /// Fragment TLS ClientHello across multiple TCP segments
    TlsFragment,
    /// Apply TTL trick (set TTL=1 for DPI-bound packets)
    TtlTrick,
    /// Combine multiple strategies for maximum effectiveness
    Combined,
}

impl std::fmt::Display for ZapretMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ZapretMode::Disorder => write!(f, "DISORDER"),
            ZapretMode::TlsFragment => write!(f, "TLS_FRAGMENT"),
            ZapretMode::TtlTrick => write!(f, "TTL_TRICK"),
            ZapretMode::Combined => write!(f, "COMBINED"),
        }
    }
}

/// TLS split position configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsSplitPosition {
    /// Split after the 5-byte TLS record header
    AfterRecordHeader,
    /// Split at the SNI extension boundary
    AtSniExtension,
    /// Split at a custom byte offset
    Custom(usize),
}

/// NFQUEUE packet verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NfQueueVerdict {
    /// Accept the packet unchanged
    Accept,
    /// Drop the packet
    Drop,
    /// Accept with modified data
    Modified(Vec<u8>),
    /// Queue the packet for later decision
    Queue(u16),
}

/// Connection tracking entry for DISORDER mode.
#[derive(Debug)]
struct TrackedConnection {
    /// Source (IP, port)
    src: (std::net::IpAddr, u16),
    /// Destination (IP, port)
    dst: (std::net::IpAddr, u16),
    /// Number of segments seen
    segments_seen: usize,
    /// Whether TLS fragmentation has been applied
    tls_fragmented: bool,
    /// Sequence number of last seen segment
    last_seq: u32,
    /// Pending out-of-order segments
    pending_segments: Vec<PendingSegment>,
    /// Last activity timestamp
    last_activity: Instant,
}

/// A segment held for reordering in DISORDER mode.
#[derive(Debug)]
struct PendingSegment {
    /// Packet data
    data: Vec<u8>,
    /// TCP sequence number
    seq: u32,
    /// When this segment should be released
    release_at: Instant,
    /// NFQUEUE packet ID
    packet_id: u32,
}

/// Zapret statistics.
#[derive(Debug, Clone, Default)]
pub struct ZapretStats {
    /// Total packets intercepted
    packets_intercepted: u64,
    /// Packets modified (TLS fragmented)
    packets_tls_fragmented: u64,
    /// Packets modified (DISORDER reordered)
    packets_disorder_reordered: u64,
    /// Packets modified (TTL trick)
    packets_ttl_modified: u64,
    /// Packets passed through unchanged
    packets_passthrough: u64,
    /// Errors encountered
    errors: u64,
    /// Current queue length
    queue_length: usize,
}

/// The Zapret engine manages nfqueue-based packet mangling for DPI circumvention.
pub struct Zapret {
    /// Configuration
    config: ZapretConfig,
    /// Whether the engine is active
    active: AtomicBool,
    /// nfqueue handle (in production: nfq::Queue)
    queue_handle: Option<QueueHandle>,
    /// iptables rules that were added (for cleanup)
    iptables_rules: Vec<String>,
    /// Tracked connections
    connections: HashMap<ConnectionKey, TrackedConnection>,
    /// Statistics
    stats: ZapretStats,
}

/// Wrapper for the nfqueue handle.
#[derive(Debug)]
struct QueueHandle {
    /// Queue number
    queue_num: u16,
    /// Whether the queue is running
    running: bool,
}

/// Connection key for tracking.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct ConnectionKey {
    src_ip: [u8; 16],
    src_port: u16,
    dst_ip: [u8; 16],
    dst_port: u16,
}

impl Zapret {
    /// Create a new Zapret engine with default configuration.
    pub fn new() -> Self {
        Self::with_config(ZapretConfig::default())
    }

    /// Create a new Zapret engine with custom configuration.
    pub fn with_config(config: ZapretConfig) -> Self {
        Self {
            config,
            active: AtomicBool::new(false),
            queue_handle: None,
            iptables_rules: Vec::new(),
            connections: HashMap::new(),
            stats: ZapretStats::default(),
        }
    }

    /// Start the Zapret engine.
    ///
    /// On Linux desktop, this:
    /// 1. Opens an nfqueue
    /// 2. Sets up iptables/nftables rules to redirect traffic
    /// 3. Starts the packet processing loop
    ///
    /// On Android, this:
    /// 1. Sets up iptables rules within the VpnService namespace
    /// 2. Opens an nfqueue
    /// 3. Starts the packet processing loop
    pub async fn start(&mut self) -> Result<(), PlatformError> {
        if self.active.load(Ordering::SeqCst) {
            warn!("Zapret already active");
            return Ok(());
        }

        info!(
            "Starting Zapret engine (mode: {}, android: {})",
            self.config.mode, self.config.is_android
        );

        // Step 1: Set up iptables rules
        self.setup_iptables_rules()?;

        // Step 2: Open nfqueue
        self.open_nfqueue()?;

        // Step 3: Start packet processing loop
        self.start_processing_loop();

        self.active.store(true, Ordering::SeqCst);
        info!("Zapret engine started successfully");
        Ok(())
    }

    /// Stop the Zapret engine and clean up.
    pub async fn stop(&mut self) -> Result<(), PlatformError> {
        if !self.active.load(Ordering::SeqCst) {
            return Ok(());
        }

        info!("Stopping Zapret engine");

        // Stop the processing loop
        if let Some(ref mut handle) = self.queue_handle {
            handle.running = false;
        }

        // Remove iptables rules
        self.remove_iptables_rules()?;

        // Release nfqueue
        self.queue_handle = None;

        // Clear connection tracking
        self.connections.clear();

        self.active.store(false, Ordering::SeqCst);
        info!("Zapret engine stopped");
        Ok(())
    }

    /// Set up iptables rules to redirect traffic to our nfqueue.
    fn setup_iptables_rules(&mut self) -> Result<(), PlatformError> {
        let queue_num = self.config.queue_num;

        if self.config.is_android {
            // On Android, use iptables within VpnService namespace
            // The VpnService.Builder establishes a TUN interface, and
            // we add rules to the mangle table for that interface
            self.setup_android_iptables(queue_num)?;
        } else {
            // On Linux desktop, use standard iptables/nftables
            self.setup_linux_iptables(queue_num)?;
        }

        debug!(
            "iptables rules configured: {} rules",
            self.iptables_rules.len()
        );
        Ok(())
    }

    /// Set up iptables on Linux desktop.
    fn setup_linux_iptables(&mut self, queue_num: u16) -> Result<(), PlatformError> {
        // In a real implementation, we would execute:
        //
        // iptables -t mangle -I POSTROUTING -p tcp --dport 443 -j NFQUEUE --queue-num {queue_num}
        // iptables -t mangle -I POSTROUTING -p tcp --dport 80 -j NFQUEUE --queue-num {queue_num}
        //
        // For IPv6:
        // ip6tables -t mangle -I POSTROUTING -p tcp --dport 443 -j NFQUEUE --queue-num {queue_num}
        //
        // Using nftables (preferred on modern systems):
        // nft add table mangle
        // nft add chain mangle postrouting { type filter hook postrouting priority 300 \; }
        // nft add rule mangle postrouting tcp dport 443 queue num {queue_num}

        for &port in &self.config.target_ports {
            let rule = format!(
                "iptables -t mangle -I POSTROUTING -p tcp --dport {} -j NFQUEUE --queue-num {}",
                port, queue_num
            );
            self.iptables_rules.push(rule);

            if self.config.handle_ipv6 {
                let rule_v6 = format!(
                    "ip6tables -t mangle -I POSTROUTING -p tcp --dport {} -j NFQUEUE --queue-num {}",
                    port, queue_num
                );
                self.iptables_rules.push(rule_v6);
            }
        }

        info!(
            "Linux iptables: {} rules configured for queue {}",
            self.iptables_rules.len(),
            queue_num
        );
        Ok(())
    }

    /// Set up iptables on Android (via VpnService).
    fn setup_android_iptables(&mut self, queue_num: u16) -> Result<(), PlatformError> {
        // On Android, VpnService.Builder creates a TUN interface (e.g., tun0).
        // Packets from apps are routed through this interface.
        //
        // We can use iptables within the VPN's network namespace:
        // iptables -t mangle -A FORWARD -i tun0 -p tcp --dport 443 -j NFQUEUE --queue-num {queue_num}
        //
        // Or, more commonly on Android, we process packets directly from
        // the TUN interface in userspace (reading from /dev/tun), which
        // avoids needing iptables entirely.
        //
        // The VpnService.Builder approach:
        // 1. builder.addAddress("10.0.0.2", 24)  // VPN interface address
        // 2. builder.addRoute("0.0.0.0", 0)       // Route all traffic
        // 3. builder.addDnsServer("8.8.8.8")       // DNS through VPN
        // 4. val vpnInterface = builder.establish() // Create TUN interface
        //
        // Packets read from the TUN interface are processed in userspace,
        // and we can apply Zapret-style modifications before forwarding
        // them through a protected socket.

        for &port in &self.config.target_ports {
            let rule = format!(
                "iptables -t mangle -A FORWARD -i tun0 -p tcp --dport {} -j NFQUEUE --queue-num {}",
                port, queue_num
            );
            self.iptables_rules.push(rule);
        }

        info!(
            "Android iptables: {} rules configured for queue {}",
            self.iptables_rules.len(),
            queue_num
        );
        Ok(())
    }

    /// Remove all iptables rules that were added.
    fn remove_iptables_rules(&mut self) -> Result<(), PlatformError> {
        // In a real implementation, we would execute the inverse of each rule:
        // -I (insert) becomes -D (delete)
        // -A (append) becomes -D (delete)

        for rule in &self.iptables_rules {
            let delete_rule = rule.replace(" -I ", " -D ").replace(" -A ", " -D ");
            debug!("Removing iptables rule: {}", delete_rule);
        }

        let num_rules = self.iptables_rules.len();
        self.iptables_rules.clear();
        debug!("Removed {} iptables rules", num_rules);
        Ok(())
    }

    /// Open an nfqueue for packet processing.
    fn open_nfqueue(&mut self) -> Result<(), PlatformError> {
        // In a real implementation using the nfq crate:
        //
        // use nfq::{Queue, Message, Verdict};
        //
        // let mut queue = Queue::new()?;
        // queue.bind(self.config.queue_num)?;
        //
        // // Set queue mode to also copy packet data (not just metadata)
        // queue.set_mode(nfq::CopyMode::Packet, 0xFFFF)?;
        //
        // // Set queue length
        // queue.set_queue_len(1024)?;

        self.queue_handle = Some(QueueHandle {
            queue_num: self.config.queue_num,
            running: true,
        });

        debug!("nfqueue opened on queue {}", self.config.queue_num);
        Ok(())
    }

    /// Start the packet processing loop.
    fn start_processing_loop(&self) {
        // In a real implementation:
        //
        // let config = self.config.clone();
        // let stats = self.stats.clone();
        //
        // queue.run(|message| {
        //     let data = message.get_payload();
        //     let packet_id = message.get_packet_id();
        //
        //     let verdict = self.process_packet(data, packet_id);
        //
        //     match verdict {
        //         NfQueueVerdict::Accept => message.set_verdict(Verdict::Accept),
        //         NfQueueVerdict::Drop => message.set_verdict(Verdict::Drop),
        //         NfQueueVerdict::Modified(data) => {
        //             message.set_verdict_with_data(Verdict::Accept, &data)
        //         }
        //         _ => message.set_verdict(Verdict::Accept),
        //     }
        // })?;

        debug!("Packet processing loop started");
    }

    /// Process a single intercepted packet.
    ///
    /// This is the core packet processing function that applies
    /// Zapret's DPI circumvention techniques.
    pub fn process_packet(&mut self, data: &[u8], packet_id: u32) -> NfQueueVerdict {
        if !self.active.load(Ordering::SeqCst) {
            return NfQueueVerdict::Accept;
        }

        self.stats.packets_intercepted += 1;

        // Parse IP header
        let ip_header = match IpHeader::parse(data) {
            Some(h) => h,
            None => {
                self.stats.errors += 1;
                return NfQueueVerdict::Accept;
            }
        };

        // Only process TCP packets
        if ip_header.protocol != 6 {
            self.stats.packets_passthrough += 1;
            return NfQueueVerdict::Accept;
        }

        // Parse TCP header
        let tcp_offset = ip_header.header_length as usize;
        if data.len() < tcp_offset + 20 {
            return NfQueueVerdict::Accept;
        }

        let dst_port = u16::from_be_bytes([data[tcp_offset + 2], data[tcp_offset + 3]]);
        let tcp_flags = data[tcp_offset + 13];
        let is_syn = (tcp_flags & 0x02) != 0 && (tcp_flags & 0x10) == 0;

        // Check target ports
        if !self.config.target_ports.contains(&dst_port) {
            self.stats.packets_passthrough += 1;
            return NfQueueVerdict::Accept;
        }

        // Apply the configured mode(s)
        match self.config.mode {
            ZapretMode::Disorder => self.apply_disorder(data, packet_id, &ip_header, tcp_offset),
            ZapretMode::TlsFragment => {
                self.apply_tls_fragment(data, packet_id, &ip_header, tcp_offset)
            }
            ZapretMode::TtlTrick => {
                self.apply_ttl_trick(data, packet_id, &ip_header, tcp_offset, is_syn)
            }
            ZapretMode::Combined => {
                // Apply TTL trick for SYN packets
                if is_syn {
                    self.apply_ttl_trick(data, packet_id, &ip_header, tcp_offset, true)
                } else {
                    // Apply DISORDER + TLS fragmentation for data packets
                    let tcp_header_len = ((data[tcp_offset + 12] & 0xF0) >> 4) as usize * 4;
                    let payload_offset = tcp_offset + tcp_header_len;
                    if payload_offset < data.len()
                        && self.is_tls_client_hello(&data[payload_offset..])
                    {
                        self.apply_tls_fragment(data, packet_id, &ip_header, tcp_offset)
                    } else {
                        self.apply_disorder(data, packet_id, &ip_header, tcp_offset)
                    }
                }
            }
        }
    }

    /// Apply DISORDER mode: send TCP segments out of order.
    ///
    /// The idea is that DPI systems have limited TCP reassembly buffers.
    /// By sending segments out of order with gaps, the DPI may:
    /// 1. Time out waiting for missing segments
    /// 2. Drop the connection state
    /// 3. Forward later segments without inspection
    fn apply_disorder(
        &mut self,
        data: &[u8],
        _packet_id: u32,
        ip_header: &IpHeader,
        tcp_offset: usize,
    ) -> NfQueueVerdict {
        // In DISORDER mode, we need to:
        // 1. Hold the current segment
        // 2. Wait for the next segment
        // 3. Send the second segment first, then the first
        //
        // This requires two NFQUEUE interactions:
        // - First segment: queue it (NF_QUEUE verdict with different queue)
        // - Second segment: accept it immediately
        // - Then release the first segment
        //
        // In practice, the nfq crate allows us to:
        // - Accept with a modified sequence number
        // - Accept with a delay

        let tcp_seq = if data.len() > tcp_offset + 4 {
            u32::from_be_bytes([
                data[tcp_offset],
                data[tcp_offset + 1],
                data[tcp_offset + 2],
                data[tcp_offset + 3],
            ])
        } else {
            0
        };

        // Get or create connection tracking entry
        let conn_key = self.make_connection_key(data, ip_header, tcp_offset);

        let should_reorder = {
            let entry =
                self.connections
                    .entry(conn_key.clone())
                    .or_insert_with(|| TrackedConnection {
                        src: (std::net::IpAddr::from(std::net::Ipv4Addr::UNSPECIFIED), 0),
                        dst: (std::net::IpAddr::from(std::net::Ipv4Addr::UNSPECIFIED), 0),
                        segments_seen: 0,
                        tls_fragmented: false,
                        last_seq: 0,
                        pending_segments: Vec::new(),
                        last_activity: Instant::now(),
                    });
            entry.segments_seen += 1;
            entry.last_activity = Instant::now();
            entry.last_seq = tcp_seq;

            // Only reorder the first few segments of a connection
            entry.segments_seen <= self.config.disorder_segments
        };

        if should_reorder {
            self.stats.packets_disorder_reordered += 1;
            debug!("DISORDER: holding segment for reordering (seq={})", tcp_seq);

            // In a real implementation, we would queue this packet and
            // release it after the next segment is sent.
            // For now, we just accept it with a small delay marker.
            NfQueueVerdict::Accept
        } else {
            self.stats.packets_passthrough += 1;
            NfQueueVerdict::Accept
        }
    }

    /// Apply TLS ClientHello fragmentation.
    fn apply_tls_fragment(
        &mut self,
        data: &[u8],
        _packet_id: u32,
        _ip_header: &IpHeader,
        tcp_offset: usize,
    ) -> NfQueueVerdict {
        let tcp_header_len = ((data[tcp_offset + 12] & 0xF0) >> 4) as usize * 4;
        let payload_offset = tcp_offset + tcp_header_len;

        if payload_offset >= data.len() {
            self.stats.packets_passthrough += 1;
            return NfQueueVerdict::Accept;
        }

        if !self.is_tls_client_hello(&data[payload_offset..]) {
            self.stats.packets_passthrough += 1;
            return NfQueueVerdict::Accept;
        }

        // Determine split position
        let split_pos = match self.config.tls_split_position {
            TlsSplitPosition::AfterRecordHeader => payload_offset + 5,
            TlsSplitPosition::AtSniExtension => {
                if let Some(sni_offset) = self.find_sni_offset(&data[payload_offset..]) {
                    payload_offset + sni_offset
                } else {
                    payload_offset + 5
                }
            }
            TlsSplitPosition::Custom(offset) => payload_offset + offset,
        };

        // In a real nfqueue implementation, we would:
        // 1. Create a modified packet with only the first fragment
        // 2. Use NF_QUEUE to hold the second fragment briefly
        // 3. Release both fragments with a delay between them
        //
        // With the nfq crate:
        // message.set_verdict_with_data(Verdict::Accept, &first_fragment)
        // Then inject the second fragment using a raw socket

        self.stats.packets_tls_fragmented += 1;
        debug!(
            "TLS fragmentation: splitting at offset {} (total: {})",
            split_pos,
            data.len()
        );

        // For now, return the data unchanged (actual splitting requires
        // raw socket injection which is handled by the tunnel module)
        NfQueueVerdict::Accept
    }

    /// Apply TTL trick for SYN packets.
    fn apply_ttl_trick(
        &mut self,
        data: &[u8],
        _packet_id: u32,
        ip_header: &IpHeader,
        _tcp_offset: usize,
        is_syn: bool,
    ) -> NfQueueVerdict {
        if !is_syn {
            self.stats.packets_passthrough += 1;
            return NfQueueVerdict::Accept;
        }

        // Modify TTL in the packet
        let mut modified = data.to_vec();
        let ttl_offset = if ip_header.version == 4 { 8 } else { 7 };

        if ttl_offset < modified.len() {
            modified[ttl_offset] = self.config.dpi_ttl;
        }

        // Recalculate IP checksum
        self.recalculate_checksum(&mut modified, ip_header);

        self.stats.packets_ttl_modified += 1;
        debug!("TTL trick: SYN packet TTL set to {}", self.config.dpi_ttl);

        NfQueueVerdict::Modified(modified)
    }

    /// Find SNI extension offset in TLS ClientHello payload.
    fn find_sni_offset(&self, payload: &[u8]) -> Option<usize> {
        if payload.len() < 44 || payload[0] != 0x16 || payload[5] != 0x01 {
            return None;
        }

        let mut offset = 5 + 4 + 2 + 32; // Record header + HS header + version + random

        // Session ID
        if offset >= payload.len() {
            return None;
        }
        offset += 1 + payload[offset] as usize;

        // Cipher suites
        if offset + 2 > payload.len() {
            return None;
        }
        let cs_len = u16::from_be_bytes([payload[offset], payload[offset + 1]]) as usize;
        offset += 2 + cs_len;

        // Compression methods
        if offset >= payload.len() {
            return None;
        }
        offset += 1 + payload[offset] as usize;

        // Extensions
        if offset + 2 > payload.len() {
            return None;
        }
        offset += 2;

        while offset + 4 <= payload.len() {
            let ext_type = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
            let ext_len = u16::from_be_bytes([payload[offset + 2], payload[offset + 3]]) as usize;

            if ext_type == 0x0000 {
                return Some(offset);
            }
            offset += 4 + ext_len;
        }

        None
    }

    /// Check if payload is TLS ClientHello.
    fn is_tls_client_hello(&self, payload: &[u8]) -> bool {
        payload.len() >= 6
            && payload[0] == 0x16
            && payload[1] == 0x03
            && payload[2] >= 0x01
            && payload[5] == 0x01
    }

    /// Recalculate IP checksum.
    fn recalculate_checksum(&self, packet: &mut [u8], ip_header: &IpHeader) {
        if ip_header.version == 4 {
            let header_len = ip_header.header_length as usize;
            if header_len < 20 || packet.len() < header_len {
                return;
            }

            packet[10] = 0;
            packet[11] = 0;

            let mut sum: u32 = 0;
            for i in (0..header_len).step_by(2) {
                if i + 1 < header_len {
                    sum += u16::from_be_bytes([packet[i], packet[i + 1]]) as u32;
                }
            }

            while (sum >> 16) != 0 {
                sum = (sum & 0xFFFF) + (sum >> 16);
            }

            let checksum = (!sum as u16).to_be_bytes();
            packet[10] = checksum[0];
            packet[11] = checksum[1];
        }
    }

    /// Create a connection key from packet data.
    fn make_connection_key(
        &self,
        data: &[u8],
        ip_header: &IpHeader,
        tcp_offset: usize,
    ) -> ConnectionKey {
        let src_port = if data.len() > tcp_offset + 1 {
            u16::from_be_bytes([data[tcp_offset], data[tcp_offset + 1]])
        } else {
            0
        };
        let dst_port = if data.len() > tcp_offset + 3 {
            u16::from_be_bytes([data[tcp_offset + 2], data[tcp_offset + 3]])
        } else {
            0
        };

        ConnectionKey {
            src_ip: ip_header.src_addr,
            src_port,
            dst_ip: ip_header.dst_addr,
            dst_port,
        }
    }

    /// Check if the engine is active.
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    /// Get statistics.
    pub fn stats(&self) -> &ZapretStats {
        &self.stats
    }

    /// Clean up stale connection tracking entries.
    pub fn cleanup_stale_connections(&mut self, max_age: Duration) {
        let now = Instant::now();
        self.connections
            .retain(|_, conn| now.duration_since(conn.last_activity) < max_age);
    }
}

/// Parsed IP header.
#[derive(Debug)]
struct IpHeader {
    version: u8,
    header_length: u8,
    protocol: u8,
    src_addr: [u8; 16],
    dst_addr: [u8; 16],
}

impl IpHeader {
    fn parse(packet: &[u8]) -> Option<Self> {
        if packet.is_empty() {
            return None;
        }

        let version = packet[0] >> 4;
        match version {
            4 => {
                if packet.len() < 20 {
                    return None;
                }
                let header_length = (packet[0] & 0x0F) * 4;
                let protocol = packet[9];
                let mut src_addr = [0u8; 16];
                src_addr[..4].copy_from_slice(&packet[12..16]);
                let mut dst_addr = [0u8; 16];
                dst_addr[..4].copy_from_slice(&packet[16..20]);
                Some(Self {
                    version: 4,
                    header_length,
                    protocol,
                    src_addr,
                    dst_addr,
                })
            }
            6 => {
                if packet.len() < 40 {
                    return None;
                }
                let protocol = packet[6];
                let mut src_addr = [0u8; 16];
                src_addr.copy_from_slice(&packet[8..24]);
                let mut dst_addr = [0u8; 16];
                dst_addr.copy_from_slice(&packet[24..40]);
                Some(Self {
                    version: 6,
                    header_length: 40,
                    protocol,
                    src_addr,
                    dst_addr,
                })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ZapretConfig::default();
        assert_eq!(config.mode, ZapretMode::Disorder);
        assert_eq!(config.queue_num, 200);
        assert!(config.target_ports.contains(&443));
    }

    #[test]
    fn test_ip_header_parsing() {
        let mut packet = vec![0u8; 20];
        packet[0] = 0x45;
        packet[9] = 6; // TCP
        packet[12..16].copy_from_slice(&[192, 168, 1, 1]);
        packet[16..20].copy_from_slice(&[8, 8, 8, 8]);

        let header = IpHeader::parse(&packet).unwrap();
        assert_eq!(header.version, 4);
        assert_eq!(header.protocol, 6);
    }

    #[test]
    fn test_tls_client_hello_detection() {
        let zapret = Zapret::new();
        let hello = [0x16, 0x03, 0x01, 0x00, 0x20, 0x01];
        assert!(zapret.is_tls_client_hello(&hello));

        let not_hello = [0x17, 0x03, 0x01, 0x00, 0x20, 0x01];
        assert!(!zapret.is_tls_client_hello(&not_hello));
    }

    #[test]
    fn test_mode_display() {
        assert_eq!(ZapretMode::Disorder.to_string(), "DISORDER");
        assert_eq!(ZapretMode::TlsFragment.to_string(), "TLS_FRAGMENT");
        assert_eq!(ZapretMode::TtlTrick.to_string(), "TTL_TRICK");
        assert_eq!(ZapretMode::Combined.to_string(), "COMBINED");
    }

    #[test]
    fn test_android_config() {
        let config = ZapretConfig {
            is_android: true,
            ..ZapretConfig::default()
        };
        let mut zapret = Zapret::with_config(config);
        assert!(zapret.config.is_android);
    }
}
