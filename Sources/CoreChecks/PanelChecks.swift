import AIUsageCore
import Foundation

private func card(_ charts: [[Double?]]) -> Card {
    Card(id: "c", title: "C", subtitle: nil, accent: RGB(0, 0, 0), pin: "c", pinned: false, message: nil, notices: [],
         charts: charts.map { rings in
             CardChart(title: nil, pin: nil, pinned: false, center: "", rings: rings.map { CardRing(fraction: $0, tone: .green) }, rows: [])
         }, overflow: [])
}

/// Fractions equal within rounding.
private func near(_ a: [[Double?]], _ b: [[Double?]]) -> Bool {
    a.count == b.count && zip(a, b).allSatisfy { x, y in
        x.count == y.count && zip(x, y).allSatisfy { p, q in
            switch (p, q) {
            case (nil, nil): return true
            case (let p?, let q?): return abs(p - q) < 1e-9
            default: return false
            }
        }
    }
}

func panelChecks() {
    let toggles = [ProviderToggle(id: "a", name: "A", enabled: true), ProviderToggle(id: "b", name: "B", enabled: false)]
    let entries: [MenuEntry] = [.refresh, .updated("Updated 1:00 PM"), .separator, .providers(toggles),
                                .hiddenCards([HiddenCard(id: "h", name: "Codex")]), .launchAtLogin(true), .openProvidersFolder, .quit]
    let closed = PanelModel.rows(entries, expanded: [], refreshing: false)
    check("rowsClosedCount", closed.count == 8)
    check("rowsRefreshShortcut", closed[0].shortcut == "⌘R" && !closed[0].spinning)
    check("rowsUpdatedDisabled", !closed[1].enabled)
    check("rowsProvidersChevron", closed[3].expanded == false)
    check("rowsLaunchChecked", closed[5].checked == true)
    let open = PanelModel.rows(entries, expanded: [PanelModel.providersSection, PanelModel.hiddenSection], refreshing: true)
    check("rowsOpenCount", open.count == 11)
    check("rowsProviderRows", open[4].kind == .provider(id: "a") && open[4].checked == true && open[5].checked == false && open[4].indented)
    check("rowsShowCard", open[7].kind == .showCard(id: "h") && open[7].title == "Show Codex")
    check("rowsSpinning", open[0].spinning)
    check("cardsSkipped", PanelModel.cards(entries).isEmpty)
    check("cardsKept", PanelModel.cards([.card(card([[0.1]])), .separator]).count == 1)
    check("rowsDetailTone", PanelModel.rows([.detail("bad", .red)], expanded: [], refreshing: false).first?.tone == .red)

    let enabled = [true, false, true, false]
    check("highlightFirst", PanelModel.highlight(from: nil, step: 1, enabled: enabled) == 0)
    check("highlightLast", PanelModel.highlight(from: nil, step: -1, enabled: enabled) == 2)
    check("highlightSkips", PanelModel.highlight(from: 0, step: 1, enabled: enabled) == 2)
    check("highlightNoWrap", PanelModel.highlight(from: 2, step: 1, enabled: enabled) == 2)
    check("highlightNone", PanelModel.highlight(from: nil, step: 1, enabled: [false]) == nil)

    let stacked = PanelLayout.stack([10, 20, 5])
    check("stackTops", stacked.tops == [0, 10, 30] && stacked.total == 35)
    check("stackEmpty", PanelLayout.stack([]).total == 0)
    check("heightCapped", PanelLayout.height(content: 900, available: 600) == 600 && PanelLayout.height(content: 300, available: 600) == 300)
    let screen = CGRect(x: 0, y: 0, width: 1000, height: 800), size = CGSize(width: 320, height: 400)
    check("originUnderAnchor", PanelLayout.origin(anchor: CGRect(x: 100, y: 776, width: 30, height: 24), size: size, screen: screen) == CGPoint(x: 100, y: 372))
    check("originClampsRight", PanelLayout.origin(anchor: CGRect(x: 950, y: 776, width: 30, height: 24), size: size, screen: screen).x == 672)
    check("originOtherScreen", PanelLayout.origin(anchor: CGRect(x: -500, y: 776, width: 30, height: 24), size: size,
                                                  screen: CGRect(x: -1000, y: 0, width: 1000, height: 800)).x == -500)

    check("tweenValueClamped", Tween.value(0, 10, 2) == 10 && Tween.value(0, 10, -1) == 0)
    check("tweenIntroFromZero", near(Tween.fractions(from: nil, to: card([[0.4]]), progress: 0.5), [[0.2]]))
    check("tweenStartsFromScreen", near(Tween.fractions(from: card([[0.2]]), to: card([[0.6]]), progress: 0.5), [[0.4]]))
    check("tweenRingAdded", near(Tween.fractions(from: card([[0.5]]), to: card([[0.5, 0.8]]), progress: 0.5), [[0.5, 0.4]]))
    check("tweenChartRemoved", Tween.fractions(from: card([[0.5], [0.5]]), to: card([[0.5]]), progress: 0.5).count == 1)
    check("tweenNilStaysNil", near(Tween.fractions(from: card([[0.5]]), to: card([[nil]]), progress: 0.5), [[nil]]))
    check("percentCounts", Tween.percent(from: "3%", to: "13%", progress: 0.5) == "8%")
    check("percentIntro", Tween.percent(from: nil, to: "40%", progress: 0.5) == "20%")
    check("percentOtherText", Tween.percent(from: "3%", to: "63% · 6280/10000 credits", progress: 0.5) == "63% · 6280/10000 credits")
    check("percentDone", Tween.percent(from: "3%", to: "13%", progress: 1) == "13%")
}
