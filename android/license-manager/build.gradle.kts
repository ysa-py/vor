import java.util.zip.ZipFile

import java.util.Properties

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.kotlin.serialization)
}

// Same release-signing wiring as :app — keystore.properties is written by
// CI when the KEYSTORE_* secrets are present. Without it the license-manager
// APK ships UNSIGNED, which the Android package installer refuses to install.
val keystoreProps = Properties().apply {
    val f = rootProject.file("keystore.properties")
    if (f.exists()) f.inputStream().use { load(it) }
}

android {
    namespace = "com.vor.licensemanager"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.vor.licensemanager"
        minSdk = 26
        targetSdk = 35
        // v1.1.0 (105): on-device offline ISSUER build variant added — the
        // public verifier build is unchanged in behavior, byte-for-byte the
        // same code paths as 1.0.4.
        // v1.2.0 (106): release-train alignment with the main app.
        versionCode = 109
        versionName = "1.5.0"

        // Same dev-key default + CI override as the main app.
        val licenseKey = (project.findProperty("vor.licensePublicKey") as? String)
            ?.takeIf { it.isNotBlank() }
            ?: "f54nNpWuth1MHZsbi6sdEODSDvWp7V6XSSDqWtmCyMA"
        buildConfigField("String", "VOR_LICENSE_PUBLIC_KEY", "\"$licenseKey\"")
    }

    androidResources {
        // Ship English + Persian; strip every other locale (incl. library ones).
        localeFilters += setOf("en", "fa")
    }

    // Two capabilities from ONE codebase (license/SPEC.md "On-device
    // issuer"). The PUBLIC build is `verifier` — it must NEVER contain issuer
    // code, issuer resources, or the biometric gate (enforced structurally
    // by the verifyVerifierPurity task below). `issuer` is the maintainer's
    // private on-device signing app; it additionally carries the verifier
    // screens, so a maintainer can check exactly what a license holder sees.
    flavorDimensions += "capability"
    productFlavors {
        create("verifier") {
            dimension = "capability"
            isDefault = true
        }
        create("issuer") {
            dimension = "capability"
            // Coexists with the installed public app for side-by-side testing.
            applicationIdSuffix = ".issuer"
            versionNameSuffix = "-issuer"
        }
    }

    signingConfigs {
        create("release") {
            if (keystoreProps.isNotEmpty()) {
                storeFile = rootProject.file(keystoreProps.getProperty("storeFile"))
                storePassword = keystoreProps.getProperty("storePassword")
                keyAlias = keystoreProps.getProperty("keyAlias")
                keyPassword = keystoreProps.getProperty("keyPassword")
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            signingConfig = if (keystoreProps.isNotEmpty()) {
                signingConfigs.getByName("release")
            } else {
                null
            }
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
    buildFeatures {
        compose = true
        buildConfig = true
    }
    packaging {
        resources {
            // BouncyCastle jars ship duplicate META-INF license files.
            excludes += "/META-INF/{AL2.0,LGPL2.1}"
            excludes += "/META-INF/LICENSE.md"
            excludes += "/META-INF/NOTICE.md"
            excludes += "/META-INF/versions/9/OSGI-INF/MANIFEST.MF"
        }
    }
}

dependencies {
    // Shares the exact same verification code as the main app.
    implementation(project(":core-license"))

    // Multi-license store (offline JSON in SharedPreferences — no network, no Room)
    implementation(libs.kotlinx.serialization.json)
    // QR import/export — pure-Java zxing core: zero permissions, zero network.
    // QR import reads from the system photo picker (no storage permission needed);
    // QR export renders the token as an on-screen bitmap.
    implementation(libs.zxing.core)

    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.lifecycle.runtime.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.ui)
    implementation(libs.androidx.ui.graphics)
    implementation(libs.androidx.ui.tooling.preview)
    implementation(libs.androidx.material3)
    debugImplementation(libs.androidx.ui.tooling)

    // ---- ISSUER-ONLY dependencies (never in the public verifier APK) ----
    // BiometricPrompt / device-lock gate + the pure-JVM signing engine
    // (canonical serialization, Ed25519 signing, Argon2id key backup,
    // batch CSV). Scoped to the issuer flavor on purpose: the verifier
    // variant cannot even compile against them, and the dex marker scan
    // (verifyVerifierPurity) turns any accidental leak into a build
    // failure.
    // String-form accessors: flavor configurations exist only after the
    // android {} block executes, so the typed Kotlin-DSL accessors
    // (issuerImplementation) are not generated for this script.
    "issuerImplementation"(project(":core-license-issuer"))
    "issuerImplementation"(libs.androidx.biometric)

    testImplementation(libs.junit)
}

// ---------------------------------------------------------------------------
// Verifier purity gate — the public APK must NEVER contain issuer code
// paths, issuer resources, or issuer dependencies.
//
// Scans every built verifier APK (debug + release) for issuer markers in
// dex files, resources.arsc and assets: our two issuer packages, the issuer
// Keystore alias / seed file / prefs file names, the androidx.biometric
// dependency, and issuer-only UI strings. Markers were chosen by scanning a
// real 1.0.4 baseline APK: BouncyCastle as a whole (including its unused
// Ed25519PrivateKeyParameters and Argon2BytesGenerator classes) is already
// shipped by the VERIFIER for offline verification — an unused class is
// not an issuer code path — so only OUR issuer code is flagged.
// ---------------------------------------------------------------------------
val verifyVerifierPurity by tasks.registering {
    group = "verification"
    description = "Fails the build if any verifier APK contains issuer code paths, resources or dependencies."
    doLast {
        val apkRoot = layout.buildDirectory.dir("outputs/apk/verifier").get().asFile
        val apks = apkRoot.walkTopDown().filter { it.isFile && it.extension == "apk" }.toList()
        check(apks.isNotEmpty()) {
            "verifyVerifierPurity: no verifier APK under ${apkRoot.absolutePath} — run :license-manager:assembleVerifierDebug first"
        }
        val markers = mapOf(
            "com/vor/licensemanager/issuer" to "issuer UI code (src/issuer)",
            "com/vor/license/issuer" to "issuer signing engine (:core-license-issuer)",
            "vor_issuer_wrap" to "issuer Android-Keystore key alias",
            "vor_issuer_seed" to "issuer wrapped-seed file name",
            "vor_issuer_store" to "issuer history/tiers store",
            "androidx/biometric" to "androidx.biometric dependency",
            "Issued by this device" to "issuer-only UI string",
            "صادرشده توسط این دستگاه" to "issuer-only UI string (fa)",
        )
        var violations = 0
        apks.forEach { apk ->
            ZipFile(apk).use { zip ->
                val scanned = zip.entries().asSequence()
                    .filter { entry ->
                        entry.name.endsWith(".dex") || entry.name == "resources.arsc" ||
                            entry.name.startsWith("assets/") || entry.name.endsWith(".xml")
                    }
                for (entry in scanned) {
                    val bytes = zip.getInputStream(entry).readBytes()
                    for ((marker, meaning) in markers) {
                        if (bytes.indexOfSub(marker.toByteArray(Charsets.UTF_8)) >= 0) {
                            violations++
                            logger.error(
                                "verifyVerifierPurity: VIOLATION in {} -> {}: '{}' present in {}",
                                apk.name, meaning, marker, entry.name,
                            )
                        }
                    }
                }
            }
        }
        check(violations == 0) {
            "verifyVerifierPurity: $violations issuer marker(s) found in the PUBLIC verifier APK. " +
                "Issuer code must stay in src/issuer + :core-license-issuer and only ever be wired " +
                "through issuerImplementation / issuer source sets."
        }
        logger.lifecycle("verifyVerifierPurity: {} verifier APK(s) clean of issuer markers", apks.size)
    }
}

// The gate runs whenever a verifier APK is assembled (covers CI's
// assembleDebug/assembleRelease and local builds alike).
tasks.matching { it.name == "assembleVerifierDebug" || it.name == "assembleVerifierRelease" }.configureEach {
    finalizedBy(verifyVerifierPurity)
}

/** First index of [needle] inside [haystack] (binary substring search), or -1. */
private fun ByteArray.indexOfSub(needle: ByteArray): Int {
    if (needle.isEmpty()) return 0
    if (size < needle.size) return -1
    outer@ for (i in 0..size - needle.size) {
        for (j in needle.indices) {
            if (this[i + j] != needle[j]) continue@outer
        }
        return i
    }
    return -1
}
