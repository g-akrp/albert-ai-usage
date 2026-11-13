import AIUsageCore
import AppKit

/// What the panel shows: the cards top to bottom, then the rows.
struct PanelContent {
    var cards: [Card]
    /// Ids of the cards shown as one line.
    var collapsed: Set<String>
    var rows: [PanelRow]
}

struct PanelActions {
    var pin: (String) -> Void
    var hide: (String) -> Void
    var move: (String, Int) -> Void
    var toggle: (String) -> Void
    var row: (PanelRow.Kind) -> Void
}

/// The window under the menu bar icon. Unlike a menu it stays open when something in it is clicked,
/// and it keeps its card and row views between updates, so a change can animate.
final class PanelController: NSObject, NSWindowDelegate {
    private static let pad: CGFloat = 5, leading: CGFloat = 14, gapUnderBar: CGFloat = 8

    private final class Panel: NSPanel {
        /// Returns whether the key was used.
        var onKey: ((NSEvent) -> Bool)?
        override var canBecomeKey: Bool { true }
        override func keyDown(with event: NSEvent) {
            if onKey?(event) != true { super.keyDown(with: event) }
        }
        override func performKeyEquivalent(with event: NSEvent) -> Bool {
            onKey?(event) == true || super.performKeyEquivalent(with: event)
        }
    }

    private final class Stack: NSView {
        override var isFlipped: Bool { true }

        /// A view that is not faded in yet, or is fading out, lies over visible ones and must not take their clicks.
        override func hitTest(_ point: NSPoint) -> NSView? {
            let local = superview.map { convert(point, from: $0) } ?? point
            guard bounds.contains(local) else { return nil }
            return subviews.reversed().first { $0.alphaValue > 0 && $0.frame.contains(local) }?.hitTest(local) ?? self
        }
    }

    private let actions: PanelActions
    private var panel: Panel?
    private var stack: Stack?
    private weak var button: NSStatusBarButton?
    private var cardViews: [CardView] = []
    private var rowViews: [RowView] = []
    /// Views fading out before they are removed.
    private var leaving: [NSView] = []
    private var width: CGFloat = 0
    /// Counts updates, so the later steps of an animation that a newer update replaced do nothing.
    private var generation = 0
    private weak var highlightedRow: RowView?
    private var monitors: [Any] = []

    private(set) var isOpen = false
    /// When the panel last closed; a click on the icon right after must not open it again.
    private var closedAt: TimeInterval = 0
    var justClosed: Bool { ProcessInfo.processInfo.systemUptime - closedAt < 0.3 }
    var onClose: (() -> Void)?

    init(actions: PanelActions) {
        self.actions = actions
    }

    // MARK: Open and close

    func open(under button: NSStatusBarButton, content: PanelContent) {
        guard !isOpen else { return }
        isOpen = true
        self.button = button

        let panel = Panel(contentRect: .zero, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: true)
        panel.level = .popUpMenu
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = true
        panel.collectionBehavior = [.transient, .ignoresCycle, .fullScreenAuxiliary]
        panel.animationBehavior = .none
        panel.delegate = self
        panel.onKey = { [weak self] in self?.key($0) ?? false }

        let background = NSVisualEffectView()
        background.material = .menu
        background.state = .active
        background.wantsLayer = true
        background.layer?.cornerRadius = 10
        background.layer?.masksToBounds = true
        let scroll = NSScrollView()
        scroll.drawsBackground = false
        scroll.hasVerticalScroller = true
        scroll.autohidesScrollers = true
        scroll.scrollerStyle = .overlay
        scroll.autoresizingMask = [.width, .height]
        let stack = Stack()
        scroll.documentView = stack
        background.addSubview(scroll)
        panel.contentView = background
        scroll.frame = background.bounds
        (self.panel, self.stack) = (panel, stack)

        update(content, animated: false)
        let frame = panel.frame
        panel.setFrame(frame.offsetBy(dx: 0, dy: 6), display: false)
        panel.alphaValue = 0
        panel.makeKeyAndOrderFront(nil)
        Motion.animate(0.15) {
            panel.animator().alphaValue = 1
            panel.animator().setFrame(frame, display: true)
        }
        cardViews.forEach { $0.playIntro() }

        // A click anywhere else closes the panel. The icon's own click is left to its action, which toggles.
        let clicks: NSEvent.EventTypeMask = [.leftMouseDown, .rightMouseDown, .otherMouseDown]
        monitors = [
            NSEvent.addGlobalMonitorForEvents(matching: clicks) { [weak self] _ in
                guard let self, let frame = self.button.flatMap({ $0.window?.convertToScreen($0.convert($0.bounds, to: nil)) }),
                      !frame.contains(NSEvent.mouseLocation) else { return }
                self.close()
            },
            NSEvent.addLocalMonitorForEvents(matching: clicks) { [weak self] event in
                if let self, event.window !== self.panel, event.window !== self.button?.window { self.close() }
                return event
            },
        ].compactMap { $0 }
    }

    func close() {
        guard isOpen, let closing = panel else { return }
        isOpen = false
        closedAt = ProcessInfo.processInfo.systemUptime
        monitors.forEach(NSEvent.removeMonitor)
        monitors = []
        generation += 1
        // The views go with the window, so a closed panel costs no memory.
        (panel, stack, cardViews, rowViews, leaving, highlightedRow) = (nil, nil, [], [], [], nil)
        closing.delegate = nil
        Motion.animate(0.1, { closing.animator().alphaValue = 0 }, completion: { closing.orderOut(nil) })
        onClose?()
    }

    func windowDidResignKey(_ notification: Notification) {
        // A click on the icon reaches its action next; closing here would make that action open the panel again.
        // `currentEvent` can be an old click, so it must be recent.
        if let event = NSApp.currentEvent, event.window != nil, event.window === button?.window,
           ProcessInfo.processInfo.systemUptime - event.timestamp < 0.5 { return }
        close()
    }

    // MARK: Content

    /// Shows `content`, animating from what is shown. Does nothing while the panel is closed.
    func update(_ content: PanelContent) {
        update(content, animated: true)
    }

    private func update(_ content: PanelContent, animated: Bool) {
        guard isOpen, let panel, let stack, let button, let anchorWindow = button.window else { return }
        generation += 1
        let run = generation
        leaving.forEach { $0.removeFromSuperview() }
        leaving = []

        // A new width moves every right edge; that is rare (a longer label), so it is not animated.
        let newWidth = max(CardView.width(for: content.cards, leading: Self.leading), RowView.width(for: content.rows))
        let animated = animated && newWidth == width
        if !animated {
            (cardViews + rowViews).forEach { $0.removeFromSuperview() }
            (cardViews, rowViews, width) = ([], [], newWidth)
        }

        var oldCards = cardViews, oldRows = rowViews
        cardViews = content.cards.enumerated().map { index, card in
            let collapsed = content.collapsed.contains(card.id)
            let canMoveUp = index > 0, canMoveDown = index < content.cards.count - 1
            if let found = oldCards.firstIndex(where: { $0.card.id == card.id }) {
                let view = oldCards.remove(at: found)
                view.update(card: card, collapsed: collapsed, canMoveUp: canMoveUp, canMoveDown: canMoveDown, animated: true)
                return view
            }
            let view = CardView(card: card, width: width, leading: Self.leading, collapsed: collapsed, canMoveUp: canMoveUp,
                                canMoveDown: canMoveDown, onPin: actions.pin, onHide: actions.hide, onMove: actions.move,
                                onToggle: actions.toggle)
            if animated { view.playIntro() }
            return view
        }
        // Rows of one kind (separators, error lines) are matched in order.
        rowViews = content.rows.map { row in
            if let found = oldRows.firstIndex(where: { $0.row.kind == row.kind }) {
                let view = oldRows.remove(at: found)
                view.update(row)
                return view
            }
            let view = RowView(row: row, onClick: actions.row)
            view.onHover = { [weak self] in if $0?.alphaValue != 0 { self?.highlight($0) } }
            return view
        }
        leaving = oldCards + oldRows
        if let row = highlightedRow, oldRows.contains(where: { $0 === row }) { highlight(nil) }

        let views: [NSView] = cardViews + rowViews
        let heights = cardViews.map(\.fittingHeight) + rowViews.map { RowView.height($0.row) }
        let stacked = PanelLayout.stack(heights)
        let frames = zip(stacked.tops, heights).map { NSRect(x: 0, y: Self.pad + $0, width: width, height: $1) }
        for (view, frame) in zip(views, frames) where view.superview == nil {
            view.frame = frame
            view.alphaValue = animated ? 0 : 1
            stack.addSubview(view)
        }

        let screen = (anchorWindow.screen ?? NSScreen.main)?.visibleFrame ?? .zero
        let contentHeight = stacked.total + 2 * Self.pad
        let size = NSSize(width: width, height: PanelLayout.height(content: contentHeight, available: screen.height - Self.gapUnderBar))
        let anchor = anchorWindow.convertToScreen(button.convert(button.bounds, to: nil))
        let panelFrame = NSRect(origin: PanelLayout.origin(anchor: anchor, size: size, screen: screen), size: size)

        guard animated else {
            stack.frame = NSRect(x: 0, y: 0, width: width, height: contentHeight)
            zip(views, frames).forEach { $0.frame = $1 }
            panel.setFrame(panelFrame, display: true)
            return
        }
        // In turn: what goes fades out, the rest slides to its place, what is new fades in.
        let slide = { [weak self] in
            guard let self, run == self.generation else { return }
            self.leaving.forEach { $0.removeFromSuperview() }
            self.leaving = []
            // The stack must hold both the old and the new places while the views slide.
            stack.frame.size = NSSize(width: self.width, height: max(stack.frame.height, contentHeight))
            Motion.animate(0.22, {
                zip(views, frames).forEach { $0.animator().frame = $1 }
                panel.animator().setFrame(panelFrame, display: true)
            }, completion: { [weak self] in
                guard let self, run == self.generation else { return }
                stack.frame.size.height = contentHeight
                Motion.animate(0.15) { views.forEach { $0.animator().alphaValue = 1 } }
                self.cardViews.forEach { $0.refreshHover() }
            })
        }
        if leaving.isEmpty {
            slide()
        } else {
            Motion.animate(0.15, { leaving.forEach { $0.animator().alphaValue = 0 } }, completion: slide)
        }
    }

    /// Shows a changed Updated row without a full update, so a ticking countdown does not interrupt animations.
    func refreshUpdated(_ row: PanelRow) {
        rowViews.first { $0.row.kind == .updated }?.update(row)
    }

    // MARK: Keyboard

    private func highlight(_ row: RowView?) {
        guard row !== highlightedRow else { return }
        highlightedRow?.highlighted = false
        highlightedRow = row
        row?.highlighted = true
    }

    private func key(_ event: NSEvent) -> Bool {
        guard event.type == .keyDown else { return false }
        if event.modifierFlags.contains(.command) {
            switch event.charactersIgnoringModifiers {
            case "r": actions.row(.refresh)
            case "q": actions.row(.quit)
            default: return false
            }
            return true
        }
        switch event.keyCode {
        case 53: close()                                        // Esc
        case 125, 126:                                          // Down, Up
            let current = highlightedRow.flatMap { row in rowViews.firstIndex { $0 === row } }
            guard let next = PanelModel.highlight(from: current, step: event.keyCode == 125 ? 1 : -1,
                                                  enabled: rowViews.map(\.row.enabled)) else { return true }
            highlight(rowViews[next])
            rowViews[next].scrollToVisible(rowViews[next].bounds)
        case 36, 76, 49:                                        // Return, Enter, Space
            guard let row = highlightedRow?.row, row.enabled else { return true }
            actions.row(row.kind)
        default: return false
        }
        return true
    }
}
