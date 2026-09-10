-- UnifiedShield LuCI Controller for OpenWrt
-- Provides web UI for managing the UnifiedShield anti-censorship VPN daemon
-- on OpenWrt routers. Includes status monitoring, configuration, and log viewing.

module("luci.controller.unifiedshield", package.seeall)

local I18N = require("luci.i18n")
local UCI = require("luci.model.uci")
local SYS = require("luci.sys")
local HTTP = require("luci.http")
local DS = require("luci.dispatcher")

-- ===================================================================
-- Index: Register navigation entries
-- ===================================================================
function index()
    -- Main entry under Services menu
    local e = entry({"admin", "services", "unifiedshield"},
        alias("admin", "services", "unifiedshield", "status"),
        _("UnifiedShield"), 60)
    e.dependent = true
    e.acl_depends = { "luci-app-unifiedshield" }

    -- Status page (default tab)
    entry({"admin", "services", "unifiedshield", "status"},
        template("unifiedshield/status"),
        _("Status"), 10)

    -- Configuration page
    entry({"admin", "services", "unifiedshield", "config"},
        cbi("unifiedshield/config"),
        _("Configuration"), 20)

    -- Log viewer page
    entry({"admin", "services", "unifiedshield", "log"},
        template("unifiedshield/log"),
        _("Log"), 30)

    -- RPC/API endpoints for AJAX real-time status
    entry({"admin", "services", "unifiedshield", "api", "status"},
        call("api_status")).leaf = true

    entry({"admin", "services", "unifiedshield", "api", "start"},
        call("api_start")).leaf = true

    entry({"admin", "services", "unifiedshield", "api", "stop"},
        call("api_stop")).leaf = true

    entry({"admin", "services", "unifiedshield", "api", "restart"},
        call("api_restart")).leaf = true

    entry({"admin", "services", "unifiedshield", "api", "get_config"},
        call("api_get_config")).leaf = true

    entry({"admin", "services", "unifiedshield", "api", "set_config"},
        call("api_set_config")).leaf = true

    entry({"admin", "services", "unifiedshield", "api", "peers"},
        call("api_peers")).leaf = true

    entry({"admin", "services", "unifiedshield", "api", "bandwidth"},
        call("api_bandwidth")).leaf = true

    entry({"admin", "services", "unifiedshield", "api", "log"},
        call("api_log")).leaf = true

    entry({"admin", "services", "unifiedshield", "api", "test_connection"},
        call("api_test_connection")).leaf = true
end

-- ===================================================================
-- Helper: Get daemon status from init script
-- ===================================================================
local function get_daemon_status()
    local running = false
    local pid = nil

    -- Check if process is running
    local pid_output = SYS.exec("pgrep -f unifiedshield-daemon 2>/dev/null")
    if pid_output and pid_output ~= "" then
        running = true
        pid = tonumber(pid_output:match("(%d+)"))
    end

    -- Read status from Unix socket if daemon is running
    local status_data = {}
    if running then
        local status_json = SYS.exec(
            "echo '{\"cmd\":\"status\"}' | " ..
            "socat - UNIX-CONNECT:/var/run/unifiedshield.sock 2>/dev/null"
        )
        if status_json and status_json ~= "" then
            local JSON = require("luci.jsonc")
            status_data = JSON.parse(status_json) or {}
        end
    end

    return {
        running = running,
        pid = pid,
        transport = status_data.transport or "unknown",
        endpoint = status_data.endpoint or "none",
        peer_count = status_data.peer_count or 0,
        bytes_sent = status_data.bytes_sent or 0,
        bytes_received = status_data.bytes_received or 0,
        uptime_seconds = status_data.uptime_seconds or 0,
        isp_name = status_data.isp_name or "unknown",
        fragment_strategy = status_data.fragment_strategy or "none",
        national_intranet_mode = status_data.national_intranet_mode or false,
        last_connected = status_data.last_connected or "never",
        latency_ms = status_data.latency_ms or 0,
        connection_quality = status_data.connection_quality or "unknown",
    }
end

-- ===================================================================
-- Helper: Get UCI configuration
-- ===================================================================
local function get_uci_config()
    local uci = UCI.cursor()
    local config = {
        enabled = uci:get("unifiedshield", "main", "enabled") or "0",
        transport = uci:get("unifiedshield", "main", "transport") or "cdn",
        endpoint_url = uci:get("unifiedshield", "main", "endpoint_url") or "",
        auth_token = uci:get("unifiedshield", "main", "auth_token") or "",
        fragment_strategy = uci:get("unifiedshield", "main", "fragment_strategy") or "auto",
        sni_domain = uci:get("unifiedshield", "main", "sni_domain") or "auto",
        dns_mode = uci:get("unifiedshield", "main", "dns_mode") or "doh",
        socks_port = uci:get("unifiedshield", "main", "socks_port") or "1080",
        tun_mode = uci:get("unifiedshield", "main", "tun_mode") or "0",
        log_level = uci:get("unifiedshield", "main", "log_level") or "info",
        auto_start = uci:get("unifiedshield", "main", "auto_start") or "1",
        isp_detect = uci:get("unifiedshield", "main", "isp_detect") or "1",
        bandwidth_limit = uci:get("unifiedshield", "main", "bandwidth_limit") or "0",
        p2p_enabled = uci:get("unifiedshield", "main", "p2p_enabled") or "0",
        yggdrasil_enabled = uci:get("unifiedshield", "main", "yggdrasil_enabled") or "0",
        mqtt_broker = uci:get("unifiedshield", "main", "mqtt_broker") or "",
    }
    uci:close()
    return config
end

-- ===================================================================
-- Helper: Format bytes to human-readable
-- ===================================================================
local function format_bytes(bytes)
    if type(bytes) ~= "number" or bytes < 0 then bytes = 0 end
    local units = {"B", "KB", "MB", "GB", "TB"}
    local i = 1
    while bytes >= 1024 and i < #units do
        bytes = bytes / 1024
        i = i + 1
    end
    return string.format("%.1f %s", bytes, units[i])
end

-- ===================================================================
-- Helper: Format seconds to human-readable uptime
-- ===================================================================
local function format_uptime(seconds)
    if type(seconds) ~= "number" or seconds < 0 then return "0s" end
    local days = math.floor(seconds / 86400)
    local hours = math.floor((seconds % 86400) / 3600)
    local mins = math.floor((seconds % 3600) / 60)
    local secs = seconds % 60

    if days > 0 then
        return string.format("%dd %dh %dm", days, hours, mins)
    elseif hours > 0 then
        return string.format("%dh %dm", hours, mins)
    elseif mins > 0 then
        return string.format("%dm %ds", mins, secs)
    else
        return string.format("%ds", secs)
    end
end

-- ===================================================================
-- API: Get current status (JSON)
-- ===================================================================
function api_status()
    HTTP.prepare_content("application/json")
    local status = get_daemon_status()
    status.bytes_sent_hr = format_bytes(status.bytes_sent)
    status.bytes_received_hr = format_bytes(status.bytes_received)
    status.uptime_hr = format_uptime(status.uptime_seconds)

    local JSON = require("luci.jsonc")
    HTTP.write(JSON.stringify(status))
end

-- ===================================================================
-- API: Start daemon
-- ===================================================================
function api_start()
    HTTP.prepare_content("application/json")
    local result = SYS.exec("/etc/init.d/unifiedshield start 2>&1")
    local success = (SYS.call("pgrep -f unifiedshield-daemon >/dev/null 2>&1") == 0)

    local JSON = require("luci.jsonc")
    HTTP.write(JSON.stringify({
        success = success,
        message = success and "Daemon started successfully" or "Failed to start daemon",
        output = result
    }))
end

-- ===================================================================
-- API: Stop daemon
-- ===================================================================
function api_stop()
    HTTP.prepare_content("application/json")
    local result = SYS.exec("/etc/init.d/unifiedshield stop 2>&1")
    local success = (SYS.call("pgrep -f unifiedshield-daemon >/dev/null 2>&1") ~= 0)

    local JSON = require("luci.jsonc")
    HTTP.write(JSON.stringify({
        success = success,
        message = success and "Daemon stopped successfully" or "Failed to stop daemon",
        output = result
    }))
end

-- ===================================================================
-- API: Restart daemon
-- ===================================================================
function api_restart()
    HTTP.prepare_content("application/json")
    SYS.exec("/etc/init.d/unifiedshield restart 2>&1")
    local success = (SYS.call("pgrep -f unifiedshield-daemon >/dev/null 2>&1") == 0)

    local JSON = require("luci.jsonc")
    HTTP.write(JSON.stringify({
        success = success,
        message = success and "Daemon restarted successfully" or "Failed to restart daemon"
    }))
end

-- ===================================================================
-- API: Get configuration (JSON)
-- ===================================================================
function api_get_config()
    HTTP.prepare_content("application/json")
    local config = get_uci_config()
    local JSON = require("luci.jsonc")
    HTTP.write(JSON.stringify(config))
end

-- ===================================================================
-- API: Set configuration (POST JSON)
-- ===================================================================
function api_set_config()
    HTTP.prepare_content("application/json")
    local JSON = require("luci.jsonc")

    -- Read request body
    local raw_body = HTTP.content()
    if not raw_body then
        HTTP.write(JSON.stringify({success = false, message = "No data received"}))
        return
    end

    local new_config = JSON.parse(raw_body)
    if not new_config then
        HTTP.write(JSON.stringify({success = false, message = "Invalid JSON"}))
        return
    end

    -- Apply configuration via UCI
    local uci = UCI.cursor()
    local allowed_keys = {
        "enabled", "transport", "endpoint_url", "auth_token",
        "fragment_strategy", "sni_domain", "dns_mode", "socks_port",
        "tun_mode", "log_level", "auto_start", "isp_detect",
        "bandwidth_limit", "p2p_enabled", "yggdrasil_enabled", "mqtt_broker"
    }

    for _, key in ipairs(allowed_keys) do
        if new_config[key] ~= nil then
            uci:set("unifiedshield", "main", key, tostring(new_config[key]))
        end
    end

    uci:commit("unifiedshield")
    uci:close()

    HTTP.write(JSON.stringify({
        success = true,
        message = "Configuration saved. Restart daemon to apply changes."
    }))
end

-- ===================================================================
-- API: Get peer information
-- ===================================================================
function api_peers()
    HTTP.prepare_content("application/json")
    local peers_json = SYS.exec(
        "echo '{\"cmd\":\"peers\"}' | " ..
        "socat - UNIX-CONNECT:/var/run/unifiedshield.sock 2>/dev/null"
    )
    local JSON = require("luci.jsonc")
    if peers_json and peers_json ~= "" then
        HTTP.write(peers_json)
    else
        HTTP.write(JSON.stringify({peers = {}}))
    end
end

-- ===================================================================
-- API: Get bandwidth statistics
-- ===================================================================
function api_bandwidth()
    HTTP.prepare_content("application/json")
    local bw_json = SYS.exec(
        "echo '{\"cmd\":\"bandwidth\"}' | " ..
        "socat - UNIX-CONNECT:/var/run/unifiedshield.sock 2>/dev/null"
    )
    local JSON = require("luci.jsonc")
    if bw_json and bw_json ~= "" then
        HTTP.write(bw_json)
    else
        HTTP.write(JSON.stringify({
            current_kbps_up = 0,
            current_kbps_down = 0,
            total_bytes_up = 0,
            total_bytes_down = 0
        }))
    end
end

-- ===================================================================
-- API: Get log entries
-- ===================================================================
function api_log()
    HTTP.prepare_content("application/json")
    local lines = tonumber(HTTP.formvalue("lines")) or 100
    local log_data = SYS.exec("logread -e unifiedshield 2>/dev/null | tail -n " .. tostring(lines))

    local JSON = require("luci.jsonc")
    local entries = {}
    for line in log_data:gmatch("[^\r\n]+") do
        local timestamp, level, message = line:match("^(%S+%s+%S+)%s+(%w+)%s+unifiedshield:%s*(.*)$")
        if not timestamp then
            timestamp = ""
            level = "info"
            message = line
        end
        table.insert(entries, {
            timestamp = timestamp,
            level = level,
            message = message
        })
    end

    HTTP.write(JSON.stringify({entries = entries, count = #entries}))
end

-- ===================================================================
-- API: Test connection to endpoint
-- ===================================================================
function api_test_connection()
    HTTP.prepare_content("application/json")
    local test_result = SYS.exec(
        "echo '{\"cmd\":\"test_connection\"}' | " ..
        "socat - UNIX-CONNECT:/var/run/unifiedshield.sock 2>/dev/null"
    )
    local JSON = require("luci.jsonc")
    if test_result and test_result ~= "" then
        HTTP.write(test_result)
    else
        HTTP.write(JSON.stringify({
            success = false,
            message = "Daemon not running or unreachable",
            latency_ms = 0,
            reachable = false
        }))
    end
end
