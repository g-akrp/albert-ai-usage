import Foundation

/// Atomic storage for the latest usage snapshot written by the menu bar app and read by `--mcp`.
public enum SnapshotFile {
    public static var url: URL {
        URL(fileURLWithPath: NSHomeDirectory())
            .appendingPathComponent("Library/Application Support/AIUsage/usage.json")
    }

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
