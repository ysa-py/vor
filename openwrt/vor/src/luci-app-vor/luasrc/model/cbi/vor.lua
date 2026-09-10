-- Vor LuCI CBI Model
-- Configuration UI for the Vor web interface

local m, s, o

m = Map("vor", translate("Vor"),
    translate("Vor is a next-generation censorship circumvention tool " ..
              "using Chinese CDN relays to bypass internet restrictions in Iran and other " ..
              "heavily censored regions. It uses X25519 ECDH key exchange and ChaCha20-Poly1305 " ..
              "encryption for secure relay connections."))

-- Main configuration section
s = m:section(TypedSection, "vor", translate("General Settings"))
s.anonymous = true
s.addremove = false

-- Enable/Disable
o = s:option(Flag, "enabled", translate("Enable"),
    translate("Enable or disable the Vor service"))
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
    translate("Hostname or IP address of the Vor relay server"))
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
s = m:section(TypedSection, "vor", translate("Advanced Settings"))
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
              "If the primary server fails, Vor will try these in priority order."))
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


-- ---------------------------------------------------------------------
-- Transport engines (SlipNet-equivalent coverage, router side)
-- ---------------------------------------------------------------------

local s2 = m:section(TypedSection, "engine", translate("Transport Engine"),
    translate("Select the transport engine (or Auto). The SNI relay strategies " ..
              "run in the C daemon; the DNS-tunnel client ships per-architecture " ..
              "in the same release; process cores install separately."))
s2.anonymous = true
s2.addremove = false

o = s2:option(ListValue, "selection", translate("Engine"))
o:value("auto", translate("Auto (adaptive)"))
o:value("sni-tunnel", translate("SNI relay (this package)"))
o:value("dns-tunnel-masterdns", translate("DNS tunnel (MasterDns)"))
o:value("dns-tunnel-dnstt", translate("DNS tunnel (DNSTT)"))
o:value("ssh-chain", translate("SSH chain (TLS/WS/CONNECT)"))
o:value("naive-https", translate("Padded HTTPS (naive)"))
o:value("tor", translate("Tor"))
o:value("xray", translate("Xray core"))
o:value("singbox", translate("sing-box core"))
o.default = "auto"
o.rmempty = false

local s3 = m:section(TypedSection, "ssh_chain", translate("SSH Chain Tunnel"))
s3.anonymous = true
s3.addremove = false
o = s3:option(Flag, "enabled", translate("Enabled"))
o = s3:option(Value, "listen_port", translate("Local SOCKS port"))
o.datatype = "port"
o.placeholder = "11080"
o = s3:option(Value, "addr", translate("SSH server"), translate("host:port"))
o.placeholder = "tunnel.example.com:22"
o = s3:option(Value, "user", translate("SSH user"))
o.placeholder = "vor"
o = s3:option(ListValue, "auth", translate("Authentication"))
o:value("password", translate("Password"))
o:value("key", translate("Private key"))
o = s3:option(ListValue, "wrap", translate("Transport wrap"))
o:value("none", translate("Plain TCP"))
o:value("tls", translate("TLS (custom SNI)"))
o:value("ws", translate("WebSocket (CDN)"))
o:value("wss", translate("WebSocket over TLS"))
o:value("http-connect", translate("HTTP CONNECT"))
o = s3:option(Value, "tls_sni", translate("TLS SNI"))
o = s3:option(Value, "ws_path", translate("WebSocket path"))
o = s3:option(Value, "ciphers", translate("Ciphers (comma list)"))

local s4 = m:section(TypedSection, "naive_https", translate("Padded HTTPS Tunnel"))
s4.anonymous = true
s4.addremove = false
o = s4:option(Flag, "enabled", translate("Enabled"))
o = s4:option(Value, "listen_port", translate("Local SOCKS port"))
o.datatype = "port"
o.placeholder = "11081"
o = s4:option(Value, "proxy_addr", translate("HTTPS proxy"), translate("host:port"))
o = s4:option(Value, "padding_bytes", translate("Padding bytes (max)"))
o.datatype = "range(0,65535)"
o.placeholder = "128"

local s5 = m:section(TypedSection, "dns_transport", translate("DNS Transport"))
s5.anonymous = true
s5.addremove = false
o = s5:option(ListValue, "transport", translate("Resolution transport"))
o:value("udp", translate("UDP"))
o:value("dot", translate("DNS over TLS"))
o:value("doh", translate("DNS over HTTPS"))
o = s5:option(Value, "resolver", translate("Resolver"))
o.placeholder = "1.1.1.1"
o = s5:option(Value, "doh_url", translate("DoH URL"))
o.placeholder = "https://cloudflare-dns.com/dns-query"

return m
