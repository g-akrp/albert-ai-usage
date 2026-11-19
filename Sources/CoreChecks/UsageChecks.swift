import AIUsageCore
import Foundation

private func run(_ id: String, name: String? = nil, result: Result<Report, ProviderFailure>?) -> ProviderRun {
    ProviderRun(id: id, name: name ?? id, iconLabel: String(id.prefix(3)), iconColor: .gray, result: result)
}

private func report(_ windows: [WindowReport]) -> Result<Report, ProviderFailure> {
    .success(Report(meters: [MeterReport(id: "plan", label: "Plan", windows: windows)]))
}

func usageChecks() {
    let now = Date(timeIntervalSince1970: 1_700_000_000)
    let r1 = Date(timeIntervalSince1970: 1_700_003_600)
    let r2 = Date(timeIntervalSince1970: 1_700_010_800)

    let tight = run("a", name: "Alpha", result: report([
        WindowReport(id: "session", label: "Session", usedPercent: 30, resetsAt: r1),
        WindowReport(id: "weekly", label: "Weekly", usedPercent: 80, resetsAt: r2),
    ]))
    let over = run("b", name: "Beta", result: report([
        WindowReport(id: "session", label: "Session", usedPercent: 120, resetsAt: r1),
    ]))
    let high = run("c", name: "Gamma", result: report([
        WindowReport(id: "session", label: "Session", usedPercent: 30, resetsAt: r1),
    ]))
    let noPct = run("d", name: "Delta", result: report([
        WindowReport(id: "session", label: "Session", usedPercent: nil, resetsAt: r1),
    ]))
    let loading = run("e", name: "Epsilon", result: nil)
    let failed = run("f", name: "Zeta", result: .failure(ProviderFailure("not logged in")))

    let snap = UsageSnapshot(runs: [failed, loading, noPct, tight, over, high], now: now, intervalSeconds: 30)
    check("usageSortedOrder", snap.providers.map(\.id) == ["c", "a", "b", "d", "e", "f"])

    let alpha = snap.providers.first { $0.id == "a" }
    check("usageTightestWindow", alpha?.status == "ok" && alpha?.remainingPercent == 20 && alpha?.resetsAt == r2 && alpha?.windows.count == 2)
    let beta = snap.providers.first { $0.id == "b" }
    check("usageClampZero", beta?.remainingPercent == 0)
    let delta = snap.providers.first { $0.id == "d" }
    check("usageNilPercent", delta?.status == "ok" && delta?.remainingPercent == nil)
    let eps = snap.providers.first { $0.id == "e" }
    check("usageLoading", eps?.status == "loading" && eps?.remainingPercent == nil)
    let zeta = snap.providers.first { $0.id == "f" }
    check("usageError", zeta?.status == "error" && zeta?.message == "not logged in" && zeta?.remainingPercent == nil)

    let encoded = try? snap.encoded()
    let decoded = encoded.flatMap { try? UsageSnapshot.decode($0) }
    check("usageRoundTrip", decoded == snap)

    let staleView = snap.view(now: now.addingTimeInterval(100))
    check("usageViewStale", (staleView["stale"] as? Bool) == true && (staleView["ageSeconds"] as? Int) == 100)
    let freshView = snap.view(now: now.addingTimeInterval(60))
    check("usageViewFresh", (freshView["stale"] as? Bool) == false && (freshView["ageSeconds"] as? Int) == 60)
}

func mcpChecks() {
    let now = Date(timeIntervalSince1970: 1_700_000_000)
    let snap = UsageSnapshot(runs: [
        run("low", name: "Low", result: report([WindowReport(id: "s", label: "Session", usedPercent: 80)])),
        run("high", name: "High", result: report([WindowReport(id: "s", label: "Session", usedPercent: 10)])),
    ], now: now, intervalSeconds: 60)
    let snapData = try! snap.encoded()

    let server = McpServer(load: { snapData }, now: { now.addingTimeInterval(15) })

    let initReply = server.reply(to: #"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#)
    let initObj = initReply.flatMap { JSON.parse(Data($0.utf8)) as? [String: Any] }
    check("mcpInitialize",
          JSON.resolve(initObj as Any, "/result/protocolVersion") as? String == "2024-11-05"
          && JSON.resolve(initObj as Any, "/result/serverInfo/name") as? String == "aiusage"
          && JSON.resolve(initObj as Any, "/result/serverInfo/version") as? String == AppVersion.current
          && JSON.number(initObj?["id"]) == 1)

    let listReply = server.reply(to: #"{"jsonrpc":"2.0","id":"req-2","method":"tools/list"}"#)
    let listObj = listReply.flatMap { JSON.parse(Data($0.utf8)) as? [String: Any] }
    check("mcpToolsList",
          JSON.resolve(listObj as Any, "/result/tools/0/name") as? String == "usage"
          && listObj?["id"] as? String == "req-2")

    let callReply = server.reply(to: #"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"usage"}}"#)
    let callObj = callReply.flatMap { JSON.parse(Data($0.utf8)) as? [String: Any] }
    let callText = JSON.resolve(callObj as Any, "/result/content/0/text") as? String
    let payload = callText.flatMap { JSON.parse(Data($0.utf8)) as? [String: Any] }
    let providers = payload?["providers"] as? [[String: Any]]
    check("mcpToolsCallUsage",
          providers?.count == 2
          && providers?.first?["id"] as? String == "high"
          && JSON.number(providers?.first?["remainingPercent"]) == 90
          && JSON.number(payload?["ageSeconds"]) == 15
          && JSON.bool(payload?["stale"]) == false)

    let missingServer = McpServer(load: { throw CocoaError(.fileReadNoSuchFile) }, now: { now })
    let missingReply = missingServer.reply(to: #"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"usage"}}"#)
    let missingObj = missingReply.flatMap { JSON.parse(Data($0.utf8)) as? [String: Any] }
    check("mcpMissingSnapshot",
          JSON.bool(JSON.resolve(missingObj as Any, "/result/isError")) == true
          && (JSON.resolve(missingObj as Any, "/result/content/0/text") as? String)?.contains("not running") == true)

    let corruptServer = McpServer(load: { Data("{".utf8) }, now: { now })
    let corruptReply = corruptServer.reply(to: #"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"usage"}}"#)
    let corruptObj = corruptReply.flatMap { JSON.parse(Data($0.utf8)) as? [String: Any] }
    check("mcpCorruptSnapshot", JSON.bool(JSON.resolve(corruptObj as Any, "/result/isError")) == true)

    check("mcpNotificationNoReply", server.reply(to: #"{"jsonrpc":"2.0","method":"notifications/initialized"}"#) == nil)
    check("mcpBlankLineNoReply", server.reply(to: "   \n") == nil)

    let parseErr = server.reply(to: "{bad")
    let parseObj = parseErr.flatMap { JSON.parse(Data($0.utf8)) as? [String: Any] }
    check("mcpParseError", JSON.number(JSON.resolve(parseObj as Any, "/error/code")) == -32700 && parseObj?["id"] is NSNull)

    let unknownErr = server.reply(to: #"{"jsonrpc":"2.0","id":"u1","method":"bogus/method"}"#)
    let unknownObj = unknownErr.flatMap { JSON.parse(Data($0.utf8)) as? [String: Any] }
    check("mcpUnknownMethod", JSON.number(JSON.resolve(unknownObj as Any, "/error/code")) == -32601 && unknownObj?["id"] as? String == "u1")
}

func snapshotFileChecks() {
    let dir = FileManager.default.temporaryDirectory.appendingPathComponent("ai-usage-snap-\(UUID().uuidString)")
    defer { try? FileManager.default.removeItem(at: dir) }
    let target = dir.appendingPathComponent("nested/usage.json")

    let first = Data(#"{"v":1}"#.utf8)
    let second = Data(#"{"v":2,"more":true}"#.utf8)
    let wroteFirst = (try? SnapshotFile.write(first, to: target)) != nil
    let readFirst = try? Data(contentsOf: target)
    check("snapshotFileWriteRead", wroteFirst && readFirst == first)

    let wroteSecond = (try? SnapshotFile.write(second, to: target)) != nil
    let readSecond = try? Data(contentsOf: target)
    check("snapshotFileOverwrite", wroteSecond && readSecond == second)
    check("snapshotFileDefaultUrl", SnapshotFile.url.path.hasSuffix("/Library/Application Support/AIUsage/usage.json"))
}
