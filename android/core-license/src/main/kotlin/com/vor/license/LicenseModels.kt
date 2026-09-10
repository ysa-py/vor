package com.vor.license

/**
 * Vor offline license — data model (see `license/SPEC.md`).
 *
 * Token format: `VOR1.<base64url(payload_json)>.<base64url(signature_64)>`
 * where the signature is Ed25519 over the exact payload bytes. Verification
 * is fully offline and identical on every platform; the Kotlin port is
 * pinned to the shared conformance vectors in `license/vectors.json`.
 */

/** Verification outcome. */
enum class LicenseStatus {
    VALID,
    EXPIRED,
    INVALID;

    companion object {
        fun fromWire(value: String): LicenseStatus = when (value) {
            "VALID" -> VALID
            "EXPIRED" -> EXPIRED
            else -> INVALID
        }
    }
}

/** Decoded license payload. */
data class LicensePayload(
    val version: Int,
    val id: String,
    val product: String,
    val issuedAt: String,
    val expiresAt: String,
    val tier: String?,
    val platforms: List<String>,
) {
    /** Expiry parsed to epoch seconds (null when unparseable). */
    fun expiresAtEpoch(): Long? = Rfc3339.parseEpoch(expiresAt)
}

/** Verification result: status + payload when decodable. */
data class LicenseResult(
    val status: LicenseStatus,
    val payload: LicensePayload?,
)

/**
 * Minimal RFC 3339 parser (UTC + offsets) shared by the license ports.
 * Mirrors the Rust reference implementation exactly.
 */
object Rfc3339 {
    /** Parse "2027-01-01T00:00:00Z" (fractional seconds and +hh:mm offsets
     * allowed) to unix epoch seconds; null when unparseable. */
    fun parseEpoch(text: String): Long? {
        val trimmed = text.trim()
        val tIndex = trimmed.indexOf('T')
        if (tIndex <= 0) return null
        val datePart = trimmed.substring(0, tIndex)
        var body = trimmed.substring(tIndex + 1)
        var offsetSeconds = 0L
        if (body.endsWith("Z")) {
            body = body.removeSuffix("Z")
        } else if (body.length > 6) {
            val sign = when (body[body.length - 6]) {
                '+' -> 1L
                '-' -> -1L
                else -> 0L
            }
            if (sign != 0L) {
                val hours = body.substring(body.length - 5, body.length - 3).toLongOrNull() ?: return null
                val minutes = body.substring(body.length - 2).toLongOrNull() ?: return null
                offsetSeconds = sign * (hours * 3600 + minutes * 60)
                body = body.substring(0, body.length - 6)
            }
        }
        val naive = dateTimeToEpoch(datePart, body) ?: return null
        return naive - offsetSeconds
    }

    private fun dateTimeToEpoch(datePart: String, timePart: String): Long? {
        val (year, month, day) = parseDate(datePart) ?: return null
        val (hour, minute, second) = parseTime(timePart) ?: return null
        val days = daysFromCivil(year, month, day)
        return days * 86400L + hour * 3600L + minute * 60L + second
    }

    private fun parseDate(text: String): Triple<Long, Long, Long>? {
        val parts = text.split("-")
        if (parts.size != 3) return null
        val year = parts[0].toLongOrNull() ?: return null
        val month = parts[1].toLongOrNull() ?: return null
        val day = parts[2].toLongOrNull() ?: return null
        if (month !in 1..12 || day !in 1..31 || year !in 1970..9999) return null
        return Triple(year, month, day)
    }

    private fun parseTime(text: String): Triple<Long, Long, Long>? {
        val cleaned = text.substringBefore('.')
        val parts = cleaned.split(":")
        if (parts.size < 2) return null
        val hour = parts[0].toLongOrNull() ?: return null
        val minute = parts[1].toLongOrNull() ?: return null
        val second = parts.getOrElse(2) { "0" }.toLongOrNull() ?: return null
        if (hour !in 0..23 || minute !in 0..59 || second !in 0..60) return null
        return Triple(hour, minute, second)
    }

    /** Days since unix epoch from a civil date (Howard Hinnant's algorithm). */
    private fun daysFromCivil(year: Long, month: Long, day: Long): Long {
        val adjustedYear = if (month <= 2) year - 1 else year
        val era = (if (adjustedYear >= 0) adjustedYear else adjustedYear - 399) / 400
        val yoe = adjustedYear - era * 400
        val mp = (month + 9) % 12
        val doy = (153 * mp + 2) / 5 + day - 1
        val doe = yoe * 365 + yoe / 4 - yoe / 100 + doy
        return era * 146097 + doe - 719468
    }
}

/** base64url (RFC 4648 §5, unpadded) codec. */
object Base64Url {
    private const val ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"

    fun encode(data: ByteArray): String {
        val out = StringBuilder((data.size + 2) / 3 * 4)
        var index = 0
        while (index < data.size) {
            val b0 = data[index].toInt() and 0xFF
            val b1 = if (index + 1 < data.size) data[index + 1].toInt() and 0xFF else 0
            val b2 = if (index + 2 < data.size) data[index + 2].toInt() and 0xFF else 0
            val triple = (b0 shl 16) or (b1 shl 8) or b2
            out.append(ALPHABET[(triple shr 18) and 63])
            out.append(ALPHABET[(triple shr 12) and 63])
            if (index + 1 < data.size) out.append(ALPHABET[(triple shr 6) and 63])
            if (index + 2 < data.size) out.append(ALPHABET[triple and 63])
            index += 3
        }
        return out.toString()
    }

    fun decode(text: String): ByteArray? {
        val cleaned = text.trimEnd('=')
        val out = ByteArray(cleaned.length * 3 / 4)
        var buffer = 0
        var bits = 0
        var offset = 0
        for (character in cleaned) {
            val value = when (character) {
                in 'A'..'Z' -> character - 'A'
                in 'a'..'z' -> character - 'a' + 26
                in '0'..'9' -> character - '0' + 52
                '-' -> 62
                '_' -> 63
                else -> return null
            }
            buffer = (buffer shl 6) or value
            bits += 6
            if (bits >= 8) {
                bits -= 8
                out[offset++] = ((buffer shr bits) and 0xFF).toByte()
            }
        }
        return out.copyOf(offset)
    }
}
