import AIUsageCore
import AppKit

/// Every animation in the panel goes through here, so Reduce Motion switches all of them off at once.
enum Motion {
    /// `seconds`, or 0 with Reduce Motion on.
    static func duration(_ seconds: TimeInterval) -> TimeInterval {
        NSWorkspace.shared.accessibilityDisplayShouldReduceMotion ? 0 : seconds
    }

    /// Runs `changes` (made through `animator()`) with an ease-out curve.
    static func animate(_ seconds: TimeInterval, _ changes: () -> Void, completion: (() -> Void)? = nil) {
        NSAnimationContext.runAnimationGroup({ context in
            context.duration = duration(seconds)
            context.timingFunction = CAMediaTimingFunction(name: .easeOut)
            context.allowsImplicitAnimation = true
            changes()
        }, completionHandler: completion)
    }

    /// Crossfades `view`'s next redraw.
    static func fade(_ view: NSView, _ seconds: TimeInterval) {
        let length = duration(seconds)
        guard length > 0 else { return }
        let transition = CATransition()
        transition.type = .fade
        transition.duration = length
        view.layer?.add(transition, forKey: "fade")
    }
}

/// One line under the cards: a title with an optional check mark, shortcut, chevron or spinner at the
/// right. It looks like a menu item, but a click calls `onClick` and leaves the panel open.
final class RowView: NSView {
    private static let font = NSFont.menuFont(ofSize: 0), side: CGFloat = 14, indent: CGFloat = 14
    private static let trailingGap: CGFloat = 24, symbolSpace: CGFloat = 16, heartSpace: CGFloat = 18

    private(set) var row: PanelRow
    private let onClick: (PanelRow.Kind) -> Void
    /// The pointer entered this row, or (`nil`) left it.
    var onHover: ((RowView?) -> Void)?
    var highlighted = false { didSet { if highlighted != oldValue { needsDisplay = true } } }
    private var pressed = false { didSet { if pressed != oldValue { needsDisplay = true } } }
    private var spinner: NSProgressIndicator?

    override var isFlipped: Bool { true }

    static func height(_ row: PanelRow) -> CGFloat { row.kind == .separator ? 11 : 22 }

    /// The width that fits every row's title and what sits at its right.
    static func width(for rows: [PanelRow]) -> CGFloat {
        rows.map { row in
            var width = side + (row.indented ? indent : 0) + size(row.title).width + trailingGap + side
            if case .credit = row.kind { width += heartSpace }
            return ceil(width + max(symbolSpace, row.shortcut.map { size($0).width } ?? 0))
        }.max() ?? 0
    }

    private static func size(_ text: String) -> NSSize {
        NSAttributedString(string: text, attributes: [.font: font]).size()
    }

    init(row: PanelRow, onClick: @escaping (PanelRow.Kind) -> Void) {
        self.row = row
        self.onClick = onClick
        super.init(frame: .zero)
        wantsLayer = true
        layerContentsRedrawPolicy = .onSetNeedsDisplay
        describe()
        spin()
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    /// Shows `row`; a changed check mark, chevron or text crossfades.
    func update(_ row: PanelRow) {
        guard row != self.row else { return }
        self.row = row
        if row.kind != .updated { Motion.fade(self, 0.12) }
        needsDisplay = true
        describe()
        spin()
    }

    private func spin() {
        if row.spinning, spinner == nil {
            let indicator = NSProgressIndicator()
            indicator.style = .spinning
            indicator.controlSize = .small
            indicator.isDisplayedWhenStopped = false
            addSubview(indicator)
            spinner = indicator
            placeSpinner()
        }
        row.spinning ? spinner?.startAnimation(nil) : spinner?.stopAnimation(nil)
    }

    override func setFrameSize(_ newSize: NSSize) {
        super.setFrameSize(newSize)
        placeSpinner()
    }

    private func placeSpinner() {
        spinner?.frame = NSRect(x: bounds.maxX - Self.side - 16, y: (bounds.height - 16) / 2, width: 16, height: 16)
    }

    // MARK: Drawing

    override func draw(_ dirtyRect: NSRect) {
        if row.kind == .separator {
            NSColor.separatorColor.setFill()
            NSRect(x: Self.side, y: floor(bounds.midY), width: bounds.width - 2 * Self.side, height: 1).fill()
            return
        }
        let lit = highlighted && row.enabled
        if lit {
            // Pressed is drawn at once, with no animation: feedback for a click must not lag.
            NSColor.selectedContentBackgroundColor.withAlphaComponent(pressed ? 0.8 : 1).setFill()
            NSBezierPath(roundedRect: bounds.insetBy(dx: 5, dy: 0), xRadius: 4, yRadius: 4).fill()
        }
        let color: NSColor = lit ? .selectedMenuItemTextColor : row.kind == .detail ? Self.color(row.tone)
            : row.enabled ? .labelColor : .tertiaryLabelColor
        let faint: NSColor = lit ? .selectedMenuItemTextColor : .secondaryLabelColor

        var x = Self.side + (row.indented ? Self.indent : 0)
        if case .credit(let before, let after) = row.kind {
            x += text(before + " ", color, x: x)
            symbol("heart.fill", color, in: NSRect(x: x, y: 0, width: Self.heartSpace - 4, height: bounds.height))
            _ = text(after, color, x: x + Self.heartSpace)
        } else {
            _ = text(row.title, color, x: x)
        }

        let trailing = NSRect(x: bounds.maxX - Self.side - Self.symbolSpace, y: 0, width: Self.symbolSpace, height: bounds.height)
        if let open = row.expanded {
            symbol(open ? "chevron.down" : "chevron.right", faint, in: trailing)
        } else if row.checked == true {
            symbol("checkmark", color, in: trailing)
        } else if let shortcut = row.shortcut, !row.spinning {
            _ = text(shortcut, faint, x: bounds.maxX - Self.side - Self.size(shortcut).width)
        }
    }

    /// Draws `string` vertically centered; returns its width.
    private func text(_ string: String, _ color: NSColor, x: CGFloat) -> CGFloat {
        let attributed = NSAttributedString(string: string, attributes: [.font: Self.font, .foregroundColor: color])
        let size = attributed.size()
        attributed.draw(at: NSPoint(x: x, y: (bounds.height - size.height) / 2))
        return size.width
    }

    /// Draws an SF Symbol in `color`, centered in `box`.
    private func symbol(_ name: String, _ color: NSColor, in box: NSRect) {
        let configuration = NSImage.SymbolConfiguration(pointSize: 11, weight: .medium)
        guard let shape = NSImage(systemSymbolName: name, accessibilityDescription: nil)?.withSymbolConfiguration(configuration) else { return }
        let size = shape.size
        // Filled over the shape, so the symbol gets exactly the text's color, alpha included.
        let image = NSImage(size: size, flipped: false) { rect in
            shape.draw(in: rect)
            color.set()
            rect.fill(using: .sourceAtop)
            return true
        }
        image.draw(in: NSRect(x: box.midX - size.width / 2, y: box.midY - size.height / 2, width: size.width, height: size.height),
                   from: .zero, operation: .sourceOver, fraction: 1, respectFlipped: true, hints: nil)
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

    // MARK: Pointer

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        trackingAreas.forEach(removeTrackingArea)
        addTrackingArea(NSTrackingArea(rect: .zero, options: [.mouseEnteredAndExited, .activeAlways, .inVisibleRect], owner: self))
    }

    override func mouseEntered(with event: NSEvent) { onHover?(self) }

    override func mouseExited(with event: NSEvent) {
        pressed = false
        onHover?(nil)
    }

    override func mouseDown(with event: NSEvent) { pressed = row.enabled }

    override func mouseUp(with event: NSEvent) {
        let inside = bounds.contains(convert(event.locationInWindow, from: nil))
        guard pressed, inside else { pressed = false; return }
        pressed = false
        onClick(row.kind)
    }

    // MARK: Accessibility

    private func describe() {
        setAccessibilityElement(row.kind != .separator)
        setAccessibilityLabel(row.title)
        setAccessibilityEnabled(row.enabled)
        if let checked = row.checked {
            setAccessibilityRole(.checkBox)
            setAccessibilityValue(checked ? 1 : 0)
        } else {
            setAccessibilityRole(row.enabled ? .button : .staticText)
            // A section row says whether it is open.
            setAccessibilityValue(row.expanded.map { $0 ? "expanded" : "collapsed" })
        }
    }

    override func accessibilityPerformPress() -> Bool {
        guard row.enabled else { return false }
        onClick(row.kind)
        return true
    }
}
