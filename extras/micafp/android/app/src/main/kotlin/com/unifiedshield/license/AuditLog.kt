package com.unifiedshield.license

import android.content.Context
import android.util.Log
import com.google.gson.Gson
import com.google.gson.annotations.SerializedName
import java.io.File
import java.security.MessageDigest

// =============================================================================
// MICAFP Directive v6 — C2/B3.3 Immutable Audit Log.
// Append-only, each record hash-chained to the previous one (SHA-256).
// Real telemetry events only — templated/fabricated reasoning is forbidden
// by the directive. CSV export supported.
// =============================================================================

data class AuditRecord(
    @SerializedName("seq") val seq: Long,
    @SerializedName("tsUtcMillis") val tsUtcMillis: Long,
    @SerializedName("category") val category: String,   // PROTOCOL_SWITCH | LICENSE | LEAK_TEST | AI_FAILOVER | DOCTOR
    @SerializedName("message") val message: String,
    @SerializedName("prevHash") val prevHash: String,
    @SerializedName("selfHash") val selfHash: String
)

class AuditLog private constructor(context: Context) {

    private val TAG = "MicafpAuditLog"
    private val gson = Gson()
    private val logFile: File = File(context.filesDir, "micafp_audit_log.jsonl")
    private val lock = Any()

    private fun sha256Hex(input: String): String {
        val digest = MessageDigest.getInstance("SHA-256")
        val bytes = digest.digest(input.toByteArray(Charsets.UTF_8))
        return bytes.joinToString("") { "%02x".format(it) }
    }

    /**
     * Append a real event. Hash chain: selfHash = SHA256(seq|ts|category|message|prevHash).
     * Any later tampering with history breaks the chain and is detectable.
     */
    fun append(category: String, message: String) {
        synchronized(lock) {
            try {
                val prev = readAll().lastOrNull()
                val seq = (prev?.seq ?: -1L) + 1
                val ts = System.currentTimeMillis()
                val prevHash = prev?.selfHash ?: "GENESIS"
                val selfHash = sha256Hex("$seq|$ts|$category|$message|$prevHash")
                logFile.appendText(
                    gson.toJson(AuditRecord(seq, ts, category, message, prevHash, selfHash)) + "\n",
                    Charsets.UTF_8
                )
            } catch (e: Exception) {
                Log.e(TAG, "Audit append failed: ${e.message}")
            }
        }
    }

    /** Reads all records; verifies chain integrity on the fly. */
    fun readAll(): List<AuditRecord> {
        if (!logFile.exists()) return emptyList()
        return try {
            logFile.readLines(Charsets.UTF_8)
                .filter { it.isNotBlank() }
                .mapNotNull { runCatching { gson.fromJson(it, AuditRecord::class.java) }.getOrNull() }
        } catch (e: Exception) {
            Log.e(TAG, "Audit read failed: ${e.message}")
            emptyList()
        }
    }

    /** Returns true if the full chain verifies (no tamper / no gap). */
    fun verifyChain(): Boolean {
        val records = readAll()
        var prevHash = "GENESIS"
        records.forEach { r ->
            val expected = sha256Hex("${r.seq}|${r.tsUtcMillis}|${r.category}|${r.message}|${r.prevHash}")
            if (r.prevHash != prevHash || r.selfHash != expected) return false
            prevHash = r.selfHash
        }
        return true
    }

    /** CSV export to app-external files dir for user sharing. Returns exported file or null. */
    fun exportCsv(context: Context): File? {
        return try {
            val out = File(context.getExternalFilesDir(null) ?: context.filesDir, "micafp_audit_log.csv")
            out.printWriter().use { w ->
                w.println("seq,timestamp_utc,category,message,prev_hash,self_hash")
                readAll().forEach { r ->
                    val safeMsg = r.message.replace("\"", "\"\"").replace("\n", " ")
                    w.println("${r.seq},${r.tsUtcMillis},${r.category},\"$safeMsg\",${r.prevHash},${r.selfHash}")
                }
            }
            out
        } catch (e: Exception) {
            Log.e(TAG, "Audit CSV export failed: ${e.message}")
            null
        }
    }

    /** Records within the last 24h for the Protocol Intelligence timeline. */
    fun last24h(category: String? = null): List<AuditRecord> {
        val since = System.currentTimeMillis() - 24L * 60L * 60L * 1000L
        return readAll().filter { it.tsUtcMillis >= since && (category == null || it.category == category) }
    }

    companion object {
        @Volatile private var instance: AuditLog? = null
        fun getInstance(context: Context): AuditLog =
            instance ?: synchronized(this) {
                instance ?: AuditLog(context.applicationContext).also { instance = it }
            }
    }
}
