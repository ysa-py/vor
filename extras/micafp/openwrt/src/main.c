/**
 * UnifiedShield OpenWrt Main Entry Point
 *
 * Minimal C main for OpenWrt router deployment.
 * Communicates with the Rust core via shared memory/IPC.
 * Uses procd for service management and UCI for configuration.
 *
 * Cloudflare is BLOCKED in Iran - Alibaba/Tencent DNS primary.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>
#include <syslog.h>
#include <errno.h>

#include "uci_config.h"
#include "netifd_proto.h"

static volatile int g_running = 1;

void signal_handler(int sig) {
    switch (sig) {
    case SIGTERM:
    case SIGINT:
        syslog(LOG_INFO, "Received signal %d, shutting down", sig);
        g_running = 0;
        break;
    case SIGHUP:
        syslog(LOG_INFO, "Received SIGHUP, reloading configuration");
        uci_reload_config();
        break;
    case SIGUSR1:
        syslog(LOG_INFO, "Core switch requested via SIGUSR1");
        break;
    }
}

static void print_usage(const char* prog) {
    fprintf(stderr,
        "UnifiedShield v1.0.0 - OpenWrt Edition\n"
        "Usage: %s [OPTIONS]\n\n"
        "Options:\n"
        "  -c, --config PATH   Path to UCI config (default: unifiedshield)\n"
        "  -s, --section NAME  UCI section name\n"
        "  -d, --daemonize     Run as daemon\n"
        "  -v, --verbose       Enable verbose logging\n"
        "  -h, --help          Show this help\n",
        prog);
}

int main(int argc, char* argv[]) {
    int daemonize = 0;
    int verbose = 0;
    const char* config_name = "unifiedshield";
    const char* section_name = "default";

    // Parse arguments
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "-d") == 0 || strcmp(argv[i], "--daemonize") == 0) {
            daemonize = 1;
        } else if (strcmp(argv[i], "-v") == 0 || strcmp(argv[i], "--verbose") == 0) {
            verbose = 1;
        } else if (strcmp(argv[i], "-c") == 0 || strcmp(argv[i], "--config") == 0) {
            if (i + 1 < argc) config_name = argv[++i];
        } else if (strcmp(argv[i], "-s") == 0 || strcmp(argv[i], "--section") == 0) {
            if (i + 1 < argc) section_name = argv[++i];
        } else if (strcmp(argv[i], "-h") == 0 || strcmp(argv[i], "--help") == 0) {
            print_usage(argv[0]);
            return 0;
        }
    }

    // Open syslog
    openlog("unifiedshield", LOG_PID | LOG_NDELAY, LOG_DAEMON);

    // Load UCI configuration
    struct unifiedshield_config config;
    memset(&config, 0, sizeof(config));

    if (uci_load_config(config_name, section_name, &config) != 0) {
        syslog(LOG_ERR, "Failed to load UCI configuration");
        closelog();
        return 1;
    }

    syslog(LOG_INFO, "Configuration loaded: core=%s, dns=%s, kill_switch=%d",
           config.core, config.dns_server, config.kill_switch);

    // Register signal handlers
    signal(SIGTERM, signal_handler);
    signal(SIGINT, signal_handler);
    signal(SIGHUP, signal_handler);
    signal(SIGUSR1, signal_handler);

    // Daemonize if requested
    if (daemonize) {
        daemon(0, 0);
    }

    // Initialize netifd protocol handler
    if (netifd_proto_init(config.tun_name) != 0) {
        syslog(LOG_ERR, "Failed to initialize netifd protocol");
        uci_free_config(&config);
        closelog();
        return 1;
    }

    // Notify procd that we're ready
    fprintf(stderr, "ready\n");  // procd readiness signal
    fflush(stderr);

    syslog(LOG_INFO, "UnifiedShield started - core: %s, DNS: %s",
           config.core, config.dns_server);

    // Main loop
    while (g_running) {
        // Process netifd events
        netifd_proto_process();

        // Process UCI config changes
        // (handled by SIGHUP -> uci_reload_config)

        sleep(1);
    }

    // Cleanup
    netifd_proto_cleanup();
    uci_free_config(&config);

    syslog(LOG_INFO, "UnifiedShield stopped");
    closelog();

    return 0;
}
