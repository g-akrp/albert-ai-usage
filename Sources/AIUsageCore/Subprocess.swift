import Darwin
import Foundation

/// A child process in its own process group, with piped stdout (and optionally stdin) and stderr
/// discarded. Reads use `poll` with a deadline, so a hung child never blocks past it, and
/// `terminate` kills the whole group, so a grandchild holding stdout open cannot leak.
///
/// Not thread-safe: one thread owns each instance.
public final class Subprocess {
    public let pid: pid_t
    private var stdinFD: Int32
    private var stdoutFD: Int32
    private var buffer: [UInt8] = []
    private var chunk = [UInt8](repeating: 0, count: 16 * 1024)
    private var reachedEOF = false
    private var reaped = false
    public private(set) var bytesRead = 0

    public enum LaunchError: Error { case pipe, spawn(Int32) }

    /// `executable` must be an absolute path. `environment` is the child's whole environment.
    public static func launch(executable: String, arguments: [String], environment: [String: String],
                              directory: String?, pipeStdin: Bool) throws -> Subprocess {
        var outPipe: [Int32] = [-1, -1], inPipe: [Int32] = [-1, -1]
        guard pipe(&outPipe) == 0 else { throw LaunchError.pipe }
        if pipeStdin, pipe(&inPipe) != 0 {
            close(outPipe[0]); close(outPipe[1])
            throw LaunchError.pipe
        }

        var actions: posix_spawn_file_actions_t?
        posix_spawn_file_actions_init(&actions)
        defer { posix_spawn_file_actions_destroy(&actions) }
        if pipeStdin {
            posix_spawn_file_actions_adddup2(&actions, inPipe[0], 0)
        } else {
            posix_spawn_file_actions_addopen(&actions, 0, "/dev/null", O_RDONLY, 0)
        }
        posix_spawn_file_actions_adddup2(&actions, outPipe[1], 1)
        posix_spawn_file_actions_addopen(&actions, 2, "/dev/null", O_WRONLY, 0)
        if let directory { posix_spawn_file_actions_addchdir_np(&actions, directory) }

        var attributes: posix_spawnattr_t?
        posix_spawnattr_init(&attributes)
        defer { posix_spawnattr_destroy(&attributes) }
        // Own process group; close every descriptor not set above (so concurrent spawns never
        // inherit each other's pipes); default SIGPIPE and an empty signal mask in the child.
        posix_spawnattr_setflags(&attributes, Int16(POSIX_SPAWN_SETPGROUP | POSIX_SPAWN_CLOEXEC_DEFAULT
                                                    | POSIX_SPAWN_SETSIGDEF | POSIX_SPAWN_SETSIGMASK))
        posix_spawnattr_setpgroup(&attributes, 0)
        var defaults = sigset_t(), empty = sigset_t()
        sigemptyset(&defaults); sigaddset(&defaults, SIGPIPE)
        sigemptyset(&empty)
        posix_spawnattr_setsigdefault(&attributes, &defaults)
        posix_spawnattr_setsigmask(&attributes, &empty)

        let argv = ([executable] + arguments).map { strdup($0) } + [nil]
        let envp = environment.map { strdup("\($0.key)=\($0.value)") } + [nil]
        defer { (argv + envp).forEach { free($0) } }

        var pid: pid_t = 0
        let status = posix_spawn(&pid, executable, &actions, &attributes, argv, envp)
        close(outPipe[1])
        if pipeStdin { close(inPipe[0]) }
        guard status == 0 else {
            close(outPipe[0])
            if pipeStdin { close(inPipe[1]) }
            throw LaunchError.spawn(status)
        }
        return Subprocess(pid: pid, stdin: pipeStdin ? inPipe[1] : -1, stdout: outPipe[0])
    }

    private init(pid: pid_t, stdin: Int32, stdout: Int32) {
        (self.pid, stdinFD, stdoutFD) = (pid, stdin, stdout)
    }

    deinit { terminate() }

    // MARK: Writing

    /// Writes all of `data` to the child's stdin. `false` if stdin is closed or the child is gone.
    public func write(_ data: Data) -> Bool {
        guard stdinFD >= 0 else { return false }
        return data.withUnsafeBytes { raw -> Bool in
            var offset = 0
            while offset < raw.count {
                let n = Darwin.write(stdinFD, raw.baseAddress! + offset, raw.count - offset)
                if n < 0 {
                    if errno == EINTR { continue }
                    return false
                }
                offset += n
            }
            return true
        }
    }

    public func closeStdin() {
        if stdinFD >= 0 { close(stdinFD); stdinFD = -1 }
    }

    // MARK: Reading

    public enum LineResult: Equatable { case line(Data), eof, timeout, tooLong, tooMuch }
    public enum AllResult: Equatable { case data(Data), timeout, tooMuch }

    /// The next newline-terminated line (without the newline), or what stopped it.
    public func readLine(deadline: Date, maxLineBytes: Int, maxTotalBytes: Int) -> LineResult {
        while true {
            if let newline = buffer.firstIndex(of: 10) {
                let line = Data(buffer[..<newline])
                buffer.removeSubrange(...newline)
                return line.count > maxLineBytes ? .tooLong : .line(line)
            }
            if buffer.count > maxLineBytes { return .tooLong }
            if reachedEOF {
                guard !buffer.isEmpty else { return .eof }
                defer { buffer.removeAll() }
                return .line(Data(buffer))
            }
            if bytesRead > maxTotalBytes { return .tooMuch }
            if let stop = fill(deadline: deadline) { return stop }
        }
    }

    /// Everything the child writes until it closes stdout.
    public func readAll(deadline: Date, maxBytes: Int) -> AllResult {
        while !reachedEOF {
            if bytesRead > maxBytes { return .tooMuch }
            if let stop = fill(deadline: deadline) { return stop == .timeout ? .timeout : .tooMuch }
        }
        return bytesRead > maxBytes ? .tooMuch : .data(Data(buffer))
    }

    /// Reads one chunk into `buffer`. Returns `.timeout` past the deadline, otherwise `nil`.
    private func fill(deadline: Date) -> LineResult? {
        while true {
            let remaining = deadline.timeIntervalSinceNow
            if remaining <= 0 { return .timeout }
            var descriptor = pollfd(fd: stdoutFD, events: Int16(POLLIN), revents: 0)
            let ready = poll(&descriptor, 1, Int32(min(remaining * 1000 + 1, 60_000)))
            if ready < 0 && errno == EINTR { continue }
            if ready == 0 { continue }
            let n = chunk.withUnsafeMutableBytes { read(stdoutFD, $0.baseAddress!, $0.count) }
            if n < 0 && (errno == EINTR || errno == EAGAIN) { continue }
            if n <= 0 {
                reachedEOF = true
            } else {
                buffer.append(contentsOf: chunk[..<n])
                bytesRead += n
            }
            return nil
        }
    }

    // MARK: Exit

    /// The exit status once the child exits (a signal counts as 128 + signal), or `nil` past the
    /// deadline. Does not reap, so `terminate` can still kill the rest of the group.
    public func waitForExit(deadline: Date) -> Int32? {
        while true {
            var info = siginfo_t()
            if waitid(P_PID, id_t(pid), &info, WEXITED | WNOHANG | WNOWAIT) == 0, info.si_pid == pid {
                return info.si_code == CLD_EXITED ? info.si_status : 128 + info.si_status
            }
            if Date() >= deadline { return nil }
            usleep(20_000)
        }
    }

    /// Kills the process group, reaps the child, and closes the pipes. Safe to call twice.
    public func terminate() {
        closeStdin()
        if !reaped {
            kill(-pid, SIGKILL)
            kill(pid, SIGKILL)
            var status: Int32 = 0
            while waitpid(pid, &status, 0) < 0 && errno == EINTR {}
            reaped = true
        }
        if stdoutFD >= 0 { close(stdoutFD); stdoutFD = -1 }
    }
}
