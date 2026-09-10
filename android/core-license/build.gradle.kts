plugins {
    kotlin("jvm")
}

java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
}

dependencies {
    // Ed25519 verification on every platform Vor ships. BouncyCastle is
    // already a dependency of the main app (MITM CA machinery), so this adds
    // no new dependency footprint to the APK.
    implementation(libs.bouncycastle.bcprov)
    implementation(libs.bouncycastle.bcpkix)
    implementation(libs.kotlinx.serialization.json)

    testImplementation(libs.junit)
}

// The license module is pure Kotlin (no Android dependencies) so the main
// app and the License Manager companion app share the exact same
// verification code — literally the same compiled classes.
