/**
 * Vor shared fragment planner — C port of core/vor-core/src/fragment.rs.
 * Pinned to the shared cross-platform conformance vectors.
 */
#ifndef VOR_PLANNER_H
#define VOR_PLANNER_H

#include <stddef.h>
#include <stdint.h>

typedef struct {
    size_t extension_start;
    size_t extension_end;
    size_t hostname_start;
    size_t hostname_len;
} vor_sni_location;

typedef struct {
    size_t start;
    size_t end;
    uint32_t delay_ms;
    int more_hint;
} vor_write;

typedef struct {
    const char *strategy;
    size_t record_len;
    vor_write writes[64];
    size_t write_count;
} vor_fragment_plan;

int vor_is_client_hello(const uint8_t *data, size_t len);
int vor_find_sni(const uint8_t *data, size_t len, vor_sni_location *out);
vor_fragment_plan vor_plan(const uint8_t *data, size_t len,
                           const char *strategy, uint32_t inter_delay_ms);
size_t vor_split_well_formed(const uint8_t *data, size_t len, uint8_t *out);

#endif /* VOR_PLANNER_H */
