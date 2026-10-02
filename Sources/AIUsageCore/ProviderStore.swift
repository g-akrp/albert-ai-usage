import Foundation

/// Loads provider files: the ones bundled with the app, then the user's folder. A user file with
/// the same id replaces the bundled one.
public enum ProviderStore {
    public static func userFolder(home: String) -> URL {
        URL(fileURLWithPath: home).appendingPathComponent(".config/ai-usage/providers")
    }

    /// Configs sorted by id, and one message per file that did not load.
    public static func load(folders: [URL]) -> (configs: [ProviderConfig], errors: [String]) {
        var byId: [String: ProviderConfig] = [:], errors: [String] = []
        for folder in folders {
            let files = (try? FileManager.default.contentsOfDirectory(at: folder, includingPropertiesForKeys: nil)) ?? []
            for file in files.filter({ $0.pathExtension == "json" }).sorted(by: { $0.lastPathComponent < $1.lastPathComponent }) {
                do {
                    let values = try file.resourceValues(forKeys: [.isRegularFileKey, .isSymbolicLinkKey, .fileSizeKey])
                    guard values.isRegularFile == true, values.isSymbolicLink != true,
                          (values.fileSize ?? 0) <= 256 * 1024 else {
                        throw ConfigError("", "only regular files up to 256 KB load")
                    }
                    let config = try ProviderConfig.parse(Data(contentsOf: file))
                    guard config.id == file.deletingPathExtension().lastPathComponent else {
                        throw ConfigError("/id", "must match the file name")
                    }
                    byId[config.id] = config
                } catch {
                    errors.append("\(file.lastPathComponent): \(error)")
                }
            }
        }
        return (byId.values.sorted { $0.id < $1.id }, errors)
    }
}
