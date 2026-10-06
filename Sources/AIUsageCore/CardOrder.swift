import Foundation

/// The order of cards in the panel: a default (Claude, Codex, Antigravity, Copilot), then whatever the
/// user saved by moving cards up and down.
public enum CardOrder {
    /// Provider file ids in default order; any other provider comes after these, in the order given.
    public static let defaultIds = ["claude", "codex", "antigravity", "copilot"]

    private static func rank(_ configId: String) -> Int {
        defaultIds.firstIndex(of: configId) ?? defaultIds.count
    }

    /// Provider file ids in default order. Stable: ids of equal rank keep their order.
    public static func defaultSorted(_ ids: [String]) -> [String] {
        ids.enumerated().sorted { a, b in
            let (x, y) = (rank(a.element), rank(b.element))
            return x != y ? x < y : a.offset < b.offset
        }.map(\.element)
    }

    /// Runs in the saved order (run ids, such as `claude` or `copilot:octocat`); runs not in `saved`,
    /// such as a new account, follow in default order. Unknown saved ids are ignored.
    public static func sorted(_ runs: [ProviderRun], saved: [String]) -> [ProviderRun] {
        runs.enumerated().sorted { a, b in
            let (x, y) = (saved.firstIndex(of: a.element.id) ?? Int.max, saved.firstIndex(of: b.element.id) ?? Int.max)
            if x != y { return x < y }
            let (rx, ry) = (rank(a.element.configId), rank(b.element.configId))
            return rx != ry ? rx < ry : a.offset < b.offset
        }.map(\.element)
    }

    /// `ids` (the current order) with `id` swapped with its nearest visible neighbor one step up (`-1`) or
    /// down (`1`). Hidden cards are stepped over. Unchanged when `id` is unknown or at the edge.
    public static func moved(_ id: String, by step: Int, in ids: [String], hidden: Set<String>) -> [String] {
        guard let from = ids.firstIndex(of: id), step == -1 || step == 1 else { return ids }
        var to = from + step
        while ids.indices.contains(to), hidden.contains(ids[to]) { to += step }
        guard ids.indices.contains(to) else { return ids }
        var result = ids
        result.swapAt(from, to)
        return result
    }
}
