import Foundation

/// The minimal environment providers run with, and the PATH used to find their executables.
///
/// An app opened from Finder gets launchd's bare PATH, so the PATH and proxy settings come from
/// the login shell (read once at launch), plus common install folders. `GITHUB_TOKEN` and
/// `GH_TOKEN` are never passed: a stale one shadows a working `gh` login.
public struct ChildEnvironment {
    public let searchPath: [String]
    public let base: [String: String]
    public let home: String

    static let passedKeys = [
        "USER", "LOGNAME", "LANG", "LC_ALL", "TMPDIR", "SHELL", "SSH_AUTH_SOCK", "__CF_USER_TEXT_ENCODING",
        "HTTP_PROXY", "HTTPS_PROXY", "http_proxy", "https_proxy", "NO_PROXY", "no_proxy", "ALL_PROXY", "all_proxy",
    ]

    public static func commonFolders(home: String) -> [String] {
        ["\(home)/.local/bin", "\(home)/bin", "/opt/homebrew/bin", "/opt/homebrew/sbin", "/usr/local/bin",
         "/usr/bin", "/bin", "/usr/sbin", "/sbin"]
    }

    /// `process` is this app's environment; `loginShell` is what the login shell printed, if anything.
    public init(process: [String: String], loginShell: [String: String]?, home: String) {
        self.home = home
        var seen = Set<String>(), path: [String] = []
        let candidates = (loginShell?["PATH"] ?? "").split(separator: ":").map(String.init)
            + (process["PATH"] ?? "").split(separator: ":").map(String.init)
            + Self.commonFolders(home: home)
        for entry in candidates {
            let expanded = entry.hasPrefix("~/") ? home + entry.dropFirst() : entry
            if expanded.hasPrefix("/"), seen.insert(expanded).inserted { path.append(expanded) }
        }
        searchPath = path

        var base = ["PATH": path.joined(separator: ":"), "HOME": home, "TERM": "dumb", "NO_COLOR": "1"]
        for key in Self.passedKeys {
            if let value = process[key] ?? loginShell?[key] { base[key] = value }
        }
        base["LANG"] = base["LANG"] ?? "en_US.UTF-8"
        self.base = base
    }

    /// The base environment with a config's `env` on top; a leading `~/` means the home folder.
    public func variables(overrides: [String: String]) -> [String: String] {
        base.merging(overrides.mapValues(expandHome)) { $1 }
    }

    public func expandHome(_ value: String) -> String {
        value.hasPrefix("~/") ? home + value.dropFirst() : value
    }

    /// An absolute or `~/` path as is; a bare name looked up on the search path.
    public func resolveExecutable(_ name: String) -> String? {
        if name.hasPrefix("/") || name.hasPrefix("~/") {
            let path = expandHome(name)
            return FileManager.default.isExecutableFile(atPath: path) ? path : nil
        }
        guard !name.contains("/") else { return nil }
        return searchPath.lazy.map { "\($0)/\(name)" }.first { FileManager.default.isExecutableFile(atPath: $0) }
    }

    // MARK: Login shell

    static let marker = "__AI_USAGE_ENV__"

    /// Runs the user's shell as an interactive login shell and returns its environment, or `nil`.
    /// Anything the shell's startup files print before the marker is ignored.
    public static func readLoginShell(shell: String, timeout: TimeInterval = 10) -> [String: String]? {
        guard let process = try? Subprocess.launch(
            executable: shell, arguments: ["-l", "-i", "-c", "printf '\\n\(marker)\\n'; /usr/bin/env -0"],
            environment: ProcessInfo.processInfo.environment, directory: nil, pipeStdin: false)
        else { return nil }
        defer { process.terminate() }
        guard case .data(let output) = process.readAll(deadline: Date() + timeout, maxBytes: 1_048_576) else {
            return nil
        }
        return parseEnvOutput(output)
    }

    public static func parseEnvOutput(_ output: Data) -> [String: String]? {
        guard let text = String(data: output, encoding: .utf8),
              let start = text.range(of: "\n\(marker)\n") else { return nil }
        var result: [String: String] = [:]
        for entry in text[start.upperBound...].split(separator: "\0") {
            guard let equals = entry.firstIndex(of: "=") else { continue }
            result[String(entry[..<equals])] = String(entry[entry.index(after: equals)...])
        }
        return result
    }
}
