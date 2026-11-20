import Foundation

public enum AppVersion {
    /// Set by scripts/release.sh, together with Info.plist.
    public static let current = "1.18.0"

    /// Bundle version at runtime (e.g. beta builds), falling back to `current`.
    public static var runtime: String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? current
    }

    public static var isBeta: Bool {
        Bundle.main.bundleIdentifier?.hasSuffix(".beta") == true
    }

    public static var appName: String {
        isBeta ? "AI Usage Beta" : "AI Usage"
    }
}
