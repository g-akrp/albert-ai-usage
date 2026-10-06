import Foundation

public enum Tone: Equatable { case normal, secondary, green, orange, red }

public enum MenuEntry: Equatable {
    /// One provider run, drawn as a card. Its pins set `Pin` keys.
    case card(Card)
    case detail(String, Tone)
    case separator
    /// The Providers submenu: one checkbox per provider file.
    case providers([ProviderToggle])
    /// Cards hidden from the panel, to show again. Their providers keep running.
    case hiddenCards([HiddenCard])
    case refresh
    case updated(String)
    case launchAtLogin(Bool)
    case openProvidersFolder
    case version(String)
    /// "Built with ♥ by g.akrp": the heart is drawn between the two texts.
    case credit(before: String, after: String)
    case quit
}

/// A card (one provider run, such as one Copilot account) hidden from the panel.
public struct HiddenCard: Equatable {
    public let id: String
    public let name: String

    public init(id: String, name: String) { (self.id, self.name) = (id, name) }
}

/// Whether a provider file is monitored. Off: it never runs and is hidden from the menu bar and menu.
public struct ProviderToggle: Equatable {
    public let id: String
    public let name: String
    public let enabled: Bool

    public init(id: String, name: String, enabled: Bool) { (self.id, self.name, self.enabled) = (id, name, enabled) }

    /// The runs of providers that are on.
    public static func visible(_ runs: [ProviderRun], disabled: Set<String>) -> [ProviderRun] {
        runs.filter { !disabled.contains($0.configId) }
    }
}

public enum MenuModel {
    public static func entries(runs: [ProviderRun], pinned: String?, configErrors: [String],
                               providers: [ProviderToggle] = [], updated: String?,
                               launchAtLogin: Bool, version: String, hidden: Set<String> = [], order: [String] = [], now: Date = Date(),
                               formatReset: (Date) -> String) -> [MenuEntry] {
        var entries: [MenuEntry] = []
        let pin = Pin.effective(pinned, in: runs)
        let shown = CardOrder.sorted(runs, saved: order).filter { !hidden.contains($0.id) }
        for run in shown {
            entries.append(.card(CardModel.card(run, pinned: pin, now: now, formatReset: formatReset)))
        }
        if !shown.isEmpty { entries.append(.separator) }
        if runs.isEmpty, !providers.isEmpty, !providers.contains(where: \.enabled) {
            entries += [.detail("All providers are off", .secondary), .separator]
        }
        if !runs.isEmpty, shown.isEmpty { entries += [.detail("All cards are hidden", .secondary), .separator] }
        if !configErrors.isEmpty {
            entries += configErrors.map { .detail($0, .red) }
            entries.append(.separator)
        }
        entries.append(.refresh)
        if let updated { entries.append(.updated(updated)) }
        entries.append(.separator)
        if !providers.isEmpty { entries.append(.providers(providers)) }
        let hiddenRuns = runs.filter { hidden.contains($0.id) }.map { HiddenCard(id: $0.id, name: $0.name) }
        if !hiddenRuns.isEmpty { entries.append(.hiddenCards(hiddenRuns)) }
        entries += [.launchAtLogin(launchAtLogin), .openProvidersFolder, .separator,
                     .version("AI Usage \(version)"), .credit(before: "Built with", after: "by g.akrp"), .quit]
        return entries
    }

    /// Readable local time, for example "Oct 2, 2026 6:23 PM".
    public static func resetFormatter(timeZone: TimeZone = .current) -> DateFormatter {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = timeZone
        formatter.dateFormat = "MMM d, h:mm a"
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
