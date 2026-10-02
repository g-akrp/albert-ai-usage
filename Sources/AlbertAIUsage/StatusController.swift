import AlbertUsageCore
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

    private var icons: [IconSpec] = []
    private var images: [NSImage] = []
    private var cycleIndex = 0
    private var cycleTimer: Timer?

    /// The provider shown in the menu bar; `nil` cycles through all of them.
    private var disabled: Set<String> {
        get { Set(defaults.stringArray(forKey: "disabledProviders") ?? []) }
        set { defaults.set(newValue.sorted(), forKey: "disabledProviders") }
    }

    private var visibleRuns: [ProviderRun] {
        ProviderToggle.visible(poller.runs, disabled: poller.disabled)
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

    /// The SwiftBar plugin stored its pin in a file.
    private func migrateOldPin() {
        guard defaults.object(forKey: "pinnedProvider") == nil else { return }
        let file = URL(fileURLWithPath: NSHomeDirectory()).appendingPathComponent(".config/albert-ai-usage/pinned")
        if let id = try? String(contentsOf: file, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines),
           !id.isEmpty {
            pinned = id
        }
    }

    // MARK: Menu bar icon

    private func updateIcons() {
        let newIcons = StatusIcons.icons(runs: visibleRuns, pinned: pinned)
        if newIcons != icons {
            icons = newIcons
            images = newIcons.map(Self.image)
            cycleIndex = 0
        }
        showIcon()
        if icons.count > 1, cycleTimer == nil {
            let timer = Timer(timeInterval: 5, repeats: true) { [weak self] _ in
                guard let self else { return }
                cycleIndex = (cycleIndex + 1) % icons.count
                showIcon()
            }
            timer.tolerance = 0.5
            RunLoop.main.add(timer, forMode: .common)
            cycleTimer = timer
        } else if icons.count <= 1 {
            cycleTimer?.invalidate()
            cycleTimer = nil
        }
    }

    private func showIcon() {
        guard let button = statusItem.button, !icons.isEmpty else { return }
        let index = cycleIndex % icons.count
        button.image = images[index]
        button.toolTip = icons[index].accessibility
        button.setAccessibilityLabel(icons[index].accessibility)
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
            version: AppVersion.current, formatReset: resetFormatter.string(from:))
        menu.removeAllItems()
        entries.forEach { menu.addItem(item(for: $0)) }
    }

    private func item(for entry: MenuEntry) -> NSMenuItem {
        switch entry {
        case .separator:
            return .separator()
        case .provider(let id, let title, _):
            let item = action(title, #selector(pin(_:)))
            item.representedObject = id
            return item
        case .meter(let pin, let title, let tone, _):
            let item = action(title, #selector(pin(_:)))
            item.representedObject = pin
            if let color = Self.color(tone) {
                item.attributedTitle = NSAttributedString(
                    string: title, attributes: [.foregroundColor: color, .font: NSFont.menuFont(ofSize: 0)])
            }
            return item
        case .detail(let text, let tone):
            let item = NSMenuItem(title: text, action: nil, keyEquivalent: "")
            if let color = Self.color(tone) {
                item.attributedTitle = NSAttributedString(
                    string: text, attributes: [.foregroundColor: color, .font: NSFont.menuFont(ofSize: 0)])
            }
            return item
        case .cycleAll(let checked):
            let item = action("Cycle All Providers", #selector(cycleAll))
            item.state = checked ? .on : .off
            return item
        case .refresh:
            return action("Refresh Now", #selector(refresh), key: "r")
        case .updated(let text), .version(let text):
            let item = NSMenuItem(title: text, action: nil, keyEquivalent: "")
            item.isEnabled = false
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
        case .launchAtLogin(let enabled):
            let item = action("Launch at Login", #selector(toggleLaunchAtLogin))
            item.state = enabled ? .on : .off
            return item
        case .openProvidersFolder:
            return action("Open Providers Folder…", #selector(openProvidersFolder))
        case .quit:
            return action("Quit Albert AI Usage", #selector(quit), key: "q")
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

    @objc private func pin(_ sender: NSMenuItem) {
        pinned = sender.representedObject as? String
        updateIcons()
    }

    @objc private func toggleProvider(_ sender: NSMenuItem) {
        guard let id = sender.representedObject as? String else { return }
        var set = disabled
        if set.contains(id) { set.remove(id) } else { set.insert(id) }
        disabled = set
        poller.disabled = set
        updateIcons()
    }

    @objc private func cycleAll() {
        pinned = nil
        updateIcons()
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
