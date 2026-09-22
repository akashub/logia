import AppKit
import ApplicationServices

@main struct TerminalDeliveryCheck {
    static func main() {
        guard CommandLine.arguments.count == 2, AXIsProcessTrusted() else { exit(2) }
        let folder = URL(fileURLWithPath: CommandLine.arguments[1])
        let app = NSApplication.shared
        app.setActivationPolicy(.prohibited)
        var window: AXUIElement?, newTab: AXUIElement?, snapshot: FocusSnapshot?
        var previousWindows: [AXUIElement] = []
        var token: UInt64 = 0, pid: pid_t = 0, phase = 0, ticks = 0, finished = false
        let clipboard = NSPasteboard.general
        let saved = (clipboard.pasteboardItems ?? []).map { item -> NSPasteboardItem in
            let copy = NSPasteboardItem()
            for type in item.types { if let data = item.data(forType: type) { copy.setData(data, forType: type) } }
            return copy
        }
        var ownedClipboard: Int?
        func element(_ parent: AXUIElement, _ name: String) -> AXUIElement? {
            var value: CFTypeRef?
            guard AXUIElementCopyAttributeValue(parent, name as CFString, &value) == .success,
                  let value, CFGetTypeID(value) == AXUIElementGetTypeID() else { return nil }
            return unsafeDowncast(value, to: AXUIElement.self)
        }
        func send(_ text: String) -> Int32 {
            let bytes = Array(text.utf8)
            let result = bytes.withUnsafeBufferPointer { targetSend(token, $0.baseAddress, $0.count) }
            if result == 5 || result == 7 { ownedClipboard = clipboard.changeCount }
            return result
        }
        func key(_ code: CGKeyCode) {
            let source = CGEventSource(stateID: .privateState)
            for down in [true, false] {
                let event = CGEvent(keyboardEventSource: source, virtualKey: code, keyDown: down)
                event?.flags = .maskCommand; event?.postToPid(pid)
            }
        }
        func finish(_ code: Int32, _ message: String) {
            guard !finished else { return }; finished = true
            print(message)
            FileManager.default.createFile(atPath: folder.appendingPathComponent("stop").path, contents: Data())
            if clipboard.changeCount == ownedClipboard {
                clipboard.clearContents(); if !saved.isEmpty { clipboard.writeObjects(saved) }
            }
            // Command-W closes a selected tab. The red AXCloseButton can close
            // a whole tab group, so never use it to clean up this fixture.
            func closeSelected(_ owned: AXUIElement?) {
                guard let owned, NSWorkspace.shared.frontmostApplication?.processIdentifier == pid,
                      let front = element(AXUIElementCreateApplication(pid), kAXFocusedWindowAttribute),
                      CFEqual(owned, front) else { return }
                key(13)
            }
            var waits = 0
            Timer.scheduledTimer(withTimeInterval: 0.1, repeats: true) { timer in
                MainActor.assumeIsolated {
                waits += 1
                guard FileManager.default.fileExists(atPath: folder.appendingPathComponent("exited").path) || waits >= 30 else { return }
                timer.invalidate()
                closeSelected(newTab)
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) {
                    closeSelected(window)
                    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) { exit(code) }
                }
                }
            }
        }
        NSWorkspace.shared.open(folder.appendingPathComponent("Logia-check.command"))
        Timer.scheduledTimer(withTimeInterval: 0.15, repeats: true) { timer in
            MainActor.assumeIsolated {
            if finished { timer.invalidate(); return }
            ticks += 1
            if ticks > 180 { finish(1, "FAIL: terminal fixture deadline"); return }
            guard let running = NSWorkspace.shared.frontmostApplication, running.bundleIdentifier == "com.apple.Terminal" else { return }
            let candidate = AXUIElementCreateApplication(running.processIdentifier)
            AXUIElementSetMessagingTimeout(candidate, 0.25)
            guard let focusedWindow = element(candidate, kAXFocusedWindowAttribute) else { return }
            if phase == 0 {
                guard FileManager.default.fileExists(atPath: folder.appendingPathComponent("ready").path) else { return }
                // Title is used only to prove ownership of this generated
                // fixture. It is never logged or used by production capture.
                var title: CFTypeRef?
                AXUIElementCopyAttributeValue(focusedWindow, kAXTitleAttribute as CFString, &title)
                guard (title as? String)?.contains("Logia target \(folder.lastPathComponent)") == true else { return }
                window = focusedWindow; pid = running.processIdentifier
                var status: Int32 = 1; token = targetCapture(&status)
                guard token != 0 else { finish(1, "FAIL: Terminal field did not arm"); return }
                guard send("Logia terminal café.") == 5 else { finish(1, "FAIL: Terminal paste not dispatched"); return }
                phase = 1; return
            }
            guard let window, running.processIdentifier == pid, phase == 2 || CFEqual(window, focusedWindow) else {
                finish(1, "FAIL: fixture window lost focus"); return
            }
            if phase == 1 {
                guard let data = try? Data(contentsOf: folder.appendingPathComponent("result.json")),
                      let result = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return }
                guard result["exact"] as? Bool == true, result["submitted"] as? Bool == false else {
                    finish(1, "FAIL: terminal bytes changed or submitted"); return
                }
                print("PASS: real Terminal receives exact Unicode paste without Enter")
                var status: Int32 = 1; token = targetCapture(&status)
                guard token != 0, send("line one\nline two") == 7 else { finish(1, "FAIL: multiline terminal text was not kept copied"); return }
                token = targetCapture(&status); targetCancel(token)
                guard send("Logia terminal café.") == 3 else { finish(1, "FAIL: canceled terminal attempt accepted"); return }
                print("PASS: multiline and canceled terminal input rejected")
                snapshot = try? FocusSnapshot.capture()
                token = targetCapture(&status)
                guard snapshot != nil, token != 0 else { finish(1, "FAIL: recapture"); return }
                var windows: CFTypeRef?
                AXUIElementCopyAttributeValue(candidate, kAXWindowsAttribute as CFString, &windows)
                previousWindows = windows as? [AXUIElement] ?? []
                guard !previousWindows.isEmpty else { finish(1, "FAIL: fixture ownership unavailable"); return }
                key(17) // Command-T: a blank tab in this owned window only.
                phase = 2; return
            }
            if phase == 2 {
                guard let fresh = try? FocusSnapshot.capture(), snapshot?.verdict(comparedTo: fresh) != .same else { return }
                if !CFEqual(window, focusedWindow) {
                    guard !previousWindows.contains(where: { CFEqual($0, focusedWindow) }) else { finish(1, "FAIL: another existing window took focus"); return }
                    newTab = focusedWindow
                }
                guard send("Logia terminal café.") == 5 else { finish(1, "FAIL: current tab did not receive a paste command"); return }
                print("PASS: paste dispatch follows the current owned Terminal tab; no Enter sent")
                finish(0, "PASS: Terminal delivery checks complete")
            }
            }
        }
        app.run()
    }
}
