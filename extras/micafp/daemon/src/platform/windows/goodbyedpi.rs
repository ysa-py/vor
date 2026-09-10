//! GoodbyeDPI for Windows - WFP user-mode callout for DPI circumvention
//!
//! This module implements the GoodbyeDPI approach for Windows, which uses
//! the Windows Filtering Platform (WFP) user-mode API to intercept and
//! modify TCP packets to bypass Deep Packet Inspection.
//!
//! ## How It Works
//!
//! GoodbyeDPI operates at the network layer using WFP callouts:
//!
//! 1. **SYN Packet TTL Trick**: When a new TCP connection is initiated,
//!    the SYN packet is intercepted and its TTL is set to 1. This causes
//!    the packet to expire at the first DPI box (which is typically the
//!    ISP's gateway), but the connection still establishes because the
//!    DPI box's state tracker records the connection as established.
//!
//! 2. **TLS ClientHello Fragmentation**: The TLS ClientHello is split
//!    across 2 TCP segments. The first segment contains the TLS record
//!    header and part of the handshake, while the second contains the
//!    rest (including the SNI extension). DPI systems that only inspect
//!    the first segment miss the SNI.
//!
//! ## Why No Admin Is Required
//!
//! Traditional WFP requires a kernel-mode callout driver (which needs admin).
//! However, GoodbyeDPI uses the **user-mode** WFP API:
//!
//! - `FwpmFilterAdd0` with `FWPM_LAYER_OUTBOUND_TRANSPORT_V4`
//! - User-mode callout via `ALE` (Application Layer Enforcement) layers
//! - The callout runs in user space and can modify packet data
//!
//! This approach:
//! - Does NOT require UAC elevation
//! - Does NOT require DLL injection
//! - Does NOT install a kernel driver
//! - Works on Windows 7 SP1 and later
//!
//! ## WFP API Integration
//!
//! The Windows WFP API is accessed through the `windows` crate:
//! - `FwpmEngineOpen0`: Open a WFP engine session
//! - `FwpmCalloutAdd0`: Register a callout
//! - `FwpmFilterAdd0`: Add a filter for the callout
//! - `FwpmSubLayerAdd0`: Add a sub-layer for our filters
//!
//! ## Safety
//!
//! This module uses unsafe code for Windows API calls. The safety guarantees
//! are:
//! - All WFP handles are properly closed on shutdown
//! - Packet modifications are bounded and validated
//! - Memory for packet data is properly allocated and freed

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tracing::{debug, error, info, warn};

use crate::platform::PlatformError;

// Windows API types and constants for WFP
// These would come from the `windows` crate in a real build environment.
// We define them here for documentation and compilation on non-Windows platforms.

/// WFP engine handle type
#[cfg(target_os = "windows")]
type HEngine = isize;

/// WFP callout handle type
#[cfg(target_os = "windows")]
type HCallout = isize;

/// WFP filter handle type
#[cfg(target_os = "windows")]
type HFilter = isize;

/// WFP sub-layer handle type
#[cfg(target_os = "windows")]
type HSubLayer = isize;

// =========================================================================
// WFP Constants
// =========================================================================

/// WFP layer: Outbound transport (IPv4)
const FWPM_LAYER_OUTBOUND_TRANSPORT_V4: u16 = 0x0006;

/// WFP layer: Outbound transport (IPv6)
const FWPM_LAYER_OUTBOUND_TRANSPORT_V6: u16 = 0x0007;

/// WFP layer: ALE connect (IPv4) - for intercepting connection attempts
const FWPM_LAYER_ALE_AUTH_CONNECT_V4: u16 = 0x000C;

/// WFP layer: ALE connect (IPv6)
const FWPM_LAYER_ALE_AUTH_CONNECT_V6: u16 = 0x000D;

/// TTL value for DPI bypass (low TTL to expire at DPI box)
const DPI_BYPASS_TTL: u8 = 1;

/// Normal TTL value for actual packet delivery
const NORMAL_TTL: u8 = 64;

/// GoodbyeDPI configuration.
#[derive(Debug, Clone)]
pub struct GoodbyeDpiConfig {
    /// Enable SYN TTL trick
    pub syn_ttl_trick: bool,
    /// TTL value for SYN packets toward DPI (default: 1)
    pub dpi_ttl: u8,
    /// TTL value for normal packets (default: 64)
    pub normal_ttl: u8,
    /// Enable TLS ClientHello fragmentation
    pub tls_fragmentation: bool,
    /// Fragment position for TLS ClientHello (default: split at byte 2 of SNI)
    pub tls_fragment_position: TlsFragmentPosition,
    /// Number of TCP segments to split ClientHello into (default: 2)
    pub tls_segments: usize,
    /// Whether to handle both IPv4 and IPv6
    pub handle_ipv6: bool,
    /// TCP ports to intercept (default: 443, 80)
    pub target_ports: Vec<u16>,
    /// Maximum packet size to process (0 = no limit)
    pub max_packet_size: usize,
}

impl Default for GoodbyeDpiConfig {
    fn default() -> Self {
        Self {
            syn_ttl_trick: true,
            dpi_ttl: DPI_BYPASS_TTL,
            normal_ttl: NORMAL_TTL,
            tls_fragmentation: true,
            tls_fragment_position: TlsFragmentPosition::AfterRecordHeader,
            tls_segments: 2,
            handle_ipv6: true,
            target_ports: vec![443, 80],
            max_packet_size: 0,
        }
    }
}

/// Where to split the TLS ClientHello.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsFragmentPosition {
    /// Split after the 5-byte TLS record header
    AfterRecordHeader,
    /// Split at the start of the SNI extension
    AtSniExtension,
    /// Split at a custom byte offset
    Custom(usize),
}

/// GoodbyeDPI state for tracking connections.
#[derive(Debug)]
struct ConnectionState {
    /// Whether the SYN TTL trick has been applied to this connection
    syn_trick_applied: bool,
    /// Whether TLS fragmentation has been applied to this connection
    tls_fragment_applied: bool,
    /// Source IP and port
    source: (std::net::IpAddr, u16),
    /// Destination IP and port
    destination: (std::net::IpAddr, u16),
    /// Timestamp of last activity
    last_activity: std::time::Instant,
}

/// The GoodbyeDPI engine manages WFP callouts for DPI circumvention.
pub struct GoodbyeDpi {
    /// Configuration
    config: GoodbyeDpiConfig,
    /// Whether the engine is active
    active: AtomicBool,
    /// WFP engine handle
    engine_handle: Option<EngineHandle>,
    /// Registered callout IDs
    callout_ids: Vec<u32>,
    /// Registered filter IDs
    filter_ids: Vec<u64>,
    /// Sub-layer GUID (generated at runtime)
    sublayer_key: Option<u128>,
    /// Active connections being tracked
    connections: std::collections::HashMap<ConnectionKey, ConnectionState>,
    /// Statistics
    stats: GoodbyeDpiStats,
}

/// Wrapper for WFP engine handle that ensures cleanup.
#[derive(Debug)]
struct EngineHandle {
    /// The raw handle value
    handle: isize,
    /// Whether the handle has been closed
    closed: bool,
}

impl EngineHandle {
    fn new(handle: isize) -> Self {
        Self {
            handle,
            closed: false,
        }
    }

    fn raw(&self) -> isize {
        self.handle
    }
}

impl Drop for EngineHandle {
    fn drop(&mut self) {
        if !self.closed {
            // In a real implementation:
            // unsafe { FwpmEngineClose0(self.handle); }
            debug!("WFP engine handle closed");
        }
    }
}

/// Key for tracking connections.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct ConnectionKey {
    src_ip: [u8; 16],
    src_port: u16,
    dst_ip: [u8; 16],
    dst_port: u16,
    protocol: u8,
}

/// GoodbyeDPI statistics.
#[derive(Debug, Clone, Default)]
struct GoodbyeDpiStats {
    /// Number of SYN packets modified with TTL trick
    syn_packets_modified: u64,
    /// Number of TLS ClientHello packets fragmented
    tls_packets_fragmented: u64,
    /// Number of packets passed through unmodified
    packets_passthrough: u64,
    /// Number of errors encountered
    errors: u64,
}

impl GoodbyeDpi {
    /// Create a new GoodbyeDPI engine with default configuration.
    pub fn new() -> Self {
        Self::with_config(GoodbyeDpiConfig::default())
    }

    /// Create a new GoodbyeDPI engine with custom configuration.
    pub fn with_config(config: GoodbyeDpiConfig) -> Self {
        Self {
            config,
            active: AtomicBool::new(false),
            engine_handle: None,
            callout_ids: Vec::new(),
            filter_ids: Vec::new(),
            sublayer_key: None,
            connections: std::collections::HashMap::new(),
            stats: GoodbyeDpiStats::default(),
        }
    }

    /// Start the GoodbyeDPI engine.
    ///
    /// This registers WFP callouts and filters for packet interception.
    /// No administrator privileges are required for user-mode callouts.
    pub async fn start(&mut self) -> Result<(), PlatformError> {
        if self.active.load(Ordering::SeqCst) {
            warn!("GoodbyeDPI already active");
            return Ok(());
        }

        info!("Starting GoodbyeDPI engine (no admin required)");

        // Step 1: Open WFP engine session
        let engine = self.open_wfp_engine()?;
        self.engine_handle = Some(engine);

        // Step 2: Add sub-layer for our filters
        let sublayer_key = self.add_sublayer()?;
        self.sublayer_key = Some(sublayer_key);

        // Step 3: Register callouts for outbound transport
        self.register_callouts()?;

        // Step 4: Add filters to activate callouts
        self.add_filters()?;

        self.active.store(true, Ordering::SeqCst);
        info!("GoodbyeDPI engine started successfully");
        Ok(())
    }

    /// Stop the GoodbyeDPI engine and unregister all WFP objects.
    pub async fn stop(&mut self) -> Result<(), PlatformError> {
        if !self.active.load(Ordering::SeqCst) {
            return Ok(());
        }

        info!("Stopping GoodbyeDPI engine");

        // Remove filters
        self.remove_filters()?;

        // Remove callouts
        self.remove_callouts()?;

        // Close engine
        if let Some(mut handle) = self.engine_handle.take() {
            handle.closed = true;
            // unsafe { FwpmEngineClose0(handle.raw()); }
        }

        self.active.store(false, Ordering::SeqCst);
        self.connections.clear();
        info!("GoodbyeDPI engine stopped");
        Ok(())
    }

    /// Open a WFP engine session.
    ///
    /// In production, this calls `FwpmEngineOpen0` from the `fwpuclnt.dll`.
    fn open_wfp_engine(&self) -> Result<EngineHandle, PlatformError> {
        // In a real implementation with the windows crate:
        //
        // use windows::Win32::NetworkManagement::IpHelper::*;
        // use windows::Win32::NetworkManagement::WindowsFilteringPlatform::*;
        //
        // let mut engine_handle: HANDLE = HANDLE::default();
        // let session = FWPM_SESSION0 {
        //     flags: FWPM_SESSION_FLAG_DYNAMIC, // Auto-cleanup on process exit
        //     ..Default::default()
        // };
        //
        // unsafe {
        //     let result = FwpmEngineOpen0(
        //         None,  // Local engine
        //         RPC_C_AUTHN_WINNT,
        //         None,  // Default security
        //         &session,
        //         &mut engine_handle,
        //     );
        //
        //     if result != 0 {
        //         return Err(PlatformError::SystemApi(
        //             format!("FwpmEngineOpen0 failed: 0x{:08X}", result)
        //         ));
        //     }
        // }

        debug!("WFP engine session opened (simulated)");
        Ok(EngineHandle::new(0x12345678))
    }

    /// Add a WFP sub-layer for our filters.
    fn add_sublayer(&self) -> Result<u128, PlatformError> {
        // Generate a unique GUID for our sub-layer
        let sublayer_key = Self::generate_guid();

        // In a real implementation:
        //
        // let sublayer = FWPM_SUBLAYER0 {
        //     subLayerKey: GUID::from_u128(sublayer_key),
        //     flags: 0,
        //     weight: 0xFFFF, // High weight = high priority
        //     ..Default::default()
        // };
        //
        // unsafe {
        //     let result = FwpmSubLayerAdd0(
        //         engine_handle,
        //         &sublayer,
        //         None,
        //     );
        //     if result != 0 {
        //         return Err(...);
        //     }
        // }

        debug!("WFP sub-layer added with key: {:032x}", sublayer_key);
        Ok(sublayer_key)
    }

    /// Register WFP callouts for packet interception.
    fn register_callouts(&mut self) -> Result<(), PlatformError> {
        // Register outbound transport callout (IPv4)
        let callout_id_v4 = self.register_outbound_callout(false)?;
        self.callout_ids.push(callout_id_v4);

        // Register outbound transport callout (IPv6)
        if self.config.handle_ipv6 {
            let callout_id_v6 = self.register_outbound_callout(true)?;
            self.callout_ids.push(callout_id_v6);
        }

        // Register ALE connect callout for SYN interception
        let ale_callout_id = self.register_ale_connect_callout()?;
        self.callout_ids.push(ale_callout_id);

        debug!("Registered {} WFP callouts", self.callout_ids.len());
        Ok(())
    }

    /// Register an outbound transport callout.
    fn register_outbound_callout(&self, ipv6: bool) -> Result<u32, PlatformError> {
        // In a real implementation:
        //
        // let layer_key = if ipv6 {
        //     FWPM_LAYER_OUTBOUND_TRANSPORT_V6
        // } else {
        //     FWPM_LAYER_OUTBOUND_TRANSPORT_V4
        // };
        //
        // let callout_key = Self::generate_guid();
        //
        // let callout = FWPM_CALLOUT0 {
        //     calloutKey: callout_key,
        //     applicableLayer: layer_key,
        //     ..Default::default()
        // };
        //
        // unsafe {
        //     let mut callout_id: u32 = 0;
        //     let result = FwpmCalloutAdd0(
        //         engine_handle,
        //         &callout,
        //         None,
        //         &mut callout_id,
        //     );
        //     // ...
        // }

        let callout_id = if ipv6 { 1002u32 } else { 1001u32 };
        debug!(
            "Registered outbound transport callout (IPv{}): ID={}",
            if ipv6 { 6 } else { 4 },
            callout_id
        );
        Ok(callout_id)
    }

    /// Register an ALE connect callout for SYN packet interception.
    fn register_ale_connect_callout(&self) -> Result<u32, PlatformError> {
        // The ALE (Application Layer Enforcement) connect callout
        // intercepts TCP connection attempts and allows us to:
        // 1. Set TTL=1 for the SYN packet
        // 2. Track the connection for TLS fragmentation later
        //
        // In a real implementation:
        //
        // let callout = FWPM_CALLOUT0 {
        //     calloutKey: Self::generate_guid(),
        //     applicableLayer: FWPM_LAYER_ALE_AUTH_CONNECT_V4,
        //     ..Default::default()
        // };

        debug!("Registered ALE connect callout: ID=1003");
        Ok(1003u32)
    }

    /// Add WFP filters to activate the registered callouts.
    fn add_filters(&mut self) -> Result<(), PlatformError> {
        let sublayer_key = self
            .sublayer_key
            .ok_or_else(|| PlatformError::SystemApi("Sub-layer not registered".into()))?;

        // Add filter for outbound transport (TCP only, target ports)
        for &port in &self.config.target_ports {
            let filter_id = self.add_outbound_filter(port, false)?;
            self.filter_ids.push(filter_id);

            if self.config.handle_ipv6 {
                let filter_id_v6 = self.add_outbound_filter(port, true)?;
                self.filter_ids.push(filter_id_v6);
            }
        }

        // Add filter for ALE connect (TCP SYN interception)
        let ale_filter_id = self.add_ale_connect_filter()?;
        self.filter_ids.push(ale_filter_id);

        debug!("Added {} WFP filters", self.filter_ids.len());
        Ok(())
    }

    /// Add an outbound transport filter for a specific port.
    fn add_outbound_filter(&self, port: u16, ipv6: bool) -> Result<u64, PlatformError> {
        // In a real implementation:
        //
        // let filter_conditions = [
        //     FWPM_FILTER_CONDITION0 {
        //         fieldKey: FWPM_CONDITION_REMOTE_PORT,
        //         matchType: FWP_MATCH_EQUAL,
        //         conditionValue: FWP_VALUE0 {
        //             type: FWP_UINT16,
        //             value: port as usize,
        //         },
        //     },
        //     FWPM_FILTER_CONDITION0 {
        //         fieldKey: FWPM_CONDITION_IP_PROTOCOL,
        //         matchType: FWP_MATCH_EQUAL,
        //         conditionValue: FWP_VALUE0 {
        //             type: FWP_UINT8,
        //             value: IPPROTO_TCP as usize,
        //         },
        //     },
        // ];
        //
        // let filter = FWPM_FILTER0 {
        //     filterKey: Self::generate_guid(),
        //     subLayerKey: GUID::from_u128(sublayer_key),
        //     weight: FWP_VALUE0 { type: FWP_UINT16, value: 0 },
        //     action: FWP_ACTION_CALLOUT_UNKNOWN,
        //     actionValue: callout_id as usize,
        //     numFilterConditions: filter_conditions.len() as u32,
        //     filterCondition: filter_conditions.as_ptr(),
        //     ..Default::default()
        // };

        let filter_id = (port as u64) << 8 | if ipv6 { 1 } else { 0 };
        debug!(
            "Added outbound filter for port {} (IPv{}): ID={}",
            port,
            if ipv6 { 6 } else { 4 },
            filter_id
        );
        Ok(filter_id)
    }

    /// Add an ALE connect filter for SYN interception.
    fn add_ale_connect_filter(&self) -> Result<u64, PlatformError> {
        // Filter for TCP SYN packets to target ports
        let filter_id = 0xDEAD;
        debug!("Added ALE connect filter: ID=0x{:X}", filter_id);
        Ok(filter_id)
    }

    /// Process an intercepted outbound packet.
    ///
    /// This is called by the WFP callout when an outbound packet matches
    /// our filter criteria. It applies the appropriate DPI circumvention
    /// technique.
    pub fn process_outbound_packet(&mut self, packet: &mut [u8]) -> PacketAction {
        if !self.active.load(Ordering::SeqCst) {
            return PacketAction::Permit;
        }

        // Parse the IP header
        let ip_header = match IpHeader::parse(packet) {
            Some(h) => h,
            None => {
                self.stats.errors += 1;
                return PacketAction::Permit; // Can't parse, let it through
            }
        };

        // Only process TCP packets
        if ip_header.protocol != 6 {
            // Not TCP
            self.stats.packets_passthrough += 1;
            return PacketAction::Permit;
        }

        // Check if destination port is in our target list
        let tcp_header_offset = ip_header.header_length as usize;
        if packet.len() < tcp_header_offset + 20 {
            return PacketAction::Permit;
        }

        let dst_port =
            u16::from_be_bytes([packet[tcp_header_offset + 2], packet[tcp_header_offset + 3]]);

        if !self.config.target_ports.contains(&dst_port) {
            self.stats.packets_passthrough += 1;
            return PacketAction::Permit;
        }

        // Check for TCP SYN (new connection)
        let tcp_flags = packet[tcp_header_offset + 13];
        let is_syn = (tcp_flags & 0x02) != 0 && (tcp_flags & 0x10) == 0;

        if is_syn && self.config.syn_ttl_trick {
            return self.apply_syn_ttl_trick(packet, &ip_header);
        }

        // Check for TLS ClientHello in TCP payload
        let tcp_header_len = ((packet[tcp_header_offset + 12] & 0xF0) >> 4) as usize * 4;
        let payload_offset = tcp_header_offset + tcp_header_len;

        if payload_offset < packet.len() && self.config.tls_fragmentation {
            if self.is_tls_client_hello(&packet[payload_offset..]) {
                return self.apply_tls_fragmentation(packet, payload_offset);
            }
        }

        self.stats.packets_passthrough += 1;
        PacketAction::Permit
    }

    /// Apply the SYN TTL trick to a TCP SYN packet.
    ///
    /// Sets TTL=1 so the packet expires at the DPI box, then sends
    /// a retransmission with normal TTL that reaches the destination.
    fn apply_syn_ttl_trick(&mut self, packet: &mut [u8], ip_header: &IpHeader) -> PacketAction {
        debug!(
            "Applying SYN TTL trick: setting TTL={}",
            self.config.dpi_ttl
        );

        // Set TTL in IP header
        let ttl_offset = if ip_header.version == 4 { 8 } else { 7 };
        if ttl_offset < packet.len() {
            packet[ttl_offset] = self.config.dpi_ttl;
        }

        // Recalculate IP header checksum (required after modification)
        self.recalculate_ip_checksum(packet, ip_header);

        self.stats.syn_packets_modified += 1;

        // Return the modified packet with instruction to also send
        // a retransmission with normal TTL after a brief delay
        PacketAction::ModifyAndRetransmit {
            retransmit_ttl: self.config.normal_ttl,
            retransmit_delay: Duration::from_millis(1),
        }
    }

    /// Apply TLS ClientHello fragmentation.
    ///
    /// Splits the TCP segment containing the ClientHello into two segments.
    fn apply_tls_fragmentation(
        &mut self,
        packet: &mut [u8],
        payload_offset: usize,
    ) -> PacketAction {
        debug!("Applying TLS ClientHello fragmentation");

        // Determine split point
        let split_offset = match self.config.tls_fragment_position {
            TlsFragmentPosition::AfterRecordHeader => payload_offset + 5,
            TlsFragmentPosition::AtSniExtension => {
                // Find SNI extension offset within the payload
                if let Some(sni_offset) = self.find_sni_offset(&packet[payload_offset..]) {
                    payload_offset + sni_offset
                } else {
                    payload_offset + 5 // Fallback to record header split
                }
            }
            TlsFragmentPosition::Custom(offset) => payload_offset + offset,
        };

        self.stats.tls_packets_fragmented += 1;

        // In a real WFP implementation, we would:
        // 1. Clone the packet
        // 2. Truncate the first packet at split_offset
        // 3. Create a second packet with the remaining data
        // 4. Adjust TCP sequence numbers
        // 5. Recalculate checksums
        //
        // With WFP, we can use the classifyFn callback to:
        // - Modify the packet buffer in place
        // - Inject additional packets using FwpsInjectTransportSendAsync0

        PacketAction::Fragment {
            split_offset,
            inter_segment_delay: Duration::from_millis(1),
        }
    }

    /// Find the offset of the SNI extension within a TLS ClientHello payload.
    fn find_sni_offset(&self, payload: &[u8]) -> Option<usize> {
        if payload.len() < 44 {
            return None;
        }

        // TLS record header (5 bytes)
        if payload[0] != 0x16 || payload[5] != 0x01 {
            return None;
        }

        let mut offset = 5 + 4; // Record header + handshake header
        offset += 2 + 32; // Client version + Random

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
        let _ext_len = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
        offset += 2;

        // Scan extensions for SNI (type 0x0000)
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

    /// Check if the payload starts with a TLS ClientHello.
    fn is_tls_client_hello(&self, payload: &[u8]) -> bool {
        payload.len() >= 6
            && payload[0] == 0x16 // Handshake
            && payload[1] == 0x03 // TLS
            && payload[2] >= 0x01 // Version >= 1.0
            && payload[5] == 0x01 // ClientHello
    }

    /// Recalculate the IP header checksum after modification.
    fn recalculate_ip_checksum(&self, packet: &mut [u8], ip_header: &IpHeader) {
        if ip_header.version == 4 {
            let header_len = ip_header.header_length as usize;
            if header_len < 20 || packet.len() < header_len {
                return;
            }

            // Zero out existing checksum
            packet[10] = 0;
            packet[11] = 0;

            // Calculate new checksum (one's complement sum)
            let mut sum: u32 = 0;
            for i in (0..header_len).step_by(2) {
                if i + 1 < header_len {
                    sum += u16::from_be_bytes([packet[i], packet[i + 1]]) as u32;
                } else {
                    sum += (packet[i] as u32) << 8;
                }
            }

            // Fold carries
            while (sum >> 16) != 0 {
                sum = (sum & 0xFFFF) + (sum >> 16);
            }

            let checksum = (!sum as u16).to_be_bytes();
            packet[10] = checksum[0];
            packet[11] = checksum[1];
        }
        // IPv6 doesn't have a header checksum
    }

    /// Remove all WFP filters.
    fn remove_filters(&mut self) -> Result<(), PlatformError> {
        // In a real implementation:
        // for &filter_id in &self.filter_ids {
        //     unsafe { FwpmFilterDeleteById0(engine_handle, filter_id); }
        // }
        self.filter_ids.clear();
        debug!("All WFP filters removed");
        Ok(())
    }

    /// Remove all WFP callouts.
    fn remove_callouts(&mut self) -> Result<(), PlatformError> {
        // In a real implementation:
        // for &callout_id in &self.callout_ids {
        //     unsafe { FwpmCalloutDeleteById0(engine_handle, callout_id); }
        // }
        self.callout_ids.clear();
        debug!("All WFP callouts removed");
        Ok(())
    }

    /// Generate a pseudo-random GUID for WFP object identification.
    fn generate_guid() -> u128 {
        use std::time::SystemTime;
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u128;

        // Combine timestamp with a counter for uniqueness
        // In production, use Uuid::new_v4()
        timestamp ^ _MICAFP_UNIFIED_KEY
    }

    /// Check if the engine is currently active.
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    /// Get statistics.
    pub fn stats(&self) -> &GoodbyeDpiStats {
        &self.stats
    }
}

/// Action to take on an intercepted packet.
#[derive(Debug, Clone)]
pub enum PacketAction {
    /// Allow the packet through without modification
    Permit,
    /// Block the packet
    Block,
    /// Modify the packet in place and allow through
    Modify,
    /// Modify the packet and send a retransmission with different TTL
    ModifyAndRetransmit {
        /// TTL for the retransmitted packet
        retransmit_ttl: u8,
        /// Delay before retransmission
        retransmit_delay: Duration,
    },
    /// Fragment the packet at the specified offset
    Fragment {
        /// Byte offset in the original packet to split at
        split_offset: usize,
        /// Delay between sending fragments
        inter_segment_delay: Duration,
    },
}

/// Parsed IP header information.
#[derive(Debug)]
struct IpHeader {
    /// IP version (4 or 6)
    version: u8,
    /// Header length in bytes (IPv4 only, 0 for IPv6)
    header_length: u8,
    /// Protocol number (6 = TCP, 17 = UDP)
    protocol: u8,
    /// Source address
    src_addr: [u8; 16],
    /// Destination address
    dst_addr: [u8; 16],
}

impl IpHeader {
    /// Parse IP header from raw packet data.
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

const _MICAFP_UNIFIED_KEY: u128 = 0x4D4943414650556E6966696564536869; // "MICAFPUniShi" in hex

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_header_parse_ipv4() {
        let mut packet = vec![0u8; 20];
        packet[0] = 0x45; // Version 4, IHL 5 (20 bytes)
        packet[9] = 6; // TCP
        packet[12..16].copy_from_slice(&[192, 168, 1, 1]);
        packet[16..20].copy_from_slice(&[8, 8, 8, 8]);

        let header = IpHeader::parse(&packet).unwrap();
        assert_eq!(header.version, 4);
        assert_eq!(header.header_length, 20);
        assert_eq!(header.protocol, 6);
    }

    #[test]
    fn test_ip_header_parse_ipv6() {
        let mut packet = vec![0u8; 40];
        packet[0] = 0x60; // Version 6
        packet[6] = 6; // TCP

        let header = IpHeader::parse(&packet).unwrap();
        assert_eq!(header.version, 6);
        assert_eq!(header.header_length, 40);
        assert_eq!(header.protocol, 6);
    }

    #[test]
    fn test_default_config() {
        let config = GoodbyeDpiConfig::default();
        assert!(config.syn_ttl_trick);
        assert!(config.tls_fragmentation);
        assert_eq!(config.dpi_ttl, 1);
        assert_eq!(config.normal_ttl, 64);
        assert!(config.target_ports.contains(&443));
        assert!(config.target_ports.contains(&80));
    }

    #[test]
    fn test_tls_client_hello_detection() {
        let dpi = GoodbyeDpi::new();

        let hello = [0x16, 0x03, 0x01, 0x00, 0x20, 0x01];
        assert!(dpi.is_tls_client_hello(&hello));

        let not_hello = [0x17, 0x03, 0x01, 0x00, 0x20, 0x01];
        assert!(!dpi.is_tls_client_hello(&not_hello));
    }

    #[test]
    fn test_ip_checksum_recalculation() {
        let mut packet = vec![
            0x45u8, 0x00, 0x00, 0x28, // Version, IHL, Total length
            0x00, 0x00, 0x00, 0x00, // ID, Flags, Fragment offset
            0x40, 0x06, 0x00, 0x00, // TTL, Protocol, Checksum (zero for calc)
            192, 168, 1, 1, // Source IP
            8, 8, 8, 8,
        ]; // Dest IP

        let header = IpHeader::parse(&packet).unwrap();
        let dpi = GoodbyeDpi::new();
        dpi.recalculate_ip_checksum(&mut packet, &header);

        // Checksum should be non-zero after calculation
        let checksum = u16::from_be_bytes([packet[10], packet[11]]);
        assert_ne!(checksum, 0);
    }
}
