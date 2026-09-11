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
        versionCode = 104
        versionName = "1.0.4"

        // Same dev-key default + CI override as the main app.
        val licenseKey = (project.findProperty("vor.licensePublicKey") as? String)
            ?.takeIf { it.isNotBlank() }
            ?: "f54nNpWuth1MHZsbi6sdEODSDvWp7V6XSSDqWtmCyMA"
        buildConfigField("String", "VOR_LICENSE_PUBLIC_KEY", "\"$licenseKey\"")
        resourceConfigurations += listOf("en", "fa")
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

    testImplementation(libs.junit)
}
