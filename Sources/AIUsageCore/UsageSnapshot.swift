import Foundation

public struct UsageWindow: Codable, Equatable {
    public let id: String
    public let label: String?
    public let remainingPercent: Double?
    public let resetsAt: Date?

    public init(id: String, label: String? = nil, remainingPercent: Double? = nil, resetsAt: Date? = nil) {
        (self.id, self.label, self.remainingPercent, self.resetsAt) = (id, label, remainingPercent, resetsAt)
    }
}

public struct UsageProvider: Codable, Equatable {
    public let id: String
    public let name: String
    public let status: String
    public let message: String?
    public let remainingPercent: Double?
    public let resetsAt: Date?
    public let windows: [UsageWindow]

    public init(id: String, name: String, status: String, message: String? = nil,
                remainingPercent: Double? = nil, resetsAt: Date? = nil, windows: [UsageWindow] = []) {
        (self.id, self.name, self.status, self.message, self.remainingPercent, self.resetsAt, self.windows) =
            (id, name, status, message, remainingPercent, resetsAt, windows)
    }
}

public struct UsageSnapshot: Codable, Equatable {
    public let generatedAt: Date
    public let intervalSeconds: Int
    public let providers: [UsageProvider]

    public init(generatedAt: Date, intervalSeconds: Int, providers: [UsageProvider]) {
        (self.generatedAt, self.intervalSeconds, self.providers) = (generatedAt, intervalSeconds, providers)
    }

    public init(runs: [ProviderRun], now: Date, intervalSeconds: Int) {
        self.generatedAt = now
        self.intervalSeconds = intervalSeconds
        self.providers = runs.map(Self.provider(from:)).enumerated().sorted { a, b in
            let ra = Self.rank(a.element.status), rb = Self.rank(b.element.status)
            if ra != rb { return ra < rb }
            switch (a.element.remainingPercent, b.element.remainingPercent) {
            case let (x?, y?) where x != y: return x > y
            case (_?, nil): return true
            case (nil, _?): return false
            default: return a.offset < b.offset
            }
        }.map(\.element)
    }

    public func encoded() throws -> Data {
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
        return try encoder.encode(self)
    }

    public static func decode(_ data: Data) throws -> UsageSnapshot {
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        return try decoder.decode(UsageSnapshot.self, from: data)
    }

    public func view(now: Date) -> [String: Any] {
        let fmt = ISO8601DateFormatter()
        let age = max(0, Int(now.timeIntervalSince(generatedAt)))
        let stale = age > 3 * intervalSeconds
        let items: [[String: Any]] = providers.map { p in
            var dict: [String: Any] = ["id": p.id, "name": p.name, "status": p.status]
            if let message = p.message { dict["message"] = message }
            if let remaining = p.remainingPercent { dict["remainingPercent"] = remaining }
            if let resetsAt = p.resetsAt { dict["resetsAt"] = fmt.string(from: resetsAt) }
            dict["windows"] = p.windows.map { w -> [String: Any] in
                var wd: [String: Any] = ["id": w.id]
                if let label = w.label { wd["label"] = label }
                if let remaining = w.remainingPercent { wd["remainingPercent"] = remaining }
                if let resetsAt = w.resetsAt { wd["resetsAt"] = fmt.string(from: resetsAt) }
                return wd
            }
            return dict
        }
        return [
            "generatedAt": fmt.string(from: generatedAt),
            "ageSeconds": age,
            "stale": stale,
            "providers": items,
        ]
    }

    private static func provider(from run: ProviderRun) -> UsageProvider {
        switch run.result {
        case nil:
            return UsageProvider(id: run.id, name: run.name, status: "loading")
        case .failure(let failure)?:
            return UsageProvider(id: run.id, name: run.name, status: "error", message: failure.message)
        case .success(let report)?:
            let multiMeter = report.meters.count > 1
            let windows: [UsageWindow] = report.meters.flatMap { meter in
                let flat = CardModel.isFlat(meter)
                return meter.windows.map { w in
                    UsageWindow(
                        id: flat && multiMeter ? meter.id : w.id,
                        label: w.label ?? (flat ? meter.label : nil),
                        remainingPercent: w.usedPercent.map { max(0, 100 - $0) },
                        resetsAt: w.resetsAt)
                }
            }
            let tightest = windows.compactMap { w in w.remainingPercent.map { (w, $0) } }
                .min { $0.1 < $1.1 }?.0
            return UsageProvider(
                id: run.id, name: run.name, status: "ok",
                remainingPercent: tightest?.remainingPercent, resetsAt: tightest?.resetsAt,
                windows: windows)
        }
    }

    private static func rank(_ status: String) -> Int {
        switch status {
        case "ok": return 0
        case "loading": return 1
        default: return 2
        }
    }
}
