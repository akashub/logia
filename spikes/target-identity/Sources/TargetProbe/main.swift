import Foundation
import TargetProbeCore

let usage = """
Usage: target-probe --case NAME --output FILE [--capture-delay-ms N] [--recheck-delay-ms N]
NAME: lowercase letters, digits and hyphens (at most 64 bytes).
Delays: 0–60000 ms, default 3000 each. Output must be a new file.
Read-only experiment: no audio, field contents, clipboard, or insertion.
Exit codes: 0 observation saved; 1 output error; 2 permission absent; 64 invalid arguments.
"""

func diagnostic(_ message: String) {
    FileHandle.standardError.write(Data((message + "\n").utf8))
}

if Array(CommandLine.arguments.dropFirst()) == ["--help"] {
    print(usage)
    exit(0)
}
let options: ProbeOptions
do { options = try ProbeOptions.parse(Array(CommandLine.arguments.dropFirst())) }
catch { diagnostic(usage); exit(64) }

guard FocusSnapshot.hasPermission else {
    diagnostic("Accessibility access is unavailable. Enable the launching app in System Settings > Privacy & Security > Accessibility, then rerun. No prompt was requested and no observation was saved.")
    exit(2)
}
guard !FileManager.default.fileExists(atPath: options.output.path) else {
    diagnostic("Output already exists; choose a new file.")
    exit(1)
}
diagnostic("Focus the initial sample field now.")
Thread.sleep(forTimeInterval: Double(options.captureDelayMS) / 1000)
let started = DispatchTime.now().uptimeNanoseconds
let initial = Result { try FocusSnapshot.capture() }
diagnostic("Initial observation taken. Perform the case's focus change now.")
Thread.sleep(forTimeInterval: Double(options.recheckDelayMS) / 1000)
let fresh = Result { try FocusSnapshot.capture() }
let verdict: TargetVerdict
let reason: String
switch (initial, fresh) {
case (.success(let before), .success(let after)):
    verdict = before.verdict(comparedTo: after)
    reason = verdict == .same ? "identity-matched" : verdict == .changed ? "identity-changed" : "retained-identity-unavailable"
case (.failure(let error), _):
    verdict = .unknown
    reason = "initial:\(error)"
case (_, .failure(let error)):
    verdict = .unknown
    reason = "recheck:\(error)"
}
let elapsed = Int((DispatchTime.now().uptimeNanoseconds - started) / 1_000_000)
do {
    try ProbeRecord(caseName: options.caseName, verdict: verdict, reason: reason, elapsedMS: elapsed)
        .writeNew(to: options.output)
    print("\(verdict.rawValue): \(reason)")
} catch {
    diagnostic("Could not create the evidence file. Existing files are never replaced.")
    exit(1)
}
