import AIUsageCore
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
    {"type": "control_response", "response": {"request_id": "ai-usage", "response": {
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

    // Copilot Business out of premium requests: has_quota is false but the entitlement is not 0, so it is 100% used.
    let exhausted = Mapper.apply(copilot.map, to: json(#"""
    {"copilot_plan": "business", "quota_snapshots": {
      "chat": {"percent_remaining": 100.0, "has_quota": true, "unlimited": true, "entitlement": 0},
      "completions": {"percent_remaining": 100.0, "has_quota": true, "unlimited": true, "entitlement": 0},
      "premium_interactions": {"percent_remaining": 0.0, "has_quota": false, "unlimited": false, "entitlement": 5000}}}
    """#))
    check("copilotShowsExhaustedQuota", exhausted.meters.map(\.id) == ["premium_interactions"] && exhausted.maxPercent == 100)
    check("notEqualsPredicate", Predicate.allHold([Predicate(path: "/a", test: .notEquals(0))], in: json(#"{"a": 5}"#))
          && !Predicate.allHold([Predicate(path: "/a", test: .notEquals(0))], in: json(#"{"a": 0}"#))
          && Predicate.allHold([Predicate(path: "/a", test: .notEquals(0))], in: json(#"{}"#)))

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

    let pinnedIcon = StatusIcons.icon(runs: [codex, copilot], pinned: "codex")
    check("pinnedShowsOne", pinnedIcon.accessibility == "Codex 45%")
    check("pinnedColors", pinnedIcon.topColor == RGB(0, 130, 0) && pinnedIcon.bottomColor == StatusIcons.green)
    check("stalePinFallsBack", StatusIcons.icon(runs: [codex], pinned: "removed").accessibility == "Codex 45%")
    check("noPinDefaultsToFirst", StatusIcons.icon(runs: [codex, copilot], pinned: nil).accessibility == "Codex 45%")
    let errorIcon = StatusIcons.icon(runs: [failed, copilot], pinned: "codex")
    check("erroredPin", errorIcon.bottom == "ERR" && errorIcon.accessibility == "Codex error: timed out")
    let loadingIcon = StatusIcons.icon(runs: [sampleRun("a", "A", "A", nil), copilot], pinned: nil)
    check("loadingPinShowsDashes", loadingIcon.top == "A" && loadingIcon.bottom == "--")
    check("noRunsPlaceholder", StatusIcons.icon(runs: [], pinned: "codex") == StatusIcons.placeholder)

    let groupRun = sampleRun("antigravity", "Antigravity", "AGY", .success(Report(meters: [
        MeterReport(id: "Gemini Models", label: "Gemini Models", windows: [WindowReport(id: "w", label: "Weekly", usedPercent: 7)]),
        MeterReport(id: "Claude and GPT models", label: "Claude and GPT models", windows: [WindowReport(id: "w", label: "Weekly", usedPercent: 95)]),
    ])))
    let firstGroup = Pin.key(run: "antigravity", meter: "Gemini Models")
    check("effectiveDefault", Pin.effective(nil, in: [codex, copilot]) == "codex" && Pin.effective(nil, in: []) == nil)
    check("effectiveGroupedProvider", Pin.effective("antigravity", in: [groupRun]) == firstGroup
          && Pin.effective("antigravity|gone", in: [groupRun]) == firstGroup && Pin.effective(nil, in: [groupRun]) == firstGroup)
    check("effectiveKeepsValidMeter", Pin.effective("antigravity|Claude and GPT models", in: [groupRun]) == "antigravity|Claude and GPT models")
    let accountPin = "copilot:octocat|premium_interactions"
    check("effectiveKeepsAccountPinWhileConfigRunLoads", Pin.effective(accountPin, in: [codex, sampleRun("copilot", "GitHub Copilot", "GHC", nil)]) == accountPin
          && Pin.effective("copilot:octocat", in: [codex, sampleRun("copilot", "GitHub Copilot", "GHC", nil)]) == "copilot:octocat")
    check("effectiveKeepsAccountPinWhenAccountListFails", Pin.effective(accountPin,
          in: [codex, sampleRun("copilot", "GitHub Copilot", "GHC", .failure(ProviderFailure("gh failed")))]) == accountPin)
    check("effectiveMissingProvider", Pin.effective("nope", in: [codex, groupRun]) == "codex")
    check("effectiveKeepsPinWhileLoading", Pin.effective("antigravity|Claude and GPT models",
          in: [sampleRun("antigravity", "Antigravity", "AGY", nil)]) == "antigravity|Claude and GPT models")
    check("effectiveKeepsPinOnFailure", Pin.effective("antigravity|Claude and GPT models",
          in: [sampleRun("antigravity", "Antigravity", "AGY", .failure(ProviderFailure("x")))]) == "antigravity|Claude and GPT models")
}

func menuChecks() {
    let utc = MenuModel.resetFormatter(timeZone: TimeZone(identifier: "UTC")!)
    check("resetFormat", utc.string(from: Date(timeIntervalSince1970: 1791202619)) == "Oct 5, 12:16 PM")

    let codex = sampleRun("codex", "Codex", "CDX", .success(sampleReport(95)))
    let failed = sampleRun("copilot", "GitHub Copilot", "GHC", .failure(ProviderFailure("timed out")))
    let entries = MenuModel.entries(runs: [codex, failed], pinned: "codex", configErrors: ["bad.json: /id: required"],
                                    updated: "Updated 10:00", launchAtLogin: true, version: "1.0.0",
                                    formatReset: utc.string(from:))
    let cards = entries.compactMap { entry -> Card? in if case .card(let card) = entry { return card }; return nil }
    check("menuOneCardPerRun", cards.map(\.id) == ["codex", "copilot"])
    check("menuCardPinState", cards.map(\.pinned) == [true, false])
    check("menuCardError", cards[1].message == CardNotice(text: "Error: timed out", tone: .red))
    check("menuConfigError", entries.contains(.detail("bad.json: /id: required", .red)))
    check("menuEnds", Array(entries.suffix(3)) == [.version("AI Usage 1.0.0"), .credit(before: "Built with", after: "by g.akrp"), .quit])
    check("menuDefaultPin", MenuModel.entries(runs: [codex, failed], pinned: nil, configErrors: [], updated: nil,
          launchAtLogin: false, version: "1", formatReset: utc.string(from:)).contains { if case .card(let c) = $0 { return c.pinned && c.id == "codex" }; return false })

    let limited = sampleRun("codex", "Codex", "CDX", .success(Report(access: false, meters: [])))
    let limitedEntries = MenuModel.entries(runs: [limited], pinned: nil, configErrors: [], updated: nil,
                                           launchAtLogin: false, version: "1", formatReset: utc.string(from:))
    check("menuLimitReached", limitedEntries.contains { if case .card(let c) = $0 { return c.notices.first == CardNotice(text: "Usage limit reached", tone: .red) }; return false })

    check("scheduleInterval", Schedule.delay(interval: 300, failures: 0) == 300)
    check("scheduleBackoff", (1...8).map { Schedule.delay(interval: 300, failures: $0) } == [60, 120, 240, 480, 960, 1800, 1800, 1800])
    check("versionFormat", AppVersion.current.range(of: #"^\d+\.\d+\.\d+$"#, options: .regularExpression) != nil)
    if let plist = NSDictionary(contentsOfFile: "Resources/Info.plist") {
        check("versionMatchesInfoPlist", plist["CFBundleShortVersionString"] as? String == AppVersion.current)
    } else {
        check("infoPlistReadable", false)
    }
}

func pinChecks() {
    let agy = Report(meters: [
        MeterReport(id: "Gemini Models", label: "Gemini Models", windows: [
            WindowReport(id: "w", label: "Weekly", usedPercent: 7), WindowReport(id: "f", label: "Five Hour", usedPercent: 3)]),
        MeterReport(id: "Claude and GPT models", label: "Claude and GPT models", windows: [
            WindowReport(id: "w", label: "Weekly", usedPercent: 95)]),
    ])
    let run = ProviderRun(id: "antigravity", name: "Antigravity", iconLabel: "AGY", iconColor: RGB(0x42, 0x85, 0xF4),
                          result: .success(agy))
    let gemini = Pin.key(run: "antigravity", meter: "Gemini Models")
    let icon = StatusIcons.icon(runs: [run], pinned: gemini)
    check("meterPinIcon", "\(icon.top)/\(icon.bottom)" == "GEM/7%" && icon.topColor == RGB(0x42, 0x85, 0xF4))
    check("meterPinAccessibility", icon.accessibility == "Antigravity Gemini Models 7%")
    check("meterLabel", [StatusIcons.label(for: "Claude and GPT models"), StatusIcons.label(for: "5-hour"), StatusIcons.label(for: "··")]
        == ["CLA", "5HO", "?"])
    check("missingMeterFallsBackToFirstGroup",
          StatusIcons.icon(runs: [run], pinned: Pin.key(run: "antigravity", meter: "gone")).accessibility == "Antigravity Gemini Models 7%")
    check("pinAccountFallback", Pin.resolve("copilot", in: [
        ProviderRun(id: "copilot:a", name: "A", iconLabel: "GH1", iconColor: .gray, configId: "copilot")])?.run.id == "copilot:a")
    check("pinUnknown", Pin.resolve("nope", in: [run]) == nil && Pin.resolve(nil, in: [run]) == nil)

    let utc = MenuModel.resetFormatter(timeZone: TimeZone(identifier: "UTC")!)
    let entries = MenuModel.entries(runs: [run], pinned: gemini, configErrors: [], updated: nil, launchAtLogin: false,
                                    version: "1", formatReset: utc.string(from:))
    let cards = entries.compactMap { entry -> Card? in if case .card(let card) = entry { return card }; return nil }
    check("menuGroupPinned", cards.count == 1 && cards[0].pinned && cards[0].charts.map(\.pinned) == [true, false])

    let copilot = Report(meters: [
        MeterReport(id: "chat", label: "Chat", windows: [WindowReport(id: "current", usedPercent: 0)]),
        MeterReport(id: "premium", label: "Premium", windows: [WindowReport(id: "current", usedPercent: 79)]),
    ])
    let copilotRun = ProviderRun(id: "copilot:a", name: "Copilot (a)", iconLabel: "GH1", iconColor: .gray,
                                 result: .success(copilot), configId: "copilot")
    let flat = MenuModel.entries(runs: [copilotRun], pinned: nil, configErrors: [], updated: nil, launchAtLogin: false,
                                 version: "1", formatReset: utc.string(from:))
    check("menuFlatMeterPinnable", flat.contains { if case .card(let c) = $0 { return c.charts[0].rows.map(\.pin) == ["copilot:a|premium", "copilot:a|chat"] }; return false })
}

func toggleChecks() {
    let runs = [
        ProviderRun(id: "claude", name: "Claude Code", iconLabel: "CLD", iconColor: .gray, result: .success(Report(meters: []))),
        ProviderRun(id: "copilot:a", name: "Copilot · a", iconLabel: "GH1", iconColor: .gray, configId: "copilot"),
        ProviderRun(id: "copilot:b", name: "Copilot · b", iconLabel: "GH2", iconColor: .gray, configId: "copilot"),
    ]
    let toggles = [ProviderToggle(id: "claude", name: "Claude Code", enabled: true),
                   ProviderToggle(id: "copilot", name: "GitHub Copilot", enabled: false)]
    check("visibleRunsHideDisabled", ProviderToggle.visible(runs, disabled: ["copilot"]).map(\.id) == ["claude"])
    check("visibleRunsAllEnabled", ProviderToggle.visible(runs, disabled: []).count == 3)
    let entries = MenuModel.entries(runs: [runs[0]], pinned: nil, configErrors: [], providers: toggles, updated: nil,
                                    launchAtLogin: false, version: "1", formatReset: { _ in "" })
    check("menuProvidersSubmenu", entries.contains(.providers(toggles)))
    check("menuProvidersBeforeLaunchAtLogin",
          entries.firstIndex(of: .providers(toggles))! < entries.firstIndex(of: .launchAtLogin(false))!)
    check("menuDisabledProviderHidden", !entries.contains { if case .card(let card) = $0 { return card.id.hasPrefix("copilot") }; return false })
    let none = MenuModel.entries(runs: [], pinned: nil, configErrors: [], providers: toggles.map { ProviderToggle(id: $0.id, name: $0.name, enabled: false) },
                                 updated: nil, launchAtLogin: false, version: "1", formatReset: { _ in "" })
    check("menuAllDisabledHint", none.contains(.detail("All providers are off", .secondary)))

    let tiers = Report(meters: [
        MeterReport(id: "plan", label: "Plan", windows: [
            WindowReport(id: "weekly", label: "Weekly", usedPercent: 80, durationSeconds: 7 * 86_400),
            WindowReport(id: "session", label: "Session", usedPercent: 10, durationSeconds: 5 * 3600)]),
    ])
    check("headlineSessionFirst", tiers.headlinePercent == 10 && tiers.maxPercent == 80)
    let weeklyOnly = Report(meters: [
        MeterReport(id: "chat", label: "Chat", windows: [WindowReport(id: "current", usedPercent: 90)]),
        MeterReport(id: "plan", label: "Plan", windows: [WindowReport(id: "w", label: "Weekly", usedPercent: 40)]),
        MeterReport(id: "premium_interactions", label: "Premium Interactions", windows: [WindowReport(id: "current", usedPercent: 5)]),
    ])
    check("headlineWeeklyBeforePremium", weeklyOnly.headlinePercent == 40)
    let premium = Report(meters: [
        MeterReport(id: "chat", label: "Chat", windows: [WindowReport(id: "current", usedPercent: 90)]),
        MeterReport(id: "premium_interactions", label: "Premium Interactions", windows: [WindowReport(id: "current", usedPercent: 5)]),
    ])
    check("headlinePremiumBeforeChat", premium.headlinePercent == 5)
    check("headlineFallsBackToMax", Report(meters: [MeterReport(id: "x", label: "X", windows: [
        WindowReport(id: "a", usedPercent: 12), WindowReport(id: "b", usedPercent: 30)])]).headlinePercent == 30)
    let tierRun = ProviderRun(id: "claude", name: "Claude", iconLabel: "CLD", iconColor: .gray, result: .success(tiers))
    let tierIcon = StatusIcons.icon(runs: [tierRun], pinned: "claude")
    check("pinnedProviderShowsHeadline", tierIcon.bottom == "10%" && tierIcon.bottomColor == StatusIcons.orange
          && tierIcon.accessibility == "Claude 10%")
}

func cardChecks() {
    let now = Date(timeIntervalSince1970: 1_000_000)
    func relative(_ seconds: TimeInterval) -> String? { RelativeTime.text(until: now + seconds, now: now) }
    check("relativeTime", [relative(45), relative(1), relative(120), relative(119), relative(3 * 3600), relative(4 * 86_400),
                           relative(86_399), relative(0), relative(-5), relative(3 * 3600 + 25 * 60 + 30), relative(3600 + 60), relative(3600 + 59),
                           relative(4 * 86_400 + 2 * 3600 + 12 * 60 + 5), relative(86_400 + 60), relative(2 * 86_400 + 3600)]
          == ["in 45s", "in 1s", "in 2m", "in 1m", "in 3h", "in 4d", "in 23h 59m", nil, nil,
              "in 3h 25m", "in 1h 1m", "in 1h", "in 4d 2h 12m", "in 1d 1m", "in 2d 1h"])

    func win(_ id: String, _ label: String?, _ percent: Double?, seconds: Int? = nil, reset: TimeInterval? = nil) -> WindowReport {
        WindowReport(id: id, label: label, usedPercent: percent, resetsAt: reset.map { now + $0 }, durationSeconds: seconds)
    }
    func make(_ id: String, _ name: String, _ report: Report) -> ProviderRun {
        ProviderRun(id: id, name: name, iconLabel: "X", iconColor: .gray, result: .success(report))
    }
    func card(_ run: ProviderRun, pinned: String? = nil) -> Card {
        CardModel.card(run, pinned: pinned, now: now, formatReset: { _ in "D" })
    }

    let claude = card(make("claude", "Claude Code", Report(plan: "max", meters: [
        MeterReport(id: "plan", label: "Plan", windows: [win("seven_day", "Weekly", 80, seconds: 604_800),
                                                        win("five_hour", "Session", 10, seconds: 18_000, reset: 120)])])), pinned: "claude")
    check("cardClaudeShape", claude.title == "Claude Code (max)" && claude.charts.count == 1 && claude.charts[0].title == nil
          && claude.charts[0].center == "10%" && claude.pin == "claude" && claude.pinned && claude.overflow.isEmpty)
    check("cardClaudeRings", claude.charts[0].rings == [CardRing(fraction: 0.10, tone: .green), CardRing(fraction: 0.80, tone: .orange)]
          && claude.charts[0].rows.map(\.ringIndex) == [0, 1] && claude.charts[0].rows.map(\.label) == ["Session", "Weekly"]
          && claude.charts[0].rows.allSatisfy { $0.pin == nil })
    let account = card(ProviderRun(id: "copilot:octocat", name: "GitHub Copilot \u{00B7} octocat", iconLabel: "GH1", iconColor: .gray,
                                   result: .success(Report(plan: "pro", meters: [])), configId: "copilot",
                                   providerName: "GitHub Copilot", account: "octocat"))
    check("cardAccountUnderName", account.title == "GitHub Copilot (pro)" && account.subtitle == "octocat" && claude.subtitle == nil)
    let brand = RGB(0xD9, 0x77, 0x57)
    let branded = CardModel.card(ProviderRun(id: "claude", name: "Claude Code", iconLabel: "CLD", iconColor: brand, result: .success(Report(meters: []))),
                                 pinned: nil, now: now, formatReset: { _ in "D" })
    check("cardAccentIsBrandColor", branded.accent == brand && claude.accent == .gray)
    check("cardResetLines", claude.charts[0].rows.map(\.reset) == ["D \u{00B7} in 2m", nil])
    let past = card(make("c", "C", Report(meters: [MeterReport(id: "m", label: "M", windows: [win("a", "Session", 5, seconds: 18_000, reset: -1)])])))
    check("cardResetPast", past.charts[0].rows[0].reset == "resetting\u{2026}")

    let agy = make("antigravity", "Antigravity", Report(meters: [
        MeterReport(id: "Gemini Models", label: "Gemini Models", windows: [win("w", "Weekly", 7), win("f", "Five Hour", 3, seconds: 18_000)]),
        MeterReport(id: "Claude and GPT models", label: "Claude and GPT models", windows: [win("w", "Weekly", 95)]),
    ]))
    let gemini = Pin.key(run: "antigravity", meter: "Gemini Models")
    let agyCard = card(agy, pinned: gemini)
    check("cardGroupedCharts", agyCard.charts.map(\.title) == ["Gemini Models", "Claude and GPT models"]
          && agyCard.charts[0].rings == [CardRing(fraction: 0.03, tone: .green), CardRing(fraction: 0.07, tone: .green)]
          && agyCard.charts[0].center == "3%" && agyCard.charts[1].rings == [CardRing(fraction: 0.95, tone: .red)]
          && agyCard.charts[1].center == "95%")
    check("cardGroupedPins", agyCard.pin == gemini && agyCard.pinned && agyCard.charts[0].pin == gemini && agyCard.charts[0].pinned
          && !agyCard.charts[1].pinned && !card(agy, pinned: "antigravity|Claude and GPT models").pinned
          && CardModel.titlePin(agy) == gemini)

    let copilot = make("copilot:a", "Copilot (a)", Report(meters: [
        MeterReport(id: "chat", label: "Chat", windows: [win("current", nil, 0)]),
        MeterReport(id: "premium_interactions", label: "Premium Interactions", windows: [win("current", nil, 79)]),
        MeterReport(id: "completions", label: "Completions", windows: [win("current", nil, 5)]),
    ]))
    let copilotCard = card(copilot, pinned: "copilot:a|chat")
    check("cardMergedChart", copilotCard.charts.count == 1 && copilotCard.charts[0].rows.map(\.label) == ["Premium Interactions", "Chat", "Completions"]
          && copilotCard.charts[0].rows.map(\.pin) == ["copilot:a|premium_interactions", "copilot:a|chat", "copilot:a|completions"]
          && copilotCard.charts[0].rows.map(\.pinned) == [false, true, false] && copilotCard.pin == "copilot:a" && !copilotCard.pinned
          && copilotCard.charts[0].center == "79%" && copilotCard.charts[0].rows.allSatisfy { $0.reset == nil })

    let five = card(make("m", "M", Report(meters: [MeterReport(id: "m", label: "M", windows: (1...5).map { win("w\($0)", "W\($0)", Double($0)) })])))
    check("cardOverflowWindows", five.charts[0].rings.count == 4 && five.charts[0].rows.count == 5 && five.charts[0].rows[4].ringIndex == nil)
    let groups = card(make("g", "G", Report(meters: (1...5).map { MeterReport(id: "g\($0)", label: "G\($0)", windows: [win("w", "Weekly", Double($0))]) })))
    check("cardOverflowGroups", groups.charts.count == 4 && groups.overflow.count == 1 && groups.overflow[0].label == "G5"
          && groups.overflow[0].percentText == "5%" && groups.overflow[0].ringIndex == nil)
    let odd = card(make("o", "O", Report(meters: [MeterReport(id: "m", label: "M", windows: [win("a", "A", nil), win("b", "B", 150)])])))
    check("cardRingClamp", odd.charts[0].rings == [CardRing(fraction: nil, tone: .secondary), CardRing(fraction: 1, tone: .red)]
          && odd.charts[0].rows.map(\.percentText) == ["?", "150%"])

    let failedCard = card(ProviderRun(id: "x", name: "X", iconLabel: "X", iconColor: .gray, result: .failure(ProviderFailure("boom"))))
    check("cardFailure", failedCard.message == CardNotice(text: "Error: boom", tone: .red) && failedCard.charts.isEmpty && failedCard.title == "X")
    let loadingCard = card(ProviderRun(id: "x", name: "X", iconLabel: "X", iconColor: .gray))
    check("cardLoading", loadingCard.message == CardNotice(text: "Loading\u{2026}", tone: .secondary) && loadingCard.charts.isEmpty)
    let flags = card(make("f", "F", Report(available: false, access: false, meters: [])))
    check("cardNotices", flags.notices == [CardNotice(text: "Usage limit reached", tone: .red),
                                           CardNotice(text: "Plan limits don't apply to this account", tone: .secondary),
                                           CardNotice(text: "No usage data", tone: .secondary)])

    // Hiding a card: only the panel changes; the provider keeps running and can still be pinned.
    func runs(_ ids: [String]) -> [ProviderRun] {
        ids.map { ProviderRun(id: $0, name: "N-\($0)", iconLabel: "X", iconColor: .gray, result: .success(Report(meters: [])), configId: String($0.prefix { $0 != ":" })) }
    }
    func entries(_ ids: [String], hidden: Set<String>, pinned: String? = nil) -> [MenuEntry] {
        MenuModel.entries(runs: runs(ids), pinned: pinned, configErrors: [], updated: nil, launchAtLogin: false, version: "1",
                          hidden: hidden, formatReset: { _ in "" })
    }
    func cardIds(_ list: [MenuEntry]) -> [String] { list.compactMap { if case .card(let card) = $0 { return card.id }; return nil } }
    let all = entries(["claude", "copilot:a", "copilot:b"], hidden: [])
    check("hideNothingByDefault", cardIds(all) == ["claude", "copilot:a", "copilot:b"] && !all.contains { if case .hiddenCards = $0 { return true }; return false })
    let some = entries(["claude", "copilot:a", "copilot:b"], hidden: ["copilot:a"])
    check("hideOneAccountCard", cardIds(some) == ["claude", "copilot:b"])
    check("hiddenCardsListed", some.contains(.hiddenCards([HiddenCard(id: "copilot:a", name: "N-copilot:a")])))
    check("hiddenStaleIdIgnored", !entries(["claude"], hidden: ["gone"]).contains { if case .hiddenCards = $0 { return true }; return false })
    // Cards are outlined, so no separator between them; one after the last.
    let kinds = all.prefix(4).map { entry -> String in
        if case .card = entry { return "card" }
        return entry == .separator ? "separator" : "other"
    }
    check("noSeparatorBetweenCards", kinds == ["card", "card", "card", "separator"])
    let none = entries(["claude"], hidden: ["claude"])
    check("allCardsHiddenHint", cardIds(none).isEmpty && none.contains(.detail("All cards are hidden", .secondary)))
    check("hiddenCardStillDefaultPin", cardIds(some).first == "claude" && Pin.effective(nil, in: runs(["copilot:a"])) == "copilot:a")
    check("hiddenCardKeepsPinState", entries(["claude", "copilot:a"], hidden: ["claude"], pinned: "claude").contains { if case .card(let c) = $0 { return c.id == "copilot:a" && !c.pinned }; return false })
}

