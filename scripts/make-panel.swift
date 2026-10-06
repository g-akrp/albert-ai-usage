// Draws docs/images/panel.png: the menu bar panel with sample data, in light and dark appearance.
// Run from the repo root:
//   grep -v '^import AIUsageCore$' Sources/AIUsage/CardView.swift > build/CardView.swift
//   swiftc -parse-as-library -o build/make-panel Sources/AIUsageCore/*.swift build/CardView.swift scripts/make-panel.swift
//   build/make-panel
// Renders the app's own CardView, so the picture cannot drift from the app.
import AppKit

let now = Date()
func at(_ seconds: TimeInterval) -> Date { now + seconds }
let hour: TimeInterval = 3600, day: TimeInterval = 86_400

func window(_ id: String, _ label: String, _ percent: Double, resets: TimeInterval, seconds: Int) -> WindowReport {
    WindowReport(id: id, label: label, usedPercent: percent, resetsAt: at(resets), durationSeconds: seconds)
}
func run(_ id: String, _ name: String, _ label: String, _ hex: String, _ report: Report) -> ProviderRun {
    ProviderRun(id: id, name: name, iconLabel: label, iconColor: RGB(hex: hex)!, result: .success(report))
}

let runs = [
    run("claude", "Claude Code", "CLD", "D97757", Report(plan: "max", meters: [
        MeterReport(id: "claude", label: "Claude", windows: [
            window("five_hour", "Session", 14, resets: 3 * hour + 25 * 60, seconds: 5 * 3600),
            window("seven_day", "Weekly", 72, resets: 4 * day + 2 * hour + 12 * 60, seconds: 7 * 86_400)])])),
    run("codex", "Codex", "CDX", "10A37F", Report(plan: "plus", meters: [
        MeterReport(id: "codex", label: "Codex", windows: [
            window("primary", "Session", 31, resets: 1 * hour + 48 * 60, seconds: 5 * 3600),
            window("secondary", "Weekly", 23, resets: 5 * day + 6 * hour, seconds: 7 * 86_400)])])),
    run("antigravity", "Antigravity", "AGY", "4285F4", Report(meters: [
        MeterReport(id: "Gemini Models", label: "Gemini Models", windows: [
            window("g5h", "Five Hour", 3, resets: 2 * hour + 10 * 60, seconds: 5 * 3600),
            window("gwk", "Weekly", 7, resets: 4 * day + 9 * hour, seconds: 7 * 86_400)]),
        MeterReport(id: "Claude and GPT models", label: "Claude and GPT models", windows: [
            window("cwk", "Weekly", 95, resets: 4 * day + 9 * hour, seconds: 7 * 86_400)])])),
]

let formatter: DateFormatter = {
    let formatter = DateFormatter()
    formatter.dateFormat = "MMM d, h:mm a"
    return formatter
}()
let cards = runs.map { CardModel.card($0, pinned: "claude", now: now, formatReset: formatter.string(from:)) }
let leading: CGFloat = 14
let panelWidth = CardView.width(for: cards, leading: leading)
let items: [(String, String)] = [("Refresh Now", "\u{2318}R"), ("Updated 11:02 AM", ""), ("Providers", "\u{203A}"),
                                 ("Launch at Login", ""), ("Open Providers Folder\u{2026}", ""), ("AI Usage \(AppVersion.current)", ""),
                                 ("Quit AI Usage", "\u{2318}Q")]
let itemHeight: CGFloat = 22, padding: CGFloat = 6, scale: CGFloat = 2, gutter: CGFloat = 40

/// Hosts a card so it is drawn over the menu's background in the given appearance, as in a real menu.
final class Backdrop: NSView {
    let color: NSColor
    init(_ card: NSView, color: NSColor, appearance: NSAppearance) {
        self.color = color
        super.init(frame: card.frame)
        self.appearance = appearance
        addSubview(card)
    }
    required init?(coder: NSCoder) { fatalError() }
    override func draw(_ dirtyRect: NSRect) {
        color.setFill()
        bounds.fill()
    }
}

func render(_ view: NSView) -> NSBitmapImageRep {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: Int(view.bounds.width * scale), pixelsHigh: Int(view.bounds.height * scale),
                               bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB,
                               bytesPerRow: 0, bitsPerPixel: 0)!
    rep.size = view.bounds.size
    view.cacheDisplay(in: view.bounds, to: rep)
    return rep
}

/// One panel: the cards, a separator, then the plain items.
func panel(_ appearance: NSAppearance, background: NSColor) -> (image: NSBitmapImageRep, size: NSSize) {
    let views = cards.enumerated().map { index, card in
        CardView(card: card, width: panelWidth, leading: leading, collapsed: index == 1, canMoveUp: index > 0,
                 canMoveDown: index < cards.count - 1, onPin: { _ in }, onHide: { _ in }, onMove: { _, _ in }, onToggle: { _ in })
    }
    let hosts = views.map { Backdrop($0, color: background, appearance: appearance) }
    let cardsHeight = views.reduce(0) { $0 + $1.frame.height }
    let size = NSSize(width: panelWidth, height: padding + cardsHeight + padding + 1 + padding + itemHeight * CGFloat(items.count) + padding)
    let image = NSImage(size: size)
    image.lockFocus()
    appearance.performAsCurrentDrawingAppearance {
        let shape = NSBezierPath(roundedRect: NSRect(origin: .zero, size: size), xRadius: 10, yRadius: 10)
        background.setFill()
        shape.fill()
        shape.addClip()
        var y = size.height - padding
        for host in hosts {
            y -= host.frame.height
            render(host).draw(in: NSRect(x: 0, y: y, width: panelWidth, height: host.frame.height))
        }
        y -= padding + 1
        NSColor.separatorColor.setFill()
        NSRect(x: 10, y: y, width: panelWidth - 20, height: 1).fill()
        y -= padding
        for (title, key) in items {
            y -= itemHeight
            let disabled = title.hasPrefix("Updated") || title.hasPrefix("AI Usage")
            let color = disabled ? NSColor.disabledControlTextColor : NSColor.labelColor
            let attributes: [NSAttributedString.Key: Any] = [.font: NSFont.menuFont(ofSize: 0), .foregroundColor: color]
            NSAttributedString(string: title, attributes: attributes).draw(at: NSPoint(x: leading + 12, y: y + 3))
            let right = NSAttributedString(string: key, attributes: [.font: NSFont.menuFont(ofSize: 0), .foregroundColor: NSColor.secondaryLabelColor])
            right.draw(at: NSPoint(x: panelWidth - 14 - right.size().width, y: y + 3))
        }
    }
    image.unlockFocus()
    return (NSBitmapImageRep(data: image.tiffRepresentation!)!, size)
}

@main
struct PanelShowcase {
    static func main() throws {
        let light = panel(NSAppearance(named: .aqua)!, background: NSColor(srgbRed: 0.96, green: 0.96, blue: 0.97, alpha: 1))
        let dark = panel(NSAppearance(named: .darkAqua)!, background: NSColor(srgbRed: 0.17, green: 0.17, blue: 0.19, alpha: 1))
        let width = light.size.width + dark.size.width + gutter * 3
        let height = max(light.size.height, dark.size.height) + gutter * 2
        let image = NSImage(size: NSSize(width: width, height: height))
        image.lockFocus()
        NSColor(srgbRed: 0.55, green: 0.6, blue: 0.7, alpha: 1).setFill()
        NSRect(x: 0, y: 0, width: width, height: height).fill()
        light.image.draw(in: NSRect(x: gutter, y: gutter, width: light.size.width, height: light.size.height))
        dark.image.draw(in: NSRect(x: gutter * 2 + light.size.width, y: gutter, width: dark.size.width, height: dark.size.height))
        image.unlockFocus()
        let png = NSBitmapImageRep(data: image.tiffRepresentation!)!.representation(using: .png, properties: [:])!
        try png.write(to: URL(fileURLWithPath: "docs/images/panel.png"))
        print("Wrote docs/images/panel.png")
    }
}
