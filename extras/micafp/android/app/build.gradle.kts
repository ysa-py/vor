plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.unifiedshield"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.unifiedshield"
        minSdk = 21
        targetSdk = 35
        versionCode = 1
        versionName = "1.0.0"

        ndk {
            abiFilters += listOf("armeabi-v7a", "arm64-v8a", "x86_64")
        }

        sourceSets {
            getByName("main") {
                jniLibs.srcDirs("src/main/jniLibs")
            }
        }

//        externalNativeBuild {
//            cmake {
//                cppFlags += ""
//                arguments += listOf("-DANDROID_STL=c++_shared")
//            }
//        }
    }

    signingConfigs {
        create("debugConfig") {
            storeFile = file("${rootDir}/debug.keystore")
            storePassword = "android"
            keyAlias = "androiddebugkey"
            keyPassword = "android"
        }
    }

    buildTypes {
        release {
            // NOTE (MICAFP delivery): R8 minification is temporarily disabled —
            // the build host (2 vCPU / 4GB RAM) could not finish R8 within the
            // tool time ceiling. Everything else (release build type, signing,
            // proguard rules file) stays configured for later re-enable.
            isMinifyEnabled = false
            isShrinkResources = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
            // MICAFP delivery agreement: release APK signed with the debug key
            // so the product owner can install and test immediately.
            signingConfig = signingConfigs.getByName("debugConfig")
        }
        debug {
            isDebuggable = true
            isMinifyEnabled = false
            signingConfig = signingConfigs.getByName("debugConfig")
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
    }

    composeOptions {
        kotlinCompilerExtensionVersion = "1.5.14"
    }

//    externalNativeBuild {
//        cmake {
//            path = file("src/main/cpp/CMakeLists.txt")
//            version = "3.22.1"
//        }
//    }
}

dependencies {
    // Core Android
    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("com.google.android.material:material:1.12.0")

    // Lifecycle
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.8.3")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.3")
    implementation("androidx.lifecycle:lifecycle-service:2.8.3")

    // Coroutines
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.8.1")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.8.1")

    // WorkManager
    implementation("androidx.work:work-runtime-ktx:2.9.0")

    // Jetpack Compose
    implementation(platform("androidx.compose:compose-bom:2024.06.00"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-graphics")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-extended")
    implementation("androidx.activity:activity-compose:1.9.0")
    implementation("androidx.navigation:navigation-compose:2.7.7")

    // Networking
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    implementation("com.google.code.gson:gson:2.11.0")

    // Security
    implementation("androidx.security:security-crypto:1.1.0-alpha06")
    // MICAFP Directive v6 / C1 — Ed25519 license signature verification (Google Tink, vetted, API 21+)
    implementation("com.google.crypto.tink:tink-android:1.12.0")

    debugImplementation("androidx.compose.ui:ui-tooling")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
}

// Custom task that ensures all .so files from compiled Rust targets are copied/verified in jniLibs
tasks.register("verifyAndPackageNativeLibs") {
    description = "Verifies and packages compiled Rust/C++ .so libraries for all target ABIs into src/main/jniLibs"
    group = "build"

    doLast {
        val jniLibsDir = file("${projectDir}/src/main/jniLibs")
        val expectedAbis = listOf("arm64-v8a", "armeabi-v7a", "x86_64")
        val rustTargetDirs = listOf(
            file("${rootDir}/target"),
            file("${rootDir}/rust/target"),
            file("${projectDir}/target")
        )

        expectedAbis.forEach { abi ->
            val abiDir = File(jniLibsDir, abi)
            if (!abiDir.exists()) {
                abiDir.mkdirs()
            }

            // Map Rust target triples to Android ABIs
            val rustTriple = when (abi) {
                "arm64-v8a" -> "aarch64-linux-android"
                "armeabi-v7a" -> "armv7-linux-androideabi"
                "x86_64" -> "x86_64-linux-android"
                else -> null
            }

            if (rustTriple != null) {
                rustTargetDirs.forEach { baseTargetDir ->
                    val releaseDir = File(baseTargetDir, "$rustTriple/release")
                    if (releaseDir.exists()) {
                        releaseDir.listFiles { _, name -> name.endsWith(".so") }?.forEach { soFile ->
                            val destFile = File(abiDir, soFile.name)
                            soFile.copyTo(destFile, overwrite = true)
                            logger.lifecycle("Copied native library ${soFile.name} from $rustTriple to jniLibs/$abi/")
                        }
                    }
                }
            }
        }
        logger.lifecycle("Verified Native jniLibs directory structure for ABIs: $expectedAbis")
    }
}

tasks.named("preBuild") {
    dependsOn("verifyAndPackageNativeLibs")
}
