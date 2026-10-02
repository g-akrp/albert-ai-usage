import AlbertUsageCore
import Foundation

func environmentChecks() {
    let env = ChildEnvironment(
        process: ["PATH": "/usr/bin:/bin", "GITHUB_TOKEN": "x", "GH_TOKEN": "y", "TMPDIR": "/tmp/a/", "USER": "me"],
        loginShell: ["PATH": "~/.local/bin:/opt/homebrew/bin:relative", "https_proxy": "http://proxy:8080", "USER": "other"],
        home: "/Users/me")
    check("pathLoginShellFirst", Array(env.searchPath.prefix(4)) == ["/Users/me/.local/bin", "/opt/homebrew/bin", "/usr/bin", "/bin"])
    check("pathNoDuplicatesOrRelative", Set(env.searchPath).count == env.searchPath.count && !env.searchPath.contains("relative"))
    check("pathHasCommonFolders", env.searchPath.contains("/usr/local/bin") && env.searchPath.contains("/Users/me/bin"))
    check("envDropsTokens", env.base["GITHUB_TOKEN"] == nil && env.base["GH_TOKEN"] == nil)
    check("envProxyFromLoginShell", env.base["https_proxy"] == "http://proxy:8080")
    check("envProcessWins", env.base["USER"] == "me" && env.base["TMPDIR"] == "/tmp/a/")
    check("envFixed", env.base["TERM"] == "dumb" && env.base["NO_COLOR"] == "1" && env.base["LANG"] == "en_US.UTF-8"
        && env.base["HOME"] == "/Users/me")
    check("envOverridesExpandHome", env.variables(overrides: ["CLAUDE_CONFIG_DIR": "~/.claude-work"])["CLAUDE_CONFIG_DIR"]
        == "/Users/me/.claude-work")

    let real = ChildEnvironment(process: ["PATH": "/usr/bin:/bin"], loginShell: nil, home: NSHomeDirectory())
    check("resolveOnPath", real.resolveExecutable("sh") == "/bin/sh")
    check("resolveAbsolute", real.resolveExecutable("/bin/sh") == "/bin/sh")
    check("resolveMissing", real.resolveExecutable("no-such-tool-albert") == nil)
    check("resolveRejectsRelativePath", real.resolveExecutable("bin/sh") == nil)

    let printed = Data("motd noise\n\n__ALBERT_AI_USAGE_ENV__\nPATH=/a:/b\0https_proxy=http://p=1\0".utf8)
    check("parseLoginShellOutput", ChildEnvironment.parseEnvOutput(printed) == ["PATH": "/a:/b", "https_proxy": "http://p=1"])
    let shell = ChildEnvironment.readLoginShell(shell: "/bin/zsh")
    check("readLoginShellHasPath", shell?["PATH"]?.isEmpty == false)
}

/// A provider config that runs `/bin/sh -c <script>`.
private func shellConfig(_ type: String, _ script: String, timeout: Int = 5, extra: String = "",
                         root: String = "") -> ProviderConfig {
    let scriptJSON = String(data: try! JSONSerialization.data(withJSONObject: [script], options: [.fragmentsAllowed]),
                            encoding: .utf8)!
    let text = #"""
    {"schemaVersion": 1, "id": "fake", "revision": 1, "name": "Fake",
     "source": {"type": "\#(type)", "executable": "/bin/sh", "args": ["-c"] , "timeoutSeconds": \#(timeout) \#(extra)},
     "map": {\#(root.isEmpty ? "" : "\"root\": \"\(root)\",") "meters": [{"id": "m", "label": {"text": "M"}, "windows": [{"id": "w", "used": {"path": "/used", "as": "percent"}}]}]}}
    """#
    // Splice the script in as the second argument.
    return try! config(text.replacingOccurrences(of: #"["-c"]"#, with: #"["-c", "# + scriptJSON.dropFirst()))
}

private func percent(_ result: Result<Report, ProviderFailure>) -> Double? {
    if case .success(let report) = result { return report.maxPercent }
    return nil
}

private func failure(_ result: Result<Report, ProviderFailure>) -> String {
    if case .failure(let failure) = result { return failure.message }
    return ""
}

func runnerChecks() {
    let runner = ProviderRunner(environment: ChildEnvironment(
        process: ["PATH": "/usr/bin:/bin", "GITHUB_TOKEN": "secret"], loginShell: nil, home: NSHomeDirectory()))

    check("commandRuns", percent(runner.run(shellConfig("command", #"echo '{"used": 42}'"#))) == 42)
    check("commandNonzeroExit", failure(runner.run(shellConfig("command", #"echo '{}'; exit 3"#))) == "/bin/sh exited with status 3")
    check("commandNotJSON", failure(runner.run(shellConfig("command", "echo hello"))) == "/bin/sh output was not valid JSON")
    check("commandNoToken", percent(runner.run(shellConfig(
        "command", #"[ -z "$GITHUB_TOKEN" ] && [ "$TERM" = dumb ] && echo '{"used": 1}'"#))) == 1)
    check("commandRunsInEmptyFolder", percent(runner.run(shellConfig(
        "command", #"[ -z "$(ls -A)" ] && case "$PWD" in *albert-ai-usage-*) echo '{"used": 2}';; esac"#))) == 2)
    check("commandStdinIsEmpty", percent(runner.run(shellConfig("command", #"cat; echo '{"used": 3}'"#))) == 3)
    check("commandExpectMatch", failure(runner.run(shellConfig(
        "command", #"echo '{"status": "FAIL", "used": 1}'"#,
        extra: #", "expect": {"match": [{"path": "/status", "equals": "SUCCESS"}]}"#)))
        == "/bin/sh output did not match the expected shape")
    check("commandExpectError", failure(runner.run(shellConfig(
        "command", #"echo '{"error": {"message": "not logged in"}}'"#, extra: #", "expect": {"error": "/error"}"#)))
        == "not logged in")
    check("commandExpectRequire", failure(runner.run(shellConfig(
        "command", #"echo '{"used": 1}'"#, extra: #", "expect": {"require": "/data"}"#))) == "the answer has no /data")
    check("commandTooMuchOutput", failure(runner.run(shellConfig(
        "command", "head -c 5000 /dev/zero", extra: #", "maxOutputBytes": 1024"#))) == "/bin/sh printed more than 1024 bytes")

    // A grandchild that keeps stdout open must not hold the run past its timeout.
    var start = Date()
    let hung = runner.run(shellConfig("command", "sleep 37 & sleep 37", timeout: 1))
    check("commandTimeout", failure(hung) == "/bin/sh timed out" && Date().timeIntervalSince(start) < 3)
    check("commandTimeoutKillsGroup", !processExists(matching: "sleep 37"))

    let exchange = #"""
    read line; case "$line" in *initialize*) echo 'noise'; echo '{"id": 9}'; echo '{"id": 1, "result": {}}';; esac
    read line; echo '{"id": 2, "result": {"used": 77}}'; sleep 30
    """#
    let steps = #", "steps": [{"write": {"id": 1, "method": "initialize"}}, {"await": {"match": [{"path": "/id", "equals": 1}], "error": "/error", "require": "/result"}}, {"write": {"id": 2}}, {"await": {"match": [{"path": "/id", "equals": 2}], "capture": "limits"}}], "output": "limits""#
    start = Date()
    let stdio = runner.run(shellConfig("stdio", exchange, extra: steps, root: "/result"))
    check("stdioExchange", percent(stdio) == 77 && Date().timeIntervalSince(start) < 3)

    let rpcError = #"read line; echo '{"id": 1, "error": {"code": -32601, "message": "no such method"}}'"#
    check("stdioMethodNotFound", failure(runner.run(shellConfig("stdio", rpcError, extra: steps))) == "Update the CLI to see usage")
    check("stdioClosed", failure(runner.run(shellConfig("stdio", "read line; exit 0", extra: steps)))
        == "/bin/sh closed its output before answering")
    start = Date()
    check("stdioTimeout", failure(runner.run(shellConfig("stdio", "sleep 30", timeout: 1, extra: steps))) == "/bin/sh timed out"
        && Date().timeIntervalSince(start) < 3)
    check("stdioEarlyExitNoCrash", failure(runner.run(shellConfig("stdio", "exit 0", extra: steps))).isEmpty == false)

    let missing = try! config(#"{"schemaVersion": 1, "id": "m", "revision": 1, "name": "M", "source": {"type": "command", "executable": "no-such-tool-albert"}, "map": {"meters": []}}"#)
    check("missingExecutable", failure(runner.run(missing)) == "no-such-tool-albert not found on PATH")
}

private func processExists(matching pattern: String) -> Bool {
    usleep(200_000)
    let process = try! Subprocess.launch(executable: "/usr/bin/pgrep", arguments: ["-f", pattern],
                                         environment: [:], directory: nil, pipeStdin: false)
    defer { process.terminate() }
    return process.waitForExit(deadline: Date() + 5) == 0
}
