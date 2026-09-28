import AppKit
import ApplicationServices
import Darwin

// A parent deadline can close stdout between keydown and its acknowledgment.
// Keep running to stdin EOF so defer still sends the matching keyups.
signal(SIGPIPE, SIG_IGN)

// Only Control/Option/Space are posted to the foreground owned blank window.
// Every path after keydown balances keyup; this helper never sends Enter.
struct CheckFailure: Error { let message: String }
func require(_ condition: Bool, _ message: String) throws {
    if !condition { throw CheckFailure(message: message) }
}
func run() throws {
    let args = CommandLine.arguments
    try require(args.count == 3, "Expected owned smoke PID and preset")
    guard let pid = Int32(args[1]), pid > 0 else { throw CheckFailure(message: "Invalid owned smoke PID") }
    try require(args[2] == "Control+Alt+Space", "Unsupported smoke preset")
    try require(AXIsProcessTrusted(), "Synthetic shortcut checks need Accessibility permission")
    func checkInstalled() throws {
        let installed = NSWorkspace.shared.runningApplications.contains {
            $0.processIdentifier != pid && ($0.bundleIdentifier == "com.akashub.logia" || $0.bundleURL?.lastPathComponent == "Logia.app")
        }
        try require(!installed, "Quit installed Logia before running shortcut smoke")
    }
    try checkInstalled()
    guard let owned = NSRunningApplication(processIdentifier: pid) else { throw CheckFailure(message: "Owned smoke process missing") }
    try require(owned.executableURL?.lastPathComponent == "shortcut_smoke", "PID does not belong to the shortcut smoke executable")
    owned.activate(options: [])
    for _ in 0..<40 {
        if NSWorkspace.shared.frontmostApplication?.processIdentifier == pid { break }
        usleep(50_000)
    }
    func checkForeground() throws {
        try checkInstalled()
        try require(NSWorkspace.shared.frontmostApplication?.processIdentifier == pid, "Owned smoke window lost foreground; no further keydowns")
    }
    try checkForeground()
    guard let source = CGEventSource(stateID: .hidSystemState) else { throw CheckFailure(message: "Could not create event source") }
    var held: [CGKeyCode] = []
    var flags: CGEventFlags = []
    func upAll() {
        while let key = held.popLast() {
            if key == 58 { flags.remove(.maskAlternate) }
            if key == 59 { flags.remove(.maskControl) }
            let event = CGEvent(keyboardEventSource: source, virtualKey: key, keyDown: false)
            event?.flags = flags
            event?.post(tap: .cghidEventTap)
        }
    }
    defer { upAll() }
    func down(_ key: CGKeyCode, _ nextFlags: CGEventFlags, repeated: Bool = false) throws {
        try checkForeground()
        guard let event = CGEvent(keyboardEventSource: source, virtualKey: key, keyDown: true) else {
            throw CheckFailure(message: "Could not create key event")
        }
        flags = nextFlags
        event.flags = flags
        event.setIntegerValueField(.keyboardEventAutorepeat, value: repeated ? 1 : 0)
        if !repeated { held.append(key) }
        event.post(tap: .cghidEventTap)
        usleep(30_000)
    }
    print("ready"); fflush(stdout)
    while let command = readLine() {
        switch command {
        case "press":
            try require(held.isEmpty, "Keys already pressed")
            for key in [CGKeyCode(59), 58, 49] {
                try require(!CGEventSource.keyState(.combinedSessionState, key: key), "Release physical shortcut keys before this smoke")
            }
            try down(59, .maskControl)
            try down(58, [.maskControl, .maskAlternate])
            try down(49, [.maskControl, .maskAlternate])
        case "repeat":
            try require(held.last == 49, "No held shortcut to repeat")
            for _ in 0..<6 { try down(49, [.maskControl, .maskAlternate], repeated: true) }
        case "release":
            try checkForeground()
            upAll()
        case "quit": return
        default: throw CheckFailure(message: "Unknown driver command")
        }
        print(command); fflush(stdout)
    }
}
do { try run() }
catch let failure as CheckFailure { fputs("FAIL: \(failure.message)\n", stderr); exit(1) }
catch { fputs("FAIL: Shortcut driver failed\n", stderr); exit(1) }
