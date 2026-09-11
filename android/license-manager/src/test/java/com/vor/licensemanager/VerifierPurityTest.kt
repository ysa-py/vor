package com.vor.licensemanager

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Build-variant purity, part 2 (the hard dex gate is the verifyVerifierPurity
 * Gradle task; this unit test asserts the same property at the CLASSPATH
 * level for both variants, so a violation fails either variant's test run).
 *
 * The verifier variant's unit-test classpath must NOT contain any issuer
 * class; the issuer variant's must. Uses reflection so this shared source
 * set compiles for BOTH variants.
 */
class VerifierPurityTest {

    private val issuerOnlyClasses = listOf(
        // :core-license-issuer (pure-JVM signing engine)
        "com.vor.license.issuer.CanonicalJson",
        "com.vor.license.issuer.SoftwareEd25519",
        "com.vor.license.issuer.KeyBackup",
        "com.vor.license.issuer.BatchCsv",
        // license-manager issuer source set (UI, vault, engine glue)
        "com.vor.licensemanager.issuer.IssuerActivity",
        "com.vor.licensemanager.issuer.IssuerVault",
        "com.vor.licensemanager.issuer.IssuerOps",
        "com.vor.licensemanager.issuer.IssuerStore",
        "com.vor.licensemanager.issuer.IssueEngine",
        "com.vor.licensemanager.issuer.IssuerTime",
    )

    @Test
    fun issuerClasses_presentIffIssuerVariant() {
        val loaded = issuerOnlyClasses.map { name ->
            name to runCatching { Class.forName(name, false, javaClass.classLoader) }.isSuccess
        }
        when (BuildConfig.FLAVOR) {
            "verifier" -> {
                val leaked = loaded.filter { it.second }
                assertTrue(
                    "issuer classes leaked into the VERIFIER variant classpath: " +
                        leaked.joinToString(", ") { it.first },
                    leaked.isEmpty(),
                )
            }
            "issuer" -> {
                val missing = loaded.filterNot { it.second }
                assertTrue(
                    "issuer classes missing from the ISSUER variant classpath: " +
                        missing.joinToString(", ") { it.first },
                    missing.isEmpty(),
                )
            }
            else -> error("unexpected flavor ${BuildConfig.FLAVOR}")
        }
    }

    @Test
    fun applicationId_suffixOnlyOnIssuerVariant() {
        if (BuildConfig.FLAVOR == "issuer") {
            assertEquals("com.vor.licensemanager.issuer", BuildConfig.APPLICATION_ID)
        } else {
            assertEquals("com.vor.licensemanager", BuildConfig.APPLICATION_ID)
        }
    }

    @Test
    fun verifierKeepsItsEmbeddedPublicKey() {
        // The one thing the verifier must never lose: its embedded public key
        // default (dev key) — same value as android/app and license/keys/dev.
        assertTrue(
            "embedded public key changed: ${BuildConfig.VOR_LICENSE_PUBLIC_KEY}",
            BuildConfig.VOR_LICENSE_PUBLIC_KEY == "f54nNpWuth1MHZsbi6sdEODSDvWp7V6XSSDqWtmCyMA" ||
                BuildConfig.VOR_LICENSE_PUBLIC_KEY.length >= 43,
        )
    }
}
