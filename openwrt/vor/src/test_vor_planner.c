/**
 * Vor planner conformance test — runs the SHARED cross-platform fragment
 * vectors (core/vor-core/tests/vectors/decision-vectors.json, cases under
 * "fragment_cases") against the C planner, pinning it to the Rust/Kotlin/
 * Go references.
 *
 * The vectors are embedded as a compact table (compiled from the JSON) so
 * the test runs on any POSIX system with nothing but libc: build with
 *   gcc -DVOR_PLANNER_ONLY -o test_vor_planner vor_main.c test_vor_planner.c
 */

#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

#include "vor_planner.h"

/* Deterministic ClientHello with SNI "example.com" + trailing
 * supported_versions extension — the same fixture as every port. */
static size_t build_client_hello(uint8_t *record, size_t cap) {
    const char *name = "example.com";
    size_t name_len = strlen(name);

    uint8_t ext[64];
    size_t ext_len = 0;
    uint16_t list_len = (uint16_t)(1 + 2 + name_len);
    ext[ext_len++] = (uint8_t)(list_len >> 8);
    ext[ext_len++] = (uint8_t)(list_len & 0xFF);
    ext[ext_len++] = 0x00;
    ext[ext_len++] = (uint8_t)(name_len >> 8);
    ext[ext_len++] = (uint8_t)(name_len & 0xFF);
    memcpy(ext + ext_len, name, name_len);
    ext_len += name_len;

    uint8_t trailing[6] = {0x00, 0x2B, 0x00, 0x02, 0x03, 0x04};
    size_t ext_total = ext_len + 4 + sizeof(trailing);

    uint8_t handshake[256];
    size_t h = 0;
    handshake[h++] = 0x03; handshake[h++] = 0x03; /* client version */
    memset(handshake + h, 0, 32); h += 32;        /* random */
    handshake[h++] = 0;                            /* session id len */
    handshake[h++] = 0; handshake[h++] = 2;        /* cipher len */
    handshake[h++] = 0x13; handshake[h++] = 0x01;
    handshake[h++] = 1;                            /* comp len */
    handshake[h++] = 0x00;
    handshake[h++] = (uint8_t)(ext_total >> 8);
    handshake[h++] = (uint8_t)(ext_total & 0xFF);
    handshake[h++] = 0x00; handshake[h++] = 0x00;  /* server_name ext type */
    handshake[h++] = (uint8_t)(ext_len >> 8);
    handshake[h++] = (uint8_t)(ext_len & 0xFF);
    memcpy(handshake + h, ext, ext_len); h += ext_len;
    memcpy(handshake + h, trailing, sizeof(trailing)); h += sizeof(trailing);

    size_t out = 0;
    record[out++] = 0x16;
    record[out++] = 0x03; record[out++] = 0x01;
    uint16_t rec_len = (uint16_t)(h + 4);
    record[out++] = (uint8_t)(rec_len >> 8);
    record[out++] = (uint8_t)(rec_len & 0xFF);
    record[out++] = 0x01; /* ClientHello */
    record[out++] = (uint8_t)((uint32_t)h >> 16);
    record[out++] = (uint8_t)((uint32_t)h >> 8);
    record[out++] = (uint8_t)((uint32_t)h & 0xFF);
    memcpy(record + out, handshake, h);
    out += h;
    (void)cap;
    return out;
}

static int failures = 0;

static void check(const char *name, const char *what, int condition) {
    if (!condition) {
        printf("FAIL  %s: %s\n", name, what);
        failures++;
    }
}

int main(void) {
    uint8_t record[512];
    size_t len = build_client_hello(record, sizeof(record));

    /* --- finds SNI --- */
    check("sni", "detected as ClientHello", vor_is_client_hello(record, len));
    vor_sni_location sni;
    check("sni", "SNI located", vor_find_sni(record, len, &sni));
    check("sni", "hostname matches",
          sni.hostname_len == strlen("example.com") &&
          memcmp(record + sni.hostname_start, "example.com", sni.hostname_len) == 0);
    check("sni", "extension inside record",
          sni.extension_start > 5 && sni.extension_end <= len);

    /* --- shared vector cases (decision-vectors.json fragment_cases) --- */
    struct {
        const char *name;
        const char *strategy;
        uint32_t delay_ms;
        int expected_writes;      /* 0 = don't check exact count */
        int expected_min_writes;  /* 0 = don't check min count */
        int expected_first_end;   /* -1 = don't check */
        int expected_first_delay; /* -1 = don't check */
        int expected_second_delay;
        int expected_chunk;       /* 0 = don't check */
    } cases[] = {
        {"record_split_point", "record_split", 10, 0, 2, 5, 0, 10, 0},
        {"half_point", "half", 0, 2, 0, -1, -1, -1, 0},
        {"raw_single_write", "raw", 5, 1, 0, -1, -1, -1, 0},
        {"full5_chunk_sizes", "full5", 0, 0, 8, -1, -1, -1, 5},
        {"sni_split_brackets_sni", "sni_split", 0, 0, 3, -1, -1, -1, 0},
    };

    for (size_t i = 0; i < sizeof(cases) / sizeof(cases[0]); i++) {
        const char *name = cases[i].name;
        vor_fragment_plan plan = vor_plan(record, len, cases[i].strategy, cases[i].delay_ms);

        if (cases[i].expected_writes > 0) {
            check(name, "write count", (int)plan.write_count == cases[i].expected_writes);
        }
        if (cases[i].expected_min_writes > 0) {
            check(name, "min write count", (int)plan.write_count >= cases[i].expected_min_writes);
        }
        if (cases[i].expected_first_end >= 0) {
            check(name, "first write end", (int)plan.writes[0].end == cases[i].expected_first_end);
        }
        if (cases[i].expected_first_delay >= 0) {
            check(name, "first write delay", (int)plan.writes[0].delay_ms == cases[i].expected_first_delay);
        }
        if (cases[i].expected_second_delay >= 0 && plan.write_count > 1) {
            check(name, "second write delay", (int)plan.writes[1].delay_ms == cases[i].expected_second_delay);
        }
        if (cases[i].expected_chunk > 0) {
            for (size_t w = 0; w + 1 < plan.write_count; w++) {
                char what[64];
                snprintf(what, sizeof(what), "chunk size at %zu", w);
                check(name, what,
                      (int)(plan.writes[w].end - plan.writes[w].start) == cases[i].expected_chunk);
            }
        }
        /* coverage invariant for every strategy */
        size_t cursor = 0;
        for (size_t w = 0; w < plan.write_count; w++) {
            if (plan.writes[w].start != cursor) {
                check(name, "contiguity", 0);
                break;
            }
            cursor = plan.writes[w].end;
        }
        check(name, "coverage", cursor == len);

        /* sni_split must bracket the SNI extension exactly */
        if (strcmp(cases[i].strategy, "sni_split") == 0) {
            check(name, "first write ends at SNI start",
                  plan.writes[0].end == sni.extension_start);
            int ends_at_sni_end = 0;
            for (size_t w = 0; w < plan.write_count; w++) {
                if (plan.writes[w].end == sni.extension_end) ends_at_sni_end = 1;
            }
            check(name, "some write ends at SNI end", ends_at_sni_end);
        }
    }

    /* --- well-formed record rewrite --- */
    uint8_t framed[512 + 5];
    size_t framed_len = vor_split_well_formed(record, len, framed);
    check("wellformed", "framed length", framed_len == len + 5);
    check("wellformed", "second record header type", framed[5] == record[0]);
    uint16_t declared = (uint16_t)((framed[8] << 8) | framed[9]);
    check("wellformed", "second record declared length", declared == (uint16_t)(len - 5));
    check("wellformed", "payload preserved",
          memcmp(framed + 10, record + 5, len - 5) == 0);

    /* --- passthrough for non-ClientHello --- */
    uint8_t junk[64];
    memset(junk, 0x17, sizeof(junk));
    vor_fragment_plan passthrough = vor_plan(junk, sizeof(junk), "sni_split", 5);
    check("passthrough", "single write", passthrough.write_count == 1);
    check("passthrough", "full coverage", passthrough.writes[0].end == sizeof(junk));

    /* --- random_split deterministic --- */
    vor_fragment_plan a = vor_plan(record, len, "random_split", 0);
    vor_fragment_plan b = vor_plan(record, len, "random_split", 0);
    check("random", "same count", a.write_count == b.write_count);
    int same = 1;
    for (size_t w = 0; w < a.write_count; w++) {
        if (a.writes[w].start != b.writes[w].start || a.writes[w].end != b.writes[w].end) {
            same = 0;
            break;
        }
    }
    check("random", "same split points", same);

    if (failures == 0) {
        printf("All Vor C planner conformance checks passed.\n");
        return 0;
    }
    printf("%d failures\n", failures);
    return 1;
}
