import Foundation

public enum RelativeTime {
    /// Clock countdown: "4:32", "0:05". `nil` when `date` is not after `now`.
    public static func countdown(until date: Date, now: Date) -> String? {
        guard date > now else { return nil }
        let seconds = Int(date.timeIntervalSince(now).rounded(.up))
        return "\(seconds / 60):" + String(format: "%02d", seconds % 60)
    }

    /// Short countdown: "in 45s", "in 2m", "in 3h 25m", "in 4d 2h 12m". Each unit rounds down; zero parts after
    /// seconds are skipped. `nil` when `date` is not after `now`.
    public static func text(until date: Date, now: Date) -> String? {
        let seconds = Int(date.timeIntervalSince(now))
        guard date > now else { return nil }
        if seconds < 60 { return "in \(max(seconds, 1))s" }
        let parts = [(seconds / 86_400, "d"), (seconds % 86_400 / 3600, "h"), (seconds % 3600 / 60, "m")]
        return "in " + parts.filter { $0.0 > 0 }.map { "\($0.0)\($0.1)" }.joined(separator: " ")
    }
}
