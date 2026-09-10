-- LuCI Controller for UnifiedShield
-- Provides web interface for managing the VPN service
-- Accessible at: http://router.lan/cgi-bin/luci/admin/services/unifiedshield

module("luci.controller.unifiedshield", package.seeall)

function index()
    if not nixio.fs.access("/etc/config/unifiedshield") then
        return
    end

    local page = entry(
        {"admin", "services", "unifiedshield"},
        alias("admin", "services", "unifiedshield", "config"),
        _("UnifiedShield VPN"),
        60
    )
    page.dependent = true
    page.acl_depends = { "luci-app-unifiedshield" }

    entry(
        {"admin", "services", "unifiedshield", "config"},
        cbi("unifiedshield/config"),
        _("Configuration"),
        10
    ).leaf = true

    entry(
        {"admin", "services", "unifiedshield", "status"},
        call("action_status"),
        _("Status"),
        20
    ).leaf = true

    entry(
        {"admin", "services", "unifiedshield", "log"},
        call("action_log"),
        _("Log"),
        30
    ).leaf = true

    -- API endpoints
    entry(
        {"admin", "services", "unifiedshield", "start"},
        call("action_start"),
        nil
    ).leaf = true

    entry(
        {"admin", "services", "unifiedshield", "stop"},
        call("action_stop"),
        nil
    ).leaf = true

    entry(
        {"admin", "services", "unifiedshield", "restart"},
        call("action_restart"),
        nil
    ).leaf = true

    entry(
        {"admin", "services", "unifiedshield", "switch_core"},
        call("action_switch_core"),
        nil
    ).leaf = true
end

-- Get VPN status
function action_status()
    local sys = require("luci.sys")
    local json = require("luci.jsonc")

    local running = sys.call("pgrep -f unifiedshield >/dev/null 2>&1") == 0

    local status = {
        running = running,
        core = uci_get("unifiedshield", "default", "core") or "xray",
        server = uci_get("unifiedshield", "default", "server") or "Not configured",
        dns_server = uci_get("unifiedshield", "default", "dns_server") or "223.5.5.5",
        kill_switch = uci_get("unifiedshield", "default", "kill_switch") == "1",
        split_tunnel = uci_get("unifiedshield", "default", "split_tunnel") == "1",
        dpi_threshold = tonumber(uci_get("unifiedshield", "default", "dpi_threshold") or "0.72"),
        uptime = "N/A",
        download = "0 B",
        upload = "0 B"
    }

    if running then
        -- Get uptime and traffic stats from the running service
        local stats = sys.exec("cat /var/run/unifiedshield/stats.json 2>/dev/null")
        if stats and stats ~= "" then
            local stats_obj = json.parse(stats)
            if stats_obj then
                status.uptime = stats_obj.uptime or "N/A"
                status.download = stats_obj.download or "0 B"
                status.upload = stats_obj.upload or "0 B"
                status.dpi_score = stats_obj.dpi_score or 0
            end
        end
    end

    luci.http.prepare_content("application/json")
    luci.http.write_json(status)
end

-- Get log
function action_log()
    local sys = require("luci.sys")
    local log = sys.exec("logread -e unifiedshield | tail -100 2>/dev/null || cat /var/log/unifiedshield.log 2>/dev/null | tail -100")

    luci.http.prepare_content("text/plain")
    luci.http.write(log)
end

-- Start VPN
function action_start()
    local sys = require("luci.sys")
    sys.call("/etc/init.d/unifiedshield start >/dev/null 2>&1")
    luci.http.redirect(luci.dispatcher.build_url("admin/services/unifiedshield/status"))
end

-- Stop VPN
function action_stop()
    local sys = require("luci.sys")
    sys.call("/etc/init.d/unifiedshield stop >/dev/null 2>&1")
    luci.http.redirect(luci.dispatcher.build_url("admin/services/unifiedshield/status"))
end

-- Restart VPN
function action_restart()
    local sys = require("luci.sys")
    sys.call("/etc/init.d/unifiedshield restart >/dev/null 2>&1")
    luci.http.redirect(luci.dispatcher.build_url("admin/services/unifiedshield/status"))
end

-- Switch core
function action_switch_core()
    local http = require("luci.http")
    local core = http.formvalue("core")

    if core and (core == "xray" or core == "naive" or core == "hysteria2" or core == "tuic") then
        local uci = require("luci.model.uci").cursor()
        uci:set("unifiedshield", "default", "core", core)
        uci:commit("unifiedshield")

        -- Send SIGUSR1 to running process to trigger core switch
        local sys = require("luci.sys")
        sys.call("pkill -USR1 unifiedshield 2>/dev/null")
    end

    luci.http.redirect(luci.dispatcher.build_url("admin/services/unifiedshield/status"))
end

-- Helper: get UCI value
function uci_get(config, section, option)
    local uci = require("luci.model.uci").cursor()
    return uci:get(config, section, option)
end
