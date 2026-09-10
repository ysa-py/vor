# Vor — universal OpenWrt package.
#
# Builds `vor` (the router DPI-evasion daemon: loopback SNI-fragmentation
# relay, portable POSIX C, no external library dependencies) plus the LuCI
# app. This package is architecture-independent in the source sense: the
# SAME package compiles for every OpenWrt target (mipsel, mips, aarch64,
# arm, x86_64, ...) via the SDK — CI builds the full matrix and attaches
# every .ipk to the release (see .github/workflows/release.yml).
#
# Usage inside an OpenWrt buildroot / SDK:
#   make package/vor/compile V=s

include $(TOPDIR)/rules.mk

PKG_NAME:=vor
PKG_VERSION:=1.0.0
PKG_RELEASE:=1
PKG_LICENSE:=GPL-3.0
PKG_MAINTAINER:=Vor maintainers

PKG_BUILD_DEPENDS:=

include $(INCLUDE_DIR)/package.mk

define Package/vor
  SECTION:=net
  CATEGORY:=Network
  TITLE:=Vor DPI-evasion router engine
  DEPENDS:=+libpthread
  MENU:=1
endef

define Package/vor/description
  Vor router engine: a loopback SNI-fragmentation relay (the "Edge Bridge"
  technique) with the shared fragment-strategy planner (sni_split,
  record_split, tls_record_frag, full5/10/20, multi64, sni_boundary, ...).
  Configure via UCI /etc/config/vor or LuCI. Splits TLS ClientHellos into
  well-formed records before forwarding to the configured edge so DPI
  reassembly fails while the server stream stays valid.
endef

define Package/vor/conffiles
/etc/config/vor
endef

define Build/Prepare
	$(call Build/Prepare/Default)
endef

define Build/Compile
	$(TARGET_CC) $(TARGET_CFLAGS) $(TARGET_LDFLAGS) \
		-DVOR_OPENWRT=1 \
		$(PKG_BUILD_DIR)/vor_main.c \
		-o $(PKG_BUILD_DIR)/vor
endef

define Package/vor/install
	$(INSTALL_DIR) $(1)/usr/bin
	$(INSTALL_BIN) $(PKG_BUILD_DIR)/vor $(1)/usr/bin/vor
	$(INSTALL_DIR) $(1)/etc/config
	$(INSTALL_CONF) ./files/etc/config/vor $(1)/etc/config/vor
	$(INSTALL_DIR) $(1)/etc/init.d
	$(INSTALL_BIN) ./files/etc/init.d/vor $(1)/etc/init.d/vor
	$(INSTALL_DIR) $(1)/lib/netifd/proto
	$(INSTALL_BIN) ./files/lib/netifd/proto/vor.sh $(1)/lib/netifd/proto/vor.sh
endef

define Package/vor/postinst
#!/bin/sh
[ -n "$${IPKG_INSTROOT}" ] || {
	/etc/init.d/vor enable
}
exit 0
endef

$(eval $(call BuildPackage,vor))
# luci-app-vor is a SEPARATE package (src/luci-app-vor/Makefile, built via
# the luci feed) — it is copied to package/luci-app-vor by CI, not eval'd
# here (a dangling BuildPackage eval without a define breaks the SDK scan).
