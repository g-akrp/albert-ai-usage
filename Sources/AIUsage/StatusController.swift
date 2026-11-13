import AIUsageCore
import AppKit
import ServiceManagement

final class StatusController: NSObject {
    private let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
    private let defaults = UserDefaults.standard
    private let poller: Poller
    private let userFolder = ProviderStore.userFolder(home: NSHomeDirectory())
    private let timeFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.timeStyle = .short
        return formatter
    }()

    private var icon: IconSpec?
    /// The Refresh Every, Providers and Hidden Cards sections that are open in the panel; all close with it.
    private var expandedSections: Set<String> = []
    private lazy var panel: PanelController = {
        let panel = PanelController(actions: PanelActions(pin: { [weak self] key in
            self?.pinned = key
            self?.updateIcons()
            self?.refreshPanel()
        }, hide: { [weak self] id in
            self?.hiddenCards.insert(id)
            self?.refreshPanel()
        }, move: { [weak self] id, step in
            self?.moveCard(id, by: step)
            self?.refreshPanel()
        }, toggle: { [weak self] id in
            guard var set = self?.collapsedCards else { return }
            if set.contains(id) { set.remove(id) } else { set.insert(id) }
            self?.collapsedCards = set
            self?.refreshPanel()
        }, row: { [weak self] in self?.rowClicked($0) }))
        panel.onClose = { [weak self] in
            self?.countdown?.invalidate()
            self?.countdown = nil
            self?.expandedSections = []
            self?.statusItem.button?.highlight(false)
        }
        return panel
    }()
    /// Set by Refresh Now until every provider it started has finished; the menu bar icon dims meanwhile.
    private var manualRefresh = false
    /// Ticks each second while the panel is open, to run the refresh countdown.
    private var countdown: Timer?

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

    /// Run ids of cards shown as one line; they keep their charts for when they are expanded again.
    private var collapsedCards: Set<String> {
        get { Set(defaults.stringArray(forKey: "collapsedCards") ?? []) }
        set { defaults.set(newValue.sorted(), forKey: "collapsedCards") }
    }

    /// Seconds between runs of every provider; 5 minutes until the user picks another.
    private var refreshSeconds: Int {
        get { Schedule.valid(defaults.integer(forKey: "refreshSeconds")) }
        set { defaults.set(newValue, forKey: "refreshSeconds") }
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

        statusItem.button?.imagePosition = .imageOnly
        statusItem.button?.target = self
        statusItem.button?.action = #selector(togglePanel)
        statusItem.button?.sendAction(on: [.leftMouseDown, .rightMouseDown])
        poller.disabled = disabled
        poller.interval = refreshSeconds
        poller.onChange = { [weak self] in self?.pollerChanged() }
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

    private func pollerChanged() {
        if manualRefresh, !poller.isRunning { manualRefresh = false }
        updateIcons()
        refreshPanel()
    }

    // MARK: Menu bar icon

    private func updateIcons() {
        let effective = Pin.effective(pinned, in: visibleRuns)
        if let effective, effective != pinned { pinned = effective }
        guard let button = statusItem.button else { return }
        let alpha: CGFloat = manualRefresh ? 0.5 : 1
        if button.alphaValue != alpha {
            NSAnimationContext.runAnimationGroup { context in
                context.duration = NSWorkspace.shared.accessibilityDisplayShouldReduceMotion ? 0 : 0.2
                button.animator().alphaValue = alpha
            }
        }
        let spec = StatusIcons.icon(runs: visibleRuns, pinned: effective)
        guard spec != icon else { return }
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

    // MARK: Panel

    @objc private func togglePanel() {
        guard let button = statusItem.button else { return }
        if panel.isOpen { return panel.close() }
        if panel.justClosed { return }
        panel.open(under: button, content: content())
        let timer = Timer(timeInterval: 1, repeats: true) { [weak self] _ in
            guard let self, let row = self.content().rows.first(where: { $0.kind == .updated }) else { return }
            self.panel.refreshUpdated(row)
        }
        RunLoop.main.add(timer, forMode: .common)
        countdown = timer
        // The button drops its highlight when this click ends; set it after that.
        DispatchQueue.main.async { [weak self] in button.highlight(self?.panel.isOpen == true) }
    }

    private func content() -> PanelContent {
        let entries = MenuModel.entries(
            runs: visibleRuns, pinned: pinned, configErrors: poller.configErrors,
            providers: poller.providerNames.map {
                ProviderToggle(id: $0.id, name: $0.name, enabled: !poller.disabled.contains($0.id))
            },
            updated: MenuModel.updatedText(poller.lastUpdate, refreshing: poller.isRunning,
                                           next: poller.nextRefresh.flatMap { RelativeTime.countdown(until: $0, now: Date()) },
                                           formatTime: timeFormatter.string(from:)),
            launchAtLogin: SMAppService.mainApp.status == .enabled,
            version: AppVersion.current, hidden: hiddenCards, order: cardOrder, refreshSeconds: refreshSeconds, formatReset: { MenuModel.resetText($0, now: Date()) })
        return PanelContent(cards: PanelModel.cards(entries), collapsed: collapsedCards,
                            rows: PanelModel.rows(entries, expanded: expandedSections, refreshing: poller.isRunning))
    }

    /// Shows a card action, a row click or a finished run in the open panel.
    private func refreshPanel() {
        if panel.isOpen { panel.update(content()) }
    }

    /// Every row leaves the panel open except Open Providers Folder and Quit.
    private func rowClicked(_ kind: PanelRow.Kind) {
        switch kind {
        case .refresh: refresh()
        case .refreshEvery: expandedSections.formSymmetricDifference([PanelModel.intervalSection])
        case .interval(let seconds):
            refreshSeconds = seconds
            poller.interval = seconds
        case .providers: expandedSections.formSymmetricDifference([PanelModel.providersSection])
        case .hiddenCards: expandedSections.formSymmetricDifference([PanelModel.hiddenSection])
        case .provider(let id): toggleProvider(id)
        case .showCard(let id): hiddenCards.remove(id)
        case .launchAtLogin: toggleLaunchAtLogin()
        case .openProvidersFolder:
            panel.close()
            openProvidersFolder()
        case .quit: quit()
        case .separator, .detail, .updated, .version, .credit: return
        }
        refreshPanel()
    }

    private func toggleProvider(_ id: String) {
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

    private func refresh() {
        poller.refreshNow()
        manualRefresh = poller.isRunning
        updateIcons()
    }

    private func toggleLaunchAtLogin() {
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

    private func openProvidersFolder() {
        try? FileManager.default.createDirectory(at: userFolder, withIntermediateDirectories: true)
        NSWorkspace.shared.open(userFolder)
    }

    private func quit() {
        NSApp.terminate(nil)
    }
}
