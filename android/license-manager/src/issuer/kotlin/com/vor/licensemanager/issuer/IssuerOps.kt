package com.vor.licensemanager.issuer

import android.content.Intent
import androidx.fragment.app.FragmentActivity

/**
 * All vault-touching operations in one place, each with the same
 * re-authentication ladder: run it; if the 60-second keyguard window has
 * elapsed (Android Keystore: UserNotAuthenticatedException), show the
 * BiometricPrompt gate again and retry once. Screens never talk to the
 * Keystore directly.
 */
class IssuerOps(
    private val activity: FragmentActivity,
    val vault: IssuerVault,
) {
    private fun <T> withAuthRetry(block: () -> T, onResult: (Result<T>) -> Unit) {
        try {
            onResult(Result.success(block()))
        } catch (e: IssuerVault.VaultException.NeedsAuthentication) {
            BiometricGate.authenticate(activity, onSuccess = {
                try {
                    onResult(Result.success(block()))
                } catch (retry: Exception) {
                    onResult(Result.failure(retry))
                }
            })
        } catch (e: Exception) {
            onResult(Result.failure(e))
        }
    }

    fun sign(request: IssueEngine.IssueRequest, onDone: (IssueEngine.Issued?) -> Unit) {
        withAuthRetry({
            IssueEngine.issue(activity, request, signer = { message -> vault.sign(message) })
        }) { result -> onDone(result.getOrNull()) }
    }

    fun signBatch(rows: List<com.vor.license.issuer.BatchCsv.Row>, onDone: (List<IssueEngine.Issued>?) -> Unit) {
        withAuthRetry({
            IssueEngine.issueBatch(activity, rows, PLATFORMS, signer = { message -> vault.sign(message) })
        }) { result -> onDone(result.getOrNull()) }
    }

    fun generateKey(onDone: (Boolean) -> Unit) {
        withAuthRetry({ vault.generateKey() }) { onDone(it.isSuccess) }
    }

    fun importSeed(seed: ByteArray, source: String, onDone: (Boolean) -> Unit) {
        withAuthRetry({ vault.importSeed(seed, source) }) { onDone(it.isSuccess) }
    }

    fun restoreBackup(text: String, passphrase: CharArray, onDone: (Boolean) -> Unit) {
        withAuthRetry({
            val seed = com.vor.license.issuer.KeyBackup.decrypt(text, passphrase)
            vault.importSeed(seed, "restored")
        }) { onDone(it.isSuccess) }
    }

    fun backupTo(target: android.net.Uri, passphrase: CharArray, onDone: (Boolean) -> Unit) {
        withAuthRetry({ vault.backupEnvelope(passphrase) }) { result ->
            val envelope = result.getOrNull()
            if (envelope == null) {
                onDone(false)
            } else {
                runCatching {
                    activity.contentResolver.openOutputStream(target)?.use { output ->
                        output.write(envelope.toByteArray(Charsets.UTF_8))
                    } ?: return@runCatching false
                    true
                }.getOrDefault(false).let(onDone)
            }
        }
    }

    fun selfTest(onDone: (IssueEngine.SelfTestResult?) -> Unit) {
        withAuthRetry({
            IssueEngine.selfTest(vault, com.vor.licensemanager.BuildConfig.VOR_LICENSE_PUBLIC_KEY)
        }) { onDone(it.getOrNull()) }
    }

    fun shareToken(token: String) {
        val send = Intent(Intent.ACTION_SEND).apply {
            type = "text/plain"
            putExtra(Intent.EXTRA_TEXT, token)
        }
        activity.startActivity(Intent.createChooser(send, null))
    }
}
