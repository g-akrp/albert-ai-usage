import AIUsageCore
import AppKit

/// One provider card in the menu: pin and title on top, then per chart a concentric ring chart on
/// the left and one row per ring on the right. Clicking a pin calls `onPin` with its `Pin` key; the
/// Hide button that shows while the pointer is over the card calls `onHide` with the card's id.
final class CardView: NSView {
    private static let minWidth: CGFloat = 300, chartSize: CGFloat = 76
    private static let thickness: CGFloat = 5, gap: CGFloat = 2, pinSize: CGFloat = 13, box: CGFloat = 19

    // Spacing scale (points). The outline is `sideGap` from the menu's sides and `outerGap` from its top and
    // bottom (so cards sit 2 x `outerGap` apart); content is `edge` inside the outline vertically and
    // `leading + innerPad` from the view edge on both sides (`leading` is where menu text starts).
    private static let outerGap: CGFloat = 4, sideGap: CGFloat = 10, edge: CGFloat = 16, innerPad: CGFloat = 12, small: CGFloat = 4, medium: CGFloat = 8, large: CGFloat = 14
    private static let line: CGFloat = 20, resetLine: CGFloat = 14, dotColumn: CGFloat = 14
    /// The tinted background behind a usage value.
    private static let pillHeight: CGFloat = 18, pillPadding: CGFloat = 7

    /// `rect` is where a click counts; `box` is the outlined button drawn around the pin icon.
    private struct PinTarget {
        enum Kind { case pin(pinned: Bool), hide }
        let rect: NSRect, box: NSRect, key: String, label: String, kind: Kind
    }

    private static let titleFont = NSFont.menuFont(ofSize: 13), boldTitleFont = NSFont.boldSystemFont(ofSize: 13)
    private static let bodyFont = NSFont.menuFont(ofSize: 12), smallFont = NSFont.menuFont(ofSize: 11)
    private static let groupFont = NSFont.boldSystemFont(ofSize: 12)
    /// The text button that hides a card: "Hide" in an outlined box.
    private static let hideLabel = NSAttributedString(string: "Hide", attributes: [.font: NSFont.menuFont(ofSize: 11)])
    private static let hideWidth = ceil(hideLabel.size().width) + 16
    /// The text button that pins: "Pin", or "Pinned" (always shown) for the current pin. Same width for both.
    private static let pinFont = NSFont.menuFont(ofSize: 11)
    private static let pinWidth = ceil(NSAttributedString(string: "Pinned", attributes: [.font: pinFont]).size().width) + 16

    private let card: Card
    private let width: CGFloat
    private let leading: CGFloat
    private let onPin: (String) -> Void
    private let onHide: (String) -> Void
    private var cardHovered = false
    private var targets: [PinTarget] = []
    private var hovered: Int?
    private var pressed: Int?

    /// For accessibility: the title, and the account when there is one.
    private var name: String { card.subtitle.map { "\(card.title) (\($0))" } ?? card.title }

    override var isFlipped: Bool { true }

    /// Every card in a menu uses this width: the widest content, so no card is clipped.
    static func width(for cards: [Card], leading menuLeading: CGFloat) -> CGFloat {
        let leading = menuLeading + innerPad
        func size(_ string: String, _ font: NSFont) -> CGFloat {
            NSAttributedString(string: string, attributes: [.font: font]).size().width
        }
        let columnX = leading + chartSize + large, indent = leading, pinSpace = pinWidth + 6
        var widest = minWidth
        for card in cards {
            widest = max(widest, indent + size(card.title, boldTitleFont) + large + pinSpace + hideWidth + leading)
            if let account = card.subtitle { widest = max(widest, indent + size(account, bodyFont) + leading) }
            for notice in [card.message].compactMap({ $0 }) + card.notices {
                widest = max(widest, indent + size(notice.text, bodyFont) + leading)
            }
            for chart in card.charts {
                if let title = chart.title {
                    widest = max(widest, columnX + size(title, groupFont) + 12 + pinSpace + leading)
                }
                for row in chart.rows {
                    widest = max(widest, columnX + dotColumn + size(row.label, bodyFont) + large
                                 + size(row.percentText, bodyFont) + 2 * pillPadding + (row.pin == nil ? 0 : pinSpace) + leading)
                    if let reset = row.reset { widest = max(widest, columnX + dotColumn + size(reset, smallFont) + leading) }
                }
            }
            for row in card.overflow {
                widest = max(widest, indent + size("\(row.label)  \(row.percentText)", bodyFont) + leading)
            }
        }
        return ceil(widest)
    }

    init(card: Card, width: CGFloat, leading: CGFloat, onPin: @escaping (String) -> Void, onHide: @escaping (String) -> Void) {
        self.card = card
        self.width = width
        self.leading = leading + Self.innerPad
        self.onPin = onPin
        self.onHide = onHide
        super.init(frame: .zero)
        let height = render(draw: false)
        frame = NSRect(x: 0, y: 0, width: width, height: height)
        setAccessibilityRole(.group)
        setAccessibilityLabel(Self.summary(card))
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    override func draw(_ dirtyRect: NSRect) {
        let outline = NSBezierPath(roundedRect: bounds.insetBy(dx: Self.sideGap + 0.5, dy: Self.outerGap + 0.5), xRadius: 10, yRadius: 10)
        outline.lineWidth = 1
        (cardHovered ? NSColor.tertiaryLabelColor : NSColor.separatorColor).setStroke()
        outline.stroke()
        _ = render(draw: true)
    }

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        trackingAreas.forEach(removeTrackingArea)
        addTrackingArea(NSTrackingArea(rect: .zero, options: [.mouseMoved, .mouseEnteredAndExited, .activeAlways, .inVisibleRect],
                                       owner: self))
    }

    override func resetCursorRects() {
        targets.forEach { addCursorRect($0.rect, cursor: .pointingHand) }
    }

    private func target(at event: NSEvent) -> Int? {
        let point = convert(event.locationInWindow, from: nil)
        return targets.firstIndex { target in
            if case .hide = target.kind, !cardHovered { return false }
            return target.rect.contains(point)
        }
    }

    private func setState(hovered: Int?, pressed: Int?) {
        guard hovered != self.hovered || pressed != self.pressed else { return }
        (self.hovered, self.pressed) = (hovered, pressed)
        needsDisplay = true
    }

    override func mouseEntered(with event: NSEvent) { setCardHovered(true, event) }
    override func mouseMoved(with event: NSEvent) { setCardHovered(true, event) }
    override func mouseExited(with event: NSEvent) {
        cardHovered = false
        setState(hovered: nil, pressed: nil)
        needsDisplay = true
    }

    private func setCardHovered(_ value: Bool, _ event: NSEvent) {
        if cardHovered != value { cardHovered = value; needsDisplay = true }
        setState(hovered: target(at: event), pressed: pressed)
    }

    override func mouseDown(with event: NSEvent) {
        let index = target(at: event)
        setState(hovered: index, pressed: index)
    }

    override func mouseUp(with event: NSEvent) {
        let index = target(at: event)
        let wasPressed = pressed
        setState(hovered: index, pressed: nil)
        guard let index, index == wasPressed else { return }
        enclosingMenuItem?.menu?.cancelTracking()
        switch targets[index].kind {
        case .pin: onPin(targets[index].key)
        case .hide: onHide(targets[index].key)
        }
    }

    // MARK: Layout and drawing

    /// Lays the card out top to bottom; draws it when `draw` is set. Returns the height.
    private func render(draw: Bool) -> CGFloat {
        targets = []
        var y = Self.outerGap + Self.edge
        let x = leading, right = width - leading, indent = x
        let titleLine = Self.box + 3

        text(card.title, font: card.pinned ? Self.boldTitleFont : Self.titleFont, color: .labelColor,
             at: NSPoint(x: indent, y: y), height: titleLine, draw: draw)
        // Hide is recorded first so it wins where its box overlaps the pin's wide hover area.
        hideButton(at: NSPoint(x: right - Self.pinWidth - 6 - Self.hideWidth, y: y + (titleLine - Self.box) / 2), draw: draw)
        pin(card.pinned, key: card.pin, label: name,
            at: NSPoint(x: right - Self.pinWidth, y: y + (titleLine - Self.box) / 2),
            hit: NSRect(x: x, y: y, width: right - x, height: titleLine), draw: draw)
        y += titleLine
        if let account = card.subtitle {
            text(account, font: Self.bodyFont, color: .secondaryLabelColor, at: NSPoint(x: indent, y: y), height: Self.line - 4, draw: draw)
            y += Self.line - 4
        }

        let notices = [card.message].compactMap { $0 } + card.notices
        for notice in notices {
            y += Self.small
            text(notice.text, font: Self.bodyFont, color: Self.color(notice.tone), at: NSPoint(x: indent, y: y), height: 18, draw: draw)
            y += 18
        }

        for (index, chart) in card.charts.enumerated() {
            y += index == 0 ? Self.medium : Self.large
            let top = y
            drawChart(chart, in: NSRect(x: x, y: top, width: Self.chartSize, height: Self.chartSize), draw: draw)
            let columnX = x + Self.chartSize + Self.large
            let headerHeight = chart.title == nil ? 0 : Self.line + Self.small
            let rowsHeight = headerHeight + chart.rows.reduce(0) { $0 + Self.rowHeight($1) } - (chart.rows.isEmpty ? 0 : Self.medium)
            var rowY = top + max(0, (Self.chartSize - rowsHeight) / 2)
            if let title = chart.title {
                text(title, font: Self.groupFont, color: .labelColor, at: NSPoint(x: columnX, y: rowY), height: Self.line, draw: draw)
                if let key = chart.pin {
                    pin(chart.pinned, key: key, label: title,
                        at: NSPoint(x: right - Self.pinWidth, y: rowY + (Self.line - Self.box) / 2),
                        hit: NSRect(x: columnX, y: rowY, width: right - columnX, height: Self.line), draw: draw)
                }
                rowY += headerHeight
            }
            for row in chart.rows {
                drawRow(row, x: columnX, right: right, y: rowY, draw: draw)
                rowY += Self.rowHeight(row)
            }
            y = top + max(Self.chartSize, rowsHeight)
        }
        for row in card.overflow {
            y += Self.small
            text("\(row.label)  \(row.percentText)", font: Self.bodyFont, color: Self.valueColor(row.tone),
                 at: NSPoint(x: indent, y: y), height: Self.line, draw: draw)
            if let key = row.pin {
                pin(row.pinned, key: key, label: row.label, at: NSPoint(x: right - Self.pinWidth, y: y + (Self.line - Self.box) / 2),
                    hit: NSRect(x: x, y: y, width: right - x, height: Self.line), draw: draw)
            }
            y += Self.line
        }
        return y + Self.edge + Self.outerGap
    }

    /// A row's height plus the gap below it.
    private static func rowHeight(_ row: CardRow) -> CGFloat {
        line + (row.reset == nil ? 0 : resetLine) + medium
    }

    /// Dot in the ring's color, label, then the value on a tinted background at the right.
    private func drawRow(_ row: CardRow, x: CGFloat, right: CGFloat, y: CGFloat, draw: Bool) {
        if draw, let index = row.ringIndex {
            shade(index).setFill()
            NSBezierPath(ovalIn: NSRect(x: x, y: y + (Self.line - 7) / 2, width: 7, height: 7)).fill()
        }
        text(row.label, font: Self.bodyFont, color: .labelColor, at: NSPoint(x: x + Self.dotColumn, y: y), height: Self.line, draw: draw)
        let pinSpace: CGFloat = row.pin == nil ? 0 : Self.pinWidth + 6
        let value = NSAttributedString(string: row.percentText, attributes: [.font: Self.bodyFont, .foregroundColor: Self.valueColor(row.tone)])
        let pill = NSRect(x: right - pinSpace - value.size().width - 2 * Self.pillPadding, y: y + (Self.line - Self.pillHeight) / 2,
                          width: value.size().width + 2 * Self.pillPadding, height: Self.pillHeight)
        if draw {
            Self.color(row.tone).withAlphaComponent(0.16).setFill()
            NSBezierPath(roundedRect: pill, xRadius: Self.pillHeight / 2, yRadius: Self.pillHeight / 2).fill()
            value.draw(at: NSPoint(x: pill.minX + Self.pillPadding, y: pill.midY - value.size().height / 2))
        }
        if let key = row.pin {
            pin(row.pinned, key: key, label: row.label, at: NSPoint(x: right - Self.pinWidth, y: y + (Self.line - Self.box) / 2),
                hit: NSRect(x: x, y: y, width: right - x, height: Self.line), draw: draw)
        }
        if let reset = row.reset {
            text(reset, font: Self.smallFont, color: .secondaryLabelColor, at: NSPoint(x: x + Self.dotColumn, y: y + Self.line),
                 height: Self.resetLine, draw: draw)
        }
    }

    private func drawChart(_ chart: CardChart, in box: NSRect, draw: Bool) {
        guard draw else { return }
        let center = NSPoint(x: box.midX, y: box.midY)
        for (index, ring) in chart.rings.enumerated() {
            let radius = box.width / 2 - Self.thickness / 2 - CGFloat(index) * (Self.thickness + Self.gap)
            Self.arc(center: center, radius: radius, fraction: 1, color: .separatorColor)
            if let fraction = ring.fraction, fraction > 0 {
                Self.arc(center: center, radius: radius, fraction: fraction, color: shade(index))
            }
        }
        let font = NSFont.menuFont(ofSize: 11)
        let label = NSAttributedString(string: chart.center, attributes: [.font: font, .foregroundColor: NSColor.labelColor])
        let size = label.size()
        label.draw(at: NSPoint(x: center.x - size.width / 2, y: center.y - size.height / 2))
    }

    /// The provider's color for the outermost ring, lighter for each ring inside it.
    private func shade(_ ring: Int) -> NSColor {
        let accent = NSColor(srgbRed: CGFloat(card.accent.r) / 255, green: CGFloat(card.accent.g) / 255,
                             blue: CGFloat(card.accent.b) / 255, alpha: 1)
        return accent.blended(withFraction: [0, 0.3, 0.5, 0.65][min(ring, 3)], of: .white) ?? accent
    }

    /// A clockwise arc from 12 o'clock; y grows downward in this view, so a growing angle runs clockwise.
    private static func arc(center: NSPoint, radius: CGFloat, fraction: Double, color: NSColor) {
        let steps = max(2, Int(64 * fraction))
        let path = NSBezierPath()
        for step in 0...steps {
            let angle = -Double.pi / 2 + 2 * Double.pi * fraction * Double(step) / Double(steps)
            let point = NSPoint(x: center.x + radius * CGFloat(cos(angle)), y: center.y + radius * CGFloat(sin(angle)))
            step == 0 ? path.move(to: point) : path.line(to: point)
        }
        path.lineWidth = thickness
        path.lineCapStyle = .round
        color.setStroke()
        path.stroke()
    }

    /// Draws `string` vertically centered in a line of `height` starting at `point`.
    @discardableResult
    private func text(_ string: String, font: NSFont, color: NSColor, at point: NSPoint, height: CGFloat, draw: Bool) -> NSSize {
        let attributed = NSAttributedString(string: string, attributes: [.font: font, .foregroundColor: color])
        let size = attributed.size()
        if draw { attributed.draw(at: NSPoint(x: point.x, y: point.y + (height - size.height) / 2)) }
        return size
    }

    /// A "Pin" button, drawn only while the pointer is over the card; the current pin always shows as "Pinned".
    private func pin(_ pinned: Bool, key: String, label: String, at origin: NSPoint, hit: NSRect, draw: Bool) {
        button(.pin(pinned: pinned), key: key, label: label, at: origin, hit: hit, draw: draw)
    }

    /// The "Hide" button at the card's top right; drawn only while the pointer is over the card.
    private func hideButton(at origin: NSPoint, draw: Bool) {
        let hit = NSRect(origin: origin, size: NSSize(width: Self.hideWidth, height: Self.box)).insetBy(dx: -2, dy: -2)
        button(.hide, key: card.id, label: name, at: origin, hit: hit, draw: draw)
    }

    /// Records the click area; draws an outlined text button that reacts to hover and press.
    private func button(_ kind: PinTarget.Kind, key: String, label: String, at origin: NSPoint, hit: NSRect, draw: Bool) {
        var pinned = false, width = Self.hideWidth, title = Self.hideLabel
        if case .pin(let value) = kind {
            (pinned, width) = (value, Self.pinWidth)
            title = NSAttributedString(string: value ? "Pinned" : "Pin", attributes: [.font: Self.pinFont])
        }
        let box = NSRect(origin: origin, size: NSSize(width: width, height: Self.box))
        let index = targets.count
        targets.append(PinTarget(rect: hit, box: box, key: key, label: label, kind: kind))
        let isPressed = pressed == index, isHovered = hovered == index
        guard draw, cardHovered || pinned else { return }
        let shape = NSBezierPath(roundedRect: box.insetBy(dx: 0.5, dy: 0.5), xRadius: 5, yRadius: 5)
        if isPressed || isHovered {
            NSColor.labelColor.withAlphaComponent(isPressed ? 0.2 : 0.08).setFill()
            shape.fill()
        }
        let outline: NSColor = pinned ? .controlAccentColor : isHovered || isPressed ? .secondaryLabelColor : .tertiaryLabelColor
        outline.setStroke()
        shape.lineWidth = 1
        shape.stroke()
        let text = NSMutableAttributedString(attributedString: title)
        text.addAttribute(.foregroundColor,
                          value: pinned ? NSColor.controlAccentColor : isHovered || isPressed ? NSColor.labelColor : NSColor.secondaryLabelColor,
                          range: NSRange(location: 0, length: text.length))
        let size = text.size()
        text.draw(at: NSPoint(x: box.midX - size.width / 2, y: box.midY - size.height / 2))
    }

    /// Text on a tinted pill: system green is too light on a light background, so light mode gets a darker green.
    private static func valueColor(_ tone: Tone) -> NSColor {
        guard tone == .green else { return color(tone) }
        return NSColor(name: nil) { appearance in
            appearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
                ? .systemGreen : NSColor(srgbRed: 0.09, green: 0.45, blue: 0.19, alpha: 1)
        }
    }

    private static func color(_ tone: Tone) -> NSColor {
        switch tone {
        case .normal: return .labelColor
        case .secondary: return .secondaryLabelColor
        case .green: return .systemGreen
        case .orange: return .systemOrange
        case .red: return .systemRed
        }
    }

    // MARK: Accessibility

    private static func summary(_ card: Card) -> String {
        var parts = [card.title] + (card.subtitle.map { [$0] } ?? [])
        if let message = card.message { parts.append(message.text) }
        parts += card.notices.map(\.text)
        for chart in card.charts {
            if let title = chart.title { parts.append(title) }
            parts += chart.rows.map { "\($0.label) \($0.percentText)" + ($0.reset.map { ", resets \($0)" } ?? "") }
        }
        return parts.joined(separator: ". ")
    }

    override func accessibilityChildren() -> [Any]? {
        targets.map { target in
            let element = PinElement(handler: { [weak self] in
                switch target.kind {
                case .pin: self?.onPin(target.key)
                case .hide: self?.onHide(target.key)
                }
            })
            element.setAccessibilityRole(.button)
            switch target.kind {
            case .pin(let pinned): element.setAccessibilityLabel("\(pinned ? "Unpin" : "Pin") \(target.label)")
            case .hide: element.setAccessibilityLabel("Hide \(target.label)")
            }
            element.setAccessibilityParent(self)
            element.setAccessibilityFrameInParentSpace(target.rect)
            return element
        }
    }

    private final class PinElement: NSAccessibilityElement {
        let handler: () -> Void
        init(handler: @escaping () -> Void) {
            self.handler = handler
            super.init()
        }
        override func accessibilityPerformPress() -> Bool { handler(); return true }
    }
}
