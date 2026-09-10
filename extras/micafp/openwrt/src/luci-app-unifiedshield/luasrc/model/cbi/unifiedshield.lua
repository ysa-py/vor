-- UnifiedShield LuCI CBI Model
-- Configuration UI for the UnifiedShield web interface

local m, s, o

m = Map("unifiedshield", translate("UnifiedShield"),
    translate("UnifiedShield is a next-generation censorship circumvention tool " ..
              "using Chinese CDN relays to bypass internet restrictions in Iran and other " ..
              "heavily censored regions. It uses X25519 ECDH key exchange and ChaCha20-Poly1305 " ..
              "encryption for secure relay connections."))

-- Main configuration section
s = m:section(TypedSection, "unifiedshield", translate("General Settings"))
s.anonymous = true
s.addremove = false

-- Enable/Disable
o = s:option(Flag, "enabled", translate("Enable"),
    translate("Enable or disable the UnifiedShield service"))
o.rmempty = false
o.default = "1"

-- CDN Core selection
o = s:option(ListValue, "core", translate("CDN Core"),
    translate("Select which Chinese CDN provider to use as relay. " ..
              "Different providers may have different availability in your region."))
o:value("alibaba", translate("Alibaba Cloud (Function Compute)"))
o:value("tencent", translate("Tencent Cloud (SCF)"))
o:value("baidu", translate("Baidu Cloud (CFC)"))
o:value("huawei", translate("Huawei Cloud (FG)"))
o:value("deno", translate("Deno Deploy (VPS)"))
o.default = "alibaba"
o.rmempty = false

-- Server address
o = s:option(Value, "server", translate("Server Address"),
    translate("Hostname or IP address of the UnifiedShield relay server"))
o.datatype = "host"
o.rmempty = false

-- Port
o = s:option(Value, "port", translate("Server Port"),
    translate("Port number of the relay server"))
o.datatype = "port"
o.default = "443"
o.rmempty = false

-- Private Key
o = s:option(Value, "private_key", translate("X25519 Private Key"),
    translate("Hex-encoded X25519 private key for ECDH session key negotiation"))
o.password = true
o.rmempty = false

-- Obfuscation mode
o = s:option(ListValue, "obfuscation", translate("Obfuscation Mode"),
    translate("Traffic obfuscation level to evade DPI detection"))
o:value("none", translate("None"))
o:value("padding", translate("Padding Only"))
o:value("fingerprint", translate("TLS Fingerprint Simulation"))
o:value("full", translate("Full (Padding + Fingerprint + Jitter)"))
o.default = "full"
o.rmempty = false

-- Split tunneling
o = s:option(Flag, "split_tunnel", translate("Split Tunneling"),
    translate("Only route non-Iranian traffic through the VPN. " ..
              "Iranian domestic traffic goes through your normal connection."))
o.rmempty = false
o.default = "1"

-- Kill switch
o = s:option(Flag, "kill_switch", translate("Kill Switch"),
    translate("Block all internet traffic if the VPN connection drops, " ..
              "preventing IP leaks."))
o.rmempty = false
o.default = "1"

-- Advanced settings section
s = m:section(TypedSection, "unifiedshield", translate("Advanced Settings"))
s.anonymous = true
s.addremove = false

-- Log level
o = s:option(ListValue, "log_level", translate("Log Level"),
    translate("Verbosity of log output"))
o:value("error", translate("Error"))
o:value("warn", translate("Warning"))
o:value("info", translate("Info"))
o:value("debug", translate("Debug"))
o:value("trace", translate("Trace"))
o.default = "info"
o.rmempty = false

-- MTU
o = s:option(Value, "mtu", translate("MTU"),
    translate("Maximum Transmission Unit for the TUN device"))
o.datatype = "range(576,9000)"
o.default = "1500"
o.rmempty = false

-- DNS port
o = s:option(Value, "dns_port", translate("Local DNS Port"),
    translate("Port for local DNS-over-HTTPS resolver"))
o.datatype = "port"
o.default = "5353"
o.rmempty = false

-- DNS server
o = s:option(Value, "dns_server", translate("Upstream DNS Server"),
    translate("DNS server to use for resolving relay server addresses"))
o.datatype = "ipaddr"
o.default = "8.8.8.8"
o.rmempty = false

-- Auto reconnect
o = s:option(Flag, "auto_reconnect", translate("Auto Reconnect"),
    translate("Automatically reconnect if the VPN connection drops"))
o.rmempty = false
o.default = "1"

-- Reconnect delay
o = s:option(Value, "reconnect_delay", translate("Reconnect Delay"),
    translate("Initial delay before attempting reconnection (seconds). " ..
              "Uses exponential backoff up to 60 seconds."))
o.datatype = "uinteger"
o.default = "5"
o:depends("auto_reconnect", "1")

-- Allowed targets
o = s:option(TextValue, "allowed_targets", translate("Allowed Targets"),
    translate("JSON array of allowed target host patterns. " ..
              "Leave empty to allow all targets. " ..
              "Example: [\"*.example.com\", \"specific.host.com\"]"))
o.rows = 3
o.rmempty = true

-- Relay failover section
s = m:section(TypedSection, "relay", translate("Relay Failover Servers"),
    translate("Additional relay servers for automatic failover. " ..
              "If the primary server fails, UnifiedShield will try these in priority order."))
s.anonymous = false
s.addremove = true
s.template = "cbi/tblsection"

o = s:option(Value, "server", translate("Server"))
o.datatype = "host"

o = s:option(Value, "port", translate("Port"))
o.datatype = "port"
o.default = "443"

o = s:option(ListValue, "core", translate("Core"))
o:value("alibaba", "Alibaba")
o:value("tencent", "Tencent")
o:value("baidu", "Baidu")
o:value("huawei", "Huawei")
o:value("deno", "Deno")
o.default = "alibaba"

o = s:option(Value, "priority", translate("Priority"))
o.datatype = "uinteger"
o.default = "2"

return m
