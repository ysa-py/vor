package com.v2rayez.app.data.ai

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import java.io.File

/**
 * Cross-platform decision conformance for the Kotlin interpreter.
 *
 * The exact same `core/vor-core/tests/vectors/decision-vectors.json` is run
 * against the Rust reference (`cargo test`) and this Kotlin port, so the
 * Android interpreter provably agrees with the shared library.
 */
class VorCoreConformanceTest {

    private fun resource(name: String): String {
        val url = Thread.currentThread().contextClassLoader?.getResource(name)
            ?: throw AssertionError("missing test resource: $name")
        return File(url.toURI()).readText()
    }

    @Before
    fun setUp() {
        VorCoreAssets.injectForTests(resource("carrier-presets.json"))
    }

    private fun vectorsRoot() = Json.parseToJsonElement(resource("decision-vectors.json")).jsonObject

    @Test
    fun reward_vectors() {
        val root = vectorsRoot()
        val tolerance = root["metadata"]!!.jsonObject["reward_tolerance"]!!.jsonPrimitive.content.toDouble()
        root["reward_cases"]!!.jsonArray.forEach { element ->
            val case = element.jsonObject
            val outcome = VorCoreKt.Outcome.fromJson(case["outcome"]!!.toString())!!
            val expected = case["expected_reward"]!!.jsonPrimitive.content.toDouble()
            val actual = VorCoreKt.rewardFor(outcome)
            assertTrue(
                "case ${case["name"]}: |$actual - $expected| > $tolerance",
                kotlin.math.abs(actual - expected) <= tolerance,
            )
        }
    }

    @Test
    fun threat_vectors() {
        val root = vectorsRoot()
        root["threat_cases"]!!.jsonArray.forEach { element ->
            val case = element.jsonObject
            val observation = VorCoreKt.ThreatObservation.fromJson(case["observation"]!!.jsonObject)
            val expected = case["expected_level"]!!.jsonPrimitive.content
            assertEquals("case ${case["name"]}", expected, observation.level())
        }
    }

    @Test
    fun decision_vectors() {
        val root = vectorsRoot()
        root["decision_cases"]!!.jsonArray.forEach { element ->
            val case = element.jsonObject
            val name = case["name"]!!.jsonPrimitive.content
            val state = when (val stateElement = case["state"]) {
                null, is kotlinx.serialization.json.JsonNull -> VorCoreKt.SelectorState()
                else -> VorCoreKt.SelectorState.fromJson(stateElement.toString())
            }
            assertNotNull("case $name: state must parse", state)
            val context = VorCoreKt.DecisionContext.fromJson(case["context"]!!.toString())
            assertNotNull("case $name: context must parse", context)
            val decision = VorCoreKt.decide(state!!, context!!)

            val expected = case["expected"]!!.jsonObject
            assertEquals("case $name: engine", expected["engine"]!!.jsonPrimitive.content, decision.engine)
            expected["first_strategy"]?.let {
                assertEquals("case $name: first strategy", it.jsonPrimitive.content, decision.strategies.first())
            }
            (expected["strategies"] as? kotlinx.serialization.json.JsonArray)?.let { want ->
                assertEquals(
                    "case $name: strategies",
                    want.map { it.jsonPrimitive.content },
                    decision.strategies,
                )
            }
            expected["fragment_delay_ms"]?.let {
                assertEquals("case $name: delay", it.jsonPrimitive.content.toInt(), decision.fragmentDelayMs)
            }
            expected["fake_sni"]?.let {
                assertEquals("case $name: fake sni", it.jsonPrimitive.content, decision.fakeSni)
            }
            expected["edge_count"]?.let {
                assertEquals("case $name: edge count", it.jsonPrimitive.content.toInt(), decision.edges.size)
            }
            expected["first_edge"]?.jsonObject?.let { want ->
                assertEquals("case $name: first edge ip", want["ip"]!!.jsonPrimitive.content, decision.edges.first().ip)
                assertEquals("case $name: first edge port", want["port"]!!.jsonPrimitive.content.toInt(), decision.edges.first().port)
                assertEquals("case $name: first edge max_split", want["max_split"]!!.jsonPrimitive.content.toInt(), decision.edges.first().maxSplit)
            }
            expected["traffic_shaping"]?.let {
                assertEquals("case $name: shaping", it.jsonPrimitive.content.toBoolean(), decision.trafficShaping)
            }
            expected["shaper_level"]?.let {
                assertEquals("case $name: shaper level", it.jsonPrimitive.content, decision.shaperLevel)
            }
            expected["threat_level"]?.let {
                assertEquals("case $name: threat level", it.jsonPrimitive.content, decision.threatLevel)
            }
            expected["is_champion"]?.let {
                assertEquals("case $name: champion", it.jsonPrimitive.content.toBoolean(), decision.isChampion)
            }
        }
    }

    @Test
    fun state_json_roundtrip() {
        var state = VorCoreKt.SelectorState()
        state = VorCoreKt.observe(
            state,
            VorCoreKt.Outcome(
                fingerprint = "fp",
                engine = "xray",
                strategy = "raw",
                success = true,
                rttMs = 20,
            ),
            nowEpochSeconds = 1789041600,
        )
        val text = state.toJson().toString()
        val back = VorCoreKt.SelectorState.fromJson(text)
        assertEquals(state, back)
        assertTrue(state.q.containsKey("fp"))
        assertEquals("xray|raw", state.q.getValue("fp").champion)
    }

    @Test
    fun carrier_lookup_matches_reference() {
        val mci = VorCoreKt.carrierByKeyOrAsn("AS41689")
        assertEquals("mci", mci?.key)
        assertEquals("www.speedtest.net", mci?.fakeSni)
        val irancell = VorCoreKt.carrierByKeyOrAsn("irancell")
        assertEquals("chatgpt.com", irancell?.fakeSni)
        assertEquals(null, VorCoreKt.carrierByKeyOrAsn("no-such-carrier"))
    }
}
