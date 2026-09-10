/**
 * systemd integration for UnifiedShield Linux daemon.
 *
 * Provides:
 * - Service notification (READY=1, STATUS=..., WATCHDOG=1)
 * - Watchdog support for automatic restart on failure
 * - Socket activation support
 */

#include "systemd_service.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <unistd.h>
#include <sys/socket.h>
#include <sys/un.h>

// sd-daemon function prototypes (inline if libsystemd not available)
#ifndef HAVE_SYSTEMD

static int sd_notify(int unset_environment, const char* state) {
    const char* sock = getenv("NOTIFY_SOCKET");
    if (!sock) return 0;

    int fd = socket(AF_UNIX, SOCK_DGRAM, 0);
    if (fd < 0) return -1;

    struct sockaddr_un addr;
    memset(&addr, 0, sizeof(addr));
    addr.sun_family = AF_UNIX;

    // Handle abstract socket (@ prefix) and filesystem socket
    if (sock[0] == '@') {
        addr.sun_path[0] = '\0';
        strncpy(addr.sun_path + 1, sock + 1, sizeof(addr.sun_path) - 2);
    } else {
        strncpy(addr.sun_path, sock, sizeof(addr.sun_path) - 1);
    }

    size_t len = strlen(state);
    ssize_t result = sendto(fd, state, len, MSG_NOSIGNAL,
                           (struct sockaddr*)&addr, sizeof(addr));
    close(fd);

    if (unset_environment) {
        unsetenv("NOTIFY_SOCKET");
    }

    return (result == (ssize_t)len) ? 1 : -1;
}

static int sd_notifyf(int unset_environment, const char* format, ...) {
    char buf[256];
    va_list ap;
    va_start(ap, format);
    vsnprintf(buf, sizeof(buf), format, ap);
    va_end(ap);
    return sd_notify(unset_environment, buf);
}

#else
#include <systemd/sd-daemon.h>
#endif

namespace SystemdService {

void NotifyReady() {
    sd_notify(0, "READY=1");
    printf("[systemd] Notified READY=1\n");
}

void NotifyStopping() {
    sd_notify(0, "STOPPING=1");
    printf("[systemd] Notified STOPPING=1\n");
}

void NotifyStatus(const char* status) {
    char msg[256];
    snprintf(msg, sizeof(msg), "STATUS=%s", status);
    sd_notify(0, msg);
}

void NotifyWatchdog() {
    sd_notify(0, "WATCHDOG=1");
}

void NotifyMainPid() {
    sd_notifyf(0, "MAINPID=%lu", (unsigned long)getpid());
}

void NotifyExtendedStatus(const char* core, bool connected, long bytesUp, long bytesDown) {
    sd_notifyf(0,
        "STATUS=Connected (%s core) - ↑%ld ↓%ld",
        core, bytesUp, bytesDown);
}

int GetListenFds() {
    const char* count = getenv("LISTEN_FDS");
    if (!count) return 0;
    return atoi(count);
}

bool IsSocketActivated() {
    return GetListenFds() > 0;
}

} // namespace SystemdService
