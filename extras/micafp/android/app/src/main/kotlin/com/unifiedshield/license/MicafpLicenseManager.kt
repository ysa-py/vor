package com.unifiedshield.license

import android.content.Context
import android.content.Intent
import android.util.Log
import com.google.gson.Gson
import com.google.gson.annotations.SerializedName
import com.google.crypto.tink.subtle.Ed25519Verify
import com.unifiedshield.VpnService
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.net.HttpURLConnection
import java.net.URL
import java.security.GeneralSecurityException
import java.time.Instant
import java.util.Base64

// =============================================================================
// MICAFP Directive v6 — C1 License Anti-Forgery (Kotlin verification layer).
//
// Design (per directive):
//  * License payload is Ed25519-signed server-side; client embeds ONLY the
//    public verification key (build-time constant below). Private key never
//    ships — it lives with the product owner's signing service.
//  * Expiry is a SINGLE configurable field inside the signed payload
//    (expiryUtcMillis). Default per master directive v6: Azar 19, 1405 =
//    2026-12-10T00:00:00Z. NOT hardcoded in multiple places.
//  * Trusted time: HTTP `Date` headers from several well-known endpoints,
//    majority-tolerant. Never trusts local clock alone.
//  * Monotonic time ratchet: persisted max(seenTime) guards against clock
//    rollback while offline (offline devices still respect signed expiry).
//  * On expiry: ONLY the international tunnel is cut (VpnService stop).
//    Domestic routing, settings, profiles, diagnostics and AI advisory panel
//    remain fully functional (product-owner decision, confirmed twice).
//  * Revocation list is optional secondary gate; signed expiry is primary.
// =============================================================================

data class MicafpLicensePayload(
    @SerializedName("licenseId") val licenseId: String,
    @SerializedName("issuedAtUtcMillis") val issuedAtUtcMillis: Long,
    @SerializedName("expiryUtcMillis") val expiryUtcMillis: Long,
    @SerializedName("deviceHash") val deviceHash: String,   // "" = not device-bound
    @SerializedName("maxDevices") val maxDevices: Int = 1,
    @SerializedName("features") val features: List<String> = emptyList()
)

enum class LicenseVerifyState { NO_LICENSE, VALID, INVALID_SIGNATURE, DEVICE_MISMATCH, EXPIRED }

data class MicafpLicenseState(
    val state: LicenseVerifyState = LicenseVerifyState.NO_LICENSE,
    val payload: MicafpLicensePayload? = null,
    val verifiedAtMillis: Long = 0,
    val trustedTimeMillis: Long = 0,
    val trustedTimeSynced: Boolean = false,
    val expiryUtcMillis: Long = DEFAULT_EXPIRY_UTC,
    val message: String = ""
) {
    companion object {
        // Master directive v6 default expiry: Azar 19, 1405 → 2026-12-10T00:00:00Z.
        // Single source of truth; overridable per-license via signed payload.
        const val DEFAULT_EXPIRY_UTC: Long = 1796841600000L // 2026-12-10T00:00:00Z
    }
}

class MicafpLicenseManager private constructor(private val context: Context) {

    private val TAG = "MicafpLicense"
    private val gson = Gson()
    private val audit = AuditLog.getInstance(context)

    // ---------------------------------------------------------------------
    // PUBLIC VERIFICATION KEY (Ed25519 raw 32-byte, base64).
    // Build-time constant. Replace with the product owner's published key
    // when the signing service is provisioned. Until then the manager runs
    // in advisory mode and reports "pending backend wiring" honestly (A2).
    // ---------------------------------------------------------------------
    private val PUBLIC_KEY_B64 = "PENDING_PROD_OWNER_PUBLISHED_KEY_REPLACE_ME_BUILDER_TIME"

    // Time ratchet: persisted monotonic max seen timestamp.
    private val prefs = context.getSharedPreferences("micafp_license", Context.MODE_PRIVATE)

    private val _state = kotlinx.coroutines.flow.MutableStateFlow(MicafpLicenseState())
    val state: kotlinx.coroutines.flow.StateFlow<MicafpLicenseState> = _state

    // --------------------------- device binding ---------------------------
    fun deviceHash(): String {
        val androidId = android.provider.Settings.Secure.getString(
            context.contentResolver, android.provider.Settings.Secure.ANDROID_ID
        ) ?: "unknown-device"
        val digest = java.security.MessageDigest.getInstance("SHA-256")
            .digest("$androidId|micafp-device-bind-v1".toByteArray(Charsets.UTF_8))
        return digest.joinToString("") { "%02x".format(it) }
    }

    // --------------------------- trusted time -----------------------------
    /**
     * Fetch trusted time from HTTP Date headers (5s timeout each), take the
     * median of successful responses. Falls back to the persisted ratchet
     * (max of all previously seen trusted-or-local times) — never to raw
     * local clock alone. Returns null pair if no source available.
     */
    suspend fun fetchTrustedTime(): Pair<Long, Boolean> = withContext(Dispatchers.IO) {
        val endpoints = listOf(
            "https://www.google.com/generate_204",
            "https://www.cloudflare.com/cdn-cgi/trace",
            "https://www.microsoft.com/favicon.ico"
        )
        val samples = mutableListOf<Long>()
        for (ep in endpoints) {
            try {
                val conn = URL(ep).openConnection() as HttpURLConnection
                conn.connectTimeout = 5000
                conn.readTimeout = 5000
                conn.requestMethod = "HEAD"
                conn.connect()
                val dateHeader = conn.getHeaderField("Date")
                conn.disconnect()
                dateHeader?.let {
                    samples.add(java.time.format.DateTimeFormatter.RFC_1123_DATE_TIME
                        .parse(it, Instant::from).toEpochMilli())
                }
            } catch (e: Exception) {
                Log.w(TAG, "Trusted time endpoint failed: ${e.message}")
            }
        }
        if (samples.isNotEmpty()) {
            samples.sort()
            val trusted = samples[samples.size / 2]
            ratchetTime(trusted)
            Pair(trusted, true)
        } else {
            Pair(ratchetTime(System.currentTimeMillis()), false)
        }
    }

    /** Monotonic ratchet — never goes backwards across restarts. */
    @Synchronized
    private fun ratchetTime(candidate: Long): Long {
        val prev = prefs.getLong("time_ratchet", 0L)
        val next = maxOf(prev, candidate)
        prefs.edit().putLong("time_ratchet", next).apply()
        return next
    }

    // --------------------------- license verify ---------------------------
    /**
     * Wire format: base64(payloadJson).base64(ed25519Signature)
     * Any deviation → INVALID_SIGNATURE (fail closed, never fail open).
     */
    suspend fun applyLicense(serialText: String): MicafpLicenseState {
        val (trustedNow, synced) = fetchTrustedTime()
        val baseState = MicafpLicenseState(
            trustedTimeMillis = trustedNow, trustedTimeSynced = synced,
            expiryUtcMillis = prefs.getLong("expiry_utc", MicafpLicenseState.DEFAULT_EXPIRY_UTC)
        )

        val parts = serialText.trim().split(".")
        if (parts.size != 2 || PUBLIC_KEY_B64.startsWith("PENDING")) {
            val st = baseState.copy(
                state = LicenseVerifyState.NO_LICENSE,
                message = if (PUBLIC_KEY_B64.startsWith("PENDING"))
                    "pending-backend-wiring" else "malformed-license"
            )
            audit.append("LICENSE", "License apply rejected: ${st.message} (payload parts=${parts.size})")
            _state.value = st
            return st
        }

        return try {
            val payloadBytes = Base64.getDecoder().decode(parts[0])
            val sigBytes = Base64.getDecoder().decode(parts[1])
            val verifier = Ed25519Verify(Base64.getDecoder().decode(PUBLIC_KEY_B64))
            verifier.verify(sigBytes, payloadBytes) // throws on bad signature — fail closed

            val payload = gson.fromJson(String(payloadBytes, Charsets.UTF_8), MicafpLicensePayload::class.java)

            if (payload.deviceHash.isNotEmpty() && payload.deviceHash != deviceHash()) {
                val st = baseState.copy(state = LicenseVerifyState.DEVICE_MISMATCH, message = "device-mismatch")
                audit.append("LICENSE", "License ${payload.licenseId} rejected: bound to different device")
                _state.value = st
                return st
            }

            prefs.edit().putLong("expiry_utc", payload.expiryUtcMillis).apply()

            val st = if (trustedNow > payload.expiryUtcMillis) {
                baseState.copy(
                    state = LicenseVerifyState.EXPIRED, payload = payload,
                    expiryUtcMillis = payload.expiryUtcMillis, message = "expired"
                )
            } else {
                baseState.copy(
                    state = LicenseVerifyState.VALID, payload = payload,
                    expiryUtcMillis = payload.expiryUtcMillis, verifiedAtMillis = trustedNow,
                    message = "signature-valid"
                )
            }
            audit.append("LICENSE", "License ${payload.licenseId} verified: ${st.state} expiry=${payload.expiryUtcMillis} trustedTime=$trustedNow synced=$synced")
            _state.value = st
            st
        } catch (e: GeneralSecurityException) {
            val st = baseState.copy(state = LicenseVerifyState.INVALID_SIGNATURE, message = "signature-invalid")
            audit.append("LICENSE", "License rejected: Ed25519 signature verification failed (${e.message})")
            _state.value = st
            st
        } catch (e: Exception) {
            val st = baseState.copy(state = LicenseVerifyState.INVALID_SIGNATURE, message = "parse-error")
            audit.append("LICENSE", "License rejected: payload parse error (${e.message})")
            _state.value = st
            st
        }
    }

    /**
     * Directive-mandated expiry behaviour: cut ONLY the international tunnel.
     * VpnService is the international path; the domestic intranet stack,
     * settings, profiles, diagnostics and AI advisory stay alive.
     */
    fun enforceExpiryPolicy() {
        val st = _state.value
        val expiry = st.payload?.expiryUtcMillis ?: st.expiryUtcMillis
        val now = st.trustedTimeMillis
        if (st.state == LicenseVerifyState.EXPIRED || (now > 0 && now > expiry)) {
            Log.w(TAG, "License expired — cutting INTERNATIONAL tunnel only (domestic stays up)")
            audit.append("LICENSE", "Expiry enforcement: international tunnel stopped; domestic routing retained")
            try {
                context.startService(Intent(context, VpnService::class.java).apply {
                    action = VpnService.ACTION_STOP
                })
            } catch (e: Exception) {
                Log.e(TAG, "Expiry tunnel cut failed: ${e.message}")
            }
        }
    }

    fun currentExpiryUtc(): Long = prefs.getLong("expiry_utc", MicafpLicenseState.DEFAULT_EXPIRY_UTC)

    companion object {
        @Volatile private var instance: MicafpLicenseManager? = null
        fun getInstance(context: Context): MicafpLicenseManager =
            instance ?: synchronized(this) {
                instance ?: MicafpLicenseManager(context.applicationContext).also { instance = it }
            }
    }
}
