// Generates Vor.xcodeproj via XcodeGen if available, else prints manual
// setup. Run: swift scripts/make-xcodeproj.swift
import Foundation

let projectYml = """
name: Vor
options:
  bundleIdPrefix: com.vor
  deploymentTarget:
    iOS: "15.0"
targets:
  Vor:
    type: application
    platform: iOS
    sources:
      - path: Vor
    settings:
      base:
        PRODUCT_BUNDLE_IDENTIFIER: com.vor.app
        MARKETING_VERSION: "1.0.0"
        CURRENT_PROJECT_VERSION: "1"
        INFOPLIST_KEY_UIApplicationSceneManifest_Generation: YES
        INFOPLIST_KEY_UILaunchScreen_Generation: YES
        GENERATE_INFOPLIST_FILE: YES
        SWIFT_VERSION: "5.9"
        CODE_SIGNING_ALLOWED: "NO"
  VorTests:
    type: bundle.unit-test
    platform: iOS
    sources:
      - path: Vor/LicenseTests.swift
    dependencies:
      - target: Vor
"""

let url = URL(fileURLWithPath: "project.yml")
try? projectYml.write(to: url, atomically: true, encoding: .utf8)
print("wrote project.yml")
if FileManager.default.fileExists(atPath: "/usr/local/bin/xcodegen") || FileManager.default.fileExists(atPath: "/opt/homebrew/bin/xcodegen") {
    _ = runXcodegen()
    print("generated Vor.xcodeproj")
} else {
    print("xcodegen not installed — install it (brew install xcodegen) and run 'xcodegen' in ios/")
    print("CI installs xcodegen automatically (see .github/workflows/ios.yml)")
}

func runXcodegen() -> Int32 {
    let process = Process()
    let path = FileManager.default.fileExists(atPath: "/opt/homebrew/bin/xcodegen") ? "/opt/homebrew/bin/xcodegen" : "/usr/local/bin/xcodegen"
    process.executableURL = URL(fileURLWithPath: path)
    process.arguments = ["--spec", "project.yml"]
    try? process.run()
    process.waitUntilExit()
    return process.terminationStatus
}
