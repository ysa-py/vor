/**
 * System tray icon management for UnifiedShield Windows client.
 */

#include "tray_icon.h"

#include <windows.h>
#include <shellapi.h>
#include <cstdio>

#define TRAY_ICON_ID 1

static HWND g_hWnd = nullptr;
static UINT g_callbackMsg = 0;
static NOTIFYICONDATAW g_nid = {};

namespace TrayIcon {

bool Initialize(HWND hWnd, UINT callbackMsg) {
    g_hWnd = hWnd;
    g_callbackMsg = callbackMsg;

    ZeroMemory(&g_nid, sizeof(g_nid));
    g_nid.cbSize = sizeof(NOTIFYICONDATAW);
    g_nid.hWnd = hWnd;
    g_nid.uID = TRAY_ICON_ID;
    g_nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    g_nid.uCallbackMessage = callbackMsg;
    g_nid.hIcon = LoadIcon(nullptr, IDI_APPLICATION);
    wcscpy_s(g_nid.szTip, L"UnifiedShield - Disconnected");

    BOOL result = Shell_NotifyIconW(NIM_ADD, &g_nid);
    if (!result) {
        printf("[TrayIcon] Failed to add tray icon (error: %lu)\n", GetLastError());
        return false;
    }

    printf("[TrayIcon] Initialized\n");
    return true;
}

void Remove() {
    Shell_NotifyIconW(NIM_DELETE, &g_nid);
    printf("[TrayIcon] Removed\n");
}

void SetIcon(HICON hIcon) {
    g_nid.hIcon = hIcon;
    g_nid.uFlags = NIF_ICON;
    Shell_NotifyIconW(NIM_MODIFY, &g_nid);
}

void SetToolTip(const char* tip) {
    // Convert ASCII to wide string for tooltip
    wchar_t wtip[128];
    MultiByteToWideChar(CP_UTF8, 0, tip, -1, wtip, 128);
    SetToolTip(wtip);
}

void SetToolTip(const wchar_t* tip) {
    wcscpy_s(g_nid.szTip, tip);
    g_nid.uFlags = NIF_TIP;
    Shell_NotifyIconW(NIM_MODIFY, &g_nid);
}

void ShowNotification(const char* title, const char* message, DWORD iconType) {
    g_nid.uFlags = NIF_INFO;
    g_nid.dwInfoFlags = iconType;

    wchar_t wtitle[64];
    wchar_t wmessage[256];
    MultiByteToWideChar(CP_UTF8, 0, title, -1, wtitle, 64);
    MultiByteToWideChar(CP_UTF8, 0, message, -1, wmessage, 256);

    wcscpy_s(g_nid.szInfoTitle, wtitle);
    wcscpy_s(g_nid.szInfo, wmessage);

    Shell_NotifyIconW(NIM_MODIFY, &g_nid);
}

} // namespace TrayIcon
