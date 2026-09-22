import AppKit
import ApplicationServices

@main struct GlobalPasteCheck { static func main() { runGlobalPasteCheck() } }

private func runGlobalPasteCheck() {
        guard CommandLine.arguments.count == 2, AXIsProcessTrusted() else { print("FAIL: native test requires Accessibility"); exit(2) }
        let app = NSApplication.shared
        app.setActivationPolicy(.prohibited)
        let fixture = Process()
        fixture.executableURL = URL(fileURLWithPath: CommandLine.arguments[1])
        let input = Pipe(), output = Pipe()
        fixture.standardInput = input; fixture.standardOutput = output
        try! fixture.run()
        let clipboard = NSPasteboard.general
        let saved = (clipboard.pasteboardItems ?? []).map { item -> NSPasteboardItem in
            let copy = NSPasteboardItem()
            for type in item.types { if let data = item.data(forType: type) { copy.setData(data, forType: type) } }
            return copy
        }
        var owned: Int?
        func onMain<T>(_ body: () -> T) -> T { DispatchQueue.main.sync(execute: body) }
        func command(_ text: String, writer: FileHandle? = nil, reader: FileHandle? = nil) -> Bool {
            (writer ?? input.fileHandleForWriting).write(Data((text + "\n").utf8))
            var line = Data()
            while let byte = try? (reader ?? output.fileHandleForReading).read(upToCount: 1), !byte.isEmpty {
                if byte == Data([10]) { break }; line.append(byte)
            }
            return line == Data("ok".utf8)
        }
        func arm() -> UInt64 { onMain { var status: Int32 = 1; return targetCapture(&status) } }
        func send(_ token: UInt64, _ text: String = "Logia café.") -> Int32 {
            onMain {
                let bytes = Array(text.utf8)
                let result = bytes.withUnsafeBufferPointer { targetSend(token, $0.baseAddress, $0.count) }
                if result == 5 || result == 7 { owned = clipboard.changeCount }
                return result
            }
        }
        func require(_ ok: Bool, _ message: String) throws {
            if !ok { throw NSError(domain: message, code: 1) }
        }
        // Hard timeout covers a fixture that stops answering its pipe.
        DispatchQueue.main.asyncAfter(deadline: .now() + 40) { fixture.terminate(); print("FAIL: native test deadline"); exit(1) }
        DispatchQueue.global().async {
            var failure: String?
            do {
                try require(command("reset"), "reset")
                var token = arm()
                try require(command("second"), "move to second field")
                try require(send(token) == 5, "paste must follow CURRENT field, not recording-start field")
                Thread.sleep(forTimeInterval: 0.15)
                try require(command("assert-second"), "exact current-field text")
                print("PASS: moving the cursor before delivery pastes into the new field")
                do {
                    try require(command("reset"), "reset")
                    token = arm()
                    let other = Process(), otherInput = Pipe(), otherOutput = Pipe()
                    other.executableURL = fixture.executableURL
                    other.standardInput = otherInput; other.standardOutput = otherOutput
                    try other.run()
                    defer { if other.isRunning { other.terminate() } }
                    try require(command("reset", writer: otherInput.fileHandleForWriting,
                                        reader: otherOutput.fileHandleForReading), "focus second owned process")
                    try require(send(token) == 5, "paste follows current app")
                    Thread.sleep(forTimeInterval: 0.15)
                    try require(command("assert-first", writer: otherInput.fileHandleForWriting,
                                        reader: otherOutput.fileHandleForReading), "exact current-app paste")
                    try require(command("assert-untouched"), "old app not edited")
                    print("PASS: switching apps before delivery follows the current cursor")
                }
                for (move, check) in [("caret", "assert-caret"), ("window", "assert-window"), ("custom", "assert-custom"), ("hidden", "assert-custom"), ("no-role", "assert-custom")] {
                    try require(command("reset"), "reset")
                    token = arm(); try require(command(move), "focus fixture")
                    try require(send(token) == 5, "dispatch to current \(move)")
                    Thread.sleep(forTimeInterval: 0.15)
                    try require(command(check), "exact received text: \(move)")
                    try require(send(token) == 3, "duplicate consumed")
                    print("PASS: \(move) receives exact Unicode paste once")
                }
                for focus in ["secure", "button", "disabled", "nowhere"] {
                    try require(command("reset"), "reset"); token = arm()
                    try require(command(focus), "focus fixture")
                    let result = send(token)
                    // A window-only AX ancestor can't distinguish an empty
                    // window from a hidden editor. Paste may be posted but the
                    // actual result must still be clipboard-only here.
                    try require(result == 7 || focus == "nowhere" && result == 5, "\(focus) fallback")
                    Thread.sleep(forTimeInterval: 0.15) // Let an ignored paste finish before changing the fixture's focus.
                    try require(onMain { clipboard.string(forType: .string) == "Logia café." }, "fallback copied")
                    try require(command("assert-untouched"), "no text inserted")
                    print("PASS: \(focus) keeps exact text copied")
                }
                // Moving during speech is allowed; moving AFTER the final
                // delivery snapshot must not redirect an in-flight paste.
                for change in ["second", "caret", "window", "secure"] {
                    try require(command("reset"), "reset")
                    let snapshot = onMain { try? PasteDestination.capture() }
                    try require(snapshot != nil, "final snapshot available")
                    let version = onMain { clipboard.changeCount }
                    try require(command(change), "move during final check")
                    let result = onMain {
                        let result = VerifiedPaste.send("Logia café.", to: snapshot!.pid,
                            clipboardVersion: version, verify: snapshot!.isCurrent)
                        if result == .copied { owned = clipboard.changeCount }
                        return result
                    }
                    try require(result == .copied, "focus change after snapshot must copy")
                    try require(command("assert-untouched"), "no paste into changed destination: \(change)")
                }
                print("PASS: changes during final verification block paste")
                try require(command("reset"), "reset"); token = arm()
                let ownWindow = onMain {
                    app.setActivationPolicy(.regular)
                    let window = NSWindow(contentRect: NSRect(x: 200, y: 280, width: 300, height: 100),
                                          styleMask: [.titled], backing: .buffered, defer: false)
                    window.isReleasedWhenClosed = false; window.makeKeyAndOrderFront(nil)
                    app.activate(ignoringOtherApps: true); return window
                }
                Thread.sleep(forTimeInterval: 0.2)
                try require(onMain { NSWorkspace.shared.frontmostApplication?.processIdentifier == ProcessInfo.processInfo.processIdentifier }, "owned helper focused")
                try require(send(token) == 7, "own app must only copy")
                onMain { ownWindow.orderOut(nil); app.setActivationPolicy(.prohibited) }
                print("PASS: Logia's own process is never an insertion destination")
                try require(command("reset"), "reset"); token = arm()
                try require(send(token, "line one\nline two") == 7, "control payload copied, not pasted")
                try require(onMain { clipboard.string(forType: .string) == "line one\nline two" }, "multiline preserved")
                try require(command("assert-untouched"), "no newline injection")
                token = arm(); onMain { targetCancel(token) }
                try require(send(token) == 3, "canceled attempt")
                let stale = arm(); token = arm(); onMain { targetCancel(stale) }
                try require(send(stale) == 3, "stale token rejected")
                try require(send(token) == 5, "stale cancel preserves new token")
                Thread.sleep(forTimeInterval: 0.15)
                try require(command("assert-first"), "new token exact text")
                try require(command("reset"), "reset"); token = arm()
                onMain { clipboard.clearContents(); clipboard.setString("Newer synthetic clipboard", forType: .string); owned = clipboard.changeCount }
                try require(send(token) == 6, "new clipboard preserved")
                try require(onMain { clipboard.string(forType: .string) == "Newer synthetic clipboard" }, "clipboard not overwritten")
                try require(command("assert-untouched"), "newer clipboard suppresses paste")
                print("PASS: newline, cancel, stale token, duplicate and clipboard protections")
            } catch { failure = (error as NSError).domain }
            onMain {
                if clipboard.changeCount == owned { clipboard.clearContents(); if !saved.isEmpty { clipboard.writeObjects(saved) } }
                fixture.terminate()
                print(failure.map { "FAIL: \($0)" } ?? "PASS: current-cursor production bridge checks complete")
                exit(failure == nil ? 0 : 1)
            }
        }
        app.run()
}
