package com.v2rayez.app.data.uac

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

/**
 * Fragmentation conformance — runs the shared cross-platform fragment
 * vectors (`core/vor-core/tests/vectors/decision-vectors.json`) against the
 * Kotlin port, pinning it to the Rust reference.
 */
class HelloFragmenterTest {

    private fun resource(name: String): String {
        val url = Thread.currentThread().contextClassLoader?.getResource(name)
            ?: throw AssertionError("missing test resource: $name")
        return File(url.toURI()).readText()
    }

    /** Same deterministic ClientHello as the Rust conformance test. */
    private fun clientHello(): ByteArray {
        val name = "example.com".toByteArray()
        val ext = mutableListOf<Byte>()
        val listLen = (1 + 2 + name.size)
        ext += ((listLen shr 8) and 0xFF).toByte()
        ext += (listLen and 0xFF).toByte()
        ext += 0x00
        ext += ((name.size shr 8) and 0xFF).toByte()
        ext += (name.size and 0xFF).toByte()
        ext += name.toList()
        val trailing = mutableListOf<Byte>()
        trailing += 0x00
        trailing += 0x2B
        trailing += 0x00
        trailing += 0x02
        trailing += 0x03
        trailing += 0x04
        val extTotal = ext.size + 4 + trailing.size

        val handshake = mutableListOf<Byte>()
        handshake += listOf(0x03, 0x03).map { it.toByte() }
        repeat(32) { handshake += 0x00 }
        handshake += 0x00
        handshake += 0x00
        handshake += 0x02
        handshake += 0x13
        handshake += 0x01
        handshake += 0x01
        handshake += 0x00
        handshake += ((extTotal shr 8) and 0xFF).toByte()
        handshake += (extTotal and 0xFF).toByte()
        handshake += 0x00 // ext type high
        handshake += 0x00 // ext type low (server_name)
        handshake += ((ext.size shr 8) and 0xFF).toByte()
        handshake += (ext.size and 0xFF).toByte()
        handshake += ext
        handshake += trailing

        val record = mutableListOf<Byte>()
        record += 0x16
        record += 0x03
        record += 0x01
        record += (((handshake.size + 4) shr 8) and 0xFF).toByte()
        record += ((handshake.size + 4) and 0xFF).toByte()
        record += 0x01
        record += ((handshake.size shr 16) and 0xFF).toByte()
        record += ((handshake.size shr 8) and 0xFF).toByte()
        record += (handshake.size and 0xFF).toByte()
        record += handshake
        return record.toByteArray()
    }

    @Test
    fun detects_client_hello_and_finds_sni() {
        val record = clientHello()
        assertTrue(HelloFragmenter.isClientHello(record))
        val sni = HelloFragmenter.findSni(record)!!
        assertEquals("example.com", String(record, sni.hostnameStart, sni.hostnameLen, Charsets.UTF_8))
    }

    @Test
    fun shared_fragment_vectors() {
        val root = Json.parseToJsonElement(resource("decision-vectors.json")).jsonObject
        val record = clientHello()
        root["fragment_cases"]!!.jsonArray.forEach { element ->
            val case = element.jsonObject
            val name = case["name"]!!.jsonPrimitive.content
            val strategy = case["strategy"]!!.jsonPrimitive.content
            val delay = case["delay_ms"]?.jsonPrimitive?.content?.toInt() ?: 0
            val writes = HelloFragmenter.plan(record, strategy, delay)

            case["expected_writes"]?.jsonPrimitive?.content?.toInt()?.let {
                assertEquals("case $name: write count", it, writes.size)
            }
            case["expected_min_writes"]?.jsonPrimitive?.content?.toInt()?.let {
                assertTrue("case $name: min writes (got ${writes.size})", writes.size >= it)
            }
            case["expected_first_write_end"]?.jsonPrimitive?.content?.toInt()?.let {
                assertEquals("case $name: first write end", it, writes.first().end)
            }
            case["expected_first_delay"]?.jsonPrimitive?.content?.toInt()?.let {
                assertEquals("case $name: first delay", it, writes.first().delayMs)
            }
            case["expected_second_delay"]?.jsonPrimitive?.content?.toInt()?.let {
                assertEquals("case $name: second delay", it, writes[1].delayMs)
            }
            case["expected_chunk_bytes"]?.jsonPrimitive?.content?.toInt()?.let { chunk ->
                writes.dropLast(1).forEach { write ->
                    assertEquals("case $name: chunk size", chunk, write.end - write.start)
                }
            }
            // coverage invariant
            var cursor = 0
            writes.forEach { write ->
                assertEquals("case $name: contiguity", cursor, write.start)
                cursor = write.end
            }
            assertEquals("case $name: coverage", record.size, cursor)
        }
    }

    @Test
    fun materialized_fragments_concatenate_to_original() {
        val record = clientHello()
        val pieces = HelloFragmenter.fragment(record, "sni_split", 5)
        assertEquals(record.toList(), pieces.flatMap { it.first.toList() })
    }

    @Test
    fun well_formed_records_roundtrip() {
        val record = clientHello()
        val pieces = HelloFragmenter.splitIntoWellFormedRecords(record)
        assertEquals(2, pieces.size)
        assertEquals(5, pieces[0].size)
        // second record: header(5) + rest
        assertEquals(record.size - 5, pieces[1].size - 5)
        assertEquals(
            record.drop(5),
            pieces[1].drop(5),
        )
        // reassembled payload
        assertEquals(record.drop(5), pieces[0].drop(5) + pieces[1].drop(5))
    }

    @Test
    fun non_client_hello_passes_through() {
        val junk = ByteArray(64) { 0x17 }
        val writes = HelloFragmenter.plan(junk, "sni_split", 0)
        assertEquals(1, writes.size)
        assertEquals(junk.size, writes[0].end)
    }
}
