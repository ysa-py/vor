# MICAFP — R8 keep rules (release minification)
# Gson reflection models (license payload, AI provider config/registry, audit records)
-keep class com.unifiedshield.license.MicafpLicensePayload { *; }
-keep class com.unifiedshield.license.AuditRecord { *; }
-keep class com.unifiedshield.aiproviders.AiProviderUserConfig { *; }
-keep class com.unifiedshield.aiproviders.AiProviderDefinition { *; }
-keep class com.unifiedshield.aiproviders.AiProviderRegistryFile { *; }
-keep class com.unifiedshield.tunnel.** { *; }
-keepattributes Signature
-keepattributes *Annotation*
-dontwarn org.bouncycastle.**
-dontwarn org.conscrypt.**
-dontwarn org.openjsse.**
# Google Tink
-keep class com.google.crypto.tink.** { *; }
# OkHttp platform warnings
-dontwarn okhttp3.internal.platform.**
-dontwarn org.codehaus.mojo.animal_sniffer.*
