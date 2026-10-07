import AIUsageCore
import Foundation

var failures: [String] = []

func check(_ name: String, _ condition: Bool) {
    if condition {
        print("PASS \(name)")
    } else {
        print("FAIL \(name)")
        failures.append(name)
    }
}

func json(_ text: String) -> Any {
    JSON.parse(Data(text.utf8))!
}

func config(_ text: String) throws -> ProviderConfig {
    try ProviderConfig.parse(Data(text.utf8))
}

func bundledConfig(_ id: String) -> ProviderConfig? {
    let url = URL(fileURLWithPath: "Resources/providers/\(id).json")
    return (try? Data(contentsOf: url)).flatMap { try? ProviderConfig.parse($0) }
}

/// Runs the bundled providers for real and prints what the menu would show.
func liveRun() {
    let environment = ChildEnvironment(
        process: ProcessInfo.processInfo.environment,
        loginShell: ChildEnvironment.readLoginShell(shell: ProcessInfo.processInfo.environment["SHELL"] ?? "/bin/zsh"),
        home: NSHomeDirectory())
    let loaded = ProviderStore.load(folders: [URL(fileURLWithPath: "Resources/providers")])
    loaded.errors.forEach { print("CONFIG ERROR \($0)") }
    let runner = ProviderRunner(environment: environment)
    for config in loaded.configs {
        let start = Date()
        let runs = runner.runAll(config)
        print(String(format: "== %@ (%.1fs)", config.id, Date().timeIntervalSince(start)))
        for run in runs {
        for entry in MenuModel.entries(runs: [run], pinned: nil, configErrors: [], updated: nil, launchAtLogin: false,
                                       version: "", formatReset: { MenuModel.resetText($0, now: Date()) }) {
            if case .card(let card) = entry {
                print("\(card.title)  \(card.message.map { $0.text } ?? "")")
                card.notices.forEach { print("  \($0.text)") }
                for chart in card.charts {
                    print("  [\(chart.title ?? "-")] center \(chart.center), rings \(chart.rings.count)")
                    chart.rows.forEach { print("    \($0.label): \($0.percentText) \($0.reset ?? "")") }
                }
                card.overflow.forEach { print("  + \($0.label): \($0.percentText)") }
            }
        }
        let icon = StatusIcons.icon(runs: [run], pinned: nil)
        print("icon: \(icon.top)/\(icon.bottom)")
        }
    }
}
