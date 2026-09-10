plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "com.vor.licensemanager"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.vor.licensemanager"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "1.0.0"

        // Same dev-key default + CI override as the main app.
        val licenseKey = (project.findProperty("vor.licensePublicKey") as? String)
            ?.takeIf { it.isNotBlank() }
            ?: "f54nNpWuth1MHZsbi6sdEODSDvWp7V6XSSDqWtmCyMA"
        buildConfigField("String", "VOR_LICENSE_PUBLIC_KEY", "\"$licenseKey\"")
        resourceConfigurations += listOf("en", "fa")
    }

    buildTypes {
        release {
            isMinifyEnabled = false
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

    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.lifecycle.runtime.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.ui)
    implementation(libs.androidx.ui.graphics)
    implementation(libs.androidx.ui.tooling.preview)
    implementation(libs.androidx.material3)
    debugImplementation(libs.androidx.ui.tooling)
}
