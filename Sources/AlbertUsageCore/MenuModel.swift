import Foundation

public enum Tone: Equatable { case normal, secondary, green, orange, red }

public enum MenuEntry: Equatable {
    /// A provider's title row. Choosing it pins the provider.
    case provider(id: String, title: String, pinned: Bool)
    case detail(String, Tone)
    case separator
    case cycleAll(checked: Bool)
    case refresh
    case updated(String)
    case launchAtLogin(Bool)
    case openProvidersFolder
    case version(String)
    case quit
}

public enum MenuModel {
    public static func entries(runs: [ProviderRun], pinned: String?, configErrors: [String], updated: String?,
                               launchAtLogin: Bool, version: String, formatReset: (Date) -> String) -> [MenuEntry] {
        var entries: [MenuEntry] = []
        for run in runs {
            entries += providerEntries(run, pinned: run.id == pinned, formatReset: formatReset)
            entries.append(.separator)
        }
        if !configErrors.isEmpty {
            entries += configErrors.map { .detail($0, .red) }
            entries.append(.separator)
        }
        entries.append(.cycleAll(checked: !runs.contains { $0.id == pinned }))
        entries.append(.refresh)
        if let updated { entries.append(.updated(updated)) }
        entries.append(.separator)
        entries += [.launchAtLogin(launchAtLogin), .openProvidersFolder, .separator, .version(version), .quit]
        return entries
    }

    static let indent = "\u{00A0}\u{00A0}\u{00A0}"

    static func providerEntries(_ run: ProviderRun, pinned: Bool, formatReset: (Date) -> String) -> [MenuEntry] {
        let mark = pinned ? "\u{2605}" : "\u{2606}"  // ★ / ☆
        switch run.result {
        case nil:
            return [.provider(id: run.id, title: "\(mark) \(run.name)", pinned: pinned),
                    .detail(indent + "Loading…", .secondary)]
        case .failure(let failure)?:
            return [.provider(id: run.id, title: "\(mark) \(run.name)", pinned: pinned),
                    .detail(indent + "Error: \(failure.message)", .red)]
        case .success(let report)?:
            let plan = report.plan.map { " (\($0))" } ?? ""
            var entries: [MenuEntry] = [.provider(id: run.id, title: "\(mark) \(run.name)\(plan)", pinned: pinned)]
            if report.access == false { entries.append(.detail(indent + "Usage limit reached", .red)) }
            if report.available == false {
                entries.append(.detail(indent + "Plan limits don't apply to this account", .secondary))
            }
            if report.meters.isEmpty { entries.append(.detail(indent + "No usage data", .secondary)) }
            for meter in report.meters {
                if meter.windows.count == 1, meter.windows[0].label == nil {
                    entries.append(windowEntry(meter.windows[0], label: meter.label, nested: false, formatReset: formatReset))
                    continue
                }
                let nested = report.meters.count > 1
                if nested { entries.append(.detail(indent + meter.label, .secondary)) }
                for window in meter.windows {
                    entries.append(windowEntry(window, label: window.label ?? window.id, nested: nested,
                                               formatReset: formatReset))
                }
            }
            return entries
        }
    }

    static func windowEntry(_ window: WindowReport, label: String, nested: Bool,
                            formatReset: (Date) -> String) -> MenuEntry {
        let value = window.usedPercent.map(StatusIcons.percentText) ?? "?"
        let reset = window.resetsAt.map { " (resets \(formatReset($0)))" } ?? ""
        let tone: Tone = window.usedPercent.map { p in p >= 90 ? .red : p >= 70 ? .orange : .green } ?? .secondary
        return .detail(indent + (nested ? indent : "") + "\(label): \(value)\(reset)", tone)
    }

    /// Readable local time, for example "Oct 2, 2026 6:23 PM".
    public static func resetFormatter(timeZone: TimeZone = .current) -> DateFormatter {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = timeZone
        formatter.dateFormat = "MMM d, yyyy h:mm a"
        return formatter
    }
}

/// When each provider runs next.
public enum Schedule {
    /// After `failures` failures in a row: 1, 2, 4, … minutes, at most 30. Otherwise the interval.
    public static func delay(interval: Int, failures: Int) -> TimeInterval {
        guard failures > 0 else { return TimeInterval(interval) }
        return min(60 * pow(2, Double(min(failures, 6) - 1)), 1800)
    }
}
