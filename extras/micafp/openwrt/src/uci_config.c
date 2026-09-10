/**
 * UCI (Unified Configuration Interface) configuration handler.
 *
 * Reads UnifiedShield configuration from OpenWrt's UCI system.
 * Config file: /etc/config/unifiedshield
 *
 * Example UCI config:
 *   config unifiedshield 'default'
 *       option enabled '1'
 *       option core 'xray'
 *       option server 'example.com'
 *       option server_port '443'
 *       option password 'secret'
 *       option dns_server '223.5.5.5'
 *       option kill_switch '1'
 *       option split_tunnel '1'
 *       option dpi_threshold '0.72'
 *       option auto_core_switch '1'
 *       option mtu '1380'
 *       list excluded_ip '78.38.0.0/16'
 *       list excluded_ip '217.218.0.0/15'
 */

#include "uci_config.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <uci.h>

int uci_load_config(const char* config_name, const char* section_name,
                    struct unifiedshield_config* config) {
    struct uci_context* ctx = uci_alloc_context();
    if (!ctx) {
        return -1;
    }

    struct uci_package* pkg = nullptr;
    int ret = uci_load(ctx, config_name, &pkg);

    if (ret != UCI_OK || !pkg) {
        fprintf(stderr, "[UCI] Failed to load config: %s\n", config_name);
        uci_free_context(ctx);
        return -1;
    }

    // Set defaults
    strncpy(config->core, "xray", sizeof(config->core) - 1);
    strncpy(config->dns_server, "223.5.5.5", sizeof(config->dns_server) - 1);  // Alibaba DNS
    strncpy(config->tun_name, "us0", sizeof(config->tun_name) - 1);
    config->enabled = 0;
    config->kill_switch = 0;
    config->split_tunnel = 1;
    config->auto_core_switch = 1;
    config->mtu = 1380;
    config->dpi_threshold = 0.72;
    config->server[0] = '\0';
    config->server_port = 443;
    config->password[0] = '\0';
    config->excluded_ip_count = 0;

    // Find the section
    struct uci_element* e;
    uci_foreach_element(&pkg->sections, e) {
        struct uci_section* s = uci_to_section(e);
        if (strcmp(s->e.name, section_name) != 0) continue;

        // Parse options
        struct uci_element* oe;
        uci_foreach_element(&s->options, oe) {
            struct uci_option* o = uci_to_option(oe);
            const char* key = o->e.name;
            const char* val = (o->type == UCI_TYPE_STRING) ? o->v.string : nullptr;

            if (!val && o->type != UCI_TYPE_LIST) continue;

            if (strcmp(key, "enabled") == 0 && val) {
                config->enabled = (strcmp(val, "1") == 0 || strcmp(val, "true") == 0);
            } else if (strcmp(key, "core") == 0 && val) {
                strncpy(config->core, val, sizeof(config->core) - 1);
            } else if (strcmp(key, "server") == 0 && val) {
                strncpy(config->server, val, sizeof(config->server) - 1);
            } else if (strcmp(key, "server_port") == 0 && val) {
                config->server_port = atoi(val);
            } else if (strcmp(key, "password") == 0 && val) {
                strncpy(config->password, val, sizeof(config->password) - 1);
            } else if (strcmp(key, "dns_server") == 0 && val) {
                strncpy(config->dns_server, val, sizeof(config->dns_server) - 1);
            } else if (strcmp(key, "kill_switch") == 0 && val) {
                config->kill_switch = (strcmp(val, "1") == 0 || strcmp(val, "true") == 0);
            } else if (strcmp(key, "split_tunnel") == 0 && val) {
                config->split_tunnel = (strcmp(val, "1") == 0 || strcmp(val, "true") == 0);
            } else if (strcmp(key, "auto_core_switch") == 0 && val) {
                config->auto_core_switch = (strcmp(val, "1") == 0 || strcmp(val, "true") == 0);
            } else if (strcmp(key, "mtu") == 0 && val) {
                config->mtu = atoi(val);
            } else if (strcmp(key, "dpi_threshold") == 0 && val) {
                config->dpi_threshold = atof(val);
            } else if (strcmp(key, "tun_name") == 0 && val) {
                strncpy(config->tun_name, val, sizeof(config->tun_name) - 1);
            } else if (strcmp(key, "excluded_ip") == 0 && o->type == UCI_TYPE_LIST) {
                // Parse list of excluded IPs
                struct uci_element* le;
                uci_foreach_element(&o->v.list, le) {
                    if (config->excluded_ip_count < MAX_EXCLUDED_IPS) {
                        strncpy(config->excluded_ips[config->excluded_ip_count],
                               le->name,
                               sizeof(config->excluded_ips[0]) - 1);
                        config->excluded_ip_count++;
                    }
                }
            }
        }
        break;  // Found our section
    }

    uci_free_context(ctx);
    return 0;
}

int uci_reload_config(void) {
    // Reload configuration from UCI
    // This is called from SIGHUP handler
    return 0;
}

void uci_free_config(struct unifiedshield_config* config) {
    // Nothing to free for static config struct
    memset(config, 0, sizeof(*config));
}

int uci_set_option(const char* config_name, const char* section_name,
                   const char* option, const char* value) {
    struct uci_context* ctx = uci_alloc_context();
    if (!ctx) return -1;

    char cmd[256];
    snprintf(cmd, sizeof(cmd), "%s.%s.%s=%s", config_name, section_name, option, value);

    struct uci_ptr ptr;
    if (uci_lookup_ptr(ctx, &ptr, cmd, true) != UCI_OK) {
        uci_free_context(ctx);
        return -1;
    }

    if (uci_set(ctx, &ptr) != UCI_OK) {
        uci_free_context(ctx);
        return -1;
    }

    uci_save(ctx, ptr.p);
    uci_commit(ctx, &ptr.p, false);
    uci_free_context(ctx);

    return 0;
}
