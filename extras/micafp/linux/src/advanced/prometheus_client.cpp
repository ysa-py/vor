// MICAFP UnifiedShield VIP-ULTRA — Linux Prometheus Metrics Client
// Polls daemon's /metrics endpoint and displays real-time status in CLI.
#include <iostream>
#include <string>
#include <regex>
#include <curl/curl.h>

static size_t write_cb(void* ptr, size_t size, size_t nmemb, std::string* s) {
    s->append(static_cast<char*>(ptr), size * nmemb);
    return size * nmemb;
}

std::string fetch_metrics(const std::string& url = "http://127.0.0.1:9090/metrics") {
    CURL* curl = curl_easy_init();
    if (!curl) return "";
    std::string response;
    curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, write_cb);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &response);
    curl_easy_setopt(curl, CURLOPT_TIMEOUT, 3L);
    curl_easy_perform(curl);
    curl_easy_cleanup(curl);
    return response;
}

double extract_metric(const std::string& metrics, const std::string& name) {
    std::regex re(name + R"((?:\{[^}]*\})? ([0-9.]+))");
    std::smatch m;
    if (std::regex_search(metrics, m, re)) return std::stod(m[1]);
    return -1.0;
}

void display_status(const std::string& metrics) {
    std::cout << "\033[2J\033[H"; // clear screen
    std::cout << "╔══════════════════════════════════════════════════════╗\n";
    std::cout << "║  MICAFP UnifiedShield VIP-ULTRA — Live Status        ║\n";
    std::cout << "╠══════════════════════════════════════════════════════╣\n";

    double dpi   = extract_metric(metrics, "shield_dpi_detection_probability");
    double uptime = extract_metric(metrics, "shield_uptime_seconds");
    double peers  = extract_metric(metrics, "shield_p2p_peers_active");
    double nain   = extract_metric(metrics, "shield_nain_status");
    double pqkex  = extract_metric(metrics, "shield_post_quantum_kex_total");
    double switches = extract_metric(metrics, "shield_core_switches_total");

    auto status_color = [](double v, double ok, double warn) -> std::string {
        if (v < 0) return "\033[90m"; // gray — no data
        if (v <= ok) return "\033[32m";  // green
        if (v <= warn) return "\033[33m"; // yellow
        return "\033[31m"; // red
    };
    const std::string RST = "\033[0m";

    std::cout << "║  DPI Risk:    " << status_color(dpi, 0.3, 0.6)
              << (dpi >= 0 ? std::to_string((int)(dpi*100)) + "%" : "N/A") << RST << "\n";
    std::cout << "║  Uptime:      " << (uptime >= 0 ? std::to_string((int)uptime) + "s" : "N/A") << "\n";
    std::cout << "║  P2P Peers:   " << (peers >= 0 ? std::to_string((int)peers) : "N/A") << "\n";
    std::cout << "║  NAIN Status: " << (nain == 0 ? "Normal" : nain == 1 ? "Partial" : "SHUTDOWN") << "\n";
    std::cout << "║  PQ-KEX:      " << (pqkex >= 0 ? std::to_string((int)pqkex) : "N/A") << "\n";
    std::cout << "║  Core Switch: " << (switches >= 0 ? std::to_string((int)switches) : "N/A") << "\n";
    std::cout << "╚══════════════════════════════════════════════════════╝\n";
}

int main(int argc, char* argv[]) {
    curl_global_init(CURL_GLOBAL_ALL);
    bool watch = argc > 1 && std::string(argv[1]) == "--watch";
    do {
        auto m = fetch_metrics();
        if (m.empty()) { std::cerr << "Daemon unreachable. Is shield-daemon running?\n"; }
        else display_status(m);
        if (watch) { std::this_thread::sleep_for(std::chrono::seconds(3)); }
    } while (watch);
    curl_global_cleanup();
    return 0;
}
