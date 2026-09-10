package com.v2rayez.app.data.ai

import android.content.Context

/**
 * Loader for the shared vor-core data files that ship as app assets.
 *
 * The JSON files under `assets/vor` are byte-identical copies of the
 * files in `core/vor-core/data` — the single source of ISP/DPI knowledge
 * shared by every platform. The [VorCoreKt] interpreter consumes them;
 * JVM unit tests inject the same files directly (see
 * `VorCoreConformanceTest`).
 */
object VorCoreAssets {

    @Volatile
    private var carrierPresetsText: String? = null

    @Volatile
    private var ispProfilesText: String? = null

    /** Carrier presets JSON (UAC carrier tuning) or null until loaded. */
    val carrierPresetsJson: String?
        get() = carrierPresetsText

    /** ISP profiles JSON (MICAFP) or null until loaded. */
    val ispProfilesJson: String?
        get() = ispProfilesText

    /** Load the shared data files from app assets. Idempotent. */
    fun load(context: Context) {
        if (carrierPresetsText == null) {
            runCatching { context.assets.open("vor/carrier-presets.json").bufferedReader().use { it.readText() } }
                .onSuccess { carrierPresetsText = it }
        }
        if (ispProfilesText == null) {
            runCatching { context.assets.open("vor/isp-profiles.json").bufferedReader().use { it.readText() } }
                .onSuccess { ispProfilesText = it }
        }
    }

    /** Test hook: inject texts directly (JVM unit tests). */
    fun injectForTests(carriers: String?, isp: String? = null) {
        carrierPresetsText = carriers
        ispProfilesText = isp
    }
}
