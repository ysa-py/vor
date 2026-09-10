import Foundation
import CryptoKit

/// Vor offline license verification — Swift port of license/SPEC.md.
///
/// Token: `VOR1.<base64url(payload_json)>.<base64url(signature_64)>`,
/// Ed25519 over the exact payload bytes, verified with the embedded public
/// key. Fully offline: no network call, ever.
///
/// The dev public key matches license/keys/dev (unit tests + dev builds);
/// release builds embed the production key via the
/// INFOPLIST_KEY_VORLicensePublicKey build setting (see project.yml).
enum LicenseVerifier {

    static let devPublicKey = "f54nNpWuth1MHZsbi6sdEODSDvWp7V6XSSDqWtmCyMA"

    /// Key priority: Info.plist (release-injected prod key) > dev default.
    static var configuredPublicKey: String {
        let injected = (Bundle.main.infoDictionary?["VORLicensePublicKey"] as? String) ?? ""
        let trimmed = injected.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? devPublicKey : trimmed
    }

    enum Status: String {
        case valid = "VALID"
        case expired = "EXPIRED"
        case invalid = "INVALID"
    }

    struct Result {
        let status: Status
        let payload: [String: Any]?
        var expiresAt: Date? {
            (payload?["expires_at"] as? String).flatMap(LicenseVerifier.parseRFC3339)
        }
        var licenseId: String? { payload?["id"] as? String }
    }

    static func verify(_ token: String, now: Date = Date(), publicKey: String? = nil) -> Result {
        let selectedKey = publicKey ?? configuredPublicKey
        let parts = token.trimmingCharacters(in: .whitespacesAndNewlines).split(separator: ".")
        guard parts.count == 3, parts[0] == "VOR1" else { return .init(status: .invalid, payload: nil) }
        guard let payload = base64urlDecode(String(parts[1])),
              let signature = base64urlDecode(String(parts[2])),
              signature.count == 64,
              let keyBytes = base64urlDecode(selectedKey.trimmingCharacters(in: .whitespacesAndNewlines)),
              keyBytes.count == 32 else { return .init(status: .invalid, payload: nil) }

        let key: Curve25519.Signing.PublicKey
        do {
            key = try Curve25519.Signing.PublicKey(rawRepresentation: keyBytes)
        } catch {
            return .init(status: .invalid, payload: nil)
        }
        guard key.isValidSignature(signature, for: payload) else {
            return .init(status: .invalid, payload: nil)
        }

        guard let object = try? JSONSerialization.jsonObject(with: payload) as? [String: Any],
              let version = object["v"] as? Int, version == 1,
              let product = object["product"] as? String, product == "vor",
              let id = object["id"] as? String, !id.isEmpty,
              let issued = object["issued_at"] as? String,
              let expires = object["expires_at"] as? String,
              let expiresDate = parseRFC3339(expires) else {
            return .init(status: .invalid, payload: try? JSONSerialization.jsonObject(with: payload) as? [String: Any])
        }
        _ = issued
        let payloadObject = (try? JSONSerialization.jsonObject(with: payload)) as? [String: Any]
        let status: Status = now >= expiresDate ? .expired : .valid
        return .init(status: status, payload: payloadObject)
    }

    /// RFC 3339 parser for the issuer's forms (Z, fractional, offsets).
    static func parseRFC3339(_ text: String) -> Date? {
        var normalized = text.trimmingCharacters(in: .whitespaces)
        if normalized.hasSuffix("z") { normalized.removeLast(); normalized += "Z" }
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        if let date = formatter.date(from: normalized) { return date }
        formatter.formatOptions = [.withInternetDateTime]
        if let date = formatter.date(from: normalized) { return date }
        return nil
    }

    static func base64urlDecode(_ text: String) -> Data? {
        var padded = text.replacingOccurrences(of: "-", with: "+")
            .replacingOccurrences(of: "_", with: "/")
        while padded.count % 4 != 0 { padded += "=" }
        return Data(base64Encoded: padded)
    }
}
