/**
 * Wintun userspace tunnel adapter for Windows.
 *
 * Creates a Wintun adapter without requiring admin privileges.
 * The adapter is configured with:
 * - IP: 172.19.0.1/24
 * - MTU: 1380
 * - Routes: 0.0.0.0/0 and ::/0 (via Rust core)
 *
 * Wintun is a GPL-compatible TUN driver for Windows by WireGuard.
 * No TAP adapter or kernel driver installation required.
 */

#include "wintun_interface.h"

#include <windows.h>
#include <iphlpapi.h>
#include <ws2tcpip.h>
#include <cstdio>
#include <cstring>

// Wintun headers
#define WINTUN_DLL "wintun.dll"
#define WINTUN_LOGGER_LEVEL_INFO 0
#define WINTUN_LOGGER_LEVEL_WARN 1
#define WINTUN_LOGGER_LEVEL_ERR 2

// Wintun function typedefs
typedef const char* (*WINTUN_GET_VERSION_FUNC)(DWORD*, DWORD*);
typedef void* (*WINTUN_CREATE_ADAPTER_FUNC)(LPCWSTR, LPCWSTR, const GUID*, BOOL*);
typedef BOOL (*WINTUN_DELETE_ADAPTER_FUNC)(void*, BOOL);
typedef void (*WINTUN_CLOSE_ADAPTER_FUNC)(void*);
typedef DWORD (*WINTUN_GET_RUNNING_ADAPTER_COUNT_FUNC)(void);
typedef HANDLE (*WINTUN_CREATE_SESSION_FUNC)(void*, DWORD);
typedef void (*WINTUN_END_SESSION_FUNC)(HANDLE);
typedef BYTE* (*WINTUN_RECEIVE_PACKET_FUNC)(HANDLE, DWORD*);
typedef void (*WINTUN_RELEASE_RECEIVE_PACKET_FUNC)(HANDLE, BYTE*);
typedef BYTE* (*WINTUN_ALLOCATE_SEND_PACKET_FUNC)(HANDLE, DWORD);
typedef void (*WINTUN_SEND_PACKET_FUNC)(HANDLE, BYTE*);

static HMODULE g_wintunDll = nullptr;
static void* g_adapter = nullptr;
static HANDLE g_session = nullptr;
static bool g_isRunning = false;

// Wintun function pointers
static WINTUN_CREATE_ADAPTER_FUNC WintunCreateAdapter = nullptr;
static WINTUN_DELETE_ADAPTER_FUNC WintunDeleteAdapter = nullptr;
static WINTUN_CLOSE_ADAPTER_FUNC WintunCloseAdapter = nullptr;
static WINTUN_GET_RUNNING_ADAPTER_COUNT_FUNC WintunGetRunningAdapterCount = nullptr;
static WINTUN_CREATE_SESSION_FUNC WintunCreateSession = nullptr;
static WINTUN_END_SESSION_FUNC WintunEndSession = nullptr;
static WINTUN_RECEIVE_PACKET_FUNC WintunReceivePacket = nullptr;
static WINTUN_RELEASE_RECEIVE_PACKET_FUNC WintunReleaseReceivePacket = nullptr;
static WINTUN_ALLOCATE_SEND_PACKET_FUNC WintunAllocateSendPacket = nullptr;
static WINTUN_SEND_PACKET_FUNC WintunSendPacket = nullptr;

// GUID for our adapter
static const GUID ADAPTER_GUID = {
    0x1a2b3c4d, 0x5e6f, 0x7a8b,
    {0x9c, 0x0d, 0x1e, 0x2f, 0x3a, 0x4b, 0x5c, 0x6d}
};

namespace WintunInterface {

bool Initialize() {
    // Load Wintun DLL
    g_wintunDll = LoadLibraryA(WINTUN_DLL);
    if (!g_wintunDll) {
        printf("[Wintun] Failed to load %s (error: %lu)\n", WINTUN_DLL, GetLastError());
        return false;
    }

    // Resolve function pointers
    #define RESOLVE_FUNC(name) \
        name = (name##_FUNC)GetProcAddress(g_wintunDll, #name); \
        if (!name) { \
            printf("[Wintun] Failed to resolve " #name "\n"); \
            FreeLibrary(g_wintunDll); \
            g_wintunDll = nullptr; \
            return false; \
        }

    WintunCreateAdapter = (WINTUN_CREATE_ADAPTER_FUNC)GetProcAddress(g_wintunDll, "WintunCreateAdapter");
    WintunCloseAdapter = (WINTUN_CLOSE_ADAPTER_FUNC)GetProcAddress(g_wintunDll, "WintunCloseAdapter");
    WintunCreateSession = (WINTUN_CREATE_SESSION_FUNC)GetProcAddress(g_wintunDll, "WintunCreateSession");
    WintunEndSession = (WINTUN_END_SESSION_FUNC)GetProcAddress(g_wintunDll, "WintunEndSession");
    WintunReceivePacket = (WINTUN_RECEIVE_PACKET_FUNC)GetProcAddress(g_wintunDll, "WintunReceivePacket");
    WintunReleaseReceivePacket = (WINTUN_RELEASE_RECEIVE_PACKET_FUNC)GetProcAddress(g_wintunDll, "WintunReleaseReceivePacket");
    WintunAllocateSendPacket = (WINTUN_ALLOCATE_SEND_PACKET_FUNC)GetProcAddress(g_wintunDll, "WintunAllocateSendPacket");
    WintunSendPacket = (WINTUN_SEND_PACKET_FUNC)GetProcAddress(g_wintunDll, "WintunSendPacket");

    if (!WintunCreateAdapter || !WintunCloseAdapter || !WintunCreateSession ||
        !WintunEndSession || !WintunReceivePacket || !WintunReleaseReceivePacket ||
        !WintunAllocateSendPacket || !WintunSendPacket) {
        printf("[Wintun] Failed to resolve required functions\n");
        FreeLibrary(g_wintunDll);
        g_wintunDll = nullptr;
        return false;
    }

    printf("[Wintun] Initialized successfully\n");
    return true;
}

bool CreateAdapter() {
    if (!WintunCreateAdapter) return false;

    BOOL rebootRequired = FALSE;
    g_adapter = WintunCreateAdapter(
        L"UnifiedShield",
        L"UnifiedShield TUN",
        &ADAPTER_GUID,
        &rebootRequired
    );

    if (!g_adapter) {
        printf("[Wintun] Failed to create adapter (error: %lu)\n", GetLastError());
        return false;
    }

    printf("[Wintun] Adapter created successfully\n");
    return true;
}

bool Configure(const char* ipAddress, int prefixLength, int mtu) {
    if (!g_adapter) return false;

    // Get adapter LUID
    NET_LUID luid;
    DWORD err = ConvertInterfaceGuidToLuid(&ADAPTER_GUID, &luid);
    if (err != NO_ERROR) {
        printf("[Wintun] Failed to get adapter LUID (error: %lu)\n", err);
        return false;
    }

    // Get interface index
    NET_IFINDEX ifIndex;
    err = ConvertInterfaceLuidToIndex(&luid, &ifIndex);
    if (err != NO_ERROR) {
        printf("[Wintun] Failed to get interface index (error: %lu)\n", err);
        return false;
    }

    // Set IP address using netsh
    char cmd[256];
    snprintf(cmd, sizeof(cmd),
        "netsh interface ip set address \"UnifiedShield\" static %s %d",
        ipAddress, prefixLength);
    system(cmd);

    // Set DNS servers (Chinese CDN - Cloudflare blocked in Iran)
    system("netsh interface ip set dns \"UnifiedShield\" static 223.5.5.5 primary");
    system("netsh interface ip add dns \"UnifiedShield\" 119.29.29.29 index=2");

    printf("[Wintun] Adapter configured: %s/%d, MTU=%d, DNS=223.5.5.5\n",
           ipAddress, prefixLength, mtu);

    return true;
}

bool StartSession(int mtu) {
    if (!g_adapter || !WintunCreateSession) return false;

    g_session = WintunCreateSession(g_adapter, (DWORD)mtu);
    if (!g_session) {
        printf("[Wintun] Failed to create session (error: %lu)\n", GetLastError());
        return false;
    }

    g_isRunning = true;
    printf("[Wintun] Session started (MTU=%d)\n", mtu);
    return true;
}

void StopSession() {
    if (g_session && WintunEndSession) {
        WintunEndSession(g_session);
        g_session = nullptr;
    }
    g_isRunning = false;
    printf("[Wintun] Session stopped\n");
}

int ReadPacket(BYTE* buffer, int bufferSize) {
    if (!g_session || !WintunReceivePacket) return -1;

    DWORD packetSize = 0;
    BYTE* packet = WintunReceivePacket(g_session, &packetSize);

    if (!packet) {
        // No packet available
        return 0;
    }

    if (packetSize > (DWORD)bufferSize) {
        WintunReleaseReceivePacket(g_session, packet);
        return -1;
    }

    memcpy(buffer, packet, packetSize);
    WintunReleaseReceivePacket(g_session, packet);

    return (int)packetSize;
}

bool WritePacket(const BYTE* data, int size) {
    if (!g_session || !WintunAllocateSendPacket || !WintunSendPacket) return false;

    BYTE* packet = WintunAllocateSendPacket(g_session, (DWORD)size);
    if (!packet) {
        return false;
    }

    memcpy(packet, data, size);
    WintunSendPacket(g_session, packet);
    return true;
}

void DestroyAdapter() {
    StopSession();

    if (g_adapter && WintunCloseAdapter) {
        WintunCloseAdapter(g_adapter);
        g_adapter = nullptr;
    }

    printf("[Wintun] Adapter destroyed\n");
}

void Shutdown() {
    DestroyAdapter();

    if (g_wintunDll) {
        FreeLibrary(g_wintunDll);
        g_wintunDll = nullptr;
    }

    WintunCreateAdapter = nullptr;
    WintunCloseAdapter = nullptr;
    WintunCreateSession = nullptr;
    WintunEndSession = nullptr;
    WintunReceivePacket = nullptr;
    WintunReleaseReceivePacket = nullptr;
    WintunAllocateSendPacket = nullptr;
    WintunSendPacket = nullptr;

    printf("[Wintun] Shutdown complete\n");
}

bool IsRunning() {
    return g_isRunning;
}

} // namespace WintunInterface
