import Foundation

/// What one provider run produced, independent of the provider's own JSON shape.
public struct Report: Equatable {
    public var plan: String?
    public var available: Bool?
    public var access: Bool?
    public var meters: [MeterReport]

    public init(plan: String? = nil, available: Bool? = nil, access: Bool? = nil, meters: [MeterReport]) {
        (self.plan, self.available, self.access, self.meters) = (plan, available, access, meters)
    }

    /// The most urgent number: the highest used percent across every window.
    public var maxPercent: Double? {
        meters.flatMap(\.windows).compactMap(\.usedPercent).max()
    }

    /// The number the menu bar shows for the provider: the first of session, weekly, premium
    /// interactions that the report has (the highest percent within that kind), else `maxPercent`.
    public var headlinePercent: Double? {
        let windows = meters.flatMap(\.windows)
        func top(_ matches: (WindowReport) -> Bool) -> Double? { windows.filter(matches).compactMap(\.usedPercent).max() }
        func named(_ window: WindowReport, _ name: String, _ seconds: Int) -> Bool {
            window.durationSeconds == seconds || window.label?.lowercased().contains(name) == true
        }
        let premium = meters.filter { ($0.id + $0.label).lowercased().contains("premium") }
            .flatMap(\.windows).compactMap(\.usedPercent).max()
        return top { named($0, "session", 5 * 3600) || named($0, "five hour", 5 * 3600) }
            ?? top { named($0, "weekly", 7 * 86_400) }
            ?? premium
            ?? maxPercent
    }
}

public struct MeterReport: Equatable {
    public var id: String
    public var label: String
    public var windows: [WindowReport]

    public init(id: String, label: String, windows: [WindowReport]) {
        (self.id, self.label, self.windows) = (id, label, windows)
    }

    public var maxPercent: Double? {
        windows.compactMap(\.usedPercent).max()
    }
}

public struct WindowReport: Equatable {
    public var id: String
    public var label: String?
    public var usedPercent: Double?
    public var resetsAt: Date?
    public var durationSeconds: Int?

    public init(id: String, label: String? = nil, usedPercent: Double? = nil, resetsAt: Date? = nil,
                durationSeconds: Int? = nil) {
        (self.id, self.label, self.usedPercent, self.resetsAt, self.durationSeconds) =
            (id, label, usedPercent, resetsAt, durationSeconds)
    }
}

public struct ProviderFailure: Error, Equatable {
    public let message: String
    public init(_ message: String) { self.message = message }
}

/// Applies a config's `map` to the JSON its source produced.
public enum Mapper {
    public static func apply(_ map: MapSpec, to captured: Any) -> Report {
        let root = map.root.flatMap { JSON.resolve(captured, $0) } ?? captured
        let plan = map.plan.flatMap { spec in
            JSON.idString(JSON.resolve(root, spec.path)).map { spec.names[$0] ?? $0 }
        }
        var meters: [MeterReport] = []
        for spec in map.meters {
            for meter in mapMeter(spec, root) where !meters.contains(where: { $0.id == meter.id }) {
                meters.append(meter)
            }
        }
        return Report(
            plan: plan,
            available: map.available.flatMap { JSON.bool(JSON.resolve(root, $0)) },
            access: map.access.flatMap { JSON.bool(JSON.resolve(root, $0)) },
            meters: meters)
    }

    static func scoped(_ value: Any, _ scope: Scope) -> [(key: String?, value: Any)] {
        switch scope {
        case .current:
            return [(nil, value)]
        case .select(let path):
            guard let found = JSON.resolve(value, path), !JSON.isNull(found) else { return [] }
            return [(nil, found)]
        case .each(let path):
            return (JSON.resolve(value, path) as? [Any] ?? []).filter { !JSON.isNull($0) }.map { (nil, $0) }
        case .eachEntry(let path):
            let object = JSON.resolve(value, path) as? [String: Any] ?? [:]
            return object.keys.sorted().compactMap { key in JSON.isNull(object[key]) ? nil : (key, object[key]!) }
        }
    }

    private static func mapMeter(_ spec: MeterSpec, _ root: Any) -> [MeterReport] {
        scoped(root, spec.scope).compactMap { key, value in
            guard Predicate.allHold(spec.match, in: value), let id = resolveId(spec.id, value, key),
                  let label = resolveLabel(spec.label, value, key) else { return nil }
            var windows: [WindowReport] = []
            for windowSpec in spec.windows {
                for window in mapWindow(windowSpec, value) where !windows.contains(where: { $0.id == window.id }) {
                    windows.append(window)
                }
            }
            return windows.isEmpty ? nil : MeterReport(id: id, label: label, windows: windows)
        }
    }

    private static func mapWindow(_ spec: WindowSpec, _ meterValue: Any) -> [WindowReport] {
        scoped(meterValue, spec.scope).compactMap { key, value in
            let used = spec.used.flatMap { Convert.usedPercent(JSON.resolve(value, $0.path), $0.as) }
            let resets = spec.resetsAt.flatMap { Convert.date(JSON.resolve(value, $0.path), $0.as) }
            // A window with neither a usage value nor a reset time is skipped.
            guard Predicate.allHold(spec.match, in: value), used != nil || resets != nil, let id = resolveId(spec.id, value, key) else { return nil }
            let duration = spec.duration.flatMap { Convert.durationSeconds(value, $0) }
            return WindowReport(
                id: id,
                label: spec.label.flatMap { resolveLabel($0, value, key) } ?? duration.map(Convert.durationLabel),
                usedPercent: used, resetsAt: resets, durationSeconds: duration)
        }
    }

    private static func resolveId(_ spec: IdSpec, _ value: Any, _ key: String?) -> String? {
        switch spec {
        case .literal(let text): return text
        case .path(let path): return JSON.idString(JSON.resolve(value, path))
        case .entryKey: return key
        }
    }

    private static func resolveLabel(_ spec: LabelSpec, _ value: Any, _ key: String?) -> String? {
        switch spec {
        case .text(let text): return text
        case .key(let key): return translate(key)
        case .entryKey: return key
        case .path(let path, let fallback):
            return JSON.idString(JSON.resolve(value, path)) ?? fallback.flatMap { resolveLabel($0, value, key) }
        }
    }

    static func translate(_ key: String) -> String {
        ["plan": "Plan", "session": "Session", "weekly": "Weekly", "monthly": "Monthly"][key] ?? key
    }
}

/// The conversions behind each `as` value.
public enum Convert {
    public static func usedPercent(_ value: Any?, _ kind: UsedAs) -> Double? {
        guard let n = JSON.number(value) else { return nil }
        switch kind {
        case .percent: return n
        case .fraction: return n * 100
        case .remainingFraction: return (1 - n) * 100
        case .remainingPercent: return 100 - n
        }
    }

    public static func date(_ value: Any?, _ kind: ResetAs) -> Date? {
        switch kind {
        case .iso8601: return (value as? String).flatMap(parseISO8601)
        case .epochSeconds: return JSON.number(value).map { Date(timeIntervalSince1970: $0) }
        case .epochMillis: return JSON.number(value).map { Date(timeIntervalSince1970: $0 / 1000) }
        }
    }

    /// RFC 3339 with `Z` or an offset, with or without fractional seconds of any length.
    public static func parseISO8601(_ text: String) -> Date? {
        let formatter = ISO8601DateFormatter()
        if let date = formatter.date(from: text) { return date }
        // Drop the fraction, then add it back: the formatter only reads up to three digits.
        guard let match = text.range(of: #"\.\d+"#, options: .regularExpression),
              let whole = formatter.date(from: text.replacingCharacters(in: match, with: "")),
              let fraction = Double("0" + text[match]) else { return nil }
        return whole.addingTimeInterval(fraction)
    }

    public static func durationSeconds(_ value: Any, _ spec: DurationSpec) -> Int? {
        switch spec {
        case .seconds(let seconds):
            return seconds
        case .path(let path, let kind):
            let found = JSON.resolve(value, path)
            switch kind {
            case .seconds: return JSON.number(found).map { Int($0) }
            case .minutes: return JSON.number(found).map { Int($0) * 60 }
            case .windowName:
                switch found as? String {
                case "five_hour", "5h": return 5 * 3600
                case "daily", "day": return 24 * 3600
                case "weekly", "week", "7d": return 7 * 24 * 3600
                default: return nil
                }
            }
        }
    }

    /// Title for a window without a label: "Session" for 5 hours, "Weekly", "Monthly", or the length.
    public static func durationLabel(_ seconds: Int) -> String {
        switch seconds {
        case 5 * 3600: return "Session"
        case 7 * 86_400: return "Weekly"
        case 28 * 86_400...31 * 86_400: return "Monthly"
        case let s where s % 86_400 == 0: return "\(s / 86_400)d"
        case let s where s % 3600 == 0: return "\(s / 3600)h"
        default: return "\(seconds / 60)m"
        }
    }
}
