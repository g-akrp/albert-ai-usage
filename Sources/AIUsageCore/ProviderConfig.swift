import Foundation

/// One provider file: which program to run and how to read its answer. The format is the
/// Maestri Agent Usage format (see `example/AGENTS.md`) plus `iconLabel` and `iconColor`.
public struct ProviderConfig {
    public let id: String
    public let name: String
    /// Top row of the menu bar icon. Defaults to the first three characters of `id`, uppercased.
    public let iconLabel: String
    /// Brand color of the icon's top row. Defaults to gray.
    public let iconColor: RGB
    public let source: Source
    public let map: MapSpec
    public let refreshSeconds: Int
    /// Project extension: run the provider once per account that this lists.
    public let accounts: AccountsSpec?
}

/// Lists accounts by running `source`, then reads each element of `each` and its id at `id`.
/// Each account runs the provider's source with `${account}` in `args` and `env` replaced by the id.
public struct AccountsSpec {
    public let source: Source
    public let each: String
    public let id: String
    public let match: [Predicate]
}

public struct RGB: Equatable, Hashable {
    public let r: UInt8, g: UInt8, b: UInt8
    public init(_ r: UInt8, _ g: UInt8, _ b: UInt8) { (self.r, self.g, self.b) = (r, g, b) }

    /// `"RRGGBB"` without `#`; anything else is `nil`.
    public init?(hex: String) {
        guard hex.count == 6, hex.allSatisfy(\.isHexDigit), let value = UInt32(hex, radix: 16) else { return nil }
        self.init(UInt8(value >> 16 & 0xFF), UInt8(value >> 8 & 0xFF), UInt8(value & 0xFF))
    }

    public static let gray = RGB(110, 110, 110)
}

public enum Source {
    case command(CommandSource)
    case stdio(StdioSource)

    /// The same source with `${account}` replaced in its arguments and environment values.
    public func replacingAccount(with account: String) -> Source {
        func fill(_ text: String) -> String { text.replacingOccurrences(of: "${account}", with: account) }
        switch self {
        case .command(let s):
            return .command(CommandSource(executable: s.executable, args: s.args.map(fill), env: s.env.mapValues(fill),
                                          timeoutSeconds: s.timeoutSeconds, maxOutputBytes: s.maxOutputBytes,
                                          expect: s.expect))
        case .stdio(let s):
            return .stdio(StdioSource(executable: s.executable, args: s.args.map(fill), env: s.env.mapValues(fill),
                                      timeoutSeconds: s.timeoutSeconds, steps: s.steps, output: s.output,
                                      maxLineBytes: s.maxLineBytes, maxTotalBytes: s.maxTotalBytes))
        }
    }

    public var executable: String {
        switch self {
        case .command(let source): return source.executable
        case .stdio(let source): return source.executable
        }
    }
}

public struct CommandSource {
    public let executable: String
    public let args: [String]
    public let env: [String: String]
    public let timeoutSeconds: Int
    public let maxOutputBytes: Int
    public let expect: Expect?
}

public struct StdioSource {
    public let executable: String
    public let args: [String]
    public let env: [String: String]
    public let timeoutSeconds: Int
    public let steps: [Step]
    public let output: String
    public let maxLineBytes: Int
    public let maxTotalBytes: Int
}

public enum Step {
    /// The message, already encoded as one line.
    case write(Data)
    case await(Expect)
}

/// `expect` on a command, or `await` in a stdio exchange (`capture` is only used there).
public struct Expect {
    public let match: [Predicate]
    public let error: String?
    public let require: String?
    public let capture: String?
}

public struct Predicate {
    public enum Test { case equals(Any), exists(Bool) }
    public let path: String
    public let test: Test

    /// True when every predicate holds for `value`; an empty list always holds.
    public static func allHold(_ predicates: [Predicate], in value: Any) -> Bool {
        predicates.allSatisfy { predicate in
            let found = JSON.resolve(value, predicate.path)
            switch predicate.test {
            case .exists(let expected): return (found != nil) == expected
            case .equals(let expected): return found.map { JSON.equal($0, expected) } ?? false
            }
        }
    }
}

public struct MapSpec {
    public let root: String?
    public let available: String?
    public let plan: PlanSpec?
    public let access: String?
    public let meters: [MeterSpec]
}

public struct PlanSpec {
    public let path: String
    public let names: [String: String]
}

public enum Scope {
    case current
    case select(String)
    case each(String)
    case eachEntry(String)
}

public enum IdSpec {
    case literal(String)
    case path(String)
    case entryKey
}

public indirect enum LabelSpec {
    case text(String)
    case key(String)
    case path(String, fallback: LabelSpec?)
    case entryKey
}

public struct MeterSpec {
    public let scope: Scope
    /// Project extension: values in scope where any predicate fails are skipped.
    public let match: [Predicate]
    public let id: IdSpec
    public let label: LabelSpec
    public let windows: [WindowSpec]
}

public struct WindowSpec {
    public let scope: Scope
    /// Project extension: values in scope where any predicate fails are skipped.
    public let match: [Predicate]
    public let id: IdSpec
    public let label: LabelSpec?
    public let used: (path: String, as: UsedAs)?
    public let resetsAt: (path: String, as: ResetAs)?
    public let duration: DurationSpec?
}

public enum UsedAs: String { case percent, fraction, remainingFraction, remainingPercent }
public enum ResetAs: String { case iso8601, epochSeconds, epochMillis }
public enum DurationAs: String { case seconds, minutes, windowName }

public enum DurationSpec {
    case seconds(Int)
    case path(String, as: DurationAs)
}

public struct ConfigError: Error, CustomStringConvertible {
    public let description: String
    init(_ path: String, _ message: String) { description = "\(path.isEmpty ? "/" : path): \(message)" }
}

// MARK: Parsing

extension ProviderConfig {
    public static func parse(_ data: Data) throws -> ProviderConfig {
        guard let root = JSON.parse(data) else { throw ConfigError("", "not valid JSON") }
        let p = Reader(root, "")
        guard try p.int("schemaVersion") == 1 else { throw ConfigError("/schemaVersion", "only 1 is understood") }
        let id = try p.string("id")
        guard id.range(of: "^[a-z0-9][a-z0-9-]{0,39}$", options: .regularExpression) != nil else {
            throw ConfigError("/id", "use 1 to 40 lowercase letters, digits, or hyphens")
        }
        let name = try p.string("name")
        let label = try p.optionalString("iconLabel") ?? String(id.prefix(3)).uppercased()
        let color = try p.optionalString("iconColor").flatMap(RGB.init(hex:)) ?? .gray
        let refresh = try p.optional("refresh").map { try $0.optionalInt("intervalSeconds") } ?? nil
        return ProviderConfig(
            id: id, name: name, iconLabel: label, iconColor: color,
            source: try parseSource(p.child("source")),
            map: try parseMap(p.child("map")),
            refreshSeconds: min(max(refresh ?? 300, 60), 86_400),
            accounts: try p.optional("accounts").map { a in
                AccountsSpec(source: try parseSource(a.child("source")), each: try a.string("each"),
                             id: try a.string("id"), match: try parsePredicates(a))
            })
    }

    private static func parseSource(_ p: Reader) throws -> Source {
        let type = try p.string("type")
        let executable = try p.string("executable")
        let args = try p.optional("args").map { try $0.strings() } ?? []
        let env = try p.optional("env").map { try $0.stringMap() } ?? [:]
        switch type {
        case "command":
            return .command(CommandSource(
                executable: executable, args: args, env: env,
                timeoutSeconds: try p.optionalInt("timeoutSeconds", in: 1...600) ?? 30,
                maxOutputBytes: try p.optionalInt("maxOutputBytes", in: 1024...8_388_608) ?? 1_048_576,
                expect: try p.optional("expect").map(parseExpect)))
        case "stdio":
            let steps = try p.child("steps").elements().map { step -> Step in
                if let write = try step.optional("write") {
                    guard let line = JSON.line(write.value) else { throw ConfigError(write.path, "expected a JSON object") }
                    return .write(line)
                }
                if let wait = try step.optional("await") { return .await(try parseExpect(wait)) }
                throw ConfigError(step.path, "expected \"write\" or \"await\"")
            }
            let maxLine = try p.optionalInt("maxLineBytes", in: 1024...8_388_608) ?? 1_048_576
            return .stdio(StdioSource(
                executable: executable, args: args, env: env,
                timeoutSeconds: try p.optionalInt("timeoutSeconds", in: 1...600) ?? 15,
                steps: steps, output: try p.string("output"),
                maxLineBytes: maxLine,
                maxTotalBytes: try p.optionalInt("maxTotalBytes", in: maxLine...16_777_216) ?? max(4_194_304, maxLine)))
        default:
            throw ConfigError(p.path + "/type", "expected \"command\" or \"stdio\"")
        }
    }

    private static func parseExpect(_ p: Reader) throws -> Expect {
        Expect(match: try parsePredicates(p), error: try p.optionalString("error"),
               require: try p.optionalString("require"), capture: try p.optionalString("capture"))
    }

    private static func parsePredicates(_ p: Reader) throws -> [Predicate] {
        try p.optional("match")?.elements().map { item -> Predicate in
            let path = try item.string("path")
            if let exists = try item.optional("exists") {
                guard let flag = JSON.bool(exists.value) else { throw ConfigError(exists.path, "expected a boolean") }
                return Predicate(path: path, test: .exists(flag))
            }
            guard let equals = try item.optional("equals") else {
                throw ConfigError(item.path, "expected \"equals\" or \"exists\"")
            }
            return Predicate(path: path, test: .equals(equals.value))
        } ?? []
    }

    private static func parseMap(_ p: Reader) throws -> MapSpec {
        MapSpec(
            root: try p.optionalString("root"),
            available: try p.optional("available").map { try $0.string("path") },
            plan: try p.optional("plan").map {
                PlanSpec(path: try $0.string("path"), names: try $0.optional("names").map { try $0.stringMap() } ?? [:])
            },
            access: try p.optional("access").map { try $0.string("path") },
            meters: try p.child("meters").elements().map { m in
                MeterSpec(scope: try parseScope(m), match: try parsePredicates(m), id: try parseId(m.child("id")),
                          label: try parseLabel(m.child("label")),
                          windows: try m.child("windows").elements().map(parseWindow))
            })
    }

    private static func parseWindow(_ p: Reader) throws -> WindowSpec {
        WindowSpec(
            scope: try parseScope(p),
            match: try parsePredicates(p),
            id: try parseId(p.child("id")),
            label: try p.optional("label").map(parseLabel),
            used: try p.optional("used").map { u in
                (try u.string("path"), try u.enumValue("as", UsedAs.self))
            },
            resetsAt: try p.optional("resetsAt").map { r in
                (try r.string("path"), try r.enumValue("as", ResetAs.self))
            },
            duration: try p.optional("duration").map { d in
                if let seconds = try d.optionalInt("seconds") { return .seconds(seconds) }
                return .path(try d.string("path"), as: try d.enumValue("as", DurationAs.self))
            })
    }

    private static func parseScope(_ p: Reader) throws -> Scope {
        let select = try p.optionalString("select"), each = try p.optionalString("each")
        let eachEntry = try p.optionalString("eachEntry")
        if [select, each, eachEntry].compactMap({ $0 }).count > 1 {
            throw ConfigError(p.path, "use at most one of select, each, eachEntry")
        }
        if let select { return .select(select) }
        if let each { return .each(each) }
        if let eachEntry { return .eachEntry(eachEntry) }
        return .current
    }

    private static func parseId(_ p: Reader) throws -> IdSpec {
        if let literal = p.value as? String { return .literal(literal) }
        if let path = try p.optionalString("path") { return .path(path) }
        if JSON.bool(try p.optional("entryKey")?.value) == true { return .entryKey }
        throw ConfigError(p.path, "expected a string, {\"path\": ...}, or {\"entryKey\": true}")
    }

    private static func parseLabel(_ p: Reader) throws -> LabelSpec {
        if let text = try p.optionalString("text") { return .text(text) }
        if let key = try p.optionalString("key") { return .key(key) }
        if JSON.bool(try p.optional("entryKey")?.value) == true { return .entryKey }
        if let path = try p.optionalString("path") {
            return .path(path, fallback: try p.optional("fallback").map(parseLabel))
        }
        throw ConfigError(p.path, "expected text, key, path, or entryKey")
    }
}

/// Walks a parsed JSON value, keeping the JSON Pointer of the current position for errors.
private struct Reader {
    let value: Any
    let path: String

    init(_ value: Any, _ path: String) { (self.value, self.path) = (value, path) }

    private func object() throws -> [String: Any] {
        guard let object = value as? [String: Any] else { throw ConfigError(path, "expected an object") }
        return object
    }

    func optional(_ key: String) throws -> Reader? {
        guard let found = try object()[key], !(found is NSNull) else { return nil }
        return Reader(found, "\(path)/\(key)")
    }

    func child(_ key: String) throws -> Reader {
        guard let found = try optional(key) else { throw ConfigError("\(path)/\(key)", "required") }
        return found
    }

    func string(_ key: String) throws -> String {
        let found = try child(key)
        guard let string = found.value as? String, !string.isEmpty else { throw ConfigError(found.path, "expected a string") }
        return string
    }

    func optionalString(_ key: String) throws -> String? {
        try optional(key).map { _ in try string(key) }
    }

    func int(_ key: String) throws -> Int {
        let found = try child(key)
        guard let number = JSON.number(found.value), number == number.rounded() else {
            throw ConfigError(found.path, "expected a whole number")
        }
        return Int(number)
    }

    func optionalInt(_ key: String, in range: ClosedRange<Int>? = nil) throws -> Int? {
        guard try optional(key) != nil else { return nil }
        let number = try int(key)
        if let range, !range.contains(number) {
            throw ConfigError("\(path)/\(key)", "expected \(range.lowerBound) to \(range.upperBound)")
        }
        return number
    }

    func enumValue<E: RawRepresentable>(_ key: String, _ type: E.Type) throws -> E where E.RawValue == String {
        guard let parsed = E(rawValue: try string(key)) else { throw ConfigError("\(path)/\(key)", "unknown value") }
        return parsed
    }

    func elements() throws -> [Reader] {
        guard let array = value as? [Any] else { throw ConfigError(path, "expected an array") }
        return array.enumerated().map { Reader($1, "\(path)/\($0)") }
    }

    func strings() throws -> [String] {
        try elements().map {
            guard let string = $0.value as? String else { throw ConfigError($0.path, "expected a string") }
            return string
        }
    }

    func stringMap() throws -> [String: String] {
        try object().reduce(into: [:]) { result, entry in
            guard let string = entry.value as? String else { throw ConfigError("\(path)/\(entry.key)", "expected a string") }
            result[entry.key] = string
        }
    }
}
