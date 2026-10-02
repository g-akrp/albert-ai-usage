import AlbertUsageCore
import Foundation

func jsonChecks() {
    let doc = json(#"{"rate_limits": {"five_hour": {"utilization": 14}}, "groups": [{"name": "a"}, {"name": "b"}], "a~b": {"c/d": 1}}"#)
    check("pointerNested", JSON.number(JSON.resolve(doc, "/rate_limits/five_hour/utilization")) == 14)
    check("pointerEmptyIsRoot", JSON.resolve(doc, "") is [String: Any])
    check("pointerMissingIsNil", JSON.resolve(doc, "/b/c") == nil)
    check("pointerArrayIndex", JSON.resolve(doc, "/groups/1/name") as? String == "b")
    check("pointerEscapes", JSON.number(JSON.resolve(doc, "/a~0b/c~1d")) == 1)

    check("equalIntAndFloat", JSON.equal(json("2"), json("2.0")))
    check("equalNumberNotString", !JSON.equal(json(#""2""#), json("2")))
    check("equalBoolNotNumber", !JSON.equal(json("true"), json("1")))
    check("equalBool", JSON.equal(json("true"), json("true")))
    check("equalNull", JSON.equal(json("null"), json("null")))
}

func configChecks() {
    let ids = ["antigravity", "claude", "codex", "copilot"]
    for id in ids {
        let loaded = bundledConfig(id)
        check("bundledConfigLoads_\(id)", loaded?.id == id)
    }
    check("bundledIconLabels", ids.compactMap { bundledConfig($0)?.iconLabel } == ["AGY", "CLD", "CDX", "GHC"])
    check("bundledIconColor", bundledConfig("claude")?.iconColor == RGB(0xD9, 0x77, 0x57))
    check("bundledRefresh", bundledConfig("antigravity")?.refreshSeconds == 600)

    check("hexColor", RGB(hex: "10A37F") == RGB(0x10, 0xA3, 0x7F))
    check("hexColorRejectsHash", RGB(hex: "#D97757") == nil)
    check("hexColorRejectsShort", RGB(hex: "D977") == nil)
    check("hexColorRejectsNonHex", RGB(hex: "ZZZZZZ") == nil)

    let minimal = #"{"schemaVersion": 1, "id": "my-agent", "revision": 1, "name": "My Agent", "source": {"type": "command", "executable": "x"}, "map": {"meters": []}}"#
    let parsed = try? config(minimal)
    check("defaultIconLabel", parsed?.iconLabel == "MY-")
    check("defaultIconColor", parsed?.iconColor == .gray)
    check("defaultRefresh", parsed?.refreshSeconds == 300)

    func error(_ text: String) -> String {
        do { _ = try config(text); return "" } catch { return "\(error)" }
    }
    check("rejectsSchemaVersion", error(minimal.replacingOccurrences(of: "\"schemaVersion\": 1", with: "\"schemaVersion\": 2"))
        .hasPrefix("/schemaVersion"))
    check("rejectsBadId", error(minimal.replacingOccurrences(of: "my-agent", with: "My Agent")).hasPrefix("/id"))
    check("errorNamesArgPath", error(minimal.replacingOccurrences(of: #""executable": "x""#, with: #""executable": "x", "args": ["a", 2]"#))
        == "/source/args/1: expected a string")
    check("rejectsUnknownSourceType", error(minimal.replacingOccurrences(of: "\"command\"", with: "\"shell\""))
        .hasPrefix("/source/type"))
}

func mappingChecks() {
    // Codex: one meter selected from the root, two windows.
    let codex = bundledConfig("codex")!
    let codexAnswer = json(#"""
    {"id": 2, "result": {"rateLimits": {"limitId": "codex", "limitName": null, "planType": "plus",
      "primary": {"usedPercent": 45, "resetsAt": 1790680997, "windowDurationMins": 300},
      "secondary": {"usedPercent": 12, "resetsAt": 1791075882, "windowDurationMins": 10080}},
     "ordinaryUsageAllowed": true}}
    """#)
    let report = Mapper.apply(codex.map, to: codexAnswer)
    check("codexMeter", report.meters.map(\.id) == ["codex"] && report.meters[0].label == "Codex")
    check("codexWindows", report.meters[0].windows.map(\.label) == ["Session", "Weekly"])
    check("codexPercent", report.meters[0].windows.map(\.usedPercent) == [45, 12])
    check("codexDuration", report.meters[0].windows[1].durationSeconds == 10080 * 60)
    check("codexReset", report.meters[0].windows[0].resetsAt == Date(timeIntervalSince1970: 1790680997))
    check("codexPlanAccess", report.plan == "plus" && report.access == true)
    check("maxPercent", report.maxPercent == 45)

    // Claude: ISO 8601 with microseconds, and a missing window is skipped.
    let claude = bundledConfig("claude")!
    let claudeAnswer = json(#"""
    {"type": "control_response", "response": {"request_id": "albert-usage", "response": {
      "subscription_type": "max", "rate_limits_available": true,
      "rate_limits": {"five_hour": {"utilization": 14, "resets_at": "2026-10-05T12:16:59.123456+00:00"},
                      "seven_day": null}}}}
    """#)
    let claudeReport = Mapper.apply(claude.map, to: claudeAnswer)
    check("claudeOneWindow", claudeReport.meters.first?.windows.map(\.id) == ["five_hour"])
    let reset = claudeReport.meters.first?.windows.first?.resetsAt?.timeIntervalSince1970 ?? 0
    check("claudeFractionalReset", abs(reset - 1791202619.123456) < 0.001)
    check("claudePlan", claudeReport.plan == "max" && claudeReport.available == true)

    // Copilot: remaining percent inverted.
    let copilot = bundledConfig("copilot")!
    let copilotReport = Mapper.apply(copilot.map, to: json(#"""
    {"copilot_plan": "business", "quota_snapshots": {
      "chat": {"percent_remaining": 100, "has_quota": true, "unlimited": true},
      "completions": {"percent_remaining": 100, "has_quota": true, "unlimited": false},
      "premium_interactions": {"percent_remaining": 40.25, "has_quota": true, "unlimited": false}}}
    """#))
    check("copilotMeters", copilotReport.meters.map(\.label) == ["Completions", "Premium Interactions"])
    check("copilotRemainingPercent", copilotReport.meters.last?.windows.first?.usedPercent == 59.75)
    // Copilot Free: premium interactions have no quota (entitlement 0, percent_remaining 0), not 100% used.
    let freeReport = Mapper.apply(copilot.map, to: json(#"""
    {"copilot_plan": "individual", "quota_snapshots": {
      "chat": {"percent_remaining": 100.0, "has_quota": true, "unlimited": false, "entitlement": 200},
      "completions": {"percent_remaining": 100.0, "has_quota": true, "unlimited": false, "entitlement": 2000},
      "premium_interactions": {"percent_remaining": 0.0, "has_quota": false, "unlimited": false, "entitlement": 0}}}
    """#))
    check("copilotSkipsNoQuota", freeReport.meters.map(\.id) == ["chat", "completions"] && freeReport.maxPercent == 0)

    // Antigravity: each group, each bucket, remaining fraction.
    let agy = bundledConfig("antigravity")!
    let agyReport = Mapper.apply(agy.map, to: json(#"""
    {"status": "SUCCESS", "command": {"data": {"groups": [{"name": "Gemini", "buckets": [
      {"id": "g5h", "name": "5 hours", "remaining_fraction": 0.75, "reset_time": "2026-10-05T12:00:00Z", "window": "five_hour"},
      {"id": "gw", "name": "Week", "remaining_fraction": 1, "window": "weekly"}]}]}}}
    """#))
    check("agyWindows", agyReport.meters.first?.windows.map(\.usedPercent) == [25, 0])
    check("agyDuration", agyReport.meters.first?.windows.map(\.durationSeconds) == [18000, 604800])

    check("fractionToPercent", Convert.usedPercent(json("0.29"), .fraction).map { abs($0 - 29) < 1e-9 } == true)
    check("boolIsNotUsage", Convert.usedPercent(json("true"), .percent) == nil)
    check("epochMillis", Convert.date(json("1790680997000"), .epochMillis) == Date(timeIntervalSince1970: 1790680997))
    check("isoWithoutFraction", Convert.parseISO8601("2026-10-05T12:16:59Z") == Date(timeIntervalSince1970: 1791202619))
    check("isoUnparseable", Convert.parseISO8601("not-a-date") == nil)
    check("durationLabels", [18000, 604800, 2_592_000, 86400, 7200].map(Convert.durationLabel)
        == ["Session", "Weekly", "Monthly", "1d", "2h"])

    // eachEntry uses the member name as id, sorted; duplicate ids keep the first.
    let entries = try! config(#"""
    {"schemaVersion": 1, "id": "x", "revision": 1, "name": "X", "source": {"type": "command", "executable": "x"},
     "map": {"meters": [{"eachEntry": "/byId", "id": {"entryKey": true}, "label": {"entryKey": true},
       "windows": [{"id": "w", "select": "/w", "used": {"path": "/p", "as": "percent"}},
                   {"id": "w", "select": "/v", "used": {"path": "/p", "as": "percent"}}]}]}}
    """#)
    let entryReport = Mapper.apply(entries.map, to: json(#"{"byId": {"b": {"w": {"p": 2}, "v": {"p": 9}}, "a": {"w": {"p": 1}}}}"#))
    check("eachEntryIds", entryReport.meters.map(\.id) == ["a", "b"])
    check("duplicateWindowKeepsFirst", entryReport.meters.last?.windows.map(\.usedPercent) == [2])
}

private func sampleRun(_ id: String, _ name: String, _ label: String,
                       _ result: Result<Report, ProviderFailure>?) -> ProviderRun {
    ProviderRun(id: id, name: name, iconLabel: label, iconColor: RGB(0, 130, 0), result: result)
}

private func sampleReport(_ percent: Double) -> Report {
    Report(plan: "Plus", meters: [MeterReport(id: "codex", label: "Codex", windows: [
        WindowReport(id: "primary", label: "Session", usedPercent: percent,
                     resetsAt: Date(timeIntervalSince1970: 1790680997)),
    ])])
}

func iconChecks() {
    check("severity", [45, 70, 90].map(StatusIcons.severity) == [StatusIcons.green, StatusIcons.orange, StatusIcons.red])
    check("lineWidth", [PixelFont.lineWidth(""), PixelFont.lineWidth("A"), PixelFont.lineWidth("AB")] == [0, 3, 7])
    let layout = PixelFont.layout(top: "A", bottom: "1")
    check("layoutSize", layout.width == 3 && layout.height == 11)  // ×2 in the menu bar: 6 × 22 points
    check("layoutPixels", layout.pixels.filter { !$0.bottomRow }.count == 10 && layout.pixels.contains { $0.bottomRow && $0.y == 6 })
    check("layoutCentered", PixelFont.layout(top: "CDX", bottom: "5%").pixels.filter(\.bottomRow).map(\.x).min() == 2)

    let codex = sampleRun("codex", "Codex", "CDX", .success(sampleReport(45)))
    let copilot = sampleRun("copilot", "GitHub Copilot", "GHC", .success(sampleReport(10)))
    let failed = sampleRun("codex", "Codex", "CDX", .failure(ProviderFailure("timed out")))

    let pinnedIcons = StatusIcons.icons(runs: [codex, copilot], pinned: "codex")
    check("pinnedShowsOne", pinnedIcons.count == 1 && pinnedIcons[0].accessibility == "Codex 45%")
    check("pinnedColors", pinnedIcons[0].topColor == RGB(0, 130, 0) && pinnedIcons[0].bottomColor == StatusIcons.green)
    check("unpinnedCycles", StatusIcons.icons(runs: [codex, copilot], pinned: nil).count == 2)
    check("stalePinCycles", StatusIcons.icons(runs: [codex], pinned: "removed").map(\.accessibility) == ["Codex 45%"])
    let errorIcon = StatusIcons.icons(runs: [failed, copilot], pinned: "codex")
    check("erroredPin", errorIcon.map(\.bottom) == ["ERR"] && errorIcon[0].accessibility == "Codex error: timed out")
    check("cycleIncludesErrors", StatusIcons.icons(runs: [failed, copilot], pinned: nil).map(\.bottom) == ["ERR", "10%"])
    check("loadingLeftOut", StatusIcons.icons(runs: [sampleRun("a", "A", "A", nil), copilot], pinned: nil).count == 1)
    check("nothingLoadedPlaceholder", StatusIcons.icons(runs: [sampleRun("a", "A", "A", nil)], pinned: nil) == [StatusIcons.placeholder])
}

func menuChecks() {
    let utc = MenuModel.resetFormatter(timeZone: TimeZone(identifier: "UTC")!)
    check("resetFormat", utc.string(from: Date(timeIntervalSince1970: 1791202619)) == "Oct 5, 2026 12:16 PM")

    let codex = sampleRun("codex", "Codex", "CDX", .success(sampleReport(95)))
    let failed = sampleRun("copilot", "GitHub Copilot", "GHC", .failure(ProviderFailure("timed out")))
    let entries = MenuModel.entries(runs: [codex, failed], pinned: "codex", configErrors: ["bad.json: /id: required"],
                                    updated: "Updated 10:00", launchAtLogin: true, version: "1.0.0",
                                    formatReset: utc.string(from:))
    check("menuPinnedTitle", entries.first == .provider(id: "codex", title: "\u{2605} Codex (Plus)", pinned: true))
    check("menuWindowRow", entries.contains(.detail("\u{00A0}\u{00A0}\u{00A0}Session: 95% (resets Sep 29, 2026 11:23 AM)", .red)))
    check("menuErrorRow", entries.contains(.detail("\u{00A0}\u{00A0}\u{00A0}Error: timed out", .red)))
    check("menuUnpinnedStar", entries.contains(.provider(id: "copilot", title: "\u{2606} GitHub Copilot", pinned: false)))
    check("menuConfigError", entries.contains(.detail("bad.json: /id: required", .red)))
    check("menuCycleUnchecked", entries.contains(.cycleAll(checked: false)))
    check("menuEnds", Array(entries.suffix(2)) == [.version("1.0.0"), .quit])

    let limited = sampleRun("codex", "Codex", "CDX", .success(Report(access: false, meters: [])))
    let limitedEntries = MenuModel.entries(runs: [limited], pinned: nil, configErrors: [], updated: nil,
                                           launchAtLogin: false, version: "1", formatReset: utc.string(from:))
    check("menuLimitReached", limitedEntries.contains(.detail("\u{00A0}\u{00A0}\u{00A0}Usage limit reached", .red)))
    check("menuCycleChecked", limitedEntries.contains(.cycleAll(checked: true)))

    let copilot = Mapper.apply(bundledConfig("copilot")!.map, to: json(#"{"quota_snapshots": {"chat": {"percent_remaining": 100, "has_quota": true, "unlimited": false}}}"#))
    let copilotEntries = MenuModel.entries(runs: [sampleRun("copilot", "GitHub Copilot", "GHC", .success(copilot))],
                                           pinned: nil, configErrors: [], updated: nil, launchAtLogin: false,
                                           version: "1", formatReset: utc.string(from:))
    check("menuSingleWindowUsesMeterLabel", copilotEntries.contains(.detail("\u{00A0}\u{00A0}\u{00A0}Chat: 0%", .green)))

    check("scheduleInterval", Schedule.delay(interval: 300, failures: 0) == 300)
    check("scheduleBackoff", (1...8).map { Schedule.delay(interval: 300, failures: $0) } == [60, 120, 240, 480, 960, 1800, 1800, 1800])
    check("versionFormat", AppVersion.current.range(of: #"^\d+\.\d+\.\d+$"#, options: .regularExpression) != nil)
    if let plist = NSDictionary(contentsOfFile: "Resources/Info.plist") {
        check("versionMatchesInfoPlist", plist["CFBundleShortVersionString"] as? String == AppVersion.current)
    } else {
        check("infoPlistReadable", false)
    }
}
