package com.v2rayez.app.data.uac

/**
 * TLS ClientHello fragmentation — pure-Kotlin port of the UAC
 * `MciFragmenter` ClientHello surgery (strategy taxonomy: FINALMASK /
 * FULLn / SNI_BOUNDARY / SNI_SPLIT / TLS_RECORD_FRAG / TLS_SNI_RECORDS /
 * HALF / RAW), byte-level rules mirrored from the shared Rust reference
 * `core/vor-core/src/fragment.rs` so the Kotlin port produces the same
 * split points for the same input (see `HelloFragmenterTest`, which runs
 * the shared conformance vectors).
 *
 * The record bytes are never modified — only the write boundaries change.
 * The Edge Bridge ([EdgeBridge]) applies these writes on its loopback
 * relay so ANY core (stock Xray AAR, sing-box process core, …) gets
 * "external finalmask" without needing the patched Xray fork.
 */
object HelloFragmenter {

    const val TLS_CONTENT_HANDSHAKE: Int = 0x16
    const val TLS_HANDSHAKE_CLIENT_HELLO: Int = 0x01

    /** Location of the SNI extension inside a TLS record. */
    data class SniLocation(
        val extensionStart: Int,
        val extensionEnd: Int,
        val hostnameStart: Int,
        val hostnameLen: Int,
    )

    /** True when the buffer looks like a TLS ClientHello handshake record. */
    fun isClientHello(data: ByteArray): Boolean {
        if (data.size < 9) return false
        return (data[0].toInt() and 0xFF) == TLS_CONTENT_HANDSHAKE &&
            (data[1].toInt() and 0xFF) == 0x03 &&
            (data[5].toInt() and 0xFF) == TLS_HANDSHAKE_CLIENT_HELLO
    }

    /** Find the server_name extension (mirrors fragment.rs find_sni). */
    fun findSni(data: ByteArray): SniLocation? {
        if (!isClientHello(data)) return null
        var pos = 5 + 4
        pos += 2 + 32
        val sessionLen = data[pos].toInt() and 0xFF
        pos += 1 + sessionLen
        if (pos + 2 > data.size) return null
        val cipherLen = ((data[pos].toInt() and 0xFF) shl 8) or (data[pos + 1].toInt() and 0xFF)
        pos += 2 + cipherLen
        if (pos >= data.size) return null
        val compLen = data[pos].toInt() and 0xFF
        pos += 1 + compLen
        if (pos + 2 > data.size) return null
        val extensionsLen = ((data[pos].toInt() and 0xFF) shl 8) or (data[pos + 1].toInt() and 0xFF)
        pos += 2
        val extensionsEnd = minOf(pos + extensionsLen, data.size)

        while (pos + 4 <= extensionsEnd) {
            val extType = ((data[pos].toInt() and 0xFF) shl 8) or (data[pos + 1].toInt() and 0xFF)
            val extLen = ((data[pos + 2].toInt() and 0xFF) shl 8) or (data[pos + 3].toInt() and 0xFF)
            val extStart = pos
            val extEnd = minOf(pos + 4 + extLen, extensionsEnd)
            if (extType == 0x0000) {
                var p = extStart + 4 + 2 // skip list length
                if (p < extEnd && (data[p].toInt() and 0xFF) == 0x00) {
                    p += 1
                    if (p + 2 <= extEnd) {
                        val nameLen = ((data[p].toInt() and 0xFF) shl 8) or (data[p + 1].toInt() and 0xFF)
                        p += 2
                        val hostnameLen = minOf(nameLen, extEnd - p).coerceAtLeast(0)
                        return SniLocation(extStart, extEnd, p, hostnameLen)
                    }
                }
                return SniLocation(extStart, extEnd, extStart + 4, 0)
            }
            pos = extEnd
        }
        return null
    }

    /** One planned write. */
    data class Write(val start: Int, val end: Int, val delayMs: Int)

    /**
     * Compute the split points for a strategy (mirrors fragment.rs plan()).
     * Returns the planned writes over the ORIGINAL record bytes.
     */
    fun plan(data: ByteArray, strategy: String, interWriteDelayMs: Int): List<Write> {
        if (!isClientHello(data) || data.size < 10 || strategy == "raw") {
            return listOf(Write(0, data.size, 0))
        }
        val sni = findSni(data)
        val points = when (strategy) {
            "half" -> mutableListOf(data.size / 2)
            "record_split", "tls_record_frag" -> mutableListOf(5)
            "full5" -> chunkPoints(data.size, 5)
            "full10" -> chunkPoints(data.size, 10)
            "full20" -> chunkPoints(data.size, 20)
            "multi64" -> chunkPoints(data.size, 64)
            "sni_boundary" -> mutableListOf((sni?.extensionStart ?: data.size / 2))
            "sni_split", "tls_sni_records" -> mutableListOf(
                sni?.extensionStart ?: data.size / 2,
                sni?.extensionEnd ?: data.size / 2,
            )
            "random_split" -> {
                val splits = pseudoRandomSplits(data, 2, 4).filter { it < (sni?.extensionEnd ?: data.size / 2) }
                (splits + (sni?.extensionEnd ?: data.size / 2)).toMutableList()
            }
            else -> return listOf(Write(0, data.size, 0))
        }

        points.sort()
        points.distinct()

        val writes = mutableListOf<Write>()
        var prev = 0
        for (point in points) {
            if (point <= prev || point >= data.size) continue
            writes.add(Write(prev, point, if (prev == 0) 0 else interWriteDelayMs))
            prev = point
        }
        if (prev < data.size) {
            writes.add(Write(prev, data.size, if (prev == 0) 0 else interWriteDelayMs))
        }
        return if (writes.size <= 1) listOf(Write(0, data.size, 0)) else writes
    }

    /**
     * Materialize the writes as actual byte arrays — the "external
     * finalmask" split the Edge Bridge performs on the wire.
     */
    fun fragment(data: ByteArray, strategy: String, interWriteDelayMs: Int): List<Pair<ByteArray, Int>> =
        plan(data, strategy, interWriteDelayMs).map { write ->
            data.copyOfRange(write.start, write.end) to write.delayMs
        }

    /**
     * TLS_RECORD_FRAG variant that rewrites the two parts as standalone
     * well-formed records (UAC Edge Bridge semantics: 5-byte prefix first
     * record, remainder re-framed as a second record with the same record
     * header type/version and remaining length).
     */
    fun splitIntoWellFormedRecords(data: ByteArray): List<ByteArray> {
        if (data.size <= 5) return listOf(data)
        val first = data.copyOfRange(0, 5)
        val restLen = data.size - 5
        val secondHeader = byteArrayOf(
            data[0],
            data[1],
            data[2],
            ((restLen shr 8) and 0xFF).toByte(),
            (restLen and 0xFF).toByte(),
        )
        val second = secondHeader + data.copyOfRange(5, data.size)
        return listOf(first, second)
    }

    // ---------------------------------------------------------------- helpers

    private fun chunkPoints(length: Int, chunk: Int): MutableList<Int> {
        val points = mutableListOf<Int>()
        var p = chunk
        while (p < length) {
            points.add(p)
            p += chunk
        }
        return points
    }

    /**
     * Deterministic pseudo-random split points (xorshift over the record
     * bytes — identical results to the Rust reference).
     */
    private fun pseudoRandomSplits(data: ByteArray, min: Int, max: Int): List<Int> {
        var seed = 0x9E3779B9.toInt()
        for (b in data) {
            seed = (seed shl 5 or (seed ushr 27)) xor (b.toInt() and 0xFF)
        }
        if (seed == 0) seed = 0x12345678
        val count = min + (seed.toInt().let { if (it < 0) it.inv() else it } % (max - min + 1))
        val points = mutableListOf<Int>()
        var value = seed
        repeat(count) {
            value = value xor (value shl 13)
            value = value xor (value ushr 17)
            value = value xor (value shl 5)
            val point = 1 + (if (value < 0) value.toLong() + 4294967296L else value.toLong()).toInt() % (data.size - 1)
            points.add(point)
        }
        return points.sorted().distinct()
    }
}
