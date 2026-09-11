pluginManagement {
    repositories {
        google {
            content {
                includeGroupByRegex("com\\.android.*")
                includeGroupByRegex("com\\.google.*")
                includeGroupByRegex("androidx.*")
            }
        }
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "Vor"
include(":app")
include(":core-license")
// Issuer-only pure-JVM module (canonical serialization, Ed25519 signing,
// Argon2id/AES-GCM key backup, batch CSV). NEVER added to the public
// verifier APK — enforced by the verifyVerifierPurity gate on :license-manager.
include(":core-license-issuer")
include(":license-manager")
