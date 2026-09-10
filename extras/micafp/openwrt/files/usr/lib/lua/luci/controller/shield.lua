--[[
    MICAFP-UnifiedShield-6.0 — LuCI Web UI Controller

    Routes:
      /admin/services/shield          — Status dashboard
      /admin/services/shield/config   — Configuration page

    Provides web UI for managing the UnifiedShield daemon on OpenWrt.
]]

module("luci.controller.shield", package.seeall)

local fs = require("nixio.fs")
local sys = require("luci.sys")
local uci = require("luci.model.uci").cursor()

-- ---------------------------------------------------------------------------
-- Routes
-- ---------------------------------------------------------------------------

function index()
    entry(
        {"admin", "services", "shield"},
        call("action_status"),
        _("UnifiedShield"),
        60
    )

    entry(
        {"admin", "services", "shield", "config"},
        cbi("shield/config"),
        _("Configuration"),
        70
    )

    entry(
        {"admin", "services", "shield", "start"},
        call("action_start"),
        nil
    )

    entry(
        {"admin", "services", "shield", "stop"},
        call("action_stop"),
        nil
    )

    entry(
        {"admin", "services", "shield", "restart"},
        call("action_restart"),
        nil
    )

    entry(
        {"admin", "services", "shield", "status_json"},
        call("action_status_json"),
        nil
    ).leaf = true
end

-- ---------------------------------------------------------------------------
-- Status page
-- ---------------------------------------------------------------------------

function action_status()
    local running = is_running()
    local status_info = get_status()

    luci.template.render("shield/status", {
        running = running,
        status = status_info,
    })
end

-- ---------------------------------------------------------------------------
-- Start / Stop / Restart
-- ---------------------------------------------------------------------------

function action_start()
    sys.call("/etc/init.d/shield start >/dev/null 2>&1")
    luci.http.redirect(luci.dispatcher.build_url("admin/services/shield"))
end

function action_stop()
    sys.call("/etc/init.d/shield stop >/dev/null 2>&1")
    luci.http.redirect(luci.dispatcher.build_url("admin/services/shield"))
end

function action_restart()
    sys.call("/etc/init.d/shield restart >/dev/null 2>&1")
    luci.http.redirect(luci.dispatcher.build_url("admin/services/shield"))
end

-- ---------------------------------------------------------------------------
-- JSON status endpoint (for AJAX updates)
-- ---------------------------------------------------------------------------

function action_status_json()
    local running = is_running()
    local status_info = get_status()

    local response = {
        running = running,
        connected = status_info.connected or false,
        endpoint = status_info.endpoint or "—",
        uptime = status_info.uptime or "0:00",
        bytes_sent = status_info.bytes_sent or 0,
        bytes_received = status_info.bytes_received or 0,
        transport = status_info.transport or "—",
        peers = status_info.peers or 0,
    }

    luci.http.prepare_content("application/json")
    luci.http.write_json(response)
end

-- ---------------------------------------------------------------------------
-- Helpers
-- ---------------------------------------------------------------------------

--- Check if the shield daemon is running.
-- @return boolean
function is_running()
    local pid = sys.exec("pidof shield-daemon")
    return pid and #pid > 0
end

--- Get the current daemon status by querying the Unix socket.
-- Falls back to reading the PID and log file if the socket is unavailable.
-- @return table Status information
function get_status()
    local status = {
        connected = false,
        endpoint = "—",
        uptime = "0:00",
        bytes_sent = 0,
        bytes_received = 0,
        transport = "—",
        peers = 0,
    }

    -- Try querying the daemon's status socket
    local status_json = sys.exec(
        "echo 'STATUS' | socat - UNIX-CONNECT:/var/run/shield.sock 2>/dev/null"
    )

    if status_json and #status_json > 0 then
        -- Parse JSON response (simple key=value for embedded systems)
        for line in status_json:gmatch("[^\n]+") do
            local key, value = line:match("^(%w+)=([^\n]*)$")
            if key and value then
                if key == "connected" then
                    status.connected = (value == "true")
                elseif key == "endpoint" then
                    status.endpoint = value
                elseif key == "uptime" then
                    status.uptime = value
                elseif key == "bytes_sent" then
                    status.bytes_sent = tonumber(value) or 0
                elseif key == "bytes_received" then
                    status.bytes_received = tonumber(value) or 0
                elseif key == "transport" then
                    status.transport = value
                elseif key == "peers" then
                    status.peers = tonumber(value) or 0
                end
            end
        end
    else
        -- Fallback: read status file
        local status_file = io.open("/var/run/shield.status", "r")
        if status_file then
            for line in status_file:lines() do
                local key, value = line:match("^(%w+)=([^\n]*)$")
                if key and value then
                    status[key] = value
                end
            end
            status_file:close()
        end
    end

    return status
end

--- Format bytes into human-readable string.
-- @param bytes number
-- @return string
function format_bytes(bytes)
    bytes = tonumber(bytes) or 0
    if bytes < 1024 then
        return string.format("%d B", bytes)
    elseif bytes < 1024 * 1024 then
        return string.format("%.1f KB", bytes / 1024)
    elseif bytes < 1024 * 1024 * 1024 then
        return string.format("%.1f MB", bytes / (1024 * 1024))
    else
        return string.format("%.1f GB", bytes / (1024 * 1024 * 1024))
    end
end
