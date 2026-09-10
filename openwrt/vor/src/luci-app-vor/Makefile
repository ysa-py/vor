# LuCI app for Vor (adapted from MICAFP's luci-app-vor).
include $(TOPDIR)/rules.mk

PKG_NAME:=luci-app-vor
PKG_VERSION:=1.0.0
PKG_RELEASE:=1
PKG_LICENSE:=GPL-3.0

LUCI_TITLE:=Vor DPI-evasion router engine — LuCI interface
LUCI_DEPENDS:=+vor
LUCI_PKGARCH:=all

include $(TOPDIR)/feeds/luci/luci.mk

# call BuildPackage - OpenWrt buildroot signature
