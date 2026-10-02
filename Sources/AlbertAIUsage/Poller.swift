import AlbertUsageCore
import Foundation

/// Runs each provider on its own interval, at most two at a time, and keeps the latest results.
/// Main thread only; provider runs happen on a background queue.
final class Poller {
    private(set) var runs: [ProviderRun] = []
    private(set) var configErrors: [String] = []
    private(set) var lastUpdate: Date?
    var onChange: (() -> Void)?

    private struct State {
        var next = Date.distantPast
        var failures = 0
        var running = false
    }

    private let folders: [URL]
    private var configs: [String: ProviderConfig] = [:]
    private var states: [String: State] = [:]
    private var runner: ProviderRunner?
    private var timer: Timer?
    private let queue: OperationQueue = {
        let queue = OperationQueue()
        queue.maxConcurrentOperationCount = 2
        queue.qualityOfService = .utility
        return queue
    }()

    init(folders: [URL]) {
        self.folders = folders
    }

    func start() {
        reloadConfigs()
        // Reading the login shell takes a moment, so it happens once, off the main thread.
        DispatchQueue.global(qos: .utility).async {
            let environment = ChildEnvironment(
                process: ProcessInfo.processInfo.environment,
                loginShell: ChildEnvironment.readLoginShell(shell: Self.loginShell()),
                home: NSHomeDirectory())
            DispatchQueue.main.async {
                self.runner = ProviderRunner(environment: environment)
                self.tick()
            }
        }
        let timer = Timer(timeInterval: 30, repeats: true) { [weak self] _ in self?.tick() }
        timer.tolerance = 10
        RunLoop.main.add(timer, forMode: .common)
        self.timer = timer
    }

    /// Rereads the provider files and runs every provider that is not already running.
    func refreshNow() {
        reloadConfigs()
        for id in states.keys { states[id]?.next = .distantPast }
        tick()
    }

    /// Starts every provider that is due.
    func tick() {
        guard let runner else { return }
        let now = Date()
        for run in runs {
            guard let state = states[run.id], !state.running, state.next <= now,
                  let config = configs[run.id] else { continue }
            states[run.id]?.running = true
            queue.addOperation { [weak self] in
                let result = runner.run(config)
                DispatchQueue.main.async { self?.finish(config, result) }
            }
        }
    }

    private func finish(_ config: ProviderConfig, _ result: Result<Report, ProviderFailure>) {
        guard var state = states[config.id] else { return }  // removed while it ran
        state.running = false
        if case .failure = result { state.failures += 1 } else { state.failures = 0 }
        state.next = Date() + Schedule.delay(interval: config.refreshSeconds, failures: state.failures)
        states[config.id] = state
        if let index = runs.firstIndex(where: { $0.id == config.id }) { runs[index].result = result }
        lastUpdate = Date()
        onChange?()
    }

    private func reloadConfigs() {
        let loaded = ProviderStore.load(folders: folders)
        configErrors = loaded.errors
        configs = Dictionary(uniqueKeysWithValues: loaded.configs.map { ($0.id, $0) })
        runs = loaded.configs.map { config in
            var run = ProviderRun(config: config)
            run.result = runs.first { $0.id == config.id }?.result
            return run
        }
        states = configs.keys.reduce(into: [:]) { $0[$1] = states[$1] ?? State() }
        onChange?()
    }

    private static func loginShell() -> String {
        if let entry = getpwuid(getuid()), let shell = entry.pointee.pw_shell {
            let path = String(cString: shell)
            if FileManager.default.isExecutableFile(atPath: path) { return path }
        }
        return "/bin/zsh"
    }
}
