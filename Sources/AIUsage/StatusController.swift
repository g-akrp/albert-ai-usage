import AIUsageCore
import AppKit
import ServiceManagement

final class StatusController: NSObject, NSMenuDelegate {
    private let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
    private let menu = NSMenu()
    private let defaults = UserDefaults.standard
    private let poller: Poller
    private let userFolder = ProviderStore.userFolder(home: NSHomeDirectory())
    private let resetFormatter = MenuModel.resetFormatter()
    private let timeFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.timeStyle = .short
        return formatter
    }()

    private var icon: IconSpec?
    private var cardWidth: CGFloat = 300
    /// Where card content starts, measured from the open menu: item text starts at 14 pt, or 22 pt when an
    /// item has a check mark (Launch at Login on) and macOS makes room for the check column. Cards follow.
    private var cardLeading: CGFloat = 14

    /// Provider files that are switched off.
    private var disabled: Set<String> {
        get { Set(defaults.stringArray(forKey: "disabledProviders") ?? []) }
        set { defaults.set(newValue.sorted(), forKey: "disabledProviders") }
    }

    private var visibleRuns: [ProviderRun] {
        ProviderToggle.visible(poller.runs, disabled: poller.disabled)
    }

    /// Run ids of cards hidden from the panel; those providers keep running.
    private var hiddenCards: Set<String> {
        get { Set(defaults.stringArray(forKey: "hiddenCards") ?? []) }
        set { defaults.set(newValue.sorted(), forKey: "hiddenCards") }
    }

    /// Run ids in the order the user arranged the cards; empty until a card is moved.
    private var cardOrder: [String] {
        get { defaults.stringArray(forKey: "cardOrder") ?? [] }
        set { defaults.set(newValue, forKey: "cardOrder") }
    }

    private var pinned: String? {
        get { defaults.string(forKey: "pinnedProvider") }
        set { defaults.set(newValue, forKey: "pinnedProvider") }
    }

    override init() {
        let bundled = Bundle.main.resourceURL!.appendingPathComponent("providers")
        poller = Poller(folders: [bundled, userFolder])
        super.init()
        migrateOldPin()

        menu.delegate = self
        menu.autoenablesItems = false
        statusItem.menu = menu
        statusItem.button?.imagePosition = .imageOnly
        poller.disabled = disabled
        poller.onChange = { [weak self] in self?.updateIcons() }
        updateIcons()
        poller.start()

        NSWorkspace.shared.notificationCenter.addObserver(
            forName: NSWorkspace.didWakeNotification, object: nil, queue: .main
        ) { [weak self] _ in self?.poller.tick() }
    }

    /// Versions before 1.3.0 were Albert AI Usage (bundle id local.albert-ai-usage), and the
    /// SwiftBar plugin stored its pin in a file. Settings from either are read once.
    private func migrateOldPin() {
        if !defaults.bool(forKey: "migratedFromAlbert"), let old = UserDefaults(suiteName: "local.albert-ai-usage") {
            for key in ["pinnedProvider", "disabledProviders"] where defaults.object(forKey: key) == nil {
                if let value = old.object(forKey: key) { defaults.set(value, forKey: key) }
            }
            defaults.set(true, forKey: "migratedFromAlbert")
        }
        guard defaults.object(forKey: "pinnedProvider") == nil else { return }
        let file = URL(fileURLWithPath: NSHomeDirectory()).appendingPathComponent(".config/albert-ai-usage/pinned")
        if let id = try? String(contentsOf: file, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines),
           !id.isEmpty {
            pinned = id
        }
    }

    // MARK: Menu bar icon

    private func updateIcons() {
        let effective = Pin.effective(pinned, in: visibleRuns)
        if let effective, effective != pinned { pinned = effective }
        let spec = StatusIcons.icon(runs: visibleRuns, pinned: effective)
        guard spec != icon, let button = statusItem.button else { return }
        icon = spec
        button.image = Self.image(spec)
        button.toolTip = spec.accessibility
        button.setAccessibilityLabel(spec.accessibility)
    }

    /// Draws the pixel icon at 2 points per font pixel, so it stays sharp on any display.
    private static func image(_ spec: IconSpec) -> NSImage {
        let scale: CGFloat = 2
        let layout = PixelFont.layout(top: spec.top, bottom: spec.bottom)
        let size = NSSize(width: CGFloat(layout.width) * scale, height: CGFloat(layout.height) * scale)
        let top = color(spec.topColor), bottom = color(spec.bottomColor)
        let pixels = layout.pixels
        let image = NSImage(size: size, flipped: true) { _ in
            for pixel in pixels {
                (pixel.bottomRow ? bottom : top).setFill()
                NSRect(x: CGFloat(pixel.x) * scale, y: CGFloat(pixel.y) * scale, width: scale, height: scale).fill()
            }
            return true
        }
        image.accessibilityDescription = spec.accessibility
        return image
    }

    private static func color(_ rgb: RGB) -> NSColor {
        NSColor(srgbRed: CGFloat(rgb.r) / 255, green: CGFloat(rgb.g) / 255, blue: CGFloat(rgb.b) / 255, alpha: 1)
    }

    // MARK: Menu

    func menuNeedsUpdate(_ menu: NSMenu) {
        let entries = MenuModel.entries(
            runs: visibleRuns, pinned: pinned, configErrors: poller.configErrors,
            providers: poller.providerNames.map {
                ProviderToggle(id: $0.id, name: $0.name, enabled: !poller.disabled.contains($0.id))
            },
            updated: poller.lastUpdate.map { "Updated \(timeFormatter.string(from: $0))" },
            launchAtLogin: SMAppService.mainApp.status == .enabled,
            version: AppVersion.current, hidden: hiddenCards, order: cardOrder, formatReset: resetFormatter.string(from:))
        cardLeading = SMAppService.mainApp.status == .enabled ? 22 : 14
        cardWidth = CardView.width(for: entries.compactMap { if case .card(let card) = $0 { return card }; return nil },
                                   leading: cardLeading)
        menu.removeAllItems()
        let cardCount = entries.filter { if case .card = $0 { return true }; return false }.count
        var cardIndex = 0
        for entry in entries {
            if case .card = entry { cardIndex += 1 }
            menu.addItem(item(for: entry, position: (cardIndex - 1, cardCount)))
        }
    }

    /// `position` is the card's index and the card count; ignored for other entries.
    private func item(for entry: MenuEntry, position: (index: Int, count: Int)) -> NSMenuItem {
        switch entry {
        case .separator:
            return .separator()
        case .card(let card):
            let item = NSMenuItem()
            item.view = CardView(card: card, width: cardWidth, leading: cardLeading, canMoveUp: position.index > 0,
                                 canMoveDown: position.index < position.count - 1, onPin: { [weak self] key in
                self?.pinned = key
                self?.updateIcons()
            }, onHide: { [weak self] id in
                self?.hiddenCards.insert(id)
            }, onMove: { [weak self] id, step in
                self?.moveCard(id, by: step)
            })
            return item
        case .detail(let text, let tone):
            let item = NSMenuItem(title: text, action: nil, keyEquivalent: "")
            if let color = Self.color(tone) {
                item.attributedTitle = NSAttributedString(
                    string: text, attributes: [.foregroundColor: color, .font: NSFont.menuFont(ofSize: 0)])
            }
            return item
        case .refresh:
            return action("Refresh Now", #selector(refresh), key: "r")
        case .updated(let text), .version(let text):
            let item = NSMenuItem(title: text, action: nil, keyEquivalent: "")
            item.isEnabled = false
            return item
        case .credit(let before, let after):
            let item = NSMenuItem(title: "\(before) heart \(after)", action: nil, keyEquivalent: "")
            item.isEnabled = false
            let heart = NSTextAttachment()
            if let symbol = NSImage(systemSymbolName: "heart.fill", accessibilityDescription: "love") {
                // The same gray as the text around it.
                heart.image = NSImage(size: symbol.size, flipped: false) { rect in
                    symbol.draw(in: rect)
                    NSColor.disabledControlTextColor.set()
                    rect.fill(using: .sourceAtop)
                    return true
                }
            }
            let text = NSMutableAttributedString(string: "\(before) ")
            text.append(NSAttributedString(attachment: heart))
            text.append(NSAttributedString(string: " \(after)"))
            text.addAttributes([.font: NSFont.menuFont(ofSize: 0), .foregroundColor: NSColor.disabledControlTextColor],
                               range: NSRange(location: 0, length: text.length))
            item.attributedTitle = text
            return item
        case .providers(let toggles):
            let item = NSMenuItem(title: "Providers", action: nil, keyEquivalent: "")
            let submenu = NSMenu()
            for toggle in toggles {
                let entry = action(toggle.name, #selector(toggleProvider(_:)))
                entry.representedObject = toggle.id
                entry.state = toggle.enabled ? .on : .off
                submenu.addItem(entry)
            }
            item.submenu = submenu
            return item
        case .hiddenCards(let cards):
            let item = NSMenuItem(title: "Hidden Cards", action: nil, keyEquivalent: "")
            let submenu = NSMenu()
            for card in cards {
                let entry = action("Show \(card.name)", #selector(showCard(_:)))
                entry.representedObject = card.id
                submenu.addItem(entry)
            }
            item.submenu = submenu
            return item
        case .launchAtLogin(let enabled):
            let item = action("Launch at Login", #selector(toggleLaunchAtLogin))
            item.state = enabled ? .on : .off
            return item
        case .openProvidersFolder:
            return action("Open Providers Folder…", #selector(openProvidersFolder))
        case .quit:
            return action("Quit AI Usage", #selector(quit), key: "q")
        }
    }

    private func action(_ title: String, _ selector: Selector, key: String = "") -> NSMenuItem {
        let item = NSMenuItem(title: title, action: selector, keyEquivalent: key)
        item.target = self
        return item
    }

    private static func color(_ tone: Tone) -> NSColor? {
        switch tone {
        case .normal: return nil
        case .secondary: return .secondaryLabelColor
        case .green: return .systemGreen
        case .orange: return .systemOrange
        case .red: return .systemRed
        }
    }

    @objc private func toggleProvider(_ sender: NSMenuItem) {
        guard let id = sender.representedObject as? String else { return }
        var set = disabled
        if set.contains(id) { set.remove(id) } else { set.insert(id) }
        disabled = set
        poller.disabled = set
        updateIcons()
    }

    private func moveCard(_ id: String, by step: Int) {
        let ids = CardOrder.sorted(visibleRuns, saved: cardOrder).map(\.id)
        cardOrder = CardOrder.moved(id, by: step, in: ids, hidden: hiddenCards)
    }

    @objc private func showCard(_ sender: NSMenuItem) {
        guard let id = sender.representedObject as? String else { return }
        hiddenCards.remove(id)
    }

    @objc private func refresh() {
        poller.refreshNow()
    }

    @objc private func toggleLaunchAtLogin() {
        do {
            if SMAppService.mainApp.status == .enabled {
                try SMAppService.mainApp.unregister()
            } else {
                try SMAppService.mainApp.register()
            }
        } catch {
            NSSound.beep()
        }
    }

    @objc private func openProvidersFolder() {
        try? FileManager.default.createDirectory(at: userFolder, withIntermediateDirectories: true)
        NSWorkspace.shared.open(userFolder)
    }

    @objc private func quit() {
        NSApp.terminate(nil)
    }
}
