plugins {
    kotlin("jvm")
    alias(libs.plugins.kotlin.serialization)
}

java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
}

dependencies {
    // The issuer shares the exact same verification core (Base64Url codec,
    // Rfc3339 parser, LicenseVerifier for the built-in self-test) as every
    // Vor client. Dependency direction is one-way: issuer -> verifier code
    // is fine; the verifier build must NEVER depend on this module (that
    // invariant is enforced by the verifyVerifierPurity build gate).
    implementation(project(":core-license"))
    // Ed25519 (BC lightweight API — the same provider :core-license verifies
    // with) plus Argon2id for the passphrase-protected backup envelope.
    // No NEW cryptographic dependency: bcprov is already shipped by every
    // Vor client for offline Ed25519 verification.
    implementation(libs.bouncycastle.bcprov)
    implementation(libs.kotlinx.serialization.json)

    testImplementation(libs.junit)
}

// Pure Kotlin on purpose (mirrors :core-license): the signing engine, the
// canonical serialization, the Argon2id/AES-GCM backup envelope and the
// batch CSV parser are all plain JVM code so they are unit-tested on the
// desktop against golden vectors produced by the repository's own Python
// reference implementation (license/python/vor_license.py).
