package com.vor.license

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Test

/**
 * Opt-in LIVE end-to-end verification of a real, freshly issued license.
 *
 * Run locally (never in CI — no production tokens belong in a repo):
 *
 *   VOR_E2E_TOKEN="$(cat vor-license.txt)" \
 *   VOR_E2E_PUBLIC_KEY="$(cat license/keys/prod/VOR_LICENSE_PUBLIC_KEY.txt)" \
 *   ./gradlew :core-license:test --tests 'com.vor.license.LiveIssuedLicenseTest'
 *
 * The token is issued by the GitHub "Issue License" workflow with the
 * production Ed25519 key; this test replays the EXACT verification path a
 * release build of the app executes on a user's phone (LicenseGateScreen ->
 * LicenseRepository -> LicenseVerifier.verify). A pass proves the whole
 * chain: secret -> workflow signing -> token -> offline client verify.
 *
 * Skips cleanly when the env vars are absent.
 */
class LiveIssuedLicenseTest {

    private fun env(name: String): String? = System.getenv(name)?.takeIf { it.isNotBlank() }

    @Test
    fun freshly_issued_prod_token_verifies_as_VALID() {
        val token = env("VOR_E2E_TOKEN")
        val publicKey = env("VOR_E2E_PUBLIC_KEY")
        assumeTrue(
            "VOR_E2E_TOKEN / VOR_E2E_PUBLIC_KEY not set — skipping live E2E (opt-in, local only)",
            token != null && publicKey != null,
        )

        val now = System.currentTimeMillis() / 1000
        val result = LicenseVerifier.verify(publicKey!!, token!!, now)

        assertEquals(
            "freshly issued prod license must verify VALID (status=${result.status})",
            LicenseStatus.VALID,
            result.status,
        )
        val payload = result.payload
        assertTrue("payload must decode", payload != null)
        assertEquals("vor", payload!!.product)
        assertTrue("expiry must be in the future", payload.expiresAt.isNotEmpty())
    }

    @Test
    fun live_token_is_rejected_by_the_wrong_public_key() {
        val token = env("VOR_E2E_TOKEN")
        assumeTrue("VOR_E2E_TOKEN not set — skipping", token != null)

        // The committed DEV key must NOT accept a production-signed token.
        val devKey = java.io.File("src/test/resources/VOR_LICENSE_PUBLIC_KEY.txt")
        assumeTrue("dev key resource not found (run from :core-license module)", devKey.exists())
        val now = System.currentTimeMillis() / 1000
        val result = LicenseVerifier.verify(devKey.readText().trim(), token!!, now)
        assertEquals(
            "prod token signed by prod key must fail against the dev key",
            LicenseStatus.INVALID,
            result.status,
        )
    }
}
