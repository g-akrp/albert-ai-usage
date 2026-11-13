import AIUsageCore
import Foundation

/// Runs every provider on `interval`, at most two at a time, and keeps the latest results.
/// Main thread only; provider runs happen on a background queue.
final class Poller {
    private(set) var runs: [ProviderRun] = []
    /// Provider file ids that are switched off: they do not run.
    var disabled: Set<String> = [] {
        didSet { tick() }
    }
    /// Every provider file, in menu order.
    var providerNames: [(id: String, name: String)] {
        order.compactMap { id in configs[id].map { (id, $0.name) } }
    }
    /// Seconds between runs of a provider, counted from when a run starts: one of `Schedule.intervals`.
    /// It replaces each provider file's `refresh.intervalSeconds`.
    var interval = Schedule.standard {
        didSet {
            guard interval != oldValue else { return }
            startTimer()
            tick()
        }
    }
    private(set) var configErrors: [String] = []
    private(set) var lastUpdate: Date?
    /// Whether any provider is running now.
    var isRunning: Bool { states.values.contains { $0.running } }
    var onChange: (() -> Void)?

    private struct State {
        var started = Date.distantPast
        var failures = 0
        var running = false
    }

    private let folders: [URL]
    private var order: [String] = []
    private var configs: [String: ProviderConfig] = [:]
    private var states: [String: State] = [:]
    private var runner: ProviderRunner?
    /// Main thread only: a login-shell read is already in flight.
    private var reloadingEnvironment = false
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
        reloadEnvironment()
        startTimer()
    }

    /// The one schedule timer; it fires faster only when the interval is short (see `Schedule.tick`).
    private func startTimer() {
        timer?.invalidate()
        let period = Schedule.tick(interval: interval)
        let timer = Timer(timeInterval: period, repeats: true) { [weak self] _ in self?.tick() }
        timer.tolerance = period / 10
        RunLoop.main.add(timer, forMode: .common)
        self.timer = timer
    }

    /// Rereads the provider files and the login shell, then runs every provider that is not already running.
    func refreshNow() {
        reloadConfigs()
        for id in states.keys { states[id]?.started = .distantPast }
        reloadEnvironment()
    }

    /// Reads the login shell off the main thread, then swaps in a runner with that environment and ticks.
    private func reloadEnvironment() {
        guard !reloadingEnvironment else { return }
        reloadingEnvironment = true
        // Reading the login shell takes a moment, so it happens off the main thread.
        DispatchQueue.global(qos: .utility).async {
            let environment = ChildEnvironment(
                process: ProcessInfo.processInfo.environment,
                loginShell: ChildEnvironment.readLoginShell(shell: Self.loginShell()),
                home: NSHomeDirectory())
            DispatchQueue.main.async {
                self.reloadingEnvironment = false
                self.runner = ProviderRunner(environment: environment)
                self.tick()
            }
        }
    }

    /// Starts every provider that is due.
    func tick() {
        guard let runner else { return }
        let now = Date()
        var started = false
        for id in order where !disabled.contains(id) {
            guard let state = states[id], !state.running, let config = configs[id],
                  Schedule.isDue(started: state.started, interval: interval, failures: state.failures, now: now) else { continue }
            states[id]?.running = true
            states[id]?.started = now
            started = true
            queue.addOperation { [weak self] in
                let results = runner.runAll(config)
                DispatchQueue.main.async { self?.finish(config, results) }
            }
        }
        if started { onChange?() }
    }

    /// Replaces the provider's runs (one per account) in place.
    private func finish(_ config: ProviderConfig, _ results: [ProviderRun]) {
        guard var state = states[config.id] else { return }  // removed while it ran
        state.running = false
        let failed = results.allSatisfy { if case .failure = $0.result { return true } else { return false } }
        state.failures = failed ? state.failures + 1 : 0
        states[config.id] = state
        let index = runs.firstIndex { $0.configId == config.id } ?? runs.endIndex
        runs.removeAll { $0.configId == config.id }
        runs.insert(contentsOf: results, at: min(index, runs.endIndex))
        lastUpdate = Date()
        onChange?()
    }

    private func reloadConfigs() {
        let loaded = ProviderStore.load(folders: folders)
        configErrors = loaded.errors
        configs = Dictionary(uniqueKeysWithValues: loaded.configs.map { ($0.id, $0) })
        order = CardOrder.defaultSorted(loaded.configs.map(\.id))
        runs = order.compactMap { configs[$0] }.flatMap { config in
            let previous = runs.filter { $0.configId == config.id }
            return previous.isEmpty ? [ProviderRun(config: config)] : previous
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
