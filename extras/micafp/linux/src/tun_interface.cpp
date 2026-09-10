/**
 * TUN device interface for Linux.
 *
 * Creates a TUN device using /dev/net/tun without requiring root
 * (needs CAP_NET_ADMIN capability).
 *
 * Configuration:
 * - Device name: us0
 * - IP: 172.19.0.1/24
 * - MTU: 1380
 * - Split tunnel: Iranian IPs excluded via routing
 * - DNS: Alibaba/Tencent (Cloudflare blocked in Iran)
 */

#include "tun_interface.h"

#include <cstdio>
#include <cstring>
#include <cstdlib>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <sys/ioctl.h>
#include <sys/socket.h>
#include <linux/if.h>
#include <linux/if_tun.h>
#include <net/if.h>
#include <arpa/inet.h>
#include <netinet/in.h>

static int g_tunFd = -1;
static int g_sockFd = -1;
static char g_devName[IFNAMSIZ] = "us0";

namespace TunInterface {

bool Initialize(const char* ipAddress, int prefixLength, int mtu) {
    // Open TUN device
    struct ifreq ifr;
    memset(&ifr, 0, sizeof(ifr));

    int tunFd = open("/dev/net/tun", O_RDWR);
    if (tunFd < 0) {
        // Try creating /dev/net/tun if it doesn't exist
        system("mkdir -p /dev/net");
        system("mknod /dev/net/tun c 10 200 2>/dev/null");
        system("chmod 666 /dev/net/tun 2>/dev/null");
        tunFd = open("/dev/net/tun", O_RDWR);
    }

    if (tunFd < 0) {
        printf("[TUN] Failed to open /dev/net/tun: %s\n", strerror(errno));
        return false;
    }

    ifr.ifr_flags = IFF_TUN | IFF_NO_PI;
    strncpy(ifr.ifr_name, g_devName, IFNAMSIZ - 1);

    if (ioctl(tunFd, TUNSETIFF, &ifr) < 0) {
        printf("[TUN] Failed to create TUN device: %s\n", strerror(errno));
        close(tunFd);
        return false;
    }

    strncpy(g_devName, ifr.ifr_name, IFNAMSIZ - 1);
    g_tunFd = tunFd;

    // Create control socket
    g_sockFd = socket(AF_INET, SOCK_DGRAM, 0);
    if (g_sockFd < 0) {
        printf("[TUN] Failed to create control socket: %s\n", strerror(errno));
        close(g_tunFd);
        g_tunFd = -1;
        return false;
    }

    // Set MTU
    if (!SetMtu(mtu)) {
        printf("[TUN] Warning: Failed to set MTU\n");
    }

    // Set IP address
    if (!SetIpAddress(ipAddress, prefixLength)) {
        printf("[TUN] Failed to set IP address\n");
        close(g_tunFd);
        close(g_sockFd);
        g_tunFd = -1;
        g_sockFd = -1;
        return false;
    }

    // Bring interface up
    if (!SetUp()) {
        printf("[TUN] Failed to bring interface up\n");
        close(g_tunFd);
        close(g_sockFd);
        g_tunFd = -1;
        g_sockFd = -1;
        return false;
    }

    printf("[TUN] Interface %s created: %s/%d, MTU=%d\n",
           g_devName, ipAddress, prefixLength, mtu);
    return true;
}

bool SetIpAddress(const char* ipAddress, int prefixLength) {
    struct ifreq ifr;
    memset(&ifr, 0, sizeof(ifr));
    strncpy(ifr.ifr_name, g_devName, IFNAMSIZ - 1);

    struct sockaddr_in* addr = (struct sockaddr_in*)&ifr.ifr_addr;
    addr->sin_family = AF_INET;
    inet_pton(AF_INET, ipAddress, &addr->sin_addr);

    if (ioctl(g_sockFd, SIOCSIFADDR, &ifr) < 0) {
        printf("[TUN] Failed to set IP address: %s\n", strerror(errno));
        return false;
    }

    // Set netmask
    struct ifreq netmask;
    memset(&netmask, 0, sizeof(netmask));
    strncpy(netmask.ifr_name, g_devName, IFNAMSIZ - 1);

    struct sockaddr_in* mask = (struct sockaddr_in*)&netmask.ifr_netmask;
    mask->sin_family = AF_INET;
    uint32_t maskValue = htonl(~((1 << (32 - prefixLength)) - 1));
    mask->sin_addr.s_addr = maskValue;

    if (ioctl(g_sockFd, SIOCSIFNETMASK, &netmask) < 0) {
        printf("[TUN] Failed to set netmask: %s\n", strerror(errno));
        return false;
    }

    return true;
}

bool SetMtu(int mtu) {
    struct ifreq ifr;
    memset(&ifr, 0, sizeof(ifr));
    strncpy(ifr.ifr_name, g_devName, IFNAMSIZ - 1);
    ifr.ifr_mtu = mtu;

    if (ioctl(g_sockFd, SIOCSIFMTU, &ifr) < 0) {
        printf("[TUN] Failed to set MTU: %s\n", strerror(errno));
        return false;
    }

    return true;
}

bool SetUp() {
    struct ifreq ifr;
    memset(&ifr, 0, sizeof(ifr));
    strncpy(ifr.ifr_name, g_devName, IFNAMSIZ - 1);

    if (ioctl(g_sockFd, SIOCGIFFLAGS, &ifr) < 0) {
        printf("[TUN] Failed to get interface flags: %s\n", strerror(errno));
        return false;
    }

    ifr.ifr_flags |= IFF_UP | IFF_RUNNING;

    if (ioctl(g_sockFd, SIOCSIFFLAGS, &ifr) < 0) {
        printf("[TUN] Failed to set interface up: %s\n", strerror(errno));
        return false;
    }

    return true;
}

bool AddRoute(const char* destination) {
    char cmd[256];
    snprintf(cmd, sizeof(cmd),
        "ip route add %s dev %s 2>/dev/null || true",
        destination, g_devName);
    int result = system(cmd);

    if (result != 0) {
        printf("[TUN] Warning: Failed to add route %s\n", destination);
        return false;
    }

    printf("[TUN] Added route: %s dev %s\n", destination, g_devName);
    return true;
}

bool ExcludeRoute(const char* destination) {
    // For split tunnel: add route via original gateway
    char cmd[256];
    snprintf(cmd, sizeof(cmd),
        "ip route add %s via $(ip route show default | awk '/default/ {print $3}') 2>/dev/null || true",
        destination);
    int result = system(cmd);

    if (result != 0) {
        printf("[TUN] Warning: Failed to exclude route %s\n", destination);
        return false;
    }

    return true;
}

void SetDnsServer(const char* dnsServer) {
    // Write resolv.conf or use resolvectl
    char cmd[256];
    snprintf(cmd, sizeof(cmd),
        "mkdir -p /etc/unifiedshield && "
        "echo 'nameserver %s' > /etc/unifiedshield/resolv.conf",
        dnsServer);
    system(cmd);

    // Try systemd-resolved first
    snprintf(cmd, sizeof(cmd),
        "resolvectl dns %s %s 2>/dev/null || true",
        g_devName, dnsServer);
    system(cmd);

    printf("[TUN] DNS set to %s\n", dnsServer);
}

void AddDnsServer(const char* dnsServer) {
    char cmd[256];
    snprintf(cmd, sizeof(cmd),
        "echo 'nameserver %s' >> /etc/unifiedshield/resolv.conf",
        dnsServer);
    system(cmd);

    snprintf(cmd, sizeof(cmd),
        "resolvectl dns %s %s 2>/dev/null || true",
        g_devName, dnsServer);
    system(cmd);
}

int ReadPacket(uint8_t* buffer, int bufferSize) {
    if (g_tunFd < 0) return -1;

    ssize_t bytesRead = read(g_tunFd, buffer, bufferSize);
    if (bytesRead < 0) {
        if (errno == EAGAIN || errno == EWOULDBLOCK) {
            return 0;
        }
        return -1;
    }

    return (int)bytesRead;
}

bool WritePacket(const uint8_t* data, int size) {
    if (g_tunFd < 0) return false;

    ssize_t bytesWritten = write(g_tunFd, data, size);
    return bytesWritten == size;
}

void Destroy() {
    if (g_tunFd >= 0) {
        close(g_tunFd);
        g_tunFd = -1;
    }
    if (g_sockFd >= 0) {
        close(g_sockFd);
        g_sockFd = -1;
    }

    // Remove routes and interface
    char cmd[128];
    snprintf(cmd, sizeof(cmd), "ip link del %s 2>/dev/null || true", g_devName);
    system(cmd);

    printf("[TUN] Interface destroyed\n");
}

bool IsOpen() {
    return g_tunFd >= 0;
}

const char* GetDeviceName() {
    return g_devName;
}

} // namespace TunInterface
