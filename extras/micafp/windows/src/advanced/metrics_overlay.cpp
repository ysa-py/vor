// MICAFP UnifiedShield VIP-ULTRA — Windows Metrics Overlay
// Displays live shield metrics in the system tray tooltip and notification area.
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <shellapi.h>
#include <string>
#include <winhttp.h>
#pragma comment(lib, "winhttp.lib")

// Fetch Prometheus metrics from daemon
std::string FetchMetrics() {
    HINTERNET hSession = WinHttpOpen(L"ShieldMetrics/1.0",
        WINHTTP_ACCESS_TYPE_NO_PROXY, WINHTTP_NO_PROXY_NAME, WINHTTP_NO_PROXY_BYPASS, 0);
    if (!hSession) return "";
    HINTERNET hConnect = WinHttpConnect(hSession, L"127.0.0.1", 9090, 0);
    HINTERNET hRequest = WinHttpOpenRequest(hConnect, L"GET", L"/metrics",
        nullptr, WINHTTP_NO_REFERER, WINHTTP_DEFAULT_ACCEPT_TYPES, 0);
    if (!WinHttpSendRequest(hRequest, WINHTTP_NO_ADDITIONAL_HEADERS, 0,
        WINHTTP_NO_REQUEST_DATA, 0, 0, 0) || !WinHttpReceiveResponse(hRequest, nullptr)) {
        WinHttpCloseHandle(hRequest); WinHttpCloseHandle(hConnect);
        WinHttpCloseHandle(hSession); return "";
    }
    std::string response;
    DWORD size = 0;
    do {
        WinHttpQueryDataAvailable(hRequest, &size);
        if (!size) break;
        std::string buf(size, 0);
        DWORD read = 0;
        WinHttpReadData(hRequest, buf.data(), size, &read);
        response += buf.substr(0, read);
    } while (size);
    WinHttpCloseHandle(hRequest); WinHttpCloseHandle(hConnect); WinHttpCloseHandle(hSession);
    return response;
}

// Update tray tooltip with live metrics
void UpdateTrayTooltip(NOTIFYICONDATA& nid, const std::string& metrics) {
    std::wstring tip = L"UnifiedShield VIP-ULTRA\n";
    // Extract DPI probability
    auto pos = metrics.find("shield_dpi_detection_probability ");
    if (pos != std::string::npos) {
        double dpi = std::stod(metrics.substr(pos + 35, 6));
        tip += L"DPI Risk: " + std::to_wstring((int)(dpi * 100)) + L"%\n";
    }
    pos = metrics.find("shield_nain_status ");
    if (pos != std::string::npos) {
        int nain = std::stoi(metrics.substr(pos + 19, 1));
        tip += nain == 0 ? L"Internet: Normal" : nain == 1 ? L"Internet: Partial" : L"Internet: BLOCKED";
    }
    wcsncpy_s(nid.szTip, tip.c_str(), 127);
    Shell_NotifyIcon(NIM_MODIFY, &nid);
}
