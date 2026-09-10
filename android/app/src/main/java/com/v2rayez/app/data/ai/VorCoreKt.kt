package com.v2rayez.app.data.ai

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.buildJsonArray
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.doubleOrNull
import kotlinx.serialization.json.intOrNull
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.longOrNull
import kotlinx.serialization.json.put
import kotlinx.serialization.json.putJsonObject

/**
 * Pure-Kotlin interpreter of the **shared Vor decision model** — the exact
 * same algorithm as the Rust reference `core/vor-core` (bandit.rs,
 * selector.rs, threat.rs, fragment.rs, isp.rs).
 *
 * Why a second implementation at all? The native `libvor_core.so` (JNI,
 * feature `jni`) is the production fast path, but the Android app must also
 * run on JVM unit tests and gracefully degrade when the `.so` is not yet
 * loaded. To prevent drift the interpreter is pinned to the SAME shared
 * conformance vectors (`core/vor-core/tests/vectors/decision-vectors.json`)
 * that CI runs against the Rust crate — see `VorCoreConformanceTest`.
 *
 * This is the honest way to have one decision model across five platforms:
 * one reference implementation + vector-locked ports, instead of
 * platform-specific forks.
 */
object VorCoreKt {

    private val json = Json { ignoreUnknownKeys = true }

    // ------------------------------------------------------------ UCB1 bandit

    /** Serializable bandit state (field names match the Rust crate exactly). */
    data class BanditState(
        val arms: Map<String, ArmStats> = emptyMap(),
        val totalPulls: Long = 0,
        val alpha: Double = 1.414,
        val decay: Double = 0.95,
    ) {
        companion object {
            const val RECENT_WINDOW = 100

            fun fromJson(root: JsonObject?): BanditState {
                if (root == null) return BanditState()
                val arms = mutableMapOf<String, ArmStats>()
                root["arms"]?.jsonObject?.forEach { (id, statsElement) ->
                    val stats = statsElement.jsonObject
                    arms[id] = ArmStats(
                        pulls = stats["pulls"]?.jsonPrimitive?.longOrNull ?: 0,
                        rewardSum = stats["reward_sum"]?.jsonPrimitive?.doubleOrNull ?: 0.0,
                        recentRewards = (stats["recent_rewards"] as? JsonArray)
                            ?.mapNotNull { it.jsonPrimitive.doubleOrNull } ?: emptyList(),
                    )
                }
                return BanditState(
                    arms = arms,
                    totalPulls = root["total_pulls"]?.jsonPrimitive?.longOrNull ?: 0,
                    alpha = root["alpha"]?.jsonPrimitive?.doubleOrNull ?: 1.414,
                    decay = root["decay"]?.jsonPrimitive?.doubleOrNull ?: 0.95,
                )
            }
        }

        fun toJson(): JsonObject = buildJsonObject {
            putJsonObject("arms") {
                arms.keys.sorted().forEach { id ->
                    val stats = arms.getValue(id)
                    putJsonObject(id) {
                        put("pulls", stats.pulls)
                        put("reward_sum", stats.rewardSum)
                        put("recent_rewards", buildJsonArray { stats.recentRewards.forEach { add(JsonPrimitive(it)) } })
                    }
                }
            }
            put("total_pulls", totalPulls)
            put("alpha", alpha)
            put("decay", decay)
        }

        /** UCB1 score (0.0 for unpulled arms — selection uses infinity). */
        fun armScore(id: String): Double {
            val stats = arms[id] ?: return 0.0
            if (stats.pulls == 0L) return 0.0
            val average = stats.rewardSum / stats.pulls
            val exploration = alpha * kotlin.math.sqrt(2.0 * kotlin.math.ln(maxOf(totalPulls, 1).toDouble()) / stats.pulls)
            return average + exploration
        }

        /** UCB1 selection; unpulled arms first, ties lexicographic. */
        fun select(): String? {
            var best: String? = null
            var bestScore = Double.NEGATIVE_INFINITY
            for (id in arms.keys.sorted()) {
                val stats = arms.getValue(id)
                val score = if (stats.pulls == 0L) Double.POSITIVE_INFINITY else armScore(id)
                if (score > bestScore) {
                    bestScore = score
                    best = id
                }
                // Equal score + sorted iteration keeps the lexicographically
                // smallest (the Rust reference's tie-break).
            }
            return best
        }

        fun update(armId: String, reward: Double): BanditState {
            val stats = arms[armId] ?: return this.copy(totalPulls = totalPulls + 1)
            val recent = (stats.recentRewards + reward).takeLast(RECENT_WINDOW)
            val next = ArmStats(
                pulls = stats.pulls + 1,
                rewardSum = stats.rewardSum * decay + reward,
                recentRewards = recent,
            )
            return copy(arms = arms + (armId to next), totalPulls = totalPulls + 1)
        }
    }

    /** Per-arm statistics. */
    data class ArmStats(
        val pulls: Long,
        val rewardSum: Double,
        val recentRewards: List<Double>,
    )

    // -------------------------------------------------------------- threat

    /** Threat observation window (fields match the Rust crate). */
    data class ThreatObservation(
        val rstAfterHello: Long = 0,
        val cleanConnections: Long = 0,
        val dnsPoisonAnswers: Long = 0,
        val httpBlockResponses: Long = 0,
        val sniFilterEvents: Long = 0,
    ) {
        companion object {
            fun fromJson(root: JsonObject?): ThreatObservation {
                if (root == null) return ThreatObservation()
                fun long(key: String) = root[key]?.jsonPrimitive?.longOrNull ?: 0
                return ThreatObservation(
                    rstAfterHello = long("rst_after_hello"),
                    cleanConnections = long("clean_connections"),
                    dnsPoisonAnswers = long("dns_poison_answers"),
                    httpBlockResponses = long("http_block_responses"),
                    sniFilterEvents = long("sni_filter_events"),
                )
            }
        }

        /** Threat level, mirroring threat.rs. */
        fun level(): String {
            val blockingSignals = rstAfterHello + sniFilterEvents
            val total = blockingSignals + maxOf(cleanConnections, 1)
            if (dnsPoisonAnswers >= 3 || httpBlockResponses >= 5) return "blackout"
            if (blockingSignals == 0L && dnsPoisonAnswers == 0L && httpBlockResponses == 0L) return "none"
            val blockRatio = blockingSignals.toDouble() / total
            return when {
                blockRatio >= 0.8 -> "blackout"
                sniFilterEvents > 0 || blockRatio >= 0.5 -> "active_v2"
                blockingSignals > 0 -> "active_v1"
                else -> "passive"
            }
        }
    }

    // ------------------------------------------------------------- selector

    /** Adaptive selector state (field names match the Rust crate exactly). */
    data class SelectorState(
        val bandit: BanditState = BanditState(),
        val q: Map<String, LearnedEntry> = emptyMap(),
        val championThreshold: Double = 0.6,
    ) {
        companion object {
            fun fromJson(text: String): SelectorState? {
                return try {
                    fromJson(json.parseToJsonElement(text).jsonObject)
                } catch (_: Exception) {
                    null
                }
            }

            fun fromJson(root: JsonObject?): SelectorState {
                if (root == null) return SelectorState()
                val q = mutableMapOf<String, LearnedEntry>()
                root["q"]?.jsonObject?.forEach { (fingerprint, entryElement) ->
                    val entry = entryElement.jsonObject
                    q[fingerprint] = LearnedEntry(
                        champion = entry["champion"]?.jsonPrimitive?.content ?: "",
                        championReward = entry["champion_reward"]?.jsonPrimitive?.doubleOrNull ?: 0.0,
                        lastGood = entry["last_good"]?.jsonPrimitive?.content ?: "",
                        updatedAt = entry["updated_at"]?.jsonPrimitive?.longOrNull ?: 0,
                    )
                }
                return SelectorState(
                    bandit = BanditState.fromJson(root["bandit"]?.jsonObject),
                    q = q,
                    championThreshold = root["champion_threshold"]?.jsonPrimitive?.doubleOrNull ?: 0.6,
                )
            }
        }

        fun toJson(): JsonObject = buildJsonObject {
            put("bandit", bandit.toJson())
            putJsonObject("q") {
                q.keys.sorted().forEach { fingerprint ->
                    val entry = q.getValue(fingerprint)
                    putJsonObject(fingerprint) {
                        put("champion", entry.champion)
                        put("champion_reward", entry.championReward)
                        put("last_good", entry.lastGood)
                        put("updated_at", entry.updatedAt)
                    }
                }
            }
            put("champion_threshold", championThreshold)
        }
    }

    /** One learned entry per network fingerprint. */
    data class LearnedEntry(
        val champion: String,
        val championReward: Double,
        val lastGood: String,
        val updatedAt: Long,
    )

    /** Decision context (fields match the Rust crate exactly). */
    data class DecisionContext(
        val carrier: String = "",
        val fingerprint: String = "",
        val threat: ThreatObservation = ThreatObservation(),
        val availableEngines: List<String> = emptyList(),
        val pinnedEngines: List<String> = emptyList(),
        val threatOverride: String? = null,
    ) {
        companion object {
            fun fromJson(text: String): DecisionContext? {
                return try {
                    val root = json.parseToJsonElement(text).jsonObject
                    DecisionContext(
                        carrier = root["carrier"]?.jsonPrimitive?.content ?: "",
                        fingerprint = root["fingerprint"]?.jsonPrimitive?.content ?: "",
                        threat = ThreatObservation.fromJson(root["threat"]?.jsonObject),
                        availableEngines = (root["available_engines"] as? JsonArray)
                            ?.mapNotNull { it.jsonPrimitive.content } ?: emptyList(),
                        pinnedEngines = (root["pinned_engines"] as? JsonArray)
                            ?.mapNotNull { it.jsonPrimitive.content } ?: emptyList(),
                        threatOverride = (root["threat_override"] as? JsonPrimitive)?.content,
                    )
                } catch (_: Exception) {
                    null
                }
            }
        }
    }

    /** One probe outcome. */
    data class Outcome(
        val fingerprint: String = "",
        val engine: String,
        val strategy: String = "",
        val success: Boolean,
        val rttMs: Long = 0,
        val dpiKill: Boolean = false,
        val throughputBps: Long = 0,
    ) {
        companion object {
            fun fromJson(text: String): Outcome? {
                return try {
                    val root = json.parseToJsonElement(text).jsonObject
                    Outcome(
                        fingerprint = root["fingerprint"]?.jsonPrimitive?.content ?: "",
                        engine = root["engine"]?.jsonPrimitive?.content ?: "",
                        strategy = root["strategy"]?.jsonPrimitive?.content ?: "",
                        success = root["success"]?.jsonPrimitive?.booleanOrNull ?: false,
                        rttMs = root["rtt_ms"]?.jsonPrimitive?.longOrNull ?: 0,
                        dpiKill = root["dpi_kill"]?.jsonPrimitive?.booleanOrNull ?: false,
                        throughputBps = root["throughput_bps"]?.jsonPrimitive?.longOrNull ?: 0,
                    )
                } catch (_: Exception) {
                    null
                }
            }
        }
    }

    /** Reward formula — mirrors selector.rs exactly. */
    fun rewardFor(outcome: Outcome): Double {
        if (!outcome.success) return 0.0
        val latencyScore = 1.0 / (1.0 + outcome.rttMs / 500.0)
        val throughputScore = outcome.throughputBps.toDouble() / (outcome.throughputBps.toDouble() + 1_000_000.0)
        var reward = 0.5 + 0.3 * latencyScore + 0.2 * throughputScore
        if (outcome.dpiKill) reward *= 0.5
        return reward
    }

    /** Engines the selector knows (matches selector.rs KNOWN_ENGINES). */
    val KNOWN_ENGINES = listOf(
        "xray", "singbox", "sni-tunnel", "pattern", "dns-tunnel-dnstt",
        "dns-tunnel-masterdns", "tor", "psiphon", "mitm-fronting",
        "ssh-chain", "naive-https",
    )

    private fun fragmentApplicable(engine: String): Boolean =
        engine in setOf("sni-tunnel", "pattern", "xray", "mitm-fronting")

    // -------------------------------------------------------- carrier presets

    /** Carrier preset — mirrors isp.rs CarrierPreset (subset used here). */
    data class CarrierPreset(
        val key: String,
        val matchAsns: List<String>,
        val defaultStrategy: String,
        val fakeSni: String,
        val fragmentStrategies: List<String>,
        val edges: List<CarrierEdge>,
        val needsTrafficShaping: Boolean,
        val tunMtu: Int,
    )

    /** One carrier edge endpoint. */
    data class CarrierEdge(
        val ip: String,
        val port: Int,
        val maxSplit: Int,
        val label: String,
    )

    /** Embedded carrier presets (copied from core/vor-core/data). */
    val carrierPresets: List<CarrierPreset> by lazy {
        VorCoreAssets.carrierPresetsJson?.let { text ->
            parseCarrierPresets(text)
        } ?: emptyList()
    }

    /** Parse carrier presets JSON (mirrors isp.rs). */
    fun parseCarrierPresets(text: String): List<CarrierPreset> = try {
        val root = json.parseToJsonElement(text).jsonObject
        (root["carriers"] as? JsonArray)?.mapNotNull { element ->
            val carrier = element.jsonObject
            CarrierPreset(
                key = carrier["key"]?.jsonPrimitive?.content ?: return@mapNotNull null,
                matchAsns = (carrier["match_asns"] as? JsonArray)?.mapNotNull { it.jsonPrimitive.content } ?: emptyList(),
                defaultStrategy = carrier["default_strategy"]?.jsonPrimitive?.content ?: "SNI_SPLIT",
                fakeSni = carrier["fake_sni"]?.jsonPrimitive?.content ?: "www.speedtest.net",
                fragmentStrategies = (carrier["fragment_strategies"] as? JsonArray)?.mapNotNull { it.jsonPrimitive.content } ?: emptyList(),
                edges = (carrier["edges"] as? JsonArray)?.mapNotNull { edgeElement ->
                    val edge = edgeElement.jsonObject
                    CarrierEdge(
                        ip = edge["ip"]?.jsonPrimitive?.content ?: return@mapNotNull null,
                        port = edge["port"]?.jsonPrimitive?.intOrNull ?: 443,
                        maxSplit = edge["max_split"]?.jsonPrimitive?.intOrNull ?: 2,
                        label = edge["label"]?.jsonPrimitive?.content ?: "",
                    )
                } ?: emptyList(),
                needsTrafficShaping = carrier["needs_traffic_shaping"]?.jsonPrimitive?.booleanOrNull ?: false,
                tunMtu = carrier["tun"]?.jsonObject?.get("mtu")?.jsonPrimitive?.intOrNull ?: 1280,
            )
        } ?: emptyList()
    } catch (_: Exception) {
        emptyList()
    }

    /** Carrier lookup by key or ASN (mirrors isp.rs). */
    fun carrierByKeyOrAsn(keyOrAsn: String): CarrierPreset? {
        val normalized = keyOrAsn.trim().lowercase()
        val asnNormalized = keyOrAsn.trim().uppercase().removePrefix("AS")
        return carrierPresets.firstOrNull { carrier ->
            carrier.key == normalized ||
                carrier.matchAsns.any { it.trim().uppercase().removePrefix("AS") == asnNormalized }
        }
    }

    /** Recommended strategies, best-first, ending with raw (mirrors isp.rs). */
    fun recommendedStrategies(carrier: CarrierPreset): List<String> {
        val strategies = carrier.fragmentStrategies.filter { it != "raw" }.toMutableList()
        val defaultStrategy = normalizeStrategyName(carrier.defaultStrategy)
        if (defaultStrategy != "raw" && defaultStrategy !in strategies) {
            strategies.add(0, defaultStrategy)
        }
        if ("random_split" !in strategies) strategies.add("random_split")
        strategies.add("raw")
        return strategies
    }

    private fun normalizeStrategyName(name: String): String = when (name) {
        "SNI_SPLIT", "sni_split" -> "sni_split"
        "RECORD_SPLIT", "record_split" -> "record_split"
        "RANDOM_SPLIT", "random_split" -> "random_split"
        else -> name.lowercase()
    }

    // -------------------------------------------------------------- decide

    /** The decision output (fields match the Rust crate exactly). */
    data class Decision(
        val engine: String,
        val strategies: List<String>,
        val fragmentDelayMs: Int,
        val fakeSni: String,
        val edges: List<CarrierEdge>,
        val trafficShaping: Boolean,
        val shaperLevel: String,
        val threatLevel: String,
        val isChampion: Boolean,
        val ranking: List<Pair<String, Double>>,
    ) {
        fun toJson(): JsonObject = buildJsonObject {
            put("engine", engine)
            put("strategies", buildJsonArray { strategies.forEach { add(JsonPrimitive(it)) } })
            put("fragment_delay_ms", fragmentDelayMs)
            put("fake_sni", fakeSni)
            put("edges", buildJsonArray {
                edges.forEach { edge ->
                    add(buildJsonObject {
                        put("ip", edge.ip)
                        put("port", edge.port)
                        put("max_split", edge.maxSplit)
                        put("label", edge.label)
                    })
                }
            })
            put("traffic_shaping", trafficShaping)
            put("shaper_level", shaperLevel)
            put("threat_level", threatLevel)
            put("is_champion", isChampion)
            put("ranking", buildJsonArray {
                ranking.forEach { (arm, score) ->
                    add(buildJsonObject {
                        put("arm", arm)
                        put("score", round3(score))
                    })
                }
            })
        }
    }

    /** Compute the next decision (mirrors selector.rs decide()). */
    fun decide(state: SelectorState, context: DecisionContext): Decision {
        // 1. Carrier profile
        val carrier = if (context.carrier.isEmpty()) null else carrierByKeyOrAsn(context.carrier)

        // 2. Threat level
        val threatLevel = context.threatOverride?.let(::threatFromWire) ?: context.threat.level()

        // 3. Candidate strategies
        val strategies: List<String> = carrier?.let { recommendedStrategies(it) } ?: run {
            val fallback = mutableListOf("sni_split")
            for (extra in listOf("record_split", "full10", "random_split", "raw")) {
                if (extra !in fallback) fallback.add(extra)
            }
            fallback
        }

        // 4. Available engines
        val available = context.availableEngines.ifEmpty { KNOWN_ENGINES }

        // 5. Arms (engine|strategy)
        val arms = mutableListOf<String>()
        for (engine in available) {
            if (fragmentApplicable(engine)) {
                for (strategy in strategies) {
                    arms.add("$engine|$strategy")
                }
            } else {
                arms.add("$engine|raw")
            }
        }

        // 6. Bandit state extended with the current arms
        var bandit = state.bandit
        for (arm in arms) {
            if (arm !in bandit.arms) {
                bandit = bandit.copy(arms = bandit.arms + (arm to ArmStats(0, 0.0, emptyList())))
            }
        }
        val armSet = arms.toSet()
        bandit = bandit.copy(arms = bandit.arms.filterKeys { it in armSet })

        // 7. Champion first when strong enough and available
        val fingerprint = context.fingerprint.ifEmpty { "default" }
        val learned = state.q[fingerprint]
        var chosenArm: String? = null
        var isChampion = false
        if (learned != null && learned.championReward >= state.championThreshold && learned.champion in arms) {
            chosenArm = learned.champion
            isChampion = true
        }
        if (chosenArm == null) {
            chosenArm = if (bandit.totalPulls == 0L) {
                // Fresh state: highest-priority strategy first, tie-break
                // lexicographically (mirrors the Rust reference exactly).
                arms.minWithOrNull(
                    compareBy(
                        { arm ->
                            val strategy = arm.substringAfter("|")
                            strategies.indexOf(strategy).let { if (it < 0) Int.MAX_VALUE else it }
                        },
                        { arm -> arm },
                    )
                )
            } else {
                bandit.select()
            }
        }
        val arm = chosenArm ?: "xray|raw"
        val engine = arm.substringBefore("|")
        val strategy = arm.substringAfter("|", "raw")

        // 8. Carrier extras
        val fakeSni = carrier?.fakeSni ?: "www.speedtest.net"
        val edges = carrier?.edges ?: emptyList()
        val fragmentDelayMs = if (threatRequiresAggressive(threatLevel)) 0 else 5

        // 9. Shaper level
        val ispNeedsShaping = carrier?.needsTrafficShaping ?: false
        val shaperLevel = when {
            !threatLayer2Shaper(threatLevel, ispNeedsShaping) -> "passthrough"
            threatRequiresAggressive(threatLevel) -> "maximum"
            threatRequiresObfuscation(threatLevel) -> "full"
            else -> "light"
        }

        val ranking = bandit.arms.keys
            .map { id -> id to bandit.armScore(id) }
            .sortedWith(compareByDescending<Pair<String, Double>> { it.second }.thenBy { it.first })

        val strategiesOrdered = strategies.toMutableList()
        if (strategy != "raw") {
            strategiesOrdered.remove(strategy)
            strategiesOrdered.add(0, strategy)
        }

        return Decision(
            engine = engine,
            strategies = strategiesOrdered,
            fragmentDelayMs = fragmentDelayMs,
            fakeSni = fakeSni,
            edges = edges,
            trafficShaping = shaperLevel != "passthrough",
            shaperLevel = shaperLevel,
            threatLevel = threatLevel,
            isChampion = isChampion,
            ranking = ranking,
        )
    }

    // -------------------------------------------------------------- observe

    /** Fold an outcome into the state (mirrors selector.rs observe()). */
    fun observe(state: SelectorState, outcome: Outcome, nowEpochSeconds: Long): SelectorState {
        val fingerprint = outcome.fingerprint.ifEmpty { "default" }
        val strategy = outcome.strategy.ifEmpty { "raw" }
        val arm = "${outcome.engine}|$strategy"
        val reward = rewardFor(outcome)

        var bandit = state.bandit
        if (arm !in bandit.arms) {
            bandit = bandit.copy(arms = bandit.arms + (arm to ArmStats(0, 0.0, emptyList())))
        }
        bandit = bandit.update(arm, reward)

        val previous = state.q[fingerprint]
            ?: LearnedEntry(arm, 0.0, "xray|raw", 0)
        val armAverage = bandit.arms[arm]?.let { if (it.pulls == 0L) 0.0 else it.rewardSum / it.pulls } ?: 0.0
        val entry = if (previous.champion != arm && armAverage > previous.championReward) {
            previous.copy(champion = arm, championReward = round3(armAverage))
        } else if (previous.champion == arm) {
            previous.copy(championReward = round3(armAverage))
        } else {
            previous
        }
        val lastGood = if (outcome.success) arm else previous.lastGood
        return state.copy(
            bandit = bandit,
            q = state.q + (fingerprint to entry.copy(lastGood = lastGood, updatedAt = nowEpochSeconds)),
        )
    }

    // ------------------------------------------------------------ threat maps

    private fun threatFromWire(value: String): String = when (value) {
        "passive" -> "passive"
        "active_v1", "active-v1", "ActiveV1" -> "active_v1"
        "active_v2", "active-v2", "ActiveV2" -> "active_v2"
        "blackout", "complete_blackout", "CompleteBlackout" -> "blackout"
        else -> "none"
    }

    private fun threatRequiresAggressive(level: String): Boolean =
        level == "active_v2" || level == "blackout"

    private fun threatRequiresObfuscation(level: String): Boolean = level != "none"

    private fun threatLayer2Shaper(level: String, ispNeedsShaping: Boolean): Boolean = when (level) {
        "none" -> false
        "passive" -> ispNeedsShaping
        else -> true
    }

    private fun round3(value: Double): Double = kotlin.math.round(value * 1000.0) / 1000.0
}
