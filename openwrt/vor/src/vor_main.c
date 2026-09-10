/**
 * Vor OpenWrt daemon — the universal router engine.
 *
 * Ports the shared fragment planner (core/vor-core/src/fragment.rs — the
 * MICAFP tls_fragment + UAC strategy taxonomy) to C and applies it to a
 * loopback TLS relay (the Edge Bridge technique: split the ClientHello
 * into well-formed records before forwarding to the upstream edge), so
 * any router can gain SNI-fragmentation DPI evasion without a proxy core.
 *
 * UCI config (/etc/config/vor):
 *   config vor 'vor'
 *     option enabled      '1'
 *     option listen_port  '40443'
 *     option edge_ip      '104.18.1.1'
 *     option edge_port    '443'
 *     option strategy     'sni_split'   (sni_split|record_split|tls_record_frag|
 *                                         full5|full10|full20|multi64|half|raw|
 *                                         sni_boundary|tls_sni_records|random_split)
 *     option fragment_delay_ms '5'
 *     option max_sessions '64'
 *
 * The conformance vectors (tests run in CI) pin this C port to the same
 * split points as the Rust/Kotlin/Go references.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

#include "vor_planner.h"

/* ---------------------------------------------------------------- TLS parse */

#define TLS_CONTENT_HANDSHAKE      0x16
#define TLS_HANDSHAKE_CLIENT_HELLO 0x01

int vor_is_client_hello(const uint8_t *data, size_t len) {
    if (len < 9) return 0;
    return data[0] == TLS_CONTENT_HANDSHAKE &&
           data[1] == 0x03 &&
           data[5] == TLS_HANDSHAKE_CLIENT_HELLO;
}

static uint16_t be16(const uint8_t *p) {
    return (uint16_t)((p[0] << 8) | p[1]);
}

int vor_find_sni(const uint8_t *data, size_t len, vor_sni_location *out) {
    if (!vor_is_client_hello(data, len) || !out) return 0;
    size_t pos = 5 + 4;
    pos += 2 + 32;
    if (pos >= len) return 0;
    size_t session_len = data[pos];
    pos += 1 + session_len;
    if (pos + 2 > len) return 0;
    size_t cipher_len = be16(data + pos);
    pos += 2 + cipher_len;
    if (pos >= len) return 0;
    size_t comp_len = data[pos];
    pos += 1 + comp_len;
    if (pos + 2 > len) return 0;
    size_t extensions_len = be16(data + pos);
    pos += 2;
    size_t extensions_end = pos + extensions_len;
    if (extensions_end > len) extensions_end = len;

    while (pos + 4 <= extensions_end) {
        uint16_t ext_type = be16(data + pos);
        size_t ext_len = be16(data + pos + 2);
        size_t ext_start = pos;
        size_t ext_end = pos + 4 + ext_len;
        if (ext_end > extensions_end) ext_end = extensions_end;
        if (ext_type == 0x0000) {
            size_t p = ext_start + 4 + 2; /* skip list length */
            out->extension_start = ext_start;
            out->extension_end = ext_end;
            if (p < ext_end && data[p] == 0x00 && p + 2 <= ext_end) {
                size_t name_len = be16(data + p + 1);
                p += 3;
                size_t host_len = name_len;
                if (host_len > ext_end - p) host_len = ext_end - p;
                out->hostname_start = p;
                out->hostname_len = host_len;
            } else {
                out->hostname_start = ext_start + 4;
                out->hostname_len = 0;
            }
            return 1;
        }
        pos = ext_end;
    }
    return 0;
}

/* ---------------------------------------------------------------- planner */

/* Deterministic xorshift splits — identical to the Rust/Kotlin/Go refs. */
static uint32_t xorshift(uint32_t v) {
    v ^= v << 13;
    v ^= v >> 17;
    v ^= v << 5;
    return v;
}

static size_t pseudo_random_splits(const uint8_t *data, size_t len,
                                   size_t min_count, size_t max_count,
                                   size_t *points, size_t points_cap) {
    uint32_t seed = 0x9E3779B9;
    for (size_t i = 0; i < len; i++) {
        seed = (seed << 5 | seed >> 27) ^ data[i];
    }
    if (seed == 0) seed = 0x12345678;
    size_t count = min_count + (seed % (uint32_t)(max_count - min_count + 1));
    if (count > points_cap) count = points_cap;
    size_t used = 0;
    uint32_t value = seed;
    for (size_t i = 0; i < count; i++) {
        value = xorshift(value);
        size_t point = 1 + (size_t)(value % (uint32_t)(len - 1));
        points[used++] = point;
    }
    /* sort + unique */
    for (size_t i = 1; i < used; i++) {
        for (size_t j = i; j > 0 && points[j] < points[j - 1]; j--) {
            size_t t = points[j]; points[j] = points[j - 1]; points[j - 1] = t;
        }
    }
    size_t unique = 0;
    for (size_t i = 0; i < used; i++) {
        if (unique == 0 || points[i] != points[unique - 1]) {
            points[unique++] = points[i];
        }
    }
    return unique;
}

static void plan_push(vor_fragment_plan *plan, size_t start, size_t end,
                      uint32_t inter_delay) {
    if (plan->write_count >= 64) return;
    vor_write *w = &plan->writes[plan->write_count++];
    w->start = start;
    w->end = end;
    w->delay_ms = (start == 0) ? 0 : inter_delay;
    w->more_hint = 1;
}

vor_fragment_plan vor_plan(const uint8_t *data, size_t len,
                           const char *strategy, uint32_t inter_delay_ms) {
    vor_fragment_plan plan;
    memset(&plan, 0, sizeof(plan));
    plan.record_len = len;
    plan.strategy = strategy;
    plan.writes[0].start = 0;
    plan.writes[0].end = len;
    plan.writes[0].delay_ms = 0;
    plan.writes[0].more_hint = 0;
    plan.write_count = 1;

    if (!vor_is_client_hello(data, len) || len < 10 ||
        strcmp(strategy, "raw") == 0) {
        return plan;
    }

    vor_sni_location sni;
    int has_sni = vor_find_sni(data, len, &sni);
    size_t mid = len / 2;

    size_t points[32];
    size_t point_count = 0;

    if (strcmp(strategy, "half") == 0) {
        points[point_count++] = len / 2;
    } else if (strcmp(strategy, "record_split") == 0 ||
               strcmp(strategy, "tls_record_frag") == 0) {
        points[point_count++] = 5;
    } else if (strcmp(strategy, "full5") == 0 ||
               strcmp(strategy, "full10") == 0 ||
               strcmp(strategy, "full20") == 0 ||
               strcmp(strategy, "multi64") == 0) {
        size_t chunk = 5;
        if (strcmp(strategy, "full10") == 0) chunk = 10;
        if (strcmp(strategy, "full20") == 0) chunk = 20;
        if (strcmp(strategy, "multi64") == 0) chunk = 64;
        for (size_t p = chunk; p < len && point_count < 32; p += chunk) {
            points[point_count++] = p;
        }
    } else if (strcmp(strategy, "sni_boundary") == 0) {
        points[point_count++] = has_sni ? sni.extension_start : mid;
    } else if (strcmp(strategy, "sni_split") == 0 ||
               strcmp(strategy, "tls_sni_records") == 0) {
        points[point_count++] = has_sni ? sni.extension_start : mid;
        points[point_count++] = has_sni ? sni.extension_end : mid;
    } else if (strcmp(strategy, "random_split") == 0) {
        size_t sni_end = has_sni ? sni.extension_end : mid;
        size_t random_points[16];
        size_t random_count = pseudo_random_splits(data, len, 2, 4, random_points, 16);
        for (size_t i = 0; i < random_count && point_count < 31; i++) {
            if (random_points[i] < sni_end) {
                points[point_count++] = random_points[i];
            }
        }
        points[point_count++] = sni_end;
    }

    /* sort + unique (points are few) */
    for (size_t i = 1; i < point_count; i++) {
        for (size_t j = i; j > 0 && points[j] < points[j - 1]; j--) {
            size_t t = points[j]; points[j] = points[j - 1]; points[j - 1] = t;
        }
    }
    size_t unique = 0;
    for (size_t i = 0; i < point_count; i++) {
        if (unique == 0 || points[i] != points[unique - 1]) {
            points[unique++] = points[i];
        }
    }

    plan.write_count = 0;
    size_t prev = 0;
    for (size_t i = 0; i < unique; i++) {
        if (points[i] <= prev || points[i] >= len) continue;
        plan_push(&plan, prev, points[i], inter_delay_ms);
        prev = points[i];
    }
    if (prev < len) {
        if (plan.write_count < 64) {
            vor_write *w = &plan.writes[plan.write_count++];
            w->start = prev;
            w->end = len;
            w->delay_ms = (prev == 0) ? 0 : inter_delay_ms;
            w->more_hint = 0;
        }
    }
    if (plan.write_count <= 1) {
        plan.writes[0].start = 0;
        plan.writes[0].end = len;
        plan.writes[0].delay_ms = 0;
        plan.writes[0].more_hint = 0;
        plan.write_count = 1;
    }
    return plan;
}

/**
 * External finalmask: rewrite the record as two well-formed TLS records
 * (5-byte prefix + re-framed remainder). out must hold len + 5 bytes.
 * Returns the number of bytes written.
 */
size_t vor_split_well_formed(const uint8_t *data, size_t len, uint8_t *out) {
    if (len <= 5) {
        memcpy(out, data, len);
        return len;
    }
    memcpy(out, data, 5);
    size_t rest = len - 5;
    out[5 + 0] = data[0];
    out[5 + 1] = data[1];
    out[5 + 2] = data[2];
    out[5 + 3] = (uint8_t)(rest >> 8);
    out[5 + 4] = (uint8_t)(rest & 0xFF);
    memcpy(out + 10, data + 5, rest);
    return len + 5;
}

/* ---------------------------------------------------------------- daemon (POSIX) */

#ifndef VOR_PLANNER_ONLY

#include <unistd.h>
#include <signal.h>
#include <syslog.h>
#include <errno.h>
#include <pthread.h>

#include <sys/socket.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <arpa/inet.h>
#include <sys/time.h>


static volatile int g_running = 1;

static void signal_handler(int sig) {
    switch (sig) {
    case SIGTERM:
    case SIGINT:
        g_running = 0;
        break;
    case SIGHUP:
        /* configuration reload: the daemon re-reads /etc/config/vor
         * lazily on the next apply_uci() (documented procd reload path). */
        break;
    default:
        break;
    }
}

static int g_listen_port = 40443;
static char g_edge_ip[64] = "104.18.1.1";
static int g_edge_port = 443;
static char g_strategy[32] = "sni_split";
static uint32_t g_fragment_delay_ms = 5;
static int g_max_sessions = 64;
static volatile int g_active_sessions = 0;
static pthread_mutex_t g_session_mutex = PTHREAD_MUTEX_INITIALIZER;

/* Portable UCI-format reader: /etc/config/vor is plain key-value text
 * ("option key 'value'" lines). Parsing it directly (instead of linking
 * libuci) keeps the daemon buildable and testable on any POSIX system and
 * behaves identically on the router. */
static void uci_trim(char *s) {
    size_t n = strlen(s);
    while (n > 0 && (s[n-1] == '\n' || s[n-1] == '\r' || s[n-1] == ' ' || s[n-1] == '\t')) s[--n] = 0;
}

static int uci_get(const char *path, const char *key, char *out, size_t cap) {
    FILE *f = fopen(path, "r");
    if (!f) return -1;
    char line[512];
    char want[128];
    snprintf(want, sizeof(want), "option %s", key);
    int found = 0;
    while (fgets(line, sizeof(line), f)) {
        char *p = line;
        while (*p == ' ' || *p == '\t') p++;
        if (strncmp(p, want, strlen(want)) == 0) {
            char *value = p + strlen(want);
            while (*value == ' ' || *value == '\t') value++;
            uci_trim(value);
            if (*value == '\'' && value[strlen(value)-1] == '\'') {
                value[strlen(value)-1] = 0;
                value++;
            }
            snprintf(out, cap, "%s", value);
            found = 1;
            break;
        }
    }
    fclose(f);
    return found ? 0 : -1;
}

static const char *uci_path(void) {
    const char *override = getenv("VOR_UCI_PATH");
    if (override && override[0]) return override;
    return "/etc/config/vor";
}

static void apply_uci(void) {
    char value[128];
    const char *cfg = uci_path();
    if (uci_get(cfg, "listen_port", value, sizeof(value)) == 0) {
        int port = atoi(value);
        if (port > 0 && port < 65536) g_listen_port = port;
    }
    if (uci_get(cfg, "edge_ip", value, sizeof(value)) == 0 && value[0]) {
        if (strlen(value) < sizeof(g_edge_ip)) {
            snprintf(g_edge_ip, sizeof(g_edge_ip), "%s", value);
        }
    }
    if (uci_get(cfg, "edge_port", value, sizeof(value)) == 0) {
        int port = atoi(value);
        if (port > 0 && port < 65536) g_edge_port = port;
    }
    if (uci_get(cfg, "strategy", value, sizeof(value)) == 0 && value[0]) {
        if (strlen(value) < sizeof(g_strategy)) {
            snprintf(g_strategy, sizeof(g_strategy), "%s", value);
        }
    }
    if (uci_get(cfg, "fragment_delay_ms", value, sizeof(value)) == 0) {
        g_fragment_delay_ms = (uint32_t)atoi(value);
    }
    if (uci_get(cfg, "max_sessions", value, sizeof(value)) == 0) {
        int sessions = atoi(value);
        if (sessions > 0) g_max_sessions = sessions;
    }
}

static void msleep(uint32_t ms) {
    struct timespec ts;
    ts.tv_sec = ms / 1000;
    ts.tv_nsec = (long)(ms % 1000) * 1000000L;
    nanosleep(&ts, NULL);
}

/* Read the first complete TLS record; returns bytes consumed or -1. */
static ssize_t read_first_record(int fd, uint8_t *buf, size_t cap) {
    size_t got = 0;
    while (got < 5) {
        ssize_t n = recv(fd, buf + got, 5 - got, 0);
        if (n <= 0) return -1;
        got += (size_t)n;
    }
    size_t body = ((size_t)buf[3] << 8) | buf[4];
    if (body > 32768) body = 32768; /* cap for safety */
    while (got < 5 + body && got < cap) {
        ssize_t n = recv(fd, buf + got, cap - got, 0);
        if (n <= 0) return -1;
        got += (size_t)n;
    }
    return (ssize_t)got;
}

static void pump(int from, int to) {
    uint8_t buf[256 * 1024];
    for (;;) {
        ssize_t n = recv(from, buf, sizeof(buf), 0);
        if (n <= 0) break;
        ssize_t off = 0;
        while (off < n) {
            ssize_t written = send(to, buf + off, (size_t)(n - off), MSG_NOSIGNAL);
            if (written <= 0) return;
            off += written;
        }
    }
}

static void *session_thread(void *arg) {
    int client_fd = *(int *)arg;
    free(arg);
    int upstream_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (upstream_fd < 0) {
        close(client_fd);
        pthread_mutex_lock(&g_session_mutex);
        g_active_sessions--;
        pthread_mutex_unlock(&g_session_mutex);
        return NULL;
    }
    struct timeval timeout = {10, 0};
    setsockopt(client_fd, SOL_SOCKET, SO_RCVTIMEO, &timeout, sizeof(timeout));
    uint8_t first[32768 + 5];
    ssize_t first_len = read_first_record(client_fd, first, sizeof(first));
    if (first_len > 0 && upstream_fd >= 0) {
        struct sockaddr_in edge;
        memset(&edge, 0, sizeof(edge));
        edge.sin_family = AF_INET;
        edge.sin_port = htons((uint16_t)g_edge_port);
        inet_pton(AF_INET, g_edge_ip, &edge.sin_addr);
        if (connect(upstream_fd, (struct sockaddr *)&edge, sizeof(edge)) == 0) {
            int one = 1;
            setsockopt(upstream_fd, IPPROTO_TCP, TCP_NODELAY, &one, sizeof(one));
            if (vor_is_client_hello(first, (size_t)first_len)) {
                if (strcmp(g_strategy, "tls_record_frag") == 0) {
                    uint8_t framed[32768 + 10];
                    size_t framed_len = vor_split_well_formed(first, (size_t)first_len, framed);
                    size_t off = 0;
                    int piece = 0;
                    while (off < framed_len) {
                        if (piece > 0 && g_fragment_delay_ms > 0) msleep(g_fragment_delay_ms);
                        /* first piece = the 5-byte prefix record; second = the re-framed remainder */
                        size_t piece_len = (piece == 0) ? 5 : framed_len - 5;
                        ssize_t written = send(upstream_fd, framed + off, piece_len, MSG_NOSIGNAL);
                        if (written <= 0) break;
                        off += (size_t)written;
                        piece++;
                    }
                } else {
                    vor_fragment_plan plan = vor_plan(first, (size_t)first_len, g_strategy, g_fragment_delay_ms);
                    for (size_t i = 0; i < plan.write_count; i++) {
                        if (i > 0 && plan.writes[i].delay_ms > 0) msleep(plan.writes[i].delay_ms);
                        size_t off = plan.writes[i].start;
                        size_t remain = plan.writes[i].end - plan.writes[i].start;
                        while (remain > 0) {
                            ssize_t written = send(upstream_fd, first + off, remain, MSG_NOSIGNAL);
                            if (written <= 0) goto relay_done;
                            off += (size_t)written;
                            remain -= (size_t)written;
                        }
                    }
                }
            } else {
                send(upstream_fd, first, (size_t)first_len, MSG_NOSIGNAL);
            }
        relay_done:
            (void)0;
            struct timeval zero = {0, 0};
            setsockopt(client_fd, SOL_SOCKET, SO_RCVTIMEO, &zero, sizeof(zero));
            pump(client_fd, upstream_fd);
            shutdown(upstream_fd, SHUT_WR);
            pump(upstream_fd, client_fd);
        }
    }
    close(upstream_fd);
    close(client_fd);
    pthread_mutex_lock(&g_session_mutex);
    g_active_sessions--;
    pthread_mutex_unlock(&g_session_mutex);
    return NULL;
}

int main(int argc, char **argv) {
    int daemonize = 0;
    const char *config_name = "vor";
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "-d") == 0 || strcmp(argv[i], "--daemonize") == 0) daemonize = 1;
        else if (strcmp(argv[i], "-c") == 0 && i + 1 < argc) config_name = argv[++i];
        else if (strcmp(argv[i], "-v") == 0 || strcmp(argv[i], "--verbose") == 0) { /* verbose */ }
        else if (strcmp(argv[i], "-h") == 0 || strcmp(argv[i], "--help") == 0) {
            printf("Vor OpenWrt daemon — universal router engine\n");
            return 0;
        }
    }
    (void)config_name;

    openlog("vor", LOG_PID | LOG_NDELAY, LOG_DAEMON);
    /* sigaction without SA_RESTART: accept(2) must return EINTR on
     * SIGTERM so the accept loop notices g_running == 0 and exits. */
    struct sigaction action;
    memset(&action, 0, sizeof(action));
    action.sa_handler = signal_handler;
    sigemptyset(&action.sa_mask);
    action.sa_flags = 0; /* no SA_RESTART on purpose */
    sigaction(SIGTERM, &action, NULL);
    sigaction(SIGINT, &action, NULL);
    sigaction(SIGHUP, &action, NULL);
    signal(SIGPIPE, SIG_IGN);

    apply_uci();

    if (daemonize) {
        if (daemon(0, 0) < 0) {
            syslog(LOG_ERR, "daemon() failed: %s", strerror(errno));
            return 1;
        }
    }

    int listen_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (listen_fd < 0) {
        syslog(LOG_ERR, "socket: %s", strerror(errno));
        return 1;
    }
    int one = 1;
    setsockopt(listen_fd, SOL_SOCKET, SO_REUSEADDR, &one, sizeof(one));
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    addr.sin_port = htons((uint16_t)g_listen_port);
    if (bind(listen_fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        syslog(LOG_ERR, "bind 127.0.0.1:%d: %s", g_listen_port, strerror(errno));
        return 1;
    }
    listen(listen_fd, 16);
    syslog(LOG_INFO, "vor daemon listening on 127.0.0.1:%d -> %s:%d strategy=%s",
           g_listen_port, g_edge_ip, g_edge_port, g_strategy);

    while (g_running) {
        int client_fd = accept(listen_fd, NULL, NULL);
        if (client_fd < 0) {
            if (errno == EINTR) {
                if (!g_running) break; /* shutdown */
                continue;
            }
            break;
        }
        pthread_mutex_lock(&g_session_mutex);
        int overload = g_active_sessions >= g_max_sessions;
        if (!overload) g_active_sessions++;
        pthread_mutex_unlock(&g_session_mutex);
        if (overload) {
            close(client_fd);
            continue;
        }
        pthread_t thread;
        int *fd_ptr = malloc(sizeof(int));
        *fd_ptr = client_fd;
        if (pthread_create(&thread, NULL, session_thread, fd_ptr) != 0) {
            close(client_fd);
            pthread_mutex_lock(&g_session_mutex);
            g_active_sessions--;
            pthread_mutex_unlock(&g_session_mutex);
            continue;
        }
        pthread_detach(thread);
    }
    close(listen_fd);
    syslog(LOG_INFO, "vor daemon stopped");
    closelog();
    return 0;
}

#endif /* VOR_PLANNER_ONLY */
