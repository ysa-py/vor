/**
 * Windows Service Manager for UnifiedShield.
 *
 * Manages the UnifiedShield Windows service that runs the Rust core.
 * The tray app communicates with this service via named pipes.
 *
 * Service lifecycle:
 * - Auto-start on boot (optional)
 * - Runs under LocalSystem or specific user account
 * - Communicates with tray app via named pipe IPC
 * - Manages Wintun adapter and routing
 */

#include "service_manager.h"

#include <windows.h>
#include <winsvc.h>
#include <cstdio>
#include <cstring>

#define SERVICE_NAME L"UnifiedShield"
#define SERVICE_DISPLAY_NAME L"UnifiedShield VPN Service"
#define SERVICE_DESCRIPTION L"UnifiedShield anti-censorship VPN core service"

static SERVICE_STATUS g_serviceStatus = {};
static SERVICE_STATUS_HANDLE g_statusHandle = nullptr;
static HANDLE g_serviceStopEvent = nullptr;

namespace ServiceManager {

bool IsServiceInstalled() {
    SC_HANDLE scm = OpenSCManagerW(nullptr, nullptr, SC_MANAGER_CONNECT);
    if (!scm) return false;

    SC_HANDLE service = OpenServiceW(scm, SERVICE_NAME, SERVICE_QUERY_STATUS);
    bool installed = (service != nullptr);

    if (service) CloseServiceHandle(service);
    CloseServiceHandle(scm);

    return installed;
}

bool IsServiceRunning() {
    SC_HANDLE scm = OpenSCManagerW(nullptr, nullptr, SC_MANAGER_CONNECT);
    if (!scm) return false;

    SC_HANDLE service = OpenServiceW(scm, SERVICE_NAME, SERVICE_QUERY_STATUS);
    if (!service) {
        CloseServiceHandle(scm);
        return false;
    }

    SERVICE_STATUS status;
    BOOL result = QueryServiceStatus(service, &status);

    CloseServiceHandle(service);
    CloseServiceHandle(scm);

    return result && (status.dwCurrentState == SERVICE_RUNNING);
}

bool InstallService() {
    SC_HANDLE scm = OpenSCManagerW(nullptr, nullptr, SC_MANAGER_CREATE_SERVICE);
    if (!scm) {
        printf("[Service] Failed to open SCM (error: %lu)\n", GetLastError());
        return false;
    }

    // Get current executable path
    wchar_t path[MAX_PATH];
    GetModuleFileNameW(nullptr, path, MAX_PATH);

    SC_HANDLE service = CreateServiceW(
        scm,
        SERVICE_NAME,
        SERVICE_DISPLAY_NAME,
        SERVICE_ALL_ACCESS,
        SERVICE_WIN32_OWN_PROCESS,
        SERVICE_AUTO_START,
        SERVICE_ERROR_NORMAL,
        path,
        nullptr, nullptr, nullptr,
        nullptr, nullptr
    );

    if (!service) {
        printf("[Service] Failed to create service (error: %lu)\n", GetLastError());
        CloseServiceHandle(scm);
        return false;
    }

    // Set service description
    SERVICE_DESCRIPTIONW desc;
    desc.lpDescription = (LPWSTR)SERVICE_DESCRIPTION;
    ChangeServiceConfig2W(service, SERVICE_CONFIG_DESCRIPTION, &desc);

    // Set delayed auto-start
    SERVICE_DELAYED_AUTO_START_INFO delayedStart;
    delayedStart.fDelayedAutostart = TRUE;
    ChangeServiceConfig2W(service, SERVICE_CONFIG_DELAYED_AUTO_START_INFO, &delayedStart);

    CloseServiceHandle(service);
    CloseServiceHandle(scm);

    printf("[Service] Service installed successfully\n");
    return true;
}

bool UninstallService() {
    SC_HANDLE scm = OpenSCManagerW(nullptr, nullptr, SC_MANAGER_CONNECT);
    if (!scm) return false;

    SC_HANDLE service = OpenServiceW(scm, SERVICE_NAME, SERVICE_ALL_ACCESS);
    if (!service) {
        CloseServiceHandle(scm);
        return false;
    }

    // Stop service first
    SERVICE_STATUS status;
    ControlService(service, SERVICE_CONTROL_STOP, &status);

    // Wait for service to stop
    for (int i = 0; i < 30; i++) {
        QueryServiceStatus(service, &status);
        if (status.dwCurrentState == SERVICE_STOPPED) break;
        Sleep(1000);
    }

    BOOL result = DeleteService(service);

    CloseServiceHandle(service);
    CloseServiceHandle(scm);

    return result != FALSE;
}

bool StartService() {
    SC_HANDLE scm = OpenSCManagerW(nullptr, nullptr, SC_MANAGER_CONNECT);
    if (!scm) return false;

    SC_HANDLE service = OpenServiceW(scm, SERVICE_NAME, SERVICE_START);
    if (!service) {
        CloseServiceHandle(scm);
        return false;
    }

    BOOL result = StartServiceW(service, 0, nullptr);

    CloseServiceHandle(service);
    CloseServiceHandle(scm);

    return result != FALSE;
}

bool StopService() {
    SC_HANDLE scm = OpenSCManagerW(nullptr, nullptr, SC_MANAGER_CONNECT);
    if (!scm) return false;

    SC_HANDLE service = OpenServiceW(scm, SERVICE_NAME, SERVICE_STOP);
    if (!service) {
        CloseServiceHandle(scm);
        return false;
    }

    SERVICE_STATUS status;
    BOOL result = ControlService(service, SERVICE_CONTROL_STOP, &status);

    CloseServiceHandle(service);
    CloseServiceHandle(scm);

    return result != FALSE;
}

// Service entry point (called by Windows Service Control Manager)
void WINAPI ServiceMain(DWORD argc, LPWSTR* argv) {
    g_statusHandle = RegisterServiceCtrlHandlerW(SERVICE_NAME, ServiceCtrlHandler);
    if (!g_statusHandle) return;

    // Report starting
    g_serviceStatus.dwServiceType = SERVICE_WIN32_OWN_PROCESS;
    g_serviceStatus.dwCurrentState = SERVICE_START_PENDING;
    g_serviceStatus.dwControlsAccepted = 0;
    SetServiceStatus(g_statusHandle, &g_serviceStatus);

    // Create stop event
    g_serviceStopEvent = CreateEventW(nullptr, TRUE, FALSE, nullptr);

    // Initialize Wintun adapter
    // (Wintun and core initialization happens here)

    // Report running
    g_serviceStatus.dwCurrentState = SERVICE_RUNNING;
    g_serviceStatus.dwControlsAccepted = SERVICE_ACCEPT_STOP | SERVICE_ACCEPT_SHUTDOWN;
    SetServiceStatus(g_statusHandle, &g_serviceStatus);

    // Wait for stop signal
    WaitForSingleObject(g_serviceStopEvent, INFINITE);

    // Cleanup
    CloseHandle(g_serviceStopEvent);

    g_serviceStatus.dwCurrentState = SERVICE_STOPPED;
    SetServiceStatus(g_statusHandle, &g_serviceStatus);
}

void WINAPI ServiceCtrlHandler(DWORD ctrlCode) {
    switch (ctrlCode) {
    case SERVICE_CONTROL_STOP:
    case SERVICE_CONTROL_SHUTDOWN:
        g_serviceStatus.dwCurrentState = SERVICE_STOP_PENDING;
        SetServiceStatus(g_statusHandle, &g_serviceStatus);
        SetEvent(g_serviceStopEvent);
        break;
    default:
        break;
    }
}

} // namespace ServiceManager
