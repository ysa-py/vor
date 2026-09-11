package com.vor.licensemanager.issuer

import android.content.Context
import com.vor.license.LicenseStatus
import com.vor.license.LicenseVerifier
import com.vor.license.issuer.BatchCsv
import com.vor.license.issuer.CanonicalJson

/**
 * The issuance pipeline: validated form/CSV input -> canonical payload ->
 * vault-signed VOR1 token -> ledger entry. Pure enough to JVM-test the
 * validation and assembly, with signing behind an injectable signer.
 */
object IssueEngine {

    /** What the form produces after validation (all strings pre-trimmed). */
    data class IssueRequest(
        val id: String,
        val tier: String,
        val expiresAt: String,
        val platforms: List<String>,
        val notes: String,
    )

    /** A signed token plus its ledger record. */
    data class Issued(
        val token: String,
        val entry: IssuerStore.IssuedEntry,
    )

    sealed class IssueError(val message: String) {
        class BadId(detail: String) : IssueError(detail)
        class BadTier(detail: String) : IssueError(detail)
        class BadExpiry(detail: String) : IssueError(detail)
    }

    /** Validate one request; null when acceptable for signing. */
    fun validate(request: IssueRequest, nowEpochSeconds: Long): IssueError? {
        val id = request.id.trim()
        if (id.isEmpty()) return IssueError.BadId("the license id is empty")
        if (id.length > BatchCsv.MAX_ID_LENGTH) {
            return IssueError.BadId("the license id is longer than ${BatchCsv.MAX_ID_LENGTH} characters")
        }
        if (id.any { it.code in 0..0x1F }) {
            return IssueError.BadId("the license id contains control characters")
        }
        val tier = request.tier.trim()
        if (tier.isEmpty()) return IssueError.BadTier("the tier is empty")
        if (tier.length > BatchCsv.MAX_TIER_LENGTH) {
            return IssueError.BadTier("the tier is longer than ${BatchCsv.MAX_TIER_LENGTH} characters")
        }
        if (tier.any { it.code in 0..0x1F }) {
            return IssueError.BadTier("the tier contains control characters")
        }
        val expiryEpoch = com.vor.license.Rfc3339.parseEpoch(request.expiresAt)
            ?: return IssueError.BadExpiry("'${request.expiresAt}' is not a valid RFC 3339 date/time")
        // Refuse to issue a license that is already expired at signing time —
        // a deliberate foot-gun guard (the verifier itself would accept such
        // a token and immediately report it as EXPIRED).
        if (expiryEpoch <= nowEpochSeconds) {
            return IssueError.BadExpiry("the expiry must be in the future (it is already past)")
        }
        return null
    }

    /**
     * Issue one license and append it to the ledger. `signer` is normally
     * the vault's Keystore-gated signer; tests inject a software signer.
     */
    fun issue(
        context: Context,
        request: IssueRequest,
        signer: (ByteArray) -> ByteArray,
        nowEpochSeconds: Long = System.currentTimeMillis() / 1000,
        nowRfc3339: String = IssuerTime.nowRfc3339Utc(),
    ): Issued {
        val error = validate(request, nowEpochSeconds)
        require(error == null) { "issue() called with an invalid request: ${error?.message}" }
        val payload = CanonicalJson.IssuerPayload(
            id = request.id.trim(),
            issuedAt = nowRfc3339,
            expiresAt = request.expiresAt,
            tier = request.tier.trim(),
            platforms = request.platforms,
        )
        val token = CanonicalJson.issueToken(payload, signer)
        val entry = IssuerStore.IssuedEntry(
            id = request.id.trim(),
            tier = request.tier.trim(),
            expiresAt = request.expiresAt,
            issuedAt = nowRfc3339,
            token = token,
            notes = request.notes.trim(),
            createdAtMs = System.currentTimeMillis(),
        )
        IssuerStore.appendHistory(context, listOf(entry))
        return Issued(token, entry)
    }

    /** Issue a whole batch; stops at the first signer failure (auth window). */
    fun issueBatch(
        context: Context,
        rows: List<BatchCsv.Row>,
        platforms: List<String>,
        signer: (ByteArray) -> ByteArray,
        nowEpochSeconds: Long = System.currentTimeMillis() / 1000,
    ): List<Issued> = rows.map { row ->
        issue(
            context,
            IssueRequest(
                id = row.id,
                tier = row.tier,
                expiresAt = row.expiresAt,
                platforms = platforms,
                notes = "",
            ),
            signer,
            nowEpochSeconds,
        )
    }

    /**
     * Self-test: sign a canary payload with the installed key and verify it
     * BOTH with the key's own public key AND with the public key embedded
     * in this build. Returns a human-presentable verdict (honest by design:
     * "own key OK but embedded key mismatch" is a WARNING, not a failure).
     */
    fun selfTest(vault: IssuerVault, embeddedPublicKeyB64Url: String): SelfTestResult {
        val payload = CanonicalJson.IssuerPayload(
            id = "self-test-${System.currentTimeMillis()}",
            issuedAt = IssuerTime.nowRfc3339Utc(),
            expiresAt = IssuerTime.epochSecondToRfc3339(System.currentTimeMillis() / 1000 + 120),
            tier = "self-test",
            platforms = emptyList(),
        )
        val token = CanonicalJson.issueToken(payload) { message -> vault.sign(message) }
        val ownResult = LicenseVerifier.verify(vault.publicKeyB64Url() ?: "", token, System.currentTimeMillis() / 1000)
        val embeddedResult = LicenseVerifier.verify(embeddedPublicKeyB64Url, token, System.currentTimeMillis() / 1000)
        return when {
            ownResult.status != LicenseStatus.VALID ->
                SelfTestResult(false, true, "the issued token did not verify with this device's own public key")
            embeddedResult.status != LicenseStatus.VALID ->
                SelfTestResult(true, false, "token verifies with this device's key, but NOT with the public key embedded in this build — licenses issued here are only accepted by builds that embed the matching public key")
            else ->
                SelfTestResult(true, true, "token verifies locally and matches the public key embedded in this build")
        }
    }

    data class SelfTestResult(
        val signsAndVerifies: Boolean,
        val matchesEmbeddedKey: Boolean,
        val detail: String,
    )
}
