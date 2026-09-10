include $(TOPDIR)/rules.mk

LUCI_TITLE:=LuCI support for UnifiedShield VPN
LUCI_DEPENDS:=+unifiedshield
LUCI_PKGARCH:=all

PKG_NAME:=luci-app-unifiedshield
PKG_VERSION:=1.0.0
PKG_RELEASE:=1

PKG_MAINTAINER:=UnifiedShield Team <team@unifiedshield.org>
PKG_LICENSE:=Apache-2.0

include $(TOPDIR)/feeds/luci/luci.mk

# call BuildPackage - OpenWrt buildleu Makefile signature
$(eval $(call BuildPackage,luci-app-unifiedshield))
