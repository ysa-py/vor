-- LuCI CBI Model for UnifiedShield Configuration
-- Form definition at: /admin/services/unifiedshield/config

local m, s, o

m = Map("unifiedshield", translate("UnifiedShield VPN"),
    translate("Next-gen anti-censorship VPN for Iran. " ..
              "Supports Xray, NaïveProxy, Hysteria2, and TUIC cores. " ..
              "DNS uses Alibaba/Tencent CDN (Cloudflare is blocked in Iran)."))

-- Main configuration section
s = m:section(TypedSection, "unifiedshield", translate("General Settings"))
s.anonymous = true
s.addremove = false

-- Enable/Disable
o = s:option(Flag, "enabled", translate("Enable"),
    translate("Enable UnifiedShield VPN service"))
o.default = "0"
o.rmempty = false

-- Protocol core selection
o = s:option(ListValue, "core", translate("Protocol Core"),
    translate("VPN protocol core. Auto-switch activates when DPI score > 0.72"))
o:value("xray", "Xray (VLESS/VMess - Recommended)")
o:value("naive", "NaïveProxy (HTTP/2 - Anti-DPI)")
o:value("hysteria2", "Hysteria2 (QUIC - Fast)")
o:value("tuic", "TUIC (QUIC - Low overhead)")
o.default = "xray"
o.rmempty = false

-- Auto core switch
o = s:option(Flag, "auto_core_switch", translate("Auto Core Switch"),
    translate("Automatically switch core when DPI is detected (score > 0.72)"))
o.default = "1"
o.rmempty = false

-- Server settings
s = m:section(TypedSection, "unifiedshield", translate("Server Configuration"))
s.anonymous = true
s.addremove = false

o = s:option(Value, "server", translate("Server Address"),
    translate("VPN server hostname or IP address"))
o.datatype = "host"
o.rmempty = false

o = s:option(Value, "server_port", translate("Server Port"),
    translate("VPN server port"))
o.datatype = "port"
o.default = "443"
o.rmempty = false

o = s:option(Value, "password", translate("Password"),
    translate("Authentication password or UUID"))
o.password = true
o.rmempty = false

-- Network settings
s = m:section(TypedSection, "unifiedshield", translate("Network Settings"))
s.anonymous = true
s.addremove = false

o = s:option(Value, "tun_name", translate("TUN Device Name"),
    translate("Name for the TUN interface"))
o.default = "us0"
o.rmempty = false

o = s:option(Value, "mtu", translate("MTU"),
    translate("Maximum Transmission Unit"))
o.datatype = "uinteger"
o.default = "1380"
o.rmempty = false

o = s:option(Value, "ip_address", translate("TUN IP Address"),
    translate("Local IP address for the TUN interface"))
o.default = "172.19.0.1"
o.datatype = "ip4addr"
o.rmempty = false

o = s:option(Value, "ip_prefix", translate("IP Prefix Length"),
    translate("Subnet prefix length"))
o.datatype = "uinteger"
o.default = "24"
o.rmempty = false

-- DNS settings
s = m:section(TypedSection, "unifiedshield", translate("DNS Settings"))
s.anonymous = true
s.addremove = false

o = s:option(ListValue, "dns_server", translate("Primary DNS"),
    translate("Chinese CDN DNS (Cloudflare is BLOCKED in Iran)"))
o:value("223.5.5.5", "Alibaba DNS (223.5.5.5)")
o:value("119.29.29.29", "Tencent DNS (119.29.29.29)")
o:value("1.12.12.12", "Tencent Backup (1.12.12.12)")
o.default = "223.5.5.5"
o.rmempty = false

o = s:option(ListValue, "dns_server_backup", translate("Backup DNS"),
    translate("Secondary DNS server"))
o:value("119.29.29.29", "Tencent DNS (119.29.29.29)")
o:value("1.12.12.12", "Tencent Backup (1.12.12.12)")
o:value("223.5.5.5", "Alibaba DNS (223.5.5.5)")
o.default = "119.29.29.29"
o.rmempty = false

-- Security settings
s = m:section(TypedSection, "unifiedshield", translate("Security Settings"))
s.anonymous = true
s.addremove = false

o = s:option(Flag, "kill_switch", translate("Kill Switch"),
    translate("Block all traffic if VPN disconnects unexpectedly"))
o.default = "1"
o.rmempty = false

o = s:option(Flag, "split_tunnel", translate("Split Tunnel"),
    translate("Exclude Iranian IPs from VPN for local service access (banking, government)"))
o.default = "1"
o.rmempty = false

o = s:option(Value, "dpi_threshold", translate("DPI Detection Threshold"),
    translate("DPI score threshold for auto core switch (0.0-1.0)"))
o.datatype = "float"
o.default = "0.72"
o.rmempty = false

o = s:option(ListValue, "obfuscation_level", translate("Obfuscation Level"),
    translate("Traffic obfuscation level (higher = more resistant but slower)"))
o:value("0", "0 - None")
o:value("1", "1 - Low (Default)")
o:value("2", "2 - Medium")
o:value("3", "3 - Maximum")
o.default = "1"
o.rmempty = false

-- Split tunnel excluded IPs
s = m:section(TypedSection, "unifiedshield", translate("Split Tunnel - Excluded Iranian IPs"))
s.anonymous = true
s.addremove = false

o = s:option(DynamicList, "excluded_ip", translate("Excluded IP Ranges"),
    translate("Iranian IP ranges that bypass the VPN. " ..
              "These allow direct access to banking and government services."))
o.datatype = "cidr4"
o.rmempty = true

return m
