/**
 * netifd protocol handler for UnifiedShield on OpenWrt.
 *
 * Integrates with OpenWrt's netifd to manage the VPN interface
 * as a standard network interface. This allows proper integration
 * with the OpenWrt networking stack (firewall, routing, DNS).
 *
 * Protocol: unifiedshield
 * Interface config: /etc/config/network
 *   config interface 'vpn'
 *       option proto 'unifiedshield'
 *       option core 'xray'
 *       option server 'example.com'
 *       option server_port '443'
 *       option password 'secret'
 *       option mtu '1380'
 *       option dns '223.5.5.5'
 */

#include "netifd_proto.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/socket.h>
#include <linux/if.h>
#include <linux/if_tun.h>
#include <sys/ioctl.h>
#include <fcntl.h>
#include <arpa/inet.h>
#include <net/route.h>
#include <errno.h>

static int g_tunFd = -1;
static char g_devName[IFNAMSIZ] = "us0";
static int g_sockFd = -1;

int netifd_proto_init(const char* dev_name) {
    if (dev_name && strlen(dev_name) > 0) {
        strncpy(g_devName, dev_name, IFNAMSIZ - 1);
    }

    // Create control socket
    g_sockFd = socket(AF_INET, SOCK_DGRAM, 0);
    if (g_sockFd < 0) {
        return -1;
    }

    printf("[netifd] Protocol handler initialized: %s\n", g_devName);
    return 0;
}

int netifd_proto_setup(const char* ip_address, int prefix_len,
                       const char* gateway, int mtu,
                       const char** dns_servers, int dns_count) {
    // Create TUN device
    struct ifreq ifr;
    memset(&ifr, 0, sizeof(ifr));

    int tunFd = open("/dev/net/tun", O_RDWR);
    if (tunFd < 0) {
        fprintf(stderr, "[netifd] Failed to open /dev/net/tun: %s\n", strerror(errno));
        return -1;
    }

    ifr.ifr_flags = IFF_TUN | IFF_NO_PI;
    strncpy(ifr.ifr_name, g_devName, IFNAMSIZ - 1);

    if (ioctl(tunFd, TUNSETIFF, &ifr) < 0) {
        fprintf(stderr, "[netifd] Failed to create TUN: %s\n", strerror(errno));
        close(tunFd);
        return -1;
    }

    g_tunFd = tunFd;

    // Set MTU
    struct ifreq mtu_req;
    memset(&mtu_req, 0, sizeof(mtu_req));
    strncpy(mtu_req.ifr_name, g_devName, IFNAMSIZ - 1);
    mtu_req.ifr_mtu = mtu;
    ioctl(g_sockFd, SIOCSIFMTU, &mtu_req);

    // Set IP address
    struct ifreq addr_req;
    memset(&addr_req, 0, sizeof(addr_req));
    strncpy(addr_req.ifr_name, g_devName, IFNAMSIZ - 1);
    struct sockaddr_in* addr = (struct sockaddr_in*)&addr_req.ifr_addr;
    addr->sin_family = AF_INET;
    inet_pton(AF_INET, ip_address, &addr->sin_addr);
    ioctl(g_sockFd, SIOCSIFADDR, &addr_req);

    // Set netmask
    struct ifreq mask_req;
    memset(&mask_req, 0, sizeof(mask_req));
    strncpy(mask_req.ifr_name, g_devName, IFNAMSIZ - 1);
    struct sockaddr_in* mask = (struct sockaddr_in*)&mask_req.ifr_netmask;
    mask->sin_family = AF_INET;
    uint32_t maskValue = htonl(~((1 << (32 - prefix_len)) - 1));
    mask->sin_addr.s_addr = maskValue;
    ioctl(g_sockFd, SIOCSIFNETMASK, &mask_req);

    // Bring interface up
    struct ifreq flags_req;
    memset(&flags_req, 0, sizeof(flags_req));
    strncpy(flags_req.ifr_name, g_devName, IFNAMSIZ - 1);
    ioctl(g_sockFd, SIOCGIFFLAGS, &flags_req);
    flags_req.ifr_flags |= IFF_UP | IFF_RUNNING;
    ioctl(g_sockFd, SIOCSIFFLAGS, &flags_req);

    // Set up routing
    struct rtentry route;
    memset(&route, 0, sizeof(route));
    ((struct sockaddr_in*)&route.rt_dst)->sin_family = AF_INET;
    ((struct sockaddr_in*)&route.rt_dst)->sin_addr.s_addr = 0;
    ((struct sockaddr_in*)&route.rt_genmask)->sin_family = AF_INET;
    ((struct sockaddr_in*)&route.rt_genmask)->sin_addr.s_addr = 0;
    route.rt_flags = RTF_UP | RTF_GATEWAY;
    route.rt_dev = g_devName;

    if (ioctl(g_sockFd, SIOCADDRT, &route) < 0) {
        // Route might already exist
        if (errno != EEXIST) {
            fprintf(stderr, "[netifd] Failed to add route: %s\n", strerror(errno));
        }
    }

    // Notify netifd of interface setup
    netifd_proto_notify("interface-up");

    printf("[netifd] Interface %s configured: %s/%d, MTU=%d\n",
           g_devName, ip_address, prefix_len, mtu);

    return 0;
}

void netifd_proto_process(void) {
    // Main event processing loop
    // Reads from TUN, forwards to core, writes responses
    // In production, this would be replaced by the Rust core's event loop

    if (g_tunFd < 0) return;

    fd_set readFds;
    struct timeval tv;
    tv.tv_sec = 0;
    tv.tv_usec = 10000; // 10ms

    FD_ZERO(&readFds);
    FD_SET(g_tunFd, &readFds);

    int result = select(g_tunFd + 1, &readFds, nullptr, nullptr, &tv);
    if (result <= 0) return;

    uint8_t buffer[2000];
    ssize_t bytesRead = read(g_tunFd, buffer, sizeof(buffer));
    if (bytesRead > 0) {
        // Forward packet to Rust core for processing
        // (handled by the core library)
    }
}

void netifd_proto_cleanup(void) {
    if (g_tunFd >= 0) {
        close(g_tunFd);
        g_tunFd = -1;
    }
    if (g_sockFd >= 0) {
        close(g_sockFd);
        g_sockFd = -1;
    }

    // Remove interface
    char cmd[64];
    snprintf(cmd, sizeof(cmd), "ip link del %s 2>/dev/null", g_devName);
    system(cmd);

    netifd_proto_notify("interface-down");
    printf("[netifd] Protocol handler cleaned up\n");
}

void netifd_proto_notify(const char* event) {
    // Send notification to netifd via ubus
    char cmd[128];
    snprintf(cmd, sizeof(cmd),
        "ubus call network.interface.%s notify '{\"event\":\"%s\"}' 2>/dev/null || true",
        g_devName, event);
    system(cmd);
}

int netifd_proto_add_excluded_route(const char* cidr) {
    // Add route for excluded (Iranian) IPs via original gateway
    char cmd[256];
    snprintf(cmd, sizeof(cmd),
        "ip route add %s via $(ip route show default 0.0.0.0/0 | awk '{print $3}' | head -1) 2>/dev/null || true",
        cidr);
    return system(cmd);
}
