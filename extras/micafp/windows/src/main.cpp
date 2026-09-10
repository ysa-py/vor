/**
 * UnifiedShield Windows Main Entry Point
 *
 * System tray VPN client for Windows. No admin required for Wintun.
 * Uses named pipe IPC to communicate with the Rust core service.
 *
 * Cloudflare is BLOCKED in Iran - Chinese CDN (Alibaba, Tencent) primary.
 */

#include <windows.h>
#include <shellapi.h>
#include <commctrl.h>
#include <stdio.h>

#include "tray_icon.h"
#include "service_manager.h"
#include "ipc_client.h"
#include "wintun_interface.h"

#define WM_TRAYICON (WM_USER + 1)
#define ID_TRAY_CONNECT 1001
#define ID_TRAY_DISCONNECT 1002
#define ID_TRAY_SWITCH_CORE 1003
#define ID_TRAY_SETTINGS 1004
#define ID_TRAY_EXIT 1005

static HWND g_hWnd = nullptr;
static HINSTANCE g_hInstance = nullptr;
static bool g_isConnected = false;
static char g_currentCore[32] = "xray";

// Forward declarations
LRESULT CALLBACK WndProc(HWND, UINT, WPARAM, LPARAM);
void UpdateTrayIcon(bool connected);
void HandleCommand(int cmdId);

int WINAPI WinMain(
    _In_ HINSTANCE hInstance,
    _In_opt_ HINSTANCE hPrevInstance,
    _In_ LPSTR lpCmdLine,
    _In_ int nCmdShow
) {
    g_hInstance = hInstance;

    // Prevent multiple instances
    HANDLE hMutex = CreateMutexW(nullptr, TRUE, L"UnifiedShieldSingleInstance");
    if (GetLastError() == ERROR_ALREADY_EXISTS) {
        // Another instance is running - activate it
        HWND existing = FindWindowW(L"UnifiedShieldWndClass", nullptr);
        if (existing) {
            ShowWindow(existing, SW_RESTORE);
            SetForegroundWindow(existing);
        }
        return 0;
    }

    // Register window class
    WNDCLASSEXW wc = {};
    wc.cbSize = sizeof(WNDCLASSEXW);
    wc.lpfnWndProc = WndProc;
    wc.hInstance = hInstance;
    wc.lpszClassName = L"UnifiedShieldWndClass";
    wc.hIcon = LoadIcon(nullptr, IDI_SHIELD);
    RegisterClassExW(&wc);

    // Create hidden message window
    g_hWnd = CreateWindowExW(
        0,
        L"UnifiedShieldWndClass",
        L"UnifiedShield",
        0,
        CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT,
        nullptr, nullptr, hInstance, nullptr
    );

    if (!g_hWnd) {
        MessageBoxW(nullptr, L"Failed to create window", L"Error", MB_ICONERROR);
        ReleaseMutex(hMutex);
        CloseHandle(hMutex);
        return 1;
    }

    // Initialize tray icon
    TrayIcon::Initialize(g_hWnd, WM_TRAYICON);

    // Initialize IPC client (connects to Rust core service)
    IpcClient::Initialize();

    // Check if service is already running
    if (ServiceManager::IsServiceRunning()) {
        g_isConnected = true;
        UpdateTrayIcon(true);
    }

    // Message loop
    MSG msg;
    while (GetMessage(&msg, nullptr, 0, 0)) {
        TranslateMessage(&msg);
        DispatchMessage(&msg);
    }

    // Cleanup
    TrayIcon::Remove();
    IpcClient::Shutdown();

    ReleaseMutex(hMutex);
    CloseHandle(hMutex);

    return (int)msg.wParam;
}

LRESULT CALLBACK WndProc(HWND hWnd, UINT message, WPARAM wParam, LPARAM lParam) {
    switch (message) {
    case WM_TRAYICON:
        switch (LOWORD(lParam)) {
        case WM_RBUTTONUP: {
            // Show context menu
            POINT pt;
            GetCursorPos(&pt);
            HMENU hMenu = CreatePopupMenu();

            if (g_isConnected) {
                AppendMenuW(hMenu, MF_STRING, ID_TRAY_DISCONNECT, L"Disconnect");
                AppendMenuW(hMenu, MF_SEPARATOR, 0, nullptr);
                AppendMenuW(hMenu, MF_STRING, ID_TRAY_SWITCH_CORE, L"Switch Core...");
            } else {
                AppendMenuW(hMenu, MF_STRING, ID_TRAY_CONNECT, L"Connect");
            }

            AppendMenuW(hMenu, MF_SEPARATOR, 0, nullptr);
            AppendMenuW(hMenu, MF_STRING, ID_TRAY_SETTINGS, L"Settings...");
            AppendMenuW(hMenu, MF_SEPARATOR, 0, nullptr);
            AppendMenuW(hMenu, MF_STRING, ID_TRAY_EXIT, L"Exit");

            // Required for menu to dismiss properly
            SetForegroundWindow(hWnd);
            TrackPopupMenu(hMenu, TPM_RIGHTBUTTON, pt.x, pt.y, 0, hWnd, nullptr);
            DestroyMenu(hMenu);
            break;
        }
        case WM_LBUTTONDBLCLK:
            // Double-click to connect/disconnect
            if (g_isConnected) {
                HandleCommand(ID_TRAY_DISCONNECT);
            } else {
                HandleCommand(ID_TRAY_CONNECT);
            }
            break;
        }
        break;

    case WM_COMMAND:
        HandleCommand(LOWORD(wParam));
        break;

    case WM_DESTROY:
        PostQuitMessage(0);
        break;

    default:
        return DefWindowProcW(hWnd, message, wParam, lParam);
    }
    return 0;
}

void HandleCommand(int cmdId) {
    switch (cmdId) {
    case ID_TRAY_CONNECT: {
        // Connect VPN
        bool success = IpcClient::SendCommand("start_daemon xray");
        if (success) {
            g_isConnected = true;
            strcpy_s(g_currentCore, "xray");
            UpdateTrayIcon(true);
        } else {
            MessageBoxW(nullptr, L"Failed to connect. Is the service running?",
                       L"Connection Error", MB_ICONERROR);
        }
        break;
    }
    case ID_TRAY_DISCONNECT: {
        IpcClient::SendCommand("stop_daemon");
        g_isConnected = false;
        UpdateTrayIcon(false);
        break;
    }
    case ID_TRAY_SWITCH_CORE: {
        // Cycle through cores: xray -> naive -> hysteria2 -> tuic -> xray
        const char* cores[] = {"xray", "naive", "hysteria2", "tuic"};
        int current = 0;
        for (int i = 0; i < 4; i++) {
            if (strcmp(g_currentCore, cores[i]) == 0) {
                current = i;
                break;
            }
        }
        int next = (current + 1) % 4;
        char cmd[64];
        snprintf(cmd, sizeof(cmd), "switch_core %s", cores[next]);
        IpcClient::SendCommand(cmd);
        strcpy_s(g_currentCore, cores[next]);

        // Update tray tooltip
        char tip[128];
        snprintf(tip, sizeof(tip), "UnifiedShield - Connected (%s)", cores[next]);
        TrayIcon::SetToolTip(tip);
        break;
    }
    case ID_TRAY_SETTINGS:
        // TODO: Open settings dialog
        MessageBoxW(nullptr, L"Settings dialog coming soon", L"Settings", MB_INFORMATION);
        break;
    case ID_TRAY_EXIT:
        if (g_isConnected) {
            int result = MessageBoxW(nullptr,
                L"VPN is still connected. Disconnect and exit?",
                L"Confirm Exit", MB_YESNO | MB_ICONQUESTION);
            if (result == IDNO) return;
            IpcClient::SendCommand("stop_daemon");
        }
        DestroyWindow(g_hWnd);
        break;
    }
}

void UpdateTrayIcon(bool connected) {
    if (connected) {
        TrayIcon::SetIcon(LoadIcon(nullptr, IDI_SHIELD));
        TrayIcon::SetToolTip("UnifiedShield - Connected");
    } else {
        TrayIcon::SetIcon(LoadIcon(nullptr, IDI_APPLICATION));
        TrayIcon::SetToolTip("UnifiedShield - Disconnected");
    }
}
