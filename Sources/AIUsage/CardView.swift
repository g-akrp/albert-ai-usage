import AIUsageCore
import AppKit

/// One provider card in the menu: pin and title on top, then per chart a concentric ring chart on
/// the left and one row per ring on the right. Clicking a pin calls `onPin` with its `Pin` key; the
/// Hide button that shows while the pointer is over the card calls `onHide` with the card's id, and its
/// up and down buttons call `onMove` with the card's id and -1 or 1. The chevron left of the title calls
/// `onToggle` with the card's id; a collapsed card shows its title and one pill per value, no charts.
/// The panel keeps the view and gives it each new `Card` through `update`, so values move from what is
/// on screen to the new ones.
final class CardView: NSView {
    private static let minWidth: CGFloat = 300, chartSize: CGFloat = 76
    private static let thickness: CGFloat = 5, gap: CGFloat = 2, pinSize: CGFloat = 13, box: CGFloat = 19

    // Spacing scale (points). The outline is `sideGap` from the menu's sides and `outerGap` from its top and
    // bottom (so cards sit 2 x `outerGap` apart); content is `edge` inside the outline vertically and
    // `leading + innerPad` from the view edge on both sides (`leading` is where menu text starts).
    private static let outerGap: CGFloat = 4, sideGap: CGFloat = 10, edge: CGFloat = 16, innerPad: CGFloat = 12, small: CGFloat = 4, medium: CGFloat = 8, large: CGFloat = 14
    private static let collapsedEdge: CGFloat = 10
    private static let line: CGFloat = 20, resetLine: CGFloat = 14, dotColumn: CGFloat = 14
    /// The tinted background behind a usage value.
    private static let pillHeight: CGFloat = 18, pillPadding: CGFloat = 7

    /// `rect` is where a click counts; `box` is the outlined button drawn around the pin icon.
    private struct PinTarget {
        enum Kind { case pin(pinned: Bool), hide, move(step: Int), toggle }
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
    /// The two arrow buttons that reorder a card sit left of Hide, each `box` wide, 4 pt apart.
    private static let moveSpace = 2 * (box + 4)
    /// The collapse chevron's column, left of the title.
    private static let chevronSpace: CGFloat = 14

    private(set) var card: Card
    /// What the card showed before the last `update`: where texts and tones animate from. `nil`: from zero.
    private var previous: Card?
    /// The ring fractions on screen when the last `update` came: where the rings animate from.
    private var startFractions: [[Double?]] = []
    /// 0: `previous` is shown. 1: `card` is shown. A card that never animated is at 1.
    @objc dynamic var progress: CGFloat = 1 { didSet { needsDisplay = true } }
    /// 0: pointer away. 1: pointer over the card, its outline darker and its buttons shown.
    @objc dynamic var hover: CGFloat = 0 { didSet { needsDisplay = true } }
    /// The chevron: 0 points right (collapsed), 1 points down (expanded).
    @objc dynamic var turn: CGFloat = 1 { didSet { needsDisplay = true } }
    private let width: CGFloat
    private let leading: CGFloat
    private let onPin: (String) -> Void
    private let onHide: (String) -> Void
    private let onMove: (String, Int) -> Void
    private let onToggle: (String) -> Void
    private var collapsed: Bool
    private var canMoveUp: Bool, canMoveDown: Bool
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
            widest = max(widest, indent + chevronSpace + size(card.title, boldTitleFont) + large + pinSpace + hideWidth + moveSpace + leading)
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
                    for line in [row.count, row.reset].compactMap({ $0 }) {
                        widest = max(widest, columnX + dotColumn + size(line, smallFont) + leading)
                    }
                }
            }
            for row in card.overflow {
                widest = max(widest, indent + size("\(row.label)  \(row.percentText)", bodyFont) + leading)
            }
        }
        return ceil(widest)
    }

    init(card: Card, width: CGFloat, leading: CGFloat, collapsed: Bool, canMoveUp: Bool, canMoveDown: Bool,
         onPin: @escaping (String) -> Void, onHide: @escaping (String) -> Void, onMove: @escaping (String, Int) -> Void,
         onToggle: @escaping (String) -> Void) {
        self.card = card
        self.collapsed = collapsed
        self.onToggle = onToggle
        self.canMoveUp = canMoveUp
        self.canMoveDown = canMoveDown
        self.onMove = onMove
        self.width = width
        self.leading = leading + Self.innerPad
        self.onPin = onPin
        self.onHide = onHide
        super.init(frame: .zero)
        turn = collapsed ? 0 : 1
        // The panel animates the height; the drawing stays its own size at the top and is cut off below.
        wantsLayer = true
        layerContentsRedrawPolicy = .onSetNeedsDisplay
        layerContentsPlacement = .topLeft
        layer?.masksToBounds = true
        frame = NSRect(x: 0, y: 0, width: width, height: fittingHeight)
        setAccessibilityRole(.group)
        setAccessibilityLabel(Self.summary(card))
    }

    /// The height the card needs as it is now.
    var fittingHeight: CGFloat { render(draw: false) }

    override class func defaultAnimation(forKey key: NSAnimatablePropertyKey) -> Any? {
        ["progress", "hover", "turn"].contains(key) ? CABasicAnimation() : super.defaultAnimation(forKey: key)
    }

    /// Shows a new state. With `animated`, rings and percents move there from what is on screen, and
    /// everything else crossfades. The caller sets the frame (see `fittingHeight`).
    func update(card: Card, collapsed: Bool, canMoveUp: Bool, canMoveDown: Bool, animated: Bool) {
        let cardChanged = card != self.card, collapsedChanged = collapsed != self.collapsed
        guard cardChanged || collapsedChanged || canMoveUp != self.canMoveUp || canMoveDown != self.canMoveDown else { return }
        if cardChanged {
            startFractions = shownFractions
            previous = self.card
        }
        (self.card, self.collapsed, self.canMoveUp, self.canMoveDown) = (card, collapsed, canMoveUp, canMoveDown)
        setAccessibilityLabel(Self.summary(card))
        needsDisplay = true
        guard animated else {
            (progress, turn) = (1, collapsed ? 0 : 1)
            return
        }
        Motion.fade(self, collapsedChanged ? 0.22 : 0.15)
        if cardChanged {
            progress = 0
            Motion.animate(0.4) { animator().progress = 1 }
        }
        if collapsedChanged { Motion.animate(0.22) { animator().turn = collapsed ? 0 : 1 } }
    }

    /// Sweeps the rings and counts the percents up from zero; played when the panel opens.
    func playIntro() {
        (previous, startFractions, progress) = (nil, [], 0)
        Motion.animate(0.5) { animator().progress = 1 }
    }

    /// The ring fractions to draw now, per chart.
    private var shownFractions: [[Double?]] {
        Tween.fractions(from: startFractions, to: card, progress: Double(progress))
    }

    /// `text` on its way from the text `old` picks out of the previous card.
    private func counted(_ text: String, _ old: (Card) -> String?) -> String {
        progress < 1 ? Tween.percent(from: previous.flatMap(old), to: text, progress: Double(progress)) : text
    }

    /// `color(tone)` on its way from the tone `old` picks out of the previous card.
    private func toned(_ tone: Tone, _ color: (Tone) -> NSColor, _ old: (Card) -> Tone?) -> NSColor {
        guard progress < 1, let from = previous.flatMap(old), from != tone else { return color(tone) }
        return color(from).blended(withFraction: progress, of: color(tone)) ?? color(tone)
    }

    /// The pointer may be over a card that moved under it without a mouse event; the panel calls this after it moves cards.
    func refreshHover() {
        guard let window else { return }
        setCardHovered(bounds.contains(convert(window.mouseLocationOutsideOfEventStream, from: nil)))
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    override func draw(_ dirtyRect: NSRect) {
        let outline = NSBezierPath(roundedRect: bounds.insetBy(dx: Self.sideGap + 0.5, dy: Self.outerGap + 0.5), xRadius: 10, yRadius: 10)
        outline.lineWidth = 1
        (NSColor.separatorColor.blended(withFraction: hover, of: .tertiaryLabelColor)
            ?? (cardHovered ? .tertiaryLabelColor : .separatorColor)).setStroke()
        outline.stroke()
        _ = render(draw: true)
    }

    /// A new card under the pointer starts hovered.
    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        guard let window else { return }
        cardHovered = bounds.contains(convert(window.mouseLocationOutsideOfEventStream, from: nil))
        hover = cardHovered ? 1 : 0
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
            if case .pin = target.kind {} else if !cardHovered { return false }
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
    override func mouseExited(with event: NSEvent) { setCardHovered(false) }

    /// The outline and the buttons fade with `hover`; clicks follow `cardHovered` at once.
    private func setCardHovered(_ value: Bool, _ event: NSEvent? = nil) {
        if cardHovered != value {
            cardHovered = value
            Motion.animate(0.12) { animator().hover = value ? 1 : 0 }
        }
        setState(hovered: value ? event.flatMap(target(at:)) : nil, pressed: value ? pressed : nil)
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
        switch targets[index].kind {
        case .pin: onPin(targets[index].key)
        case .hide: onHide(targets[index].key)
        case .move(let step): onMove(targets[index].key, step)
        case .toggle: onToggle(targets[index].key)
        }
    }

    // MARK: Layout and drawing

    /// Lays the card out top to bottom; draws it when `draw` is set. Returns the height.
    private func render(draw: Bool) -> CGFloat {
        targets = []
        let edge = collapsed ? Self.collapsedEdge : Self.edge
        var y = Self.outerGap + edge
        let x = leading, right = width - leading, indent = x
        let titleLine = Self.box + 3, titleX = indent + Self.chevronSpace

        chevron(at: NSPoint(x: indent, y: y), height: titleLine, draw: draw)
        text(card.title, font: card.pinned ? Self.boldTitleFont : Self.titleFont, color: .labelColor,
             at: NSPoint(x: titleX, y: y), height: titleLine, draw: draw)
        // Hide and the arrows are recorded first so they win where their boxes overlap the pin's wide hover area.
        let hideX = right - Self.pinWidth - 6 - Self.hideWidth, buttonY = y + (titleLine - Self.box) / 2
        hideButton(at: NSPoint(x: hideX, y: buttonY), draw: draw)
        if canMoveDown { moveButton(1, at: NSPoint(x: hideX - 4 - Self.box, y: buttonY), draw: draw) }
        if canMoveUp { moveButton(-1, at: NSPoint(x: hideX - 8 - 2 * Self.box, y: buttonY), draw: draw) }
        pin(card.pinned, key: card.pin, label: name,
            at: NSPoint(x: right - Self.pinWidth, y: y + (titleLine - Self.box) / 2),
            hit: NSRect(x: x, y: y, width: right - x, height: titleLine), draw: draw)
        if collapsed, draw, hover < 1 {
            faded(1 - hover) { pills(right: right - (card.pinned ? Self.pinWidth + 6 : 0), y: y, height: titleLine) }
        }
        y += titleLine
        if let account = card.subtitle {
            text(account, font: Self.bodyFont, color: .secondaryLabelColor, at: NSPoint(x: titleX, y: y), height: Self.line - 4, draw: draw)
            y += Self.line - 4
        }

        let notices = [card.message].compactMap { $0 } + card.notices
        for notice in notices {
            y += Self.small
            text(notice.text, font: Self.bodyFont, color: Self.color(notice.tone), at: NSPoint(x: indent, y: y), height: 18, draw: draw)
            y += 18
        }

        for (index, chart) in (collapsed ? [] : card.charts).enumerated() {
            y += index == 0 ? Self.medium : Self.large
            let top = y
            drawChart(chart, index, in: NSRect(x: x, y: top, width: Self.chartSize, height: Self.chartSize), draw: draw)
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
            for (rowIndex, row) in chart.rows.enumerated() {
                drawRow(row, chart: index, index: rowIndex, x: columnX, right: right, y: rowY, draw: draw)
                rowY += Self.rowHeight(row)
            }
            y = top + max(Self.chartSize, rowsHeight)
        }
        for row in collapsed ? [] : card.overflow {
            y += Self.small
            text("\(row.label)  \(row.percentText)", font: Self.bodyFont, color: Self.valueColor(row.tone),
                 at: NSPoint(x: indent, y: y), height: Self.line, draw: draw)
            if let key = row.pin {
                pin(row.pinned, key: key, label: row.label, at: NSPoint(x: right - Self.pinWidth, y: y + (Self.line - Self.box) / 2),
                    hit: NSRect(x: x, y: y, width: right - x, height: Self.line), draw: draw)
            }
            y += Self.line
        }
        return y + edge + Self.outerGap
    }

    /// Draws `body` at `alpha`.
    private func faded(_ alpha: CGFloat, _ body: () -> Void) {
        guard let context = NSGraphicsContext.current?.cgContext else { return body() }
        context.saveGState()
        context.setAlpha(alpha)
        context.beginTransparencyLayer(auxiliaryInfo: nil)
        body()
        context.endTransparencyLayer()
        context.restoreGState()
    }

    /// The disclosure chevron left of the title: right-pointing when collapsed, down when expanded. Always drawn.
    private func chevron(at origin: NSPoint, height: CGFloat, draw: Bool) {
        let hit = NSRect(x: origin.x - 6, y: origin.y, width: Self.chevronSpace + 8, height: height)
        let index = targets.count
        targets.append(PinTarget(rect: hit, box: hit, key: card.id, label: name, kind: .toggle))
        guard draw else { return }
        let center = NSPoint(x: origin.x + 4, y: origin.y + height / 2)
        let path = NSBezierPath()
        // The right-pointing shape, turned a quarter clockwise as `turn` goes to 1.
        let angle = Double(turn) * .pi / 2
        for (step, point) in [(-2.0, -4.0), (2, 0), (-2, 4)].enumerated() {
            let p = NSPoint(x: center.x + CGFloat(point.0 * cos(angle) - point.1 * sin(angle)),
                            y: center.y + CGFloat(point.0 * sin(angle) + point.1 * cos(angle)))
            step == 0 ? path.move(to: p) : path.line(to: p)
        }
        path.lineWidth = 1.5
        path.lineCapStyle = .round
        path.lineJoinStyle = .round
        (hovered == index || pressed == index ? NSColor.labelColor : NSColor.secondaryLabelColor).setStroke()
        path.stroke()
    }

    /// A collapsed card's values, right-aligned to `right`, in the same tinted pills as the rows.
    private func pills(right: CGFloat, y: CGFloat, height: CGFloat) {
        var x = right
        for (index, pill) in card.pills.enumerated().reversed() {
            let old: (Card) -> CardPill? = { $0.pills.indices.contains(index) ? $0.pills[index] : nil }
            let value = NSAttributedString(string: counted(pill.text) { old($0)?.text },
                                           attributes: [.font: Self.bodyFont, .foregroundColor: toned(pill.tone, Self.valueColor) { old($0)?.tone }])
            let box = NSRect(x: x - value.size().width - 2 * Self.pillPadding, y: y + (height - Self.pillHeight) / 2,
                             width: value.size().width + 2 * Self.pillPadding, height: Self.pillHeight)
            toned(pill.tone, Self.color) { old($0)?.tone }.withAlphaComponent(0.16).setFill()
            NSBezierPath(roundedRect: box, xRadius: Self.pillHeight / 2, yRadius: Self.pillHeight / 2).fill()
            value.draw(at: NSPoint(x: box.minX + Self.pillPadding, y: box.midY - value.size().height / 2))
            x = box.minX - 6
        }
    }

    /// A row's height plus the gap below it.
    private static func rowHeight(_ row: CardRow) -> CGFloat {
        line + CGFloat([row.count, row.reset].compactMap { $0 }.count) * resetLine + medium
    }

    /// Dot in the ring's color, label, then the value on a tinted background at the right; under the
    /// label, in small gray text, the count and the reset time when the row has them.
    private func drawRow(_ row: CardRow, chart: Int, index: Int, x: CGFloat, right: CGFloat, y: CGFloat, draw: Bool) {
        let old: (Card) -> CardRow? = { card in
            guard card.charts.indices.contains(chart), card.charts[chart].rows.indices.contains(index) else { return nil }
            return card.charts[chart].rows[index]
        }
        if draw, let index = row.ringIndex {
            shade(index).setFill()
            NSBezierPath(ovalIn: NSRect(x: x, y: y + (Self.line - 7) / 2, width: 7, height: 7)).fill()
        }
        text(row.label, font: Self.bodyFont, color: .labelColor, at: NSPoint(x: x + Self.dotColumn, y: y), height: Self.line, draw: draw)
        let pinSpace: CGFloat = row.pin == nil ? 0 : Self.pinWidth + 6
        let value = NSAttributedString(string: counted(row.percentText) { old($0)?.percentText },
                                       attributes: [.font: Self.bodyFont, .foregroundColor: toned(row.tone, Self.valueColor) { old($0)?.tone }])
        let pill = NSRect(x: right - pinSpace - value.size().width - 2 * Self.pillPadding, y: y + (Self.line - Self.pillHeight) / 2,
                          width: value.size().width + 2 * Self.pillPadding, height: Self.pillHeight)
        if draw {
            toned(row.tone, Self.color) { old($0)?.tone }.withAlphaComponent(0.16).setFill()
            NSBezierPath(roundedRect: pill, xRadius: Self.pillHeight / 2, yRadius: Self.pillHeight / 2).fill()
            value.draw(at: NSPoint(x: pill.minX + Self.pillPadding, y: pill.midY - value.size().height / 2))
        }
        if let key = row.pin {
            pin(row.pinned, key: key, label: row.label, at: NSPoint(x: right - Self.pinWidth, y: y + (Self.line - Self.box) / 2),
                hit: NSRect(x: x, y: y, width: right - x, height: Self.line), draw: draw)
        }
        for (index, line) in [row.count, row.reset].compactMap({ $0 }).enumerated() {
            text(line, font: Self.smallFont, color: .secondaryLabelColor,
                 at: NSPoint(x: x + Self.dotColumn, y: y + Self.line + CGFloat(index) * Self.resetLine), height: Self.resetLine, draw: draw)
        }
    }

    private func drawChart(_ chart: CardChart, _ chartIndex: Int, in box: NSRect, draw: Bool) {
        guard draw else { return }
        let center = NSPoint(x: box.midX, y: box.midY)
        let fractions = shownFractions[chartIndex]
        for index in chart.rings.indices {
            let radius = box.width / 2 - Self.thickness / 2 - CGFloat(index) * (Self.thickness + Self.gap)
            Self.arc(center: center, radius: radius, fraction: 1, color: .separatorColor)
            if let fraction = fractions[index], fraction > 0 {
                Self.arc(center: center, radius: radius, fraction: fraction, color: shade(index))
            }
        }
        let font = NSFont.menuFont(ofSize: 11)
        let text = counted(chart.center) { $0.charts.indices.contains(chartIndex) ? $0.charts[chartIndex].center : nil }
        let label = NSAttributedString(string: text, attributes: [.font: font, .foregroundColor: NSColor.labelColor])
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

    /// An up (`-1`) or down (`1`) arrow button; drawn only while the pointer is over the card.
    private func moveButton(_ step: Int, at origin: NSPoint, draw: Bool) {
        let hit = NSRect(origin: origin, size: NSSize(width: Self.box, height: Self.box)).insetBy(dx: -2, dy: -2)
        button(.move(step: step), key: card.id, label: name, at: origin, hit: hit, draw: draw)
    }

    /// Records the click area; draws an outlined text button that reacts to hover and press.
    private func button(_ kind: PinTarget.Kind, key: String, label: String, at origin: NSPoint, hit: NSRect, draw: Bool) {
        var pinned = false, width = Self.hideWidth, title = Self.hideLabel
        if case .pin(let value) = kind {
            (pinned, width) = (value, Self.pinWidth)
            title = NSAttributedString(string: value ? "Pinned" : "Pin", attributes: [.font: Self.pinFont])
        } else if case .move(let step) = kind {
            width = Self.box
            title = NSAttributedString(string: step < 0 ? "\u{2191}" : "\u{2193}", attributes: [.font: Self.pinFont])
        }
        let box = NSRect(origin: origin, size: NSSize(width: width, height: Self.box))
        let index = targets.count
        targets.append(PinTarget(rect: hit, box: box, key: key, label: label, kind: kind))
        let isPressed = pressed == index, isHovered = hovered == index
        guard draw, hover > 0 || pinned else { return }
        guard pinned || hover == 1 else {
            return faded(hover) { paint(box, title, pinned: pinned, isHovered: isHovered, isPressed: isPressed) }
        }
        paint(box, title, pinned: pinned, isHovered: isHovered, isPressed: isPressed)
    }

    private func paint(_ box: NSRect, _ title: NSAttributedString, pinned: Bool, isHovered: Bool, isPressed: Bool) {
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
                case .move(let step): self?.onMove(target.key, step)
                case .toggle: self?.onToggle(target.key)
                }
            })
            element.setAccessibilityRole(.button)
            switch target.kind {
            case .pin(let pinned): element.setAccessibilityLabel("\(pinned ? "Unpin" : "Pin") \(target.label)")
            case .hide: element.setAccessibilityLabel("Hide \(target.label)")
            case .move(let step): element.setAccessibilityLabel("Move \(target.label) \(step < 0 ? "up" : "down")")
            case .toggle: element.setAccessibilityLabel("\(collapsed ? "Expand" : "Collapse") \(target.label)")
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
