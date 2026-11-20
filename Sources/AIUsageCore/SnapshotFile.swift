import Foundation

/// Atomic storage for the latest usage snapshot written by the menu bar app and read by `--mcp`.
public enum SnapshotFile {
    public static let missingMessage = "AIUsage app is not running or has no snapshot yet"

    public static func isNotFound(_ error: Error) -> Bool {
        let ns = error as NSError
        return (ns.domain == NSCocoaErrorDomain && ns.code == NSFileReadNoSuchFileError)
            || (ns.domain == NSPOSIXErrorDomain && ns.code == Int(ENOENT))
    }

    public static func url(home: String = NSHomeDirectory(), isBeta: Bool = AppVersion.isBeta) -> URL {
        URL(fileURLWithPath: home)
            .appendingPathComponent("Library/Application Support/\(isBeta ? "AIUsageBeta" : "AIUsage")/usage.json")
    }

    public static var url: URL { url() }

    public static func write(_ data: Data, to url: URL = SnapshotFile.url) throws {
        let fm = FileManager.default
        let dir = url.deletingLastPathComponent()
        try fm.createDirectory(at: dir, withIntermediateDirectories: true)
        let temp = dir.appendingPathComponent(".\(url.lastPathComponent).\(UUID().uuidString).tmp")
        do {
            try data.write(to: temp)
            if rename(temp.path, url.path) != 0 {
                let code = errno
                throw NSError(domain: NSPOSIXErrorDomain, code: Int(code))
            }
        } catch {
            try? fm.removeItem(at: temp)
            throw error
        }
    }
}
