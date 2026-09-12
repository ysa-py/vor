package com.v2rayez.app.data.license

import android.annotation.SuppressLint
import android.content.Context
import android.media.MediaDrm
import android.os.Build
import android.provider.Settings
import java.util.UUID

/**
 * Permission-free device identity components for the native DRM core's
 * hardware binding (see core/vor-drm/src/hwid.rs).
 *
 * Three stable signals, NONE of which needs a runtime permission:
 *  1. Widevine MediaDRM device unique ID — the same per-factory identity
 *     signal commercial DRM stacks use (API 18+, no permission);
 *  2. ANDROID_ID (SSAID) — resets only on factory reset;
 *  3. Build.FINGERPRINT — model/hardware/build stability marker.
 *
 * The native core hashes the length-prefixed triple with SHA-256 into the
 * HWID a VOR2 token can lock to. Gathering is best-effort and NEVER throws:
 * an unavailable component contributes an empty string (deterministic and
 * still device-stable — the license simply locks to whatever the device
 * can produce consistently).
 */
object VorDeviceIdentity {

    /** The standard Widevine system UUID (edef8ba9-79d6-4ace-a3c8-27dcd51d21ed). */
    private val WIDEVINE_UUID: UUID = UUID(
        -1301668207276963122L,
        -6645017420763422227L,
    )

    /** Gather the identity triple; each component degrades to "" on failure. */
    @SuppressLint("HardwareIds")
    fun gather(context: Context): Triple<String, String, String> = Triple(
        first = widevineIdHex(),
        second = runCatching {
            Settings.Secure.getString(context.contentResolver, Settings.Secure.ANDROID_ID)
        }.getOrNull().orEmpty(),
        third = runCatching { Build.FINGERPRINT }.getOrNull().orEmpty(),
    )

    /**
     * Widevine MediaDRM device unique ID, lowercase hex; "" when the stack is
     * unavailable (no Widevine on the device, or a transient MediaDrm failure).
     */
    private fun widevineIdHex(): String = runCatching {
        val drm = MediaDrm(WIDEVINE_UUID)
        try {
            val id = drm.getPropertyByteArray(MediaDrm.PROPERTY_DEVICE_UNIQUE_ID)
            buildString(id.size * 2) {
                for (byte in id) {
                    append("0123456789abcdef"[byte.toInt() shr 4 and 0x0F])
                    append("0123456789abcdef"[byte.toInt() and 0x0F])
                }
            }
        } finally {
            runCatching { drm.close() }
        }
    }.getOrNull().orEmpty()
}
