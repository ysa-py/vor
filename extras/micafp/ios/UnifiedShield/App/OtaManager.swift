import Foundation
import UIKit

/**
 * OTA (Over-The-Air) updater for iOS.
 *
 * Update strategy:
 * - Checks GitHub Releases API every 6 hours via background fetch
 * - Uses Chinese CDN mirrors (Alibaba, Tencent) for downloads
 *   (Cloudflare is BLOCKED in Iran)
 * - Verifies SHA256 checksums before installation
 * - Supports TestFlight and direct distribution
 */
class OtaManager: ObservableObject {

    @Published var updateAvailable = false
    @Published var latestVersion: String?
    @Published var changelog: String?
    @Published var isChecking = false
    @Published var downloadProgress: Float = 0

    private let defaults = UserDefaults.standard
    private let session: URLSession

    companion object {
        static let shared = OtaManager()
        static let checkInterval: TimeInterval = 6 * 3600 // 6 hours
        static let lastCheckKey = "ota_last_check"
        static let skippedVersionKey = "ota_skipped_version"

        // CDN mirrors for Iran (Cloudflare is BLOCKED)
        static let githubApiUrl = "https://api.github.com/repos/unifiedshield/unifiedshield-ios/releases/latest"
        static let alibabaMirror = "https://unifiedshield-cn.oss-cn-beijing.aliyuncs.com/releases"
        static let tencentMirror = "https://unifiedshield-1250000000.cos.ap-shanghai.myqcloud.com/releases"
        static let ghproxyMirror = "https://mirror.ghproxy.com"
    }

    struct GitHubRelease: Codable {
        let tagName: String
        let name: String
        let body: String
        let assets: [GitHubAsset]
        let publishedAt: String

        enum CodingKeys: String, CodingKey {
            case tagName = "tag_name"
            case name
            case body
            case assets
            case publishedAt = "published_at"
        }
    }

    struct GitHubAsset: Codable {
        let name: String
        let browserDownloadUrl: String
        let size: Int
        let digest: String?

        enum CodingKeys: String, CodingKey {
            case name
            case browserDownloadUrl = "browser_download_url"
            case size
            case digest
        }
    }

    struct UpdateInfo {
        let versionName: String
        let changelog: String
        let downloadUrl: String
        let fileSize: Int
        let sha256: String
        let isCritical: Bool
    }

    init() {
        let config = URLSessionConfiguration.default
        config.timeoutIntervalForRequest = 15
        config.timeoutIntervalForResource = 60
        self.session = URLSession(configuration: config)
    }

    /**
     * Check if enough time has passed since last check.
     */
    var shouldCheckForUpdate: Bool {
        guard let lastCheck = defaults.object(forKey: Companion.lastCheckKey) as? Date else {
            return true
        }
        return Date().timeIntervalSince(lastCheck) >= Companion.checkInterval
    }

    /**
     * Check for updates with fallback mirrors.
     */
    @MainActor
    func checkForUpdate() async -> UpdateInfo? {
        isChecking = true
        defer { isChecking = false }

        let urls = [
            Companion.githubApiUrl,
            "\(Companion.ghproxyMirror)/\(Companion.githubApiUrl.removePrefix("https://"))"
        ]

        for url in urls {
            guard let requestUrl = URL(string: url) else { continue }

            do {
                let (data, response) = try await session.data(from: requestUrl)

                guard let httpResponse = response as? HTTPURLResponse,
                      httpResponse.statusCode == 200 else {
                    continue
                }

                let release = try JSONDecoder().decode(GitHubRelease.self, from: data)

                // Find IPA asset
                guard let ipaAsset = release.assets.first(where: { $0.name.hasSuffix(".ipa") }) else {
                    continue
                }

                // Get SHA256
                let sha256Asset = release.assets.first(where: { $0.name.hasSuffix(".sha256") })
                let sha256 = sha256Asset != nil ? fetchSha256(from: sha256Asset!.browserDownloadUrl) : ""

                // Determine CDN mirror URL
                let downloadUrl = "\(Companion.alibabaMirror)/\(ipaAsset.name)"

                let isCritical = release.body.localizedCaseInsensitiveContains("CRITICAL") ||
                    release.body.localizedCaseInsensitiveContains("SECURITY")

                let updateInfo = UpdateInfo(
                    versionName: release.tagName.removePrefix("v"),
                    changelog: release.body,
                    downloadUrl: downloadUrl,
                    fileSize: ipaAsset.size,
                    sha256: sha256,
                    isCritical: isCritical
                )

                // Update published state
                updateAvailable = true
                latestVersion = updateInfo.versionName
                changelog = updateInfo.changelog

                defaults.set(Date(), forKey: Companion.lastCheckKey)

                return updateInfo
            } catch {
                continue
            }
        }

        return nil
    }

    /**
     * Verify SHA256 checksum of a downloaded file.
     */
    func verifyChecksum(filePath: String, expectedSha256: String) -> Bool {
        guard let fileData = try? Data(contentsOf: URL(fileURLWithPath: filePath)) else {
            return false
        }

        var hash = [UInt8](repeating: 0, count: Int(CC_SHA256_DIGEST_LENGTH))
        fileData.withUnsafeBytes {
            _ = CC_SHA256($0.baseAddress, CC_LONG(fileData.count), &hash)
        }

        let computedHash = hash.map { String(format: "%02x", $0) }.joined()
        return computedHash.caseInsensitiveCompare(expectedSha256) == .orderedSame
    }

    /**
     * Fetch SHA256 from a .sha256 file.
     */
    private func fetchSha256(from url: String) -> String {
        guard let requestUrl = URL(string: url) else { return "" }

        let semaphore = DispatchSemaphore(value: 0)
        var result = ""

        session.dataTask(with: requestUrl) { data, _, _ in
            if let data = data, let content = String(data: data, encoding: .utf8) {
                result = content.split(separator: " ").first?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
            }
            semaphore.signal()
        }.resume()

        semaphore.wait(timeout: .now() + 10)
        return result
    }

    /**
     * Skip a specific version.
     */
    func skipVersion(_ version: String) {
        defaults.set(version, forKey: Companion.skippedVersionKey)
    }
}

// CommonCrypto bridge
import CommonCrypto
