/**
 * Unix Domain Socket IPC Client for UnifiedShield Linux.
 *
 * Communicates with the UnifiedShield daemon via Unix sockets.
 * Socket path: /run/unifiedshield/ipc.sock
 *
 * Commands:
 * - start_daemon <core>
 * - stop_daemon
 * - switch_core <core>
 * - get_status
 * - set_kill_switch <true|false>
 * - trigger_obfuscation
 */

#include "ipc_client.h"

#include <cstdio>
#include <cstring>
#include <unistd.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <errno.h>

#define IPC_SOCKET_PATH "/run/unifiedshield/ipc.sock"
#define BUFFER_SIZE 4096
#define CONNECT_TIMEOUT_MS 3000

static int g_sockFd = -1;
static bool g_initialized = false;

namespace IpcClient {

bool Initialize() {
    g_initialized = true;
    printf("[IPC] Client initialized\n");
    return true;
}

bool Connect() {
    if (g_sockFd >= 0) return true;

    g_sockFd = socket(AF_UNIX, SOCK_STREAM, 0);
    if (g_sockFd < 0) {
        printf("[IPC] Failed to create socket: %s\n", strerror(errno));
        return false;
    }

    struct sockaddr_un addr;
    memset(&addr, 0, sizeof(addr));
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, IPC_SOCKET_PATH, sizeof(addr.sun_path) - 1);

    if (connect(g_sockFd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        printf("[IPC] Failed to connect to %s: %s\n", IPC_SOCKET_PATH, strerror(errno));
        close(g_sockFd);
        g_sockFd = -1;
        return false;
    }

    printf("[IPC] Connected to daemon\n");
    return true;
}

void Disconnect() {
    if (g_sockFd >= 0) {
        close(g_sockFd);
        g_sockFd = -1;
    }
}

bool SendCommand(const char* command) {
    if (!Connect()) return false;

    size_t len = strlen(command);
    ssize_t sent = send(g_sockFd, command, len, MSG_NOSIGNAL);
    if (sent != (ssize_t)len) {
        printf("[IPC] Failed to send command: %s\n", strerror(errno));
        Disconnect();
        return false;
    }

    // Read response
    char response[BUFFER_SIZE] = {};
    ssize_t received = recv(g_sockFd, response, BUFFER_SIZE - 1, 0);
    if (received < 0) {
        printf("[IPC] Failed to read response: %s\n", strerror(errno));
        Disconnect();
        return false;
    }

    response[received] = '\0';
    bool success = (strncmp(response, "OK", 2) == 0);

    if (!success) {
        printf("[IPC] Command '%s' returned: %s\n", command, response);
    }

    return success;
}

char* SendQuery(const char* command) {
    if (!Connect()) return nullptr;

    size_t len = strlen(command);
    ssize_t sent = send(g_sockFd, command, len, MSG_NOSIGNAL);
    if (sent != (ssize_t)len) {
        Disconnect();
        return nullptr;
    }

    char* response = new char[BUFFER_SIZE];
    memset(response, 0, BUFFER_SIZE);

    ssize_t received = recv(g_sockFd, response, BUFFER_SIZE - 1, 0);
    if (received < 0) {
        delete[] response;
        Disconnect();
        return nullptr;
    }

    response[received] = '\0';
    return response;
}

void FreeResponse(char* response) {
    delete[] response;
}

void ProcessCommands() {
    // Non-blocking check for pending IPC commands
    // Used in the main loop of the daemon
    if (g_sockFd < 0) return;

    fd_set readFds;
    struct timeval tv;
    tv.tv_sec = 0;
    tv.tv_usec = 1000; // 1ms timeout

    FD_ZERO(&readFds);
    FD_SET(g_sockFd, &readFds);

    int result = select(g_sockFd + 1, &readFds, nullptr, nullptr, &tv);
    if (result <= 0) return;

    char buffer[BUFFER_SIZE];
    ssize_t received = recv(g_sockFd, buffer, BUFFER_SIZE - 1, MSG_DONTWAIT);
    if (received <= 0) return;

    buffer[received] = '\0';
    // Process command and send response
    // (handled by the daemon's command processor)
}

void Shutdown() {
    Disconnect();
    g_initialized = false;
    printf("[IPC] Client shutdown\n");
}

bool IsConnected() {
    return g_sockFd >= 0;
}

} // namespace IpcClient
