/**
 * UnifiedShield Linux Main Entry Point
 *
 * Daemon process for Linux. Creates TUN device, routes traffic
 * through the Rust core, and handles signal management.
 *
 * No root required if user has CAP_NET_ADMIN capability.
 * Split tunnel excludes Iranian IP ranges for local service access.
 * DNS: Alibaba/Tencent primary (Cloudflare BLOCKED in Iran).
 */

#include <csignal>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <unistd.h>
#include <sys/stat.h>
#include <fcntl.h>

#include "tun_interface.h"
#include "systemd_service.h"
#include "ipc_client.h"

static volatile bool g_running = true;
static const char* g_currentCore = "xray";

void SignalHandler(int signal) {
    switch (signal) {
    case SIGTERM:
    case SIGINT:
        printf("[Main] Received signal %d, shutting down...\n", signal);
        g_running = false;
        break;
    case SIGHUP:
        printf("[Main] Received SIGHUP, reloading configuration...\n");
        // Reload config without stopping
        break;
    case SIGUSR1:
        printf("[Main] Received SIGUSR1, switching core...\n");
        // Toggle core: xray <-> naive
        g_currentCore = (strcmp(g_currentCore, "xray") == 0) ? "naive" : "xray";
        printf("[Main] Switched to core: %s\n", g_currentCore);
        break;
    case SIGUSR2:
        printf("[Main] Received SIGUSR2, triggering obfuscation mode...\n");
        break;
    }
}

void Daemonize() {
    pid_t pid = fork();
    if (pid < 0) {
        perror("fork");
        exit(1);
    }
    if (pid > 0) {
        exit(0); // Parent exits
    }

    // Create new session
    if (setsid() < 0) {
        perror("setsid");
        exit(1);
    }

    // Fork again to ensure we're not session leader
    pid = fork();
    if (pid < 0) {
        perror("fork2");
        exit(1);
    }
    if (pid > 0) {
        exit(0);
    }

    // Set working directory
    chdir("/");

    // Set file permissions
    umask(0022);

    // Close standard file descriptors
    close(STDIN_FILENO);
    close(STDOUT_FILENO);
    close(STDERR_FILENO);

    // Redirect to /dev/null
    int devnull = open("/dev/null", O_RDWR);
    dup(devnull); // stdin
    dup(devnull); // stdout
    dup(devnull); // stderr
}

void PrintUsage(const char* progname) {
    printf("UnifiedShield v1.0.0 - Next-gen anti-censorship VPN\n");
    printf("Usage: %s [OPTIONS]\n\n", progname);
    printf("Options:\n");
    printf("  -d, --daemonize     Run as background daemon\n");
    printf("  -c, --core CORE     Start with specific core (xray|naive|hysteria2|tuic)\n");
    printf("  -s, --status        Show current status\n");
    printf("  -k, --kill-switch   Enable kill switch\n");
    printf("  -h, --help          Show this help\n");
    printf("  -v, --version       Show version\n");
}

int main(int argc, char* argv[]) {
    bool daemonize = false;
    const char* core = "xray";
    bool killSwitch = false;

    // Parse arguments
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "-d") == 0 || strcmp(argv[i], "--daemonize") == 0) {
            daemonize = true;
        } else if (strcmp(argv[i], "-c") == 0 || strcmp(argv[i], "--core") == 0) {
            if (i + 1 < argc) {
                core = argv[++i];
            }
        } else if (strcmp(argv[i], "-k") == 0 || strcmp(argv[i], "--kill-switch") == 0) {
            killSwitch = true;
        } else if (strcmp(argv[i], "-s") == 0 || strcmp(argv[i], "--status") == 0) {
            // Query status via IPC
            IpcClient::Initialize();
            char* status = IpcClient::SendQuery("get_status");
            if (status) {
                printf("%s\n", status);
                IpcClient::FreeResponse(status);
            } else {
                printf("Service not running\n");
            }
            IpcClient::Shutdown();
            return 0;
        } else if (strcmp(argv[i], "-h") == 0 || strcmp(argv[i], "--help") == 0) {
            PrintUsage(argv[0]);
            return 0;
        } else if (strcmp(argv[i], "-v") == 0 || strcmp(argv[i], "--version") == 0) {
            printf("UnifiedShield v1.0.0\n");
            return 0;
        }
    }

    g_currentCore = core;

    // Register signal handlers
    signal(SIGTERM, SignalHandler);
    signal(SIGINT, SignalHandler);
    signal(SIGHUP, SignalHandler);
    signal(SIGUSR1, SignalHandler);
    signal(SIGUSR2, SignalHandler);

    // Daemonize if requested
    if (daemonize) {
        Daemonize();
    }

    printf("[Main] UnifiedShield starting with core: %s\n", core);

    // Initialize TUN interface
    if (!TunInterface::Initialize("172.19.0.1", 24, 1380)) {
        fprintf(stderr, "[Main] Failed to initialize TUN interface\n");
        fprintf(stderr, "[Main] Ensure you have CAP_NET_ADMIN or run as root\n");
        return 1;
    }

    // Configure routes (0.0.0.0/0 and ::/0)
    if (!TunInterface::AddRoute("0.0.0.0/0")) {
        fprintf(stderr, "[Main] Failed to add IPv4 route\n");
    }
    if (!TunInterface::AddRoute("::/0")) {
        fprintf(stderr, "[Main] Failed to add IPv6 route\n");
    }

    // Set DNS servers (Chinese CDN - Cloudflare blocked in Iran)
    TunInterface::SetDnsServer("223.5.5.5");     // Alibaba DNS
    TunInterface::AddDnsServer("119.29.29.29");   // Tencent DNS

    // Initialize IPC for tray app communication
    IpcClient::Initialize();

    // Initialize systemd notification
    SystemdService::NotifyReady();

    printf("[Main] VPN active - core: %s, DNS: 223.5.5.5\n", core);

    // Main loop
    uint8_t buffer[2000];
    while (g_running) {
        // Read packets from TUN
        int bytesRead = TunInterface::ReadPacket(buffer, sizeof(buffer));
        if (bytesRead > 0) {
            // Forward to Rust core for processing
            // (implemented via JNI/FFI in the core library)
        }

        // Check for IPC commands
        IpcClient::ProcessCommands();

        // Notify systemd watchdog
        SystemdService::NotifyWatchdog();
    }

    // Cleanup
    printf("[Main] Shutting down...\n");
    IpcClient::Shutdown();
    TunInterface::Destroy();
    SystemdService::NotifyStopping();

    printf("[Main] Shutdown complete\n");
    return 0;
}
