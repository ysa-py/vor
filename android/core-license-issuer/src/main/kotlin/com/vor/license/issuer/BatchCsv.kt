package com.vor.license.issuer

import com.vor.license.Rfc3339

/**
 * Batch issuance from plain CSV/text — one license per line:
 *
 *     id[,tier][,expires]
 *
 * - `#` starts a comment line; blank lines are ignored.
 * - `tier` omitted  -> falls back to [defaultTier].
 * - `expires` omitted -> falls back to [defaultExpiryRfc3339].
 * - `expires` accepts any RFC 3339 timestamp the verifier itself accepts
 *   (see com.vor.license.Rfc3339 — the same parser every Vor client uses).
 *
 * The parser NEVER silently repairs a bad row: every problem is reported in
 * [Result.errors] with its 1-based line number, and the caller decides
 * whether to abort or issue the clean rows only.
 */
object BatchCsv {

    data class Row(
        val id: String,
        val tier: String,
        val expiresAt: String,
    )

    data class Result(
        val rows: List<Row>,
        val errors: List<String>,
    ) {
        val isUsable: Boolean get() = rows.isNotEmpty()
    }

    fun parse(
        text: String,
        defaultTier: String,
        defaultExpiryRfc3339: String,
    ): Result {
        val rows = ArrayList<Row>()
        val errors = ArrayList<String>()
        text.lineSequence().forEachIndexed { index, rawLine ->
            val lineNumber = index + 1
            val line = rawLine.trim()
            if (line.isEmpty() || line.startsWith("#")) return@forEachIndexed

            val fields = line.split(',')
            if (fields.size > 3) {
                errors.add("line $lineNumber: too many fields (${fields.size}; expected id[,tier][,expires])")
                return@forEachIndexed
            }
            val id = fields[0].trim()
            if (id.isEmpty()) {
                errors.add("line $lineNumber: empty license id")
                return@forEachIndexed
            }
            if (id.length > MAX_ID_LENGTH || id.any { it.code in 0..0x1F }) {
                errors.add("line $lineNumber: invalid license id (max $MAX_ID_LENGTH chars, no control characters)")
                return@forEachIndexed
            }
            val tier = if (fields.size >= 2) fields[1].trim() else defaultTier
            if (tier.isEmpty()) {
                errors.add("line $lineNumber: empty tier (use '-' placeholders are not supported; omit the field instead)")
                return@forEachIndexed
            }
            if (tier.length > MAX_TIER_LENGTH || tier.any { it.code in 0..0x1F }) {
                errors.add("line $lineNumber: invalid tier (max $MAX_TIER_LENGTH chars, no control characters)")
                return@forEachIndexed
            }
            val expires = if (fields.size == 3) {
                val candidate = fields[2].trim()
                if (Rfc3339.parseEpoch(candidate) == null) {
                    errors.add("line $lineNumber: '$candidate' is not a valid RFC 3339 expiry")
                    return@forEachIndexed
                }
                candidate
            } else {
                defaultExpiryRfc3339
            }
            rows.add(Row(id, tier, expires))
        }
        return Result(rows, errors)
    }

    /** Render the batch output text: one token per line, in input order. */
    fun renderTokensText(tokens: List<String>): String = tokens.joinToString("\n")

    /** Filesystem-safe name for one license's QR image inside a ZIP. */
    fun qrFileName(id: String): String {
        val sanitized = id.map { c ->
            if (c in 'a'..'z' || c in 'A'..'Z' || c in '0'..'9' || c == '-' || c == '_' || c == '.') c else '_'
        }.joinToString("")
        val base = sanitized.ifEmpty { "license" }.take(64)
        return "qr-$base.png"
    }

    const val MAX_ID_LENGTH = 128
    const val MAX_TIER_LENGTH = 64
}
