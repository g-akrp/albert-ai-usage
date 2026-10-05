import Foundation

public struct CardRing: Equatable {
    /// 0 to 1, clamped; `nil` when the window has no percent.
    public let fraction: Double?
    public let tone: Tone

    public init(fraction: Double?, tone: Tone) { (self.fraction, self.tone) = (fraction, tone) }
}

public struct CardRow: Equatable {
    public let label: String
    public let percentText: String
    public let tone: Tone
    /// `Oct 5, 3:20 PM · in 2 minutes`, `resetting…`, or `nil` without a reset time.
    public let reset: String?
    /// The ring this row belongs to; `nil` for rows past the fourth ring.
    public let ringIndex: Int?
    /// The pin this row sets, if it is pinnable on its own.
    public let pin: String?
    public let pinned: Bool
}

public struct CardChart: Equatable {
    /// The group's name; `nil` when the provider has no groups.
    public let title: String?
    public let pin: String?
    public let pinned: Bool
    public let center: String
    public let rings: [CardRing]
    public let rows: [CardRow]
}

public struct CardNotice: Equatable {
    public let text: String
    public let tone: Tone

    public init(text: String, tone: Tone) { (self.text, self.tone) = (text, tone) }
}

public struct Card: Equatable {
    public let id: String
    public let title: String
    /// The account name, shown under the title; `nil` for a provider without accounts.
    public let subtitle: String?
    /// The provider's brand color; the rings are shades of it.
    public let accent: RGB
    /// What the title's pin sets: the provider, or the first group of a provider with groups.
    public let pin: String
    public let pinned: Bool
    /// Loading or error text; when set there are no charts.
    public let message: CardNotice?
    public let notices: [CardNotice]
    public let charts: [CardChart]
    /// Groups past the fourth chart, as rows without rings.
    public let overflow: [CardRow]
}

public enum CardModel {
    static let maxRings = 4, maxCharts = 4

    /// A provider with several model groups (Antigravity): one chart per meter. Meters that are each
    /// one unlabeled window (Copilot) are not groups; they share one chart.
    public static func isGrouped(_ report: Report) -> Bool {
        report.meters.count > 1 && !report.meters.allSatisfy(isFlat)
    }

    static func isFlat(_ meter: MeterReport) -> Bool { meter.windows.count == 1 && meter.windows[0].label == nil }

    public static func titlePin(_ run: ProviderRun) -> String {
        if case .success(let report)? = run.result, isGrouped(report), let first = report.meters.first {
            return Pin.key(run: run.id, meter: first.id)
        }
        return run.id
    }

    public static func card(_ run: ProviderRun, pinned: String?, now: Date, formatReset: (Date) -> String) -> Card {
        let pin = titlePin(run)
        func make(_ title: String, message: CardNotice? = nil, notices: [CardNotice] = [], charts: [CardChart] = [],
                  overflow: [CardRow] = []) -> Card {
            Card(id: run.id, title: title, subtitle: run.account, accent: run.iconColor, pin: pin, pinned: pinned == pin, message: message, notices: notices,
                 charts: charts, overflow: overflow)
        }
        switch run.result {
        case nil:
            return make(run.providerName, message: CardNotice(text: "Loading\u{2026}", tone: .secondary))
        case .failure(let failure)?:
            return make(run.providerName, message: CardNotice(text: "Error: \(failure.message)", tone: .red))
        case .success(let report)?:
            let title = run.providerName + (report.plan.map { " (\($0))" } ?? "")
            var notices: [CardNotice] = []
            if report.access == false { notices.append(CardNotice(text: "Usage limit reached", tone: .red)) }
            if report.available == false {
                notices.append(CardNotice(text: "Plan limits don't apply to this account", tone: .secondary))
            }
            if report.meters.isEmpty { notices.append(CardNotice(text: "No usage data", tone: .secondary)) }
            return withoutActuallyEscaping(formatReset) { format in
                let env = Env(run: run.id, pinned: pinned, now: now, formatReset: format)
                var charts: [CardChart] = [], overflow: [CardRow] = []
                if isGrouped(report) {
                    charts = report.meters.prefix(maxCharts).map { env.groupChart($0) }
                    overflow = report.meters.dropFirst(maxCharts).map { env.groupRow($0) }
                } else if report.meters.count > 1 {
                    charts = [env.mergedChart(report.meters)]
                } else if let meter = report.meters.first {
                    charts = [env.windowChart(meter, title: nil, pin: nil)]
                }
                return make(title, notices: notices, charts: charts, overflow: overflow)
            }
        }
    }

    static func tone(_ percent: Double?) -> Tone {
        percent.map { $0 >= 90 ? .red : $0 >= 70 ? .orange : .green } ?? .secondary
    }

    private struct Env {
        let run: String, pinned: String?, now: Date, formatReset: (Date) -> String

        func percentText(_ percent: Double?) -> String { percent.map(StatusIcons.percentText) ?? "?" }

        func ring(_ percent: Double?) -> CardRing {
            CardRing(fraction: percent.map { min(max($0 / 100, 0), 1) }, tone: tone(percent))
        }

        func reset(_ window: WindowReport?) -> String? {
            guard let date = window?.resetsAt else { return nil }
            return RelativeTime.text(until: date, now: now).map { "\(formatReset(date)) \u{00B7} \($0)" } ?? "resetting\u{2026}"
        }

        func center(_ meters: [MeterReport]) -> String { percentText(Report(meters: meters).headlinePercent) }

        func row(_ window: WindowReport, label: String, ring: Int?, pin: String? = nil) -> CardRow {
            CardRow(label: label, percentText: percentText(window.usedPercent), tone: tone(window.usedPercent),
                    reset: reset(window), ringIndex: ring, pin: pin, pinned: pin != nil && pin == pinned)
        }

        /// One ring per window, session first, then weekly, then the rest in order.
        func windowChart(_ meter: MeterReport, title: String?, pin: String?) -> CardChart {
            let ordered = meter.windows.enumerated().sorted { ($0.element.kind, $0.offset) < ($1.element.kind, $1.offset) }.map(\.element)
            let rings = ordered.prefix(CardModel.maxRings).map { ring($0.usedPercent) }
            let rows = ordered.enumerated().map { index, window in
                row(window, label: window.label ?? (isFlat(meter) ? meter.label : window.id),
                    ring: index < CardModel.maxRings ? index : nil)
            }
            return CardChart(title: title, pin: pin, pinned: pin != nil && pin == pinned, center: center([meter]),
                             rings: Array(rings), rows: rows)
        }

        func groupChart(_ meter: MeterReport) -> CardChart {
            windowChart(meter, title: meter.label, pin: Pin.key(run: run, meter: meter.id))
        }

        func groupRow(_ meter: MeterReport) -> CardRow {
            let percent = Report(meters: [meter]).headlinePercent
            let key = Pin.key(run: run, meter: meter.id)
            return CardRow(label: meter.label, percentText: percentText(percent), tone: tone(percent), reset: nil,
                           ringIndex: nil, pin: key, pinned: key == pinned)
        }

        /// One ring per meter, premium interactions first, then in order.
        func mergedChart(_ meters: [MeterReport]) -> CardChart {
            func isPremium(_ m: MeterReport) -> Bool { (m.id + m.label).lowercased().contains("premium") }
            let ordered = meters.enumerated().sorted { (isPremium($0.element) ? 0 : 1, $0.offset) < (isPremium($1.element) ? 0 : 1, $1.offset) }
                .map(\.element)
            let rings = ordered.prefix(CardModel.maxRings).map { ring($0.maxPercent) }
            let rows = ordered.enumerated().map { index, meter in
                row(WindowReport(id: meter.id, usedPercent: meter.maxPercent, resetsAt: meter.windows.first?.resetsAt),
                    label: meter.label, ring: index < CardModel.maxRings ? index : nil, pin: Pin.key(run: run, meter: meter.id))
            }
            return CardChart(title: nil, pin: nil, pinned: false, center: center(meters), rings: Array(rings), rows: rows)
        }
    }
}
