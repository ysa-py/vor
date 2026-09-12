package com.v2rayez.app.data.license

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.longPreferencesKey
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import com.vor.license.LicenseResult
import com.vor.license.LicenseStatus
import com.vor.license.LicenseVerifier
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.catch
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.runBlocking
import dagger.hilt.android.qualifiers.ApplicationContext
import javax.inject.Inject
import javax.inject.Singleton

private val Context.licenseDataStore: DataStore<Preferences> by preferencesDataStore(name = "vor_license")

/**
 * Offline license gate state — persistence + verification.
 *
 * The gate is the FIRST screen of Vor: the user pastes/imports a token, it
 * is verified locally against the embedded public key (no network), and
 * only then does the rest of the app unlock. Re-checked on every launch and
 * at least every 24 hours while running — an expired license re-locks the
 * app automatically at the next check (no admin action, per spec).
 *
 * 2026-09 v1.0.4 anti-rollback: every verification now runs against
 * [LicenseClock.nowSeconds] — max(device clock, monotonic ratchet, trusted
 * HTTPS time) — so winding the device clock backward cannot resurrect an
 * expired license. Trusted time refresh is opportunistic (throttled,
 * failure-tolerant) and verification itself stays fully offline.
 */
@Singleton
class LicenseRepository @Inject constructor(
    @ApplicationContext private val context: Context,
    private val licenseClock: LicenseClock,
) {
    companion object {
        private val KEY_TOKEN = stringPreferencesKey("vor_license_token")
        private val KEY_LAST_CHECK = longPreferencesKey("vor_license_last_check_epoch")

        /** Re-verification cadence while the app runs (24h). */
        const val RECHECK_INTERVAL_S = 24 * 3600L

        /**
         * DEV public key (base64url) — used by debug builds and unit tests.
         * Release builds override this via [publicKeyOverride] at init time
         * with the production key baked by CI.
         */
        val DEV_PUBLIC_KEY: String = BuildConfigLicenses.DEV_PUBLIC_KEY

        /** Production public key override (set from app init when present). */
        @Volatile
        var publicKeyOverride: String? = null

        /** The public key this build verifies against. */
        val activePublicKey: String
            get() = publicKeyOverride ?: DEV_PUBLIC_KEY
    }

    /**
     * Hardware identity for the native DRM core (v1.3.0). Gathered lazily,
     * cached for the process — the components are factory-stable by design,
     * and the gather itself costs one MediaDrm construction.
     */
    private val deviceHwid: Triple<String, String, String>? by lazy {
        runCatching { VorDeviceIdentity.gather(context) }.getOrNull()
    }

    /** Gate state surfaced to the UI. */
    data class GateState(
        val hydrated: Boolean = false,
        val status: LicenseStatus = LicenseStatus.INVALID,
        val token: String? = null,
        val payload: com.vor.license.LicensePayload? = null,
    )

    /**
     * Flow of the current gate state (null token -> locked).
     *
     * 2026-09 crash hardening: the verification used to run bare inside [map] — any
     * error escaping it (storage failure, an unexpected throw on one device) would
     * kill the eager [androidx.lifecycle.ViewModel.viewModelScope] collector and
     * take the whole process down. The flow is now guarded with [catch] and falls
     * back to a locked-but-hydrated state, so the WORST possible failure mode is a
     * re-locked gate — never a crash.
     */
    val gateState: Flow<GateState> = context.licenseDataStore.data
        .map { preferences ->
            val token = preferences[KEY_TOKEN]
            if (token.isNullOrBlank()) {
                GateState(hydrated = true)
            } else {
                val result = verifyBlocking(token, licenseClock.nowSeconds())
                GateState(
                    hydrated = true,
                    status = result.status,
                    token = token,
                    payload = result.payload,
                )
            }
        }
        .catch { emit(GateState(hydrated = true)) }

    /** Current token (blocking — used by launch-time checks only). */
    private fun currentToken(): String? = runBlocking {
        context.licenseDataStore.data.first()[KEY_TOKEN]
    }

    /** Verify a token string; returns [LicenseResult]. Never touches the network
     *  for the verification itself (a throttled, opportunistic trusted-time
     *  sample may be fetched in the background — see [LicenseClock]). */
    fun verify(token: String): LicenseResult {
        licenseClock.refreshTrustedTimeAsync()
        return verifyBlocking(token, licenseClock.nowSeconds())
    }

    private fun verifyBlocking(token: String, nowEpochSeconds: Long): LicenseResult {
        // Native DRM core first (v1.3.0): when libvor_drm.so ships with the
        // build, it verifies BOTH generations (VOR1 + hardware-locked VOR2)
        // with the full native ladder — signature, encrypted payload, HWID
        // binding, anti-rollback, anti-analysis — and its verdict wins.
        // Stock builds without the .so never even reach the identity gather
        // (zero overhead, zero behavior change).
        if (VorDrmClient.available) {
            VorDrmClient.verify(
                token = token,
                publicKeyB64 = activePublicKey,
                persistedRatchet = licenseClock.ratchetSecondsSnapshot(),
                trusted = licenseClock.trustedSecondsSnapshot(),
                deviceWall = nowEpochSeconds,
                hwid = deviceHwid,
            )?.let { native -> return native.asLicenseResult() }
        }
        return LicenseVerifier.verify(activePublicKey, token, nowEpochSeconds)
    }

    /**
     * Activate a token: verifies first, persists only when valid (or
     * expired-with-valid-signature so the UI can show the expiry).
     *
     * 2026-09 crash hardening: the DataStore write is failure-tolerant — a storage
     * error must surface as "not activated", never as a process crash.
     */
    suspend fun activate(token: String): LicenseResult {
        val result = runCatching { verify(token) }.getOrElse {
            return LicenseResult(LicenseStatus.INVALID, null)
        }
        if (result.status != LicenseStatus.INVALID) {
            runCatching {
                context.licenseDataStore.edit { preferences ->
                    preferences[KEY_TOKEN] = token.trim()
                    preferences[KEY_LAST_CHECK] = System.currentTimeMillis() / 1000
                }
            }.onFailure { android.util.Log.w("LicenseRepository", "persist failed", it) }
        }
        return result
    }

    /** Clear the stored license (locks the app). */
    suspend fun clear() {
        context.licenseDataStore.edit { preferences ->
            preferences.remove(KEY_TOKEN)
            preferences.remove(KEY_LAST_CHECK)
        }
    }

    /**
     * Periodic re-check: returns true when the license is still valid and
     * refreshes the last-check timestamp. An expired license returns false —
     * the caller re-locks the UI (no admin action needed, per spec).
     */
    suspend fun recheck(): Boolean {
        val token = currentToken() ?: return false
        val result = runCatching { verify(token) }
            .getOrElse { return false }
        runCatching {
            context.licenseDataStore.edit { preferences ->
                preferences[KEY_LAST_CHECK] = System.currentTimeMillis() / 1000
            }
        }
        return result.status == LicenseStatus.VALID
    }

    /**
     * Re-verify against the current device clock (launch / app-to-foreground /
     * pre-connect). Touches DataStore so [gateState] re-emits with a fresh
     * verdict — the gate re-locks by itself the moment a license expires,
     * purely from the signed expiry claim vs. clock, no network involved.
     */
    suspend fun refresh() {
        runCatching {
            context.licenseDataStore.edit { preferences ->
                preferences[KEY_LAST_CHECK] = System.currentTimeMillis() / 1000
            }
        }
    }

    /** Synchronous validity probe for connect-time gating (offline, fast). */
    fun isValidNow(): Boolean {
        val token = currentToken() ?: return false
        return runCatching { verify(token).status == LicenseStatus.VALID }
            .getOrDefault(false)
    }
}

/** Build-config license key holder (see build.gradle.kts VOR_LICENSE_PUBLIC_KEY). */
object BuildConfigLicenses {
    /** Public key for this build: dev key by default, production key when CI
     * passes -Pvor.licensePublicKey for release builds. */
    val DEV_PUBLIC_KEY: String = com.v2rayez.app.BuildConfig.VOR_LICENSE_PUBLIC_KEY
}

/**
 * Map a native DRM verdict onto the shared [LicenseResult] shape the UI
 * already consumes (v1.3.0). The native core returns the SAME wire status
 * names and the SAME payload fields, so the mapping is a pure translation.
 */
private fun VorDrmClient.NativeVerdict.asLicenseResult(): LicenseResult {
    val status = when (status) {
        "VALID" -> LicenseStatus.VALID
        "EXPIRED" -> LicenseStatus.EXPIRED
        else -> LicenseStatus.INVALID
    }
    val mapped = payload?.let { native ->
        com.vor.license.LicensePayload(
            version = native.generation,
            id = native.id,
            product = "vor",
            issuedAt = native.issuedAt,
            expiresAt = native.expiresAt,
            tier = native.tier,
            platforms = native.platforms,
        )
    }
    return LicenseResult(status, mapped)
}
