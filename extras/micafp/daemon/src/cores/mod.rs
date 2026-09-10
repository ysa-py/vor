pub mod amneziavpn;
pub mod core_manager;
pub mod defyx;
pub mod hiddify;
pub mod lantern;
pub mod mahsang;
pub mod moav;
pub mod psiphon;
pub mod singbox;
pub mod xray;

pub use amneziavpn::AmneziaVpnAdapter;
pub use core_manager::CoreManager;
pub use defyx::DefyxVpnAdapter;
pub use hiddify::HiddifyCoreAdapter;
pub use lantern::LanternAdapter;
pub use mahsang::MahsangAdapter;
pub use moav::MoavAdapter;
pub use psiphon::PsiphonAdapter;
pub use singbox::SingboxCoreAdapter;
pub use xray::XrayCoreAdapter;

// Type aliases for convenience
pub type SingboxCore = SingboxCoreAdapter;
pub type SingBoxCore = SingboxCoreAdapter;
pub type XrayCore = XrayCoreAdapter;
pub type HiddifyCore = HiddifyCoreAdapter;
pub type PsiphonCore = PsiphonAdapter;
pub type LanternCore = LanternAdapter;
pub type AmneziaVpnCore = AmneziaVpnAdapter;
pub type DefyxCore = DefyxVpnAdapter;
pub type MahsangCore = MahsangAdapter;
pub type MoavCore = MoavAdapter;
