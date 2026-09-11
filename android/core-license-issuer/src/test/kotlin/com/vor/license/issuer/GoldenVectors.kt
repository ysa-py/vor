package com.vor.license.issuer

/**
 * Golden vectors pinning the on-device issuer to the repository's reference
 * implementation (license/python/vor_license.py) and to audited
 * cross-implementation references (OpenSSL-backed `cryptography`,
 * argon2-cffi).
 *
 * How these were produced (2026-09-12, reproducible): fixed seed (the
 * committed DEV keypair in license/keys/dev — a test key, NOT production),
 * fixed `issued_at`, fixed payloads; tokens/canonical bytes emitted by the
 * Python reference tool; RFC 8032 vectors independently confirmed against
 * OpenSSL; Argon2id digests from argon2-cffi. Any divergence in the Kotlin
 * port changes a byte somewhere and fails these tests.
 */
internal object GoldenVectors {

    const val DEV_PUBLIC_KEY_B64URL = "f54nNpWuth1MHZsbi6sdEODSDvWp7V6XSSDqWtmCyMA"
    const val DEV_SEED_B64URL = "pZ3UV5OaWXMkuxdm2IycwBLjilUBK_vB7tZLRlOzd08"

    const val FIXED_ISSUED_AT = "2026-09-12T00:00:00Z"

    data class ParityCase(
        val name: String,
        val id: String,
        val tier: String,
        val platforms: List<String>,
        val expiresAt: String,
        val extraEntitlements: Map<String, CanonicalJson.EntitlementValue>,
        val expectedToken: String,
        /** Standard (padded) base64 of the canonical payload bytes. */
        val expectedCanonicalBase64: String,
    )

    val TOKEN_PARITY: List<ParityCase> = listOf(
        ParityCase(
            "standard_ascii",
            "0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0", "standard",
            listOf("android", "windows", "linux", "openwrt", "ios"), "2027-09-12T00:00:00Z",
            emptyMap(),
            "VOR1.eyJ2IjoxLCJpZCI6IjBmMWUyZDNjLTRiNWEtNjk3OC04Nzk2LWE1YjRjM2QyZTFmMCIsInByb2R1Y3QiOiJ2b3IiLCJpc3N1ZWRfYXQiOiIyMDI2LTA5LTEyVDAwOjAwOjAwWiIsImV4cGlyZXNfYXQiOiIyMDI3LTA5LTEyVDAwOjAwOjAwWiIsImVudGl0bGVtZW50cyI6eyJ0aWVyIjoic3RhbmRhcmQiLCJwbGF0Zm9ybXMiOlsiYW5kcm9pZCIsIndpbmRvd3MiLCJsaW51eCIsIm9wZW53cnQiLCJpb3MiXX19.obMamT-x4-c_qX-s3ITygMZQ4VacKwc8n4SVU_YI3tZSroJ-39uuye_XiF7kYN5mNBVZSBRi3uRs81kUK5ErDw",
            "eyJ2IjoxLCJpZCI6IjBmMWUyZDNjLTRiNWEtNjk3OC04Nzk2LWE1YjRjM2QyZTFmMCIsInByb2R1Y3QiOiJ2b3IiLCJpc3N1ZWRfYXQiOiIyMDI2LTA5LTEyVDAwOjAwOjAwWiIsImV4cGlyZXNfYXQiOiIyMDI3LTA5LTEyVDAwOjAwOjAwWiIsImVudGl0bGVtZW50cyI6eyJ0aWVyIjoic3RhbmRhcmQiLCJwbGF0Zm9ybXMiOlsiYW5kcm9pZCIsIndpbmRvd3MiLCJsaW51eCIsIm9wZW53cnQiLCJpb3MiXX19",
        ),
        ParityCase(
            "persian_id",
            "کاربر-تهران-۰۰۱", "standard",
            listOf("android"), "2026-12-31T23:59:59Z",
            emptyMap(),
            "VOR1.eyJ2IjoxLCJpZCI6Itqp2KfYsdio2LEt2KrZh9ix2KfZhi3bsNuw27EiLCJwcm9kdWN0Ijoidm9yIiwiaXNzdWVkX2F0IjoiMjAyNi0wOS0xMlQwMDowMDowMFoiLCJleHBpcmVzX2F0IjoiMjAyNi0xMi0zMVQyMzo1OTo1OVoiLCJlbnRpdGxlbWVudHMiOnsidGllciI6InN0YW5kYXJkIiwicGxhdGZvcm1zIjpbImFuZHJvaWQiXX19.-ClaSrwoKLfKsCyO2613I-La-eVkRExsECFEiKX1TDgssN-j8Dq4KYQ93RfpmoCPPwz1iPozRwJx3AdA5OyhAw",
            "eyJ2IjoxLCJpZCI6Itqp2KfYsdio2LEt2KrZh9ix2KfZhi3bsNuw27EiLCJwcm9kdWN0Ijoidm9yIiwiaXNzdWVkX2F0IjoiMjAyNi0wOS0xMlQwMDowMDowMFoiLCJleHBpcmVzX2F0IjoiMjAyNi0xMi0zMVQyMzo1OTo1OVoiLCJlbnRpdGxlbWVudHMiOnsidGllciI6InN0YW5kYXJkIiwicGxhdGZvcm1zIjpbImFuZHJvaWQiXX19",
        ),
        ParityCase(
            "tier_special_chars",
            "weird \"tier\" \\ back /slash", "pro",
            listOf("linux", "openwrt"), "2028-02-29T12:34:56Z",
            emptyMap(),
            "VOR1.eyJ2IjoxLCJpZCI6IndlaXJkIFwidGllclwiIFxcIGJhY2sgL3NsYXNoIiwicHJvZHVjdCI6InZvciIsImlzc3VlZF9hdCI6IjIwMjYtMDktMTJUMDA6MDA6MDBaIiwiZXhwaXJlc19hdCI6IjIwMjgtMDItMjlUMTI6MzQ6NTZaIiwiZW50aXRsZW1lbnRzIjp7InRpZXIiOiJwcm8iLCJwbGF0Zm9ybXMiOlsibGludXgiLCJvcGVud3J0Il19fQ.gV24wGKOt0vBpnO2Sj0kL-ZHBTM1Mi_kPJZgCo-wYP8asuxjcGbZdbNc2x6JHJH4LHp12ekA9dHKNjPKiEHaBA",
            "eyJ2IjoxLCJpZCI6IndlaXJkIFwidGllclwiIFxcIGJhY2sgL3NsYXNoIiwicHJvZHVjdCI6InZvciIsImlzc3VlZF9hdCI6IjIwMjYtMDktMTJUMDA6MDA6MDBaIiwiZXhwaXJlc19hdCI6IjIwMjgtMDItMjlUMTI6MzQ6NTZaIiwiZW50aXRsZW1lbnRzIjp7InRpZXIiOiJwcm8iLCJwbGF0Zm9ybXMiOlsibGludXgiLCJvcGVud3J0Il19fQ==",
        ),
        ParityCase(
            "minimal_platforms",
            "11111111-2222-3333-4444-555555555555", "trial",
            emptyList(), "2026-10-01T00:00:00Z",
            emptyMap(),
            "VOR1.eyJ2IjoxLCJpZCI6IjExMTExMTExLTIyMjItMzMzMy00NDQ0LTU1NTU1NTU1NTU1NSIsInByb2R1Y3QiOiJ2b3IiLCJpc3N1ZWRfYXQiOiIyMDI2LTA5LTEyVDAwOjAwOjAwWiIsImV4cGlyZXNfYXQiOiIyMDI2LTEwLTAxVDAwOjAwOjAwWiIsImVudGl0bGVtZW50cyI6eyJ0aWVyIjoidHJpYWwiLCJwbGF0Zm9ybXMiOltdfX0.5WiwNM8A6X4X2vaHh4Zq5qAOQRlV8T-2xH1Ys0wr8_3FGXptOdLFWX-WScHOKHInm_Ho0l5yRU2piof1dz7nDw",
            "eyJ2IjoxLCJpZCI6IjExMTExMTExLTIyMjItMzMzMy00NDQ0LTU1NTU1NTU1NTU1NSIsInByb2R1Y3QiOiJ2b3IiLCJpc3N1ZWRfYXQiOiIyMDI2LTA5LTEyVDAwOjAwOjAwWiIsImV4cGlyZXNfYXQiOiIyMDI2LTEwLTAxVDAwOjAwOjAwWiIsImVudGl0bGVtZW50cyI6eyJ0aWVyIjoidHJpYWwiLCJwbGF0Zm9ybXMiOltdfX0=",
        ),
        ParityCase(
            "extras_sorted",
            "extras-test-001", "standard",
            listOf("android"), "2027-01-01T00:00:00Z",
            mapOf(
                "zzz" to CanonicalJson.EntitlementValue.Text("last"),
                "aaa" to CanonicalJson.EntitlementValue.Text("first"),
                "notes" to CanonicalJson.EntitlementValue.Text("x"),
            ),
            "VOR1.eyJ2IjoxLCJpZCI6ImV4dHJhcy10ZXN0LTAwMSIsInByb2R1Y3QiOiJ2b3IiLCJpc3N1ZWRfYXQiOiIyMDI2LTA5LTEyVDAwOjAwOjAwWiIsImV4cGlyZXNfYXQiOiIyMDI3LTAxLTAxVDAwOjAwOjAwWiIsImVudGl0bGVtZW50cyI6eyJ0aWVyIjoic3RhbmRhcmQiLCJwbGF0Zm9ybXMiOlsiYW5kcm9pZCJdLCJhYWEiOiJmaXJzdCIsIm5vdGVzIjoieCIsInp6eiI6Imxhc3QifX0.kNkaHe5ja0uzbIvL9NvXZ43py0MFKLrdkhilujJGkX-7vJAatU6TArhPVF6AIhmL4-3cFsd-xBozJ8LuhzZEAg",
            "eyJ2IjoxLCJpZCI6ImV4dHJhcy10ZXN0LTAwMSIsInByb2R1Y3QiOiJ2b3IiLCJpc3N1ZWRfYXQiOiIyMDI2LTA5LTEyVDAwOjAwOjAwWiIsImV4cGlyZXNfYXQiOiIyMDI3LTAxLTAxVDAwOjAwOjAwWiIsImVudGl0bGVtZW50cyI6eyJ0aWVyIjoic3RhbmRhcmQiLCJwbGF0Zm9ybXMiOlsiYW5kcm9pZCJdLCJhYWEiOiJmaXJzdCIsIm5vdGVzIjoieCIsInp6eiI6Imxhc3QifX0=",
        ),
        ParityCase(
            "unicode_tier",
            "unicode-001", "طلا",
            listOf("ios"), "2029-06-30T01:02:03Z",
            emptyMap(),
            "VOR1.eyJ2IjoxLCJpZCI6InVuaWNvZGUtMDAxIiwicHJvZHVjdCI6InZvciIsImlzc3VlZF9hdCI6IjIwMjYtMDktMTJUMDA6MDA6MDBaIiwiZXhwaXJlc19hdCI6IjIwMjktMDYtMzBUMDE6MDI6MDNaIiwiZW50aXRsZW1lbnRzIjp7InRpZXIiOiLYt9mE2KciLCJwbGF0Zm9ybXMiOlsiaW9zIl19fQ.wdcZyBqdVOrXVeJF4FSCfYFxxQh-83EQhUlaHH5qgqN6xpECGYGH7I6aIlpb5sg_4slooW1rs1aEwseu5bZyCw",
            "eyJ2IjoxLCJpZCI6InVuaWNvZGUtMDAxIiwicHJvZHVjdCI6InZvciIsImlzc3VlZF9hdCI6IjIwMjYtMDktMTJUMDA6MDA6MDBaIiwiZXhwaXJlc19hdCI6IjIwMjktMDYtMzBUMDE6MDI6MDNaIiwiZW50aXRsZW1lbnRzIjp7InRpZXIiOiLYt9mE2KciLCJwbGF0Zm9ybXMiOlsiaW9zIl19fQ==",
        ),
    )

    data class Rfc8032(
        val seedHex: String,
        val pubHex: String,
        val msgHex: String,
        val sigHex: String,
    )

    /** RFC 8032 test vectors 1-2 (SHA-512, empty/1-byte messages). */
    val RFC_8032: List<Rfc8032> = listOf(
        Rfc8032(
            "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
            "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
            "",
            "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
        ),
        Rfc8032(
            "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb",
            "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
            "72",
            "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00",
        ),
    )

    data class Argon2id(
        val name: String,
        val password: String,
        val saltHex: String,
        val t: Int,
        val mKib: Int,
        val p: Int,
        val digestHex: String,
    )

    /** argon2-cffi digests (v1.3 Argon2id, version 0x13). */
    val ARGON2ID: List<Argon2id> = listOf(
        Argon2id(
            "rfc_params_small", "correct horse battery staple",
            "30313233343536373839616263646566", 3, 32, 4,
            "cf7329fd141727707e95625db0df906d08b4bcff2d65f7ca02f9c2c0a1793182",
        ),
        Argon2id(
            "production_params", "طوفان-بلند-پسوند-۲۰۲۶",
            "766f726973737565726261636b757073616c7431", 3, 65_536, 1,
            "5459047c07fc323ed966c806f4c21ef6597cfddbbd11a818172eee6b83fee534",
        ),
    )

    data class Derive(
        val seedHex: String,
        val pubB64Url: String,
    )

    /** seed -> public-key derivation vs OpenSSL (cryptography lib). */
    val DERIVE_PUBLIC: List<Derive> = listOf(
        Derive("422041a0ee95c2a2bc02d1375748004783bc7d106d90c2660fb0c80b1136eefa", "Cvkcg66G3n1gGK-Go2Ev8kieR_KNvS5ElcAWQQizXKc"),
        Derive("1e35e6a8f183938a438dce9cab2305f3e6f1eb9d52824c1a3eaa5aa4d989cf44", "yAzDIcUe0W1HPgGZ8MmOR-MrpNwCvkAm_ckT3OiLqZ0"),
        Derive("18648080b0d991eaa2e8d3453cb9cd18cf73ee1ac9428d788624b925693870a6", "kfaqba5dC7wu_5gbhySuEjL6y3AamcnPeH5wSr3dcmU"),
    )

    fun hexToBytes(hex: String): ByteArray =
        ByteArray(hex.length / 2) { index -> hex.substring(index * 2, index * 2 + 2).toInt(16).toByte() }
}
