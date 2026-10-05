import Foundation

public enum RelativeTime {
    /// "in 2 minutes": seconds under a minute, then minutes, hours, days, each rounded down.
    /// `nil` when `date` is not after `now`.
    public static func text(until date: Date, now: Date) -> String? {
        let seconds = Int(date.timeIntervalSince(now))
        guard date > now else { return nil }
        let (value, unit): (Int, String) =
            seconds < 60 ? (max(seconds, 1), "second")
            : seconds < 3600 ? (seconds / 60, "minute")
            : seconds < 86_400 ? (seconds / 3600, "hour")
            : (seconds / 86_400, "day")
        return "in \(value) \(unit)\(value == 1 ? "" : "s")"
    }
}
