# =============================================================================
# MICAFP-UnifiedShield-6.0 — OpenWrt Package Makefile
#
# Build the UnifiedShield daemon and LuCI web interface for OpenWrt.
#
# Supported targets:
#   mipsel_24kc     — Most common MIPS routers (TP-Link, D-Link, etc.)
#   x86_64         — x86 routers and VMs
#   aarch64_cortex-a53 — ARM routers (NanoPi, etc.)
#
# Usage:
#   make package/shield/compile V=s
#
# Prerequisites:
#   - OpenWrt SDK for the target architecture
#   - Rust cross-compilation toolchain installed via rustup
# =============================================================================

include $(TOPDIR)/rules.mk

# ---------------------------------------------------------------------------
# Package definition
# ---------------------------------------------------------------------------

PKG_NAME:=shield
PKG_VERSION:=6.0.0
PKG_RELEASE:=1

PKG_MAINTAINER:=MICAFP Team <team@micAFP.org>
PKG_LICENSE:=MIT

PKG_SOURCE_PROTO:=git
PKG_SOURCE_URL:=https://github.com/MICAFP/UnifiedShield.git
PKG_SOURCE_VERSION:=v$(PKG_VERSION)
PKG_MIRROR_HASH:=skip

PKG_BUILD_DEPENDS:=rust/host

include $(INCLUDE_DIR)/package.mk

# ---------------------------------------------------------------------------
# Package metadata
# ---------------------------------------------------------------------------

define Package/shield
  SECTION:=net
  CATEGORY:=Network
  TITLE:=UnifiedShield Anti-Censorship VPN Daemon
  DEPENDS:=+libc +libpthread +ca-bundle +libopenssl +kmod-tun
  PKGARCH:=all
endef

define Package/shield/description
  UnifiedShield is an anti-censorship VPN designed for Iran.
  It uses WebTransport/HTTPS tunnels through CDN workers with
  traffic obfuscation to bypass DPI and the national firewall.

  Features:
  - WebTransport/HTTPS tunneling through CDN workers
  - Traffic obfuscation (WASM + XOR + rotation)
  - TLS fragmentation to defeat SNI-based filtering
  - Adversarial traffic shaping (AI-based)
  - P2P mesh via Yggdrasil overlay
  - Covert channels (NTP, acoustic, BLE mesh, WiFi NAN, SMS)
  - LuCI web management interface
endef

define Package/shield/conffiles
/etc/config/shield
/etc/shield/cdn-endpoints.json
/etc/shield/p2p-bootstrap-peers.json
/etc/shield/isp-profiles.json
endef

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------

# Rust target mapping for OpenWrt architectures
ifeq ($(CONFIG_ARCH),mipsel)
  RUST_TARGET:=mipsel-unknown-linux-musl
else ifeq ($(CONFIG_ARCH),x86_64)
  RUST_TARGET:=x86_64-unknown-linux-musl
else ifeq ($(CONFIG_ARCH),aarch64)
  RUST_TARGET:=aarch64-unknown-linux-musl
else
  RUST_TARGET:=mipsel-unknown-linux-musl
endif

# Cargo environment
CARGO_HOME:=$(STAGING_DIR_HOSTPKG)/cargo
RUSTC:=$(STAGING_DIR_HOSTPKG)/rust/bin/rustc
CARGO:=$(STAGING_DIR_HOSTPKG)/rust/bin/cargo

define Build/Compile
	# Set up cross-compilation environment
	mkdir -p $(PKG_BUILD_DIR)/daemon/.cargo
	echo '[target.$(RUST_TARGET)]' > $(PKG_BUILD_DIR)/daemon/.cargo/config.toml
	echo 'linker = "$(TARGET_CC)"' >> $(PKG_BUILD_DIR)/daemon/.cargo/config.toml

	# Build the daemon with release optimizations
	cd $(PKG_BUILD_DIR)/daemon && \
		CARGO_HOME=$(CARGO_HOME) \
		RUSTFLAGS="-C target-feature=+crt-static -C opt-level=z -C panic=abort" \
		$(CARGO) build \
			--release \
			--target $(RUST_TARGET) \
			--no-default-features \
			--features openwrt \
			--manifest-path $(PKG_BUILD_DIR)/daemon/Cargo.toml
endef

# ---------------------------------------------------------------------------
# Install
# ---------------------------------------------------------------------------

define Package/shield/install
	# Daemon binary
	$(INSTALL_DIR) $(1)/usr/bin
	$(INSTALL_BIN) \
		$(PKG_BUILD_DIR)/daemon/target/$(RUST_TARGET)/release/shield-daemon \
		$(1)/usr/bin/shield-daemon

	# Configuration files
	$(INSTALL_DIR) $(1)/etc/config
	$(INSTALL_CONF) ./files/etc/config/shield $(1)/etc/config/shield

	$(INSTALL_DIR) $(1)/etc/shield
	$(INSTALL_CONF) $(PKG_BUILD_DIR)/configs/cdn-endpoints.json $(1)/etc/shield/
	$(INSTALL_CONF) $(PKG_BUILD_DIR)/configs/p2p-bootstrap-peers.json $(1)/etc/shield/
	$(INSTALL_CONF) $(PKG_BUILD_DIR)/configs/isp-profiles.json $(1)/etc/shield/

	# Init script
	$(INSTALL_DIR) $(1)/etc/init.d
	$(INSTALL_BIN) ./files/etc/init.d/shield $(1)/etc/init.d/shield

	# LuCI web interface
	$(INSTALL_DIR) $(1)/usr/lib/lua/luci/controller
	$(INSTALL_DATA) $(PKG_BUILD_DIR)/openwrt/files/usr/lib/lua/luci/controller/shield.lua \
		$(1)/usr/lib/lua/luci/controller/shield.lua

	$(INSTALL_DIR) $(1)/usr/lib/lua/luci/view/shield
	$(INSTALL_DATA) ./files/usr/lib/lua/luci/view/shield/status.htm \
		$(1)/usr/lib/lua/luci/view/shield/status.htm

	$(INSTALL_DIR) $(1)/usr/lib/lua/luci/model/cbi/shield
	$(INSTALL_DATA) ./files/usr/lib/lua/luci/model/cbi/shield/config.lua \
		$(1)/usr/lib/lua/luci/model/cbi/shield/config.lua

	# Firewall integration
	$(INSTALL_DIR) $(1)/etc/firewall.d
	$(INSTALL_BIN) ./files/etc/firewall.d/shield $(1)/etc/firewall.d/shield
endef

define Package/shield/postinst
#!/bin/sh
[ -n "$${IPKG_INSTROOT}" ] || {
	# Enable the service on install
	/etc/init.d/shield enable
	# Add shield user (non-privileged)
	id shield >/dev/null 2>&1 || \
		echo "Adding shield system user" && \
		useradd -r -s /bin/false -d /var/run/shield shield 2>/dev/null || true
}
endef

define Package/shield/prerm
#!/bin/sh
[ -n "$${IPKG_INSTROOT}" ] || {
	# Stop and disable the service
	/etc/init.d/shield stop 2>/dev/null || true
	/etc/init.d/shield disable 2>/dev/null || true
}
endef

$(eval $(call BuildPackage,shield))
