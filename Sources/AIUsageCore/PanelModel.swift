import CoreGraphics
import Foundation

/// One line under the cards in the panel.
public struct PanelRow: Equatable {
    public enum Kind: Equatable {
        case separator, detail, refresh, updated, providers, provider(id: String), hiddenCards,
             showCard(id: String), launchAtLogin, openProvidersFolder, version, credit(before: String, after: String), quit
    }

    public let kind: Kind
    public let title: String
    public let tone: Tone
    /// Off: gray, no highlight, skipped by the keyboard.
    public let enabled: Bool
    /// `nil` when the row has no check mark.
    public let checked: Bool?
    public let shortcut: String?
    /// A row inside an open Providers or Hidden Cards section.
    public let indented: Bool
    /// `nil` when the row does not open a section.
    public let expanded: Bool?
    /// Refresh Now while a provider runs.
    public let spinning: Bool

    init(_ kind: Kind, _ title: String = "", tone: Tone = .normal, enabled: Bool = true, checked: Bool? = nil,
         shortcut: String? = nil, indented: Bool = false, expanded: Bool? = nil, spinning: Bool = false) {
        (self.kind, self.title, self.tone, self.enabled, self.checked) = (kind, title, tone, enabled, checked)
        (self.shortcut, self.indented, self.expanded, self.spinning) = (shortcut, indented, expanded, spinning)
    }
}

public enum PanelModel {
    public static let providersSection = "providers", hiddenSection = "hiddenCards"

    public static func cards(_ entries: [MenuEntry]) -> [Card] {
        entries.compactMap { if case .card(let card) = $0 { return card }; return nil }
    }

    /// The rows under the cards. Providers and Hidden Cards open in place when their section is in `expanded`.
    public static func rows(_ entries: [MenuEntry], expanded: Set<String>, refreshing: Bool) -> [PanelRow] {
        var rows: [PanelRow] = []
        for entry in entries {
            switch entry {
            case .card: break
            case .separator: rows.append(PanelRow(.separator, enabled: false))
            case .detail(let text, let tone): rows.append(PanelRow(.detail, text, tone: tone, enabled: false))
            case .refresh: rows.append(PanelRow(.refresh, "Refresh Now", shortcut: "\u{2318}R", spinning: refreshing))
            case .updated(let text): rows.append(PanelRow(.updated, text, enabled: false))
            case .providers(let toggles):
                let open = expanded.contains(providersSection)
                rows.append(PanelRow(.providers, "Providers", expanded: open))
                if open { rows += toggles.map { PanelRow(.provider(id: $0.id), $0.name, checked: $0.enabled, indented: true) } }
            case .hiddenCards(let cards):
                let open = expanded.contains(hiddenSection)
                rows.append(PanelRow(.hiddenCards, "Hidden Cards", expanded: open))
                if open { rows += cards.map { PanelRow(.showCard(id: $0.id), "Show \($0.name)", indented: true) } }
            case .launchAtLogin(let on): rows.append(PanelRow(.launchAtLogin, "Launch at Login", checked: on))
            case .openProvidersFolder: rows.append(PanelRow(.openProvidersFolder, "Open Providers Folder\u{2026}"))
            case .version(let text): rows.append(PanelRow(.version, text, enabled: false))
            case .credit(let before, let after):
                rows.append(PanelRow(.credit(before: before, after: after), "\(before) \(after)", enabled: false))
            case .quit: rows.append(PanelRow(.quit, "Quit AI Usage", shortcut: "\u{2318}Q"))
            }
        }
        return rows
    }

    /// The row Up (`step` -1) or Down (1) lands on. With no current row: the first enabled row for 1,
    /// the last for -1. Stays put at either end.
    public static func highlight(from current: Int?, step: Int, enabled: [Bool]) -> Int? {
        var index = current ?? (step > 0 ? -1 : enabled.count)
        while true {
            index += step
            guard enabled.indices.contains(index) else { return current }
            if enabled[index] { return index }
        }
    }
}

public enum PanelLayout {
    /// The top of each view stacked from 0, and the total height.
    public static func stack(_ heights: [CGFloat]) -> (tops: [CGFloat], total: CGFloat) {
        var tops: [CGFloat] = [], y: CGFloat = 0
        for height in heights {
            tops.append(y)
            y += height
        }
        return (tops, y)
    }

    /// The panel's height: its content, at most `available`.
    public static func height(content: CGFloat, available: CGFloat) -> CGFloat { min(content, available) }

    /// The panel's bottom left corner in screen coordinates: its top `gap` under `anchor`, left edges
    /// aligned, moved to stay `margin` inside `screen`'s sides.
    public static func origin(anchor: CGRect, size: CGSize, screen: CGRect, gap: CGFloat = 4, margin: CGFloat = 8) -> CGPoint {
        let x = max(screen.minX + margin, min(anchor.minX, screen.maxX - margin - size.width))
        return CGPoint(x: x, y: anchor.minY - gap - size.height)
    }
}

/// Values part way between what a card showed and what it shows now.
public enum Tween {
    /// `progress` is clamped to 0...1.
    public static func value(_ from: Double, _ to: Double, _ progress: Double) -> Double {
        from + (to - from) * min(1, max(0, progress))
    }

    /// One array per chart of `new`, one fraction per ring. A ring with no counterpart in `old` (same
    /// chart and ring position) starts at 0. A ring without a value stays without one.
    public static func fractions(from old: Card?, to new: Card, progress: Double) -> [[Double?]] {
        fractions(from: old?.charts.map { $0.rings.map(\.fraction) } ?? [], to: new, progress: progress)
    }

    /// Like `fractions(from:to:progress:)`, starting from fractions that are on screen mid-animation.
    public static func fractions(from old: [[Double?]], to new: Card, progress: Double) -> [[Double?]] {
        new.charts.enumerated().map { chartIndex, chart in
            chart.rings.enumerated().map { ringIndex, ring in
                guard let target = ring.fraction else { return nil }
                let rings = old.indices.contains(chartIndex) ? old[chartIndex] : []
                let start = rings.indices.contains(ringIndex) ? rings[ringIndex] ?? 0 : 0
                return value(start, target, progress)
            }
        }
    }

    /// Counts between two texts that are digits and "%" ("3%" to "13%"); any other text is `to` at once.
    public static func percent(from: String?, to: String, progress: Double) -> String {
        func number(_ text: String) -> Double? {
            guard text.hasSuffix("%"), text.dropLast().allSatisfy(\.isASCII), let value = UInt(text.dropLast()) else { return nil }
            return Double(value)
        }
        guard let target = number(to) else { return to }
        return "\(Int(value(from.flatMap(number) ?? 0, target, progress).rounded()))%"
    }
}
