// swift-tools-version:6.0
import PackageDescription

let package = Package(
    name: "AlbertAIUsage",
    platforms: [.macOS(.v13)],
    targets: [
        .target(name: "AlbertUsageCore"),
        .executableTarget(name: "CoreChecks", dependencies: ["AlbertUsageCore"]),
        .executableTarget(name: "AlbertAIUsage", dependencies: ["AlbertUsageCore"]),
    ],
    swiftLanguageModes: [.v5]
)
