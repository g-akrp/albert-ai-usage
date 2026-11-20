import AIUsageCore
import AppKit
import Foundation

// A provider that exits early must not kill the app when its stdin is written.
signal(SIGPIPE, SIG_IGN)

if CommandLine.arguments.contains("--mcp") {
    let server = McpServer(load: { try Data(contentsOf: SnapshotFile.url) }, now: Date.init)
    while let line = readLine() {
        if let response = server.reply(to: line) {
            print(response)
            fflush(stdout)
        }
    }
    exit(0)
}

if CommandLine.arguments.contains("--json") {
    let result = UsageSnapshot.cliJSON(load: { try Data(contentsOf: SnapshotFile.url) }, now: Date())
    print(result.output)
    fflush(stdout)
    exit(result.exitCode)
}

let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let controller = StatusController()
app.run()
