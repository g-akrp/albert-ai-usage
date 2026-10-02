import Foundation

/// Helpers for values produced by `JSONSerialization`, where booleans and numbers are both
/// `NSNumber` and must be told apart explicitly.
public enum JSON {
    /// Parses one JSON document (an object, array, or bare value).
    public static func parse(_ data: Data) -> Any? {
        try? JSONSerialization.jsonObject(with: data, options: [.fragmentsAllowed])
    }

    /// RFC 6901 JSON Pointer lookup. `""` is the whole document; a missing segment is `nil`.
    public static func resolve(_ root: Any, _ pointer: String) -> Any? {
        if pointer.isEmpty { return root }
        var current = root
        let body = pointer.drop(while: { $0 == "/" })
        for raw in body.split(separator: "/", omittingEmptySubsequences: false) {
            let segment = raw.replacingOccurrences(of: "~1", with: "/").replacingOccurrences(of: "~0", with: "~")
            if let object = current as? [String: Any] {
                guard let next = object[segment] else { return nil }
                current = next
            } else if let array = current as? [Any], let index = Int(segment), array.indices.contains(index) {
                current = array[index]
            } else {
                return nil
            }
        }
        return current
    }

    public static func isBool(_ value: Any) -> Bool {
        guard let number = value as? NSNumber else { return false }
        return CFGetTypeID(number) == CFBooleanGetTypeID()
    }

    public static func bool(_ value: Any?) -> Bool? {
        guard let value, isBool(value) else { return nil }
        return (value as! NSNumber).boolValue
    }

    public static func number(_ value: Any?) -> Double? {
        guard let value, let number = value as? NSNumber, !isBool(number) else { return nil }
        return number.doubleValue
    }

    public static func isNull(_ value: Any?) -> Bool {
        value == nil || value is NSNull
    }

    /// A string, or an integer rendered as a string (ids and labels accept both).
    public static func idString(_ value: Any?) -> String? {
        if let string = value as? String { return string }
        if let number = number(value), number == number.rounded(), abs(number) < 1e15 {
            return String(Int64(number))
        }
        return nil
    }

    /// Predicate equality: `2` equals `2.0` but not `"2"`, and booleans equal only booleans.
    public static func equal(_ a: Any, _ b: Any) -> Bool {
        if isBool(a) || isBool(b) { return bool(a) != nil && bool(a) == bool(b) }
        if let x = number(a), let y = number(b) { return x == y }
        if let x = a as? String, let y = b as? String { return x == y }
        if a is NSNull, b is NSNull { return true }
        return (a as AnyObject).isEqual(b)
    }

    /// One-line encoding for a `write` step.
    public static func line(_ value: Any) -> Data? {
        guard JSONSerialization.isValidJSONObject(value) else { return nil }
        return try? JSONSerialization.data(withJSONObject: value, options: [.withoutEscapingSlashes])
    }
}
