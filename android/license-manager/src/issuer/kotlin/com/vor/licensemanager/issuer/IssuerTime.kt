package com.vor.licensemanager.issuer

import java.time.Instant
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter

/**
 * Time helpers shared by the issuer UI (and unit-tested on the JVM).
 * Everything a token carries is UTC, seconds precision — the exact shape
 * the reference Python issuer emits (`%Y-%m-%dT%H:%M:%SZ`).
 */
object IssuerTime {

    private val format = DateTimeFormatter.ofPattern("yyyy-MM-dd'T'HH:mm:ss'Z'").withZone(ZoneOffset.UTC)

    /** Format an instant exactly like the reference tool: 2027-01-01T00:00:00Z */
    fun formatRfc3339Utc(instant: Instant): String = format.format(instant)

    fun nowRfc3339Utc(): String = formatRfc3339Utc(Instant.now())

    /** Seconds-epoch -> RFC 3339 (what the expiry picker produces). */
    fun epochSecondToRfc3339(epochSeconds: Long): String = formatRfc3339Utc(Instant.ofEpochSecond(epochSeconds))

    /** Millis-epoch variant. */
    fun epochMilliToRfc3339(epochMillis: Long): String =
        formatRfc3339Utc(Instant.ofEpochSecond(epochMillis / 1000))

    /** Valid-RFC3339 check using the SAME parser every Vor client uses. */
    fun isValidRfc3339(text: String): Boolean = com.vor.license.Rfc3339.parseEpoch(text) != null
}
