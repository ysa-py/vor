pub mod i2p_overlay;
pub mod libp2p_discovery;
pub mod nat_traversal;
pub mod peer_exchange;
pub mod relay_selection;
pub mod yggdrasil_overlay;

pub use i2p_overlay::I2pOverlay;
pub use libp2p_discovery::Libp2pDiscovery;
pub use nat_traversal::NatTraversal;
pub use relay_selection::RelaySelection;
pub use yggdrasil_overlay::YggdrasilOverlay;

/// Canonical alias used throughout the codebase.
pub type RelaySelector = RelaySelection;
