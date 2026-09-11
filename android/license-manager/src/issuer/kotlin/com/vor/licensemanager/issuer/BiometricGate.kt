package com.vor.licensemanager.issuer

import androidx.fragment.app.FragmentActivity
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt

/**
 * The lock-screen gate shown on every app open (and re-shown after every
 * backgrounding). Accepts STRONG biometrics OR the device PIN/pattern/
 * password — whatever the maintainer uses — because the requirement is
 * proof of device control, not a specific finger.
 *
 * Note on authenticator selection: on API 30+ BIOMETRIC_STRONG may be
 * combined with DEVICE_CREDENTIAL; below 30 the supported pair is
 * BIOMETRIC_WEAK | DEVICE_CREDENTIAL (androidx.biometric then routes
 * through the framework keyguard prompt, which offers both anyway).
 */
object BiometricGate {

    fun canAuthenticate(activity: FragmentActivity): Boolean {
        val managers = BiometricManager.from(activity).canAuthenticate(allowedAuthenticators())
        return managers == BiometricManager.BIOMETRIC_SUCCESS
    }

    fun authenticate(activity: FragmentActivity, onSuccess: () -> Unit, onError: (String) -> Unit = {}) {
        val prompt = BiometricPrompt(
            activity,
            androidx.core.content.ContextCompat.getMainExecutor(activity),
            object : BiometricPrompt.AuthenticationCallback() {
                override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                    onSuccess()
                }

                override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                    // Error codes 5 (cancelled) and 13 (negative button) are the
                    // user backing out — not a failure worth alarming anyone.
                    if (errorCode != BiometricPrompt.ERROR_NEGATIVE_BUTTON &&
                        errorCode != BiometricPrompt.ERROR_USER_CANCELED
                    ) {
                        onError(errString.toString())
                    }
                }
            },
        )
        prompt.authenticate(promptInfo(activity))
    }

    private fun promptInfo(activity: FragmentActivity): BiometricPrompt.PromptInfo {
        val builder = BiometricPrompt.PromptInfo.Builder()
            .setTitle(activity.getString(com.vor.licensemanager.R.string.issuer_gate_title))
            .setSubtitle(activity.getString(com.vor.licensemanager.R.string.issuer_gate_subtitle))
            .setAllowedAuthenticators(allowedAuthenticators())
        return builder.build()
    }

    private fun allowedAuthenticators(): Int =
        if (android.os.Build.VERSION.SDK_INT >= 30) {
            BiometricManager.Authenticators.BIOMETRIC_STRONG or BiometricManager.Authenticators.DEVICE_CREDENTIAL
        } else {
            BiometricManager.Authenticators.BIOMETRIC_WEAK or BiometricManager.Authenticators.DEVICE_CREDENTIAL
        }
}
