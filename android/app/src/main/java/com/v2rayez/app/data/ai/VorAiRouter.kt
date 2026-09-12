package com.v2rayez.app.data.ai

import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.telephony.TelephonyManager
import com.v2rayez.app.data.core.DeviceCountry
import com.v2rayez.app.data.service.VpnStateHolder
import com.v2rayez.app.domain.model.ConnectionStatus
import com.v2rayez.app.domain.model.ProxyCoreType
import dagger.hilt.android.qualifiers.ApplicationContext
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import java.io.File
import java.util.Locale
import javax.inject.Inject
import javax.inject.Singleton

/**
 * Production wiring of the shared Vor decision model ([VorCoreKt] + the
 * `assets/vor` data files) into the real connect flow.
 *
 * What it does — and deliberately does NOT do — (UX-honesty guardrail):
 *  * Records a real outcome (success/failure + connect RTT) for every
 *    connect attempt, tagged with the engine that actually ran.
 *  * Ranks the known engines with the UCB1 bandit and exposes the learned
 *    champion per network fingerprint. [recommendCoreType] only returns a
 *    hint once the bandit actually has observation data — a fresh install
 *    never claims knowledge it does not have.
 *  * It is a *preference hint*: it never overrides a user-pinned per-server
 *    core, never overrides protocol constraints (WireGuard/SSH need
 *    sing-box; Tor needs Xray), and never bypasses binary availability.
 *  * Fully offline: state lives in `files/vor-ai-state.json`; no network
 *    call, no telemetry, nothing leaves the device.
 */
@Singleton
class VorAiRouter @Inject constructor(
    @ApplicationContext private val context: Context,
    private val stateHolder: VpnStateHolder,
) {

    companion object {
        private const val STATE_FILE = "vor-ai-state.json"
        /** Below this many total pulls the bandit has no basis for a hint. */
        private const val MIN_PULLS_FOR_HINT = 3L
        /** Wire names for the proxy cores (shared with selector.rs). */
        fun wireName(core: ProxyCoreType): String = when (core) {
            ProxyCoreType.XRAY -> "xray"
            ProxyCoreType.SING_BOX -> "singbox"
            ProxyCoreType.CLASH -> "mihomo"
        }

        fun coreFromWire(name: String): ProxyCoreType? = when (name) {
            "xray" -> ProxyCoreType.XRAY
            "singbox" -> ProxyCoreType.SING_BOX
            "mihomo", "clash" -> ProxyCoreType.CLASH
            else -> null
        }

        /**
         * Pure ranking core of the engine hint (JVM-testable, no Android deps).
         * Returns null when the bandit has no observation data yet or when no
         * available core has ever been tried — a fresh install must not claim
         * knowledge it does not have (UX-honesty guardrail).
         */
        fun recommendCore(
            state: VorCoreKt.SelectorState,
            fp: String,
            available: List<ProxyCoreType>
        ): ProxyCoreType? {
            if (state.bandit.totalPulls < MIN_PULLS_FOR_HINT) return null
            val availableWire = available.map { wireName(it) }
            // Champion of this fingerprint first, when strong and available.
            val champion = state.q[fp]?.takeIf { it.championReward >= state.championThreshold }
            if (champion != null) {
                coreFromWire(champion.champion.split("|").first())?.let { if (it in available) return it }
            }
            // Otherwise the best mean-reward engine that is actually available.
            val byEngine = state.bandit.arms.entries
                .groupBy({ it.key.substringBefore("|") }, { it.value })
                .mapValues { (_, stats) ->
                    val pulls = stats.sumOf { it.pulls }
                    if (pulls == 0L) 0.0 else stats.sumOf { it.rewardSum } / pulls
                }
            return byEngine.entries
                .filter { it.key in availableWire }
                .filter { entry -> state.bandit.arms.keys.any { arm -> arm.startsWith("${entry.key}|") && state.bandit.arms.getValue(arm).pulls > 0 } }
                .maxByOrNull { it.value }
                ?.let { coreFromWire(it.key) }
        }
    }

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val lock = Any()

    private var state = VorCoreKt.SelectorState()
    private var loaded = false

    /** Engine selected for the connect attempt currently in flight. */
    @Volatile
    private var pendingEngine: String? = null

    @Volatile
    private var connectStartMs = 0L

    @Volatile
    private var awaitingOutcome = false

    init {
        scope.launch { loadLocked() }
        // Real-outcome recording: CONNECTING opens an attempt, CONNECTED
        // closes it as a success with the measured connect latency.
        // Failures are recorded from the service's failAndStop() (the only
        // place that knows the failure category), never guessed from text.
        scope.launch {
            stateHolder.connectionState.collect { s ->
                when (s.status) {
                    ConnectionStatus.CONNECTING -> {
                        awaitingOutcome = true
                        connectStartMs = System.currentTimeMillis()
                    }
                    ConnectionStatus.CONNECTED -> {
                        if (awaitingOutcome) {
                            awaitingOutcome = false
                            val engine = pendingEngine ?: "xray"
                            val rtt = System.currentTimeMillis() - connectStartMs
                            recordOutcome(engine, success = true, rttMs = rtt, dpiKill = false)
                        }
                    }
                    ConnectionStatus.DISCONNECTED -> {
                        // Failure path is recorded by the service (category-aware);
                        // a clean user stop after a connected session is not a
                        // connect-outcome at all. Just close the attempt window.
                        awaitingOutcome = false
                    }
                }
            }
        }
    }

    // -------------------------------------------------------- identification

    /**
     * Honest network fingerprint: transport type + country + carrier.
     * No fabricated ASN (that would need a GeoIP database we do not ship).
     */
    fun fingerprint(): String {
        val transport = runCatching {
            val cm = context.getSystemService(Context.CONNECTIVITY_SERVICE) as? ConnectivityManager
            val caps = cm?.getNetworkCapabilities(cm.activeNetwork)
            when {
                caps == null -> "unknown"
                caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) -> "wifi"
                caps.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR) -> "cellular"
                caps.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET) -> "ethernet"
                else -> "other"
            }
        }.getOrDefault("unknown")
        val country = DeviceCountry.detect(context) ?: "zz"
        val carrier = runCatching {
            val tm = context.getSystemService(Context.TELEPHONY_SERVICE) as? TelephonyManager
            tm?.networkOperator?.takeIf { it.isNotBlank() }
        }.getOrNull() ?: "na"
        return "$transport|$country|$carrier".lowercase(Locale.US)
    }

    // -------------------------------------------------------- engine hints

    /**
     * UCB1-ranked engine hint among the *actually available* cores.
     * Returns null when the bandit has no data yet (fresh install) or the
     * ranked engine is not available on this device — the caller then keeps
     * its own resolution untouched.
     */
    fun recommendCoreType(available: List<ProxyCoreType>): ProxyCoreType? {
        synchronized(lock) { ensureLoaded() }
        return recommendCore(state, fingerprint(), available)
    }

    /** Wire-name ranking for diagnostics (best first). */
    fun ranking(): List<Pair<String, Double>> {
        synchronized(lock) {
            ensureLoaded()
            return state.bandit.arms.keys
                .map { it to state.bandit.armScore(it) }
                .sortedWith(compareByDescending<Pair<String, Double>> { it.second }.thenBy { it.first })
        }
    }

    /** The learned champion arm for the current network, if confident. */
    fun championArm(): String? {
        synchronized(lock) {
            ensureLoaded()
            val entry = state.q[fingerprint()] ?: return null
            return entry.takeIf { it.championReward >= state.championThreshold }?.champion
        }
    }

    /** One-line human summary for diagnostics. */
    fun diagnosticsSummary(): String {
        synchronized(lock) {
            ensureLoaded()
            val champ = state.q[fingerprint()]?.let { e ->
                if (e.championReward >= state.championThreshold) e.champion else null
            } ?: "learning"
            return "engine champion: $champ · pulls: ${state.bandit.totalPulls} · fingerprint: ${fingerprint()}"
        }
    }

    // -------------------------------------------------------- observation

    /** The service reports which engine actually runs this attempt. */
    fun noteEngineSelected(engineWireName: String) {
        pendingEngine = engineWireName
    }

    fun noteEngineSelected(core: ProxyCoreType) {
        pendingEngine = wireName(core)
    }

    /**
     * Record a connect outcome (called by the service with the real failure
     * category; success is recorded automatically from the CONNECTED state).
     */
    fun recordConnectFailure(dpiKill: Boolean) {
        if (!awaitingOutcome) return
        awaitingOutcome = false
        val engine = pendingEngine ?: "xray"
        recordOutcome(engine, success = false, rttMs = 0L, dpiKill = dpiKill)
    }

    /** Fold one outcome into the bandit + champion cache and persist. */
    fun recordOutcome(engine: String, success: Boolean, rttMs: Long, dpiKill: Boolean) {
        val next: VorCoreKt.SelectorState
        synchronized(lock) {
            ensureLoaded()
            next = VorCoreKt.observe(
                state,
                VorCoreKt.Outcome(
                    fingerprint = fingerprint(),
                    engine = engine,
                    strategy = "raw",
                    success = success,
                    rttMs = rttMs,
                    dpiKill = dpiKill,
                ),
                System.currentTimeMillis() / 1000L,
            )
            state = next
        }
        scope.launch { persistLocked(next) }
    }

    /** Exposed for tests. */
    fun stateForTests(): VorCoreKt.SelectorState = synchronized(lock) {
        ensureLoaded()
        return state
    }

    // -------------------------------------------------------- persistence

    private fun stateFile(): File = File(context.filesDir, STATE_FILE)

    private fun loadLocked() {
        synchronized(lock) {
            ensureLoaded()
        }
    }

    private fun ensureLoaded() {
        if (loaded) return
        loaded = true
        VorCoreAssets.load(context)
        val text = runCatching { stateFile().readText() }.getOrNull()
        if (text != null) {
            VorCoreKt.SelectorState.fromJson(text)?.let { state = it }
        }
    }

    private fun persistLocked(snapshot: VorCoreKt.SelectorState) {
        runCatching {
            val tmp = File(context.filesDir, "$STATE_FILE.tmp")
            tmp.writeText(snapshot.toJson().toString())
            if (!tmp.renameTo(stateFile())) {
                stateFile().delete()
                tmp.renameTo(stateFile())
            }
        }
    }
}
