import AIUsageCore
import Foundation

signal(SIGPIPE, SIG_IGN)

if CommandLine.arguments.contains("--live") {
    liveRun()
    exit(0)
}

jsonChecks()
configChecks()
mappingChecks()
iconChecks()
menuChecks()
environmentChecks()
runnerChecks()
accountChecks()
pinChecks()
toggleChecks()

if failures.isEmpty {
    print("ALL PASS")
} else {
    print("\(failures.count) FAILED: \(failures.joined(separator: ", "))")
    exit(1)
}
