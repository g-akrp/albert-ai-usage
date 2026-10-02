// swift-tools-version:6.0
import PackageDescription

let package = Package(
    name: "AIUsage",
    platforms: [.macOS(.v13)],
    targets: [
        .target(name: "AIUsageCore"),
        .executableTarget(name: "CoreChecks", dependencies: ["AIUsageCore"]),
        .executableTarget(name: "AIUsage", dependencies: ["AIUsageCore"]),
    ],
    swiftLanguageModes: [.v5]
)
