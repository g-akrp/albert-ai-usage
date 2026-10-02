import Foundation

/// Runs a provider's source and maps its answer. Blocking: call it off the main thread.
public struct ProviderRunner {
    public let environment: ChildEnvironment

    public init(environment: ChildEnvironment) { self.environment = environment }

    public func run(_ config: ProviderConfig) -> Result<Report, ProviderFailure> {
        Result { Mapper.apply(config.map, to: try capture(config.source)) }
            .mapError { $0 as? ProviderFailure ?? ProviderFailure(String(describing: $0)) }
    }

    /// The JSON document the map reads.
    public func capture(_ source: Source) throws -> Any {
        let name = source.executable
        guard let executable = environment.resolveExecutable(name) else {
            throw ProviderFailure("\(name) not found on PATH")
        }
        // Each run starts in an empty temporary folder.
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("albert-ai-usage-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        switch source {
        case .command(let command):
            return try runCommand(command, executable: executable, directory: directory.path)
        case .stdio(let stdio):
            return try runStdio(stdio, executable: executable, directory: directory.path)
        }
    }

    private func launch(_ executable: String, _ name: String, _ args: [String], _ env: [String: String],
                        _ directory: String, pipeStdin: Bool) throws -> Subprocess {
        do {
            return try Subprocess.launch(executable: executable, arguments: args,
                                         environment: environment.variables(overrides: env),
                                         directory: directory, pipeStdin: pipeStdin)
        } catch {
            throw ProviderFailure("could not start \(name)")
        }
    }

    private func runCommand(_ source: CommandSource, executable: String, directory: String) throws -> Any {
        let name = source.executable
        let process = try launch(executable, name, source.args, source.env, directory, pipeStdin: false)
        defer { process.terminate() }
        let deadline = Date() + TimeInterval(source.timeoutSeconds)

        let output: Data
        switch process.readAll(deadline: deadline, maxBytes: source.maxOutputBytes) {
        case .data(let data): output = data
        case .timeout: throw ProviderFailure("\(name) timed out")
        case .tooMuch: throw ProviderFailure("\(name) printed more than \(source.maxOutputBytes) bytes")
        }
        guard let status = process.waitForExit(deadline: deadline) else { throw ProviderFailure("\(name) timed out") }
        guard status == 0 else { throw ProviderFailure("\(name) exited with status \(status)") }
        guard let value = JSON.parse(output) else { throw ProviderFailure("\(name) output was not valid JSON") }

        if let expect = source.expect {
            guard Self.matches(value, expect.match) else {
                throw ProviderFailure("\(name) output did not match the expected shape")
            }
            try Self.check(value, expect)
        }
        return value
    }

    private func runStdio(_ source: StdioSource, executable: String, directory: String) throws -> Any {
        let name = source.executable
        let process = try launch(executable, name, source.args, source.env, directory, pipeStdin: true)
        defer { process.terminate() }
        let deadline = Date() + TimeInterval(source.timeoutSeconds)
        var captures: [String: Any] = [:]

        for step in source.steps {
            switch step {
            case .write(let line):
                guard process.write(line + Data([10])) else { throw ProviderFailure("could not write to \(name)") }
            case .await(let spec):
                let answer = try awaitAnswer(process, source, spec, deadline)
                try Self.check(answer, spec)
                if let capture = spec.capture { captures[capture] = answer }
            }
        }
        process.closeStdin()
        guard let output = captures[source.output] else {
            throw ProviderFailure("no captured message named \(source.output)")
        }
        return output
    }

    /// The first line that is JSON and matches every predicate. Other lines are skipped.
    private func awaitAnswer(_ process: Subprocess, _ source: StdioSource, _ spec: Expect,
                             _ deadline: Date) throws -> Any {
        let name = source.executable
        while true {
            switch process.readLine(deadline: deadline, maxLineBytes: source.maxLineBytes,
                                    maxTotalBytes: source.maxTotalBytes) {
            case .line(let line):
                if let value = JSON.parse(line), Self.matches(value, spec.match) { return value }
            case .eof: throw ProviderFailure("\(name) closed its output before answering")
            case .timeout: throw ProviderFailure("\(name) timed out")
            case .tooLong: throw ProviderFailure("\(name) printed a line longer than \(source.maxLineBytes) bytes")
            case .tooMuch: throw ProviderFailure("\(name) printed more than \(source.maxTotalBytes) bytes")
            }
        }
    }

    // MARK: Checks on an answer

    static func matches(_ value: Any, _ predicates: [Predicate]) -> Bool {
        Predicate.allHold(predicates, in: value)
    }

    /// `error`: present and not null fails the run with its text. `require`: must be present and not null.
    static func check(_ value: Any, _ spec: Expect) throws {
        if let pointer = spec.error, let error = JSON.resolve(value, pointer), !JSON.isNull(error) {
            let object = error as? [String: Any]
            if JSON.number(object?["code"]) == -32601 { throw ProviderFailure("Update the CLI to see usage") }
            if let message = object?["message"] as? String { throw ProviderFailure(message) }
            if let text = error as? String { throw ProviderFailure(text) }
            throw ProviderFailure("the provider reported an error")
        }
        if let pointer = spec.require, JSON.isNull(JSON.resolve(value, pointer)) {
            throw ProviderFailure("the answer has no \(pointer)")
        }
    }
}
