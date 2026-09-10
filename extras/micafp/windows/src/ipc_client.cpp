/**
 * Named Pipe IPC Client for UnifiedShield Windows.
 *
 * Communicates with the UnifiedShield service (Rust core) via
 * named pipes. Commands include:
 * - start_daemon <core>
 * - stop_daemon
 * - switch_core <core>
 * - get_status
 * - set_kill_switch <true|false>
 * - trigger_obfuscation
 *
 * The pipe name is: \\.\pipe\unifiedshield-ipc
 */

#include "ipc_client.h"

#include <windows.h>
#include <cstdio>
#include <cstring>

#define PIPE_NAME L"\\\\.\\pipe\\unifiedshield-ipc"
#define BUFFER_SIZE 4096
#define CONNECT_TIMEOUT_MS 5000

static HANDLE g_pipeHandle = INVALID_HANDLE_VALUE;
static bool g_initialized = false;

namespace IpcClient {

bool Initialize() {
    g_initialized = true;
    printf("[IPC] Client initialized\n");
    return true;
}

bool Connect() {
    if (g_pipeHandle != INVALID_HANDLE_VALUE) {
        return true; // Already connected
    }

    // Try to connect to the named pipe
    g_pipeHandle = CreateFileW(
        PIPE_NAME,
        GENERIC_READ | GENERIC_WRITE,
        0,
        nullptr,
        OPEN_EXISTING,
        0,
        nullptr
    );

    if (g_pipeHandle == INVALID_HANDLE_VALUE) {
        // Try again if pipe is busy
        if (GetLastError() == ERROR_PIPE_BUSY) {
            if (!WaitNamedPipeW(PIPE_NAME, CONNECT_TIMEOUT_MS)) {
                printf("[IPC] Timed out waiting for pipe\n");
                return false;
            }
            g_pipeHandle = CreateFileW(
                PIPE_NAME,
                GENERIC_READ | GENERIC_WRITE,
                0,
                nullptr,
                OPEN_EXISTING,
                0,
                nullptr
            );
        }

        if (g_pipeHandle == INVALID_HANDLE_VALUE) {
            printf("[IPC] Failed to connect to pipe (error: %lu)\n", GetLastError());
            return false;
        }
    }

    // Set pipe to message mode
    DWORD mode = PIPE_READMODE_MESSAGE;
    SetNamedPipeHandleState(g_pipeHandle, &mode, nullptr, nullptr);

    printf("[IPC] Connected to service\n");
    return true;
}

void Disconnect() {
    if (g_pipeHandle != INVALID_HANDLE_VALUE) {
        CloseHandle(g_pipeHandle);
        g_pipeHandle = INVALID_HANDLE_VALUE;
    }
}

bool SendCommand(const char* command) {
    if (!Connect()) {
        return false;
    }

    DWORD bytesWritten = 0;
    BOOL result = WriteFile(
        g_pipeHandle,
        command,
        (DWORD)strlen(command),
        &bytesWritten,
        nullptr
    );

    if (!result) {
        printf("[IPC] Failed to send command (error: %lu)\n", GetLastError());
        Disconnect();
        return false;
    }

    // Read response
    char response[BUFFER_SIZE] = {};
    DWORD bytesRead = 0;
    result = ReadFile(
        g_pipeHandle,
        response,
        BUFFER_SIZE - 1,
        &bytesRead,
        nullptr
    );

    if (!result && GetLastError() != ERROR_MORE_DATA) {
        printf("[IPC] Failed to read response (error: %lu)\n", GetLastError());
        Disconnect();
        return false;
    }

    response[bytesRead] = '\0';

    // Check for success response
    bool success = (strncmp(response, "OK", 2) == 0);

    if (!success) {
        printf("[IPC] Command '%s' returned error: %s\n", command, response);
    }

    return success;
}

char* SendQuery(const char* command) {
    if (!Connect()) {
        return nullptr;
    }

    DWORD bytesWritten = 0;
    BOOL result = WriteFile(
        g_pipeHandle,
        command,
        (DWORD)strlen(command),
        &bytesWritten,
        nullptr
    );

    if (!result) {
        Disconnect();
        return nullptr;
    }

    // Read response
    char* response = new char[BUFFER_SIZE];
    ZeroMemory(response, BUFFER_SIZE);
    DWORD bytesRead = 0;

    result = ReadFile(
        g_pipeHandle,
        response,
        BUFFER_SIZE - 1,
        &bytesRead,
        nullptr
    );

    if (!result && GetLastError() != ERROR_MORE_DATA) {
        delete[] response;
        Disconnect();
        return nullptr;
    }

    response[bytesRead] = '\0';
    return response;
}

void FreeResponse(char* response) {
    delete[] response;
}

void Shutdown() {
    Disconnect();
    g_initialized = false;
    printf("[IPC] Client shutdown\n");
}

bool IsConnected() {
    return g_pipeHandle != INVALID_HANDLE_VALUE;
}

} // namespace IpcClient
