import AppKit
import ApplicationServices

// Only the fresh Chrome process owned by the JS test may be captured. Reads of
// field contents happen in that test's synthetic DOM, never in this AX helper.
@main struct BrowserDeliveryCheck {
    static func main() {
        guard CommandLine.arguments.count == 2, let pid = Int32(CommandLine.arguments[1]),
              let running = NSRunningApplication(processIdentifier: pid),
              running.bundleIdentifier == "com.google.Chrome", AXIsProcessTrusted() else { exit(2) }
        let app = NSApplication.shared
        app.setActivationPolicy(.prohibited)
        var token: UInt64 = 0
        let text = "Logia café — delivery check."
        let clipboard = NSPasteboard.general
        let saved = (clipboard.pasteboardItems ?? []).map { item -> NSPasteboardItem in
            let copy = NSPasteboardItem()
            for type in item.types { if let data = item.data(forType: type) { copy.setData(data, forType: type) } }
            return copy
        }
        var ownedClipboard: Int?
        func reply(_ object: [String: Any]) {
            FileHandle.standardOutput.write(try! JSONSerialization.data(withJSONObject: object, options: [.sortedKeys]) + Data([10]))
        }
        func handle(_ command: String) {
            if command == "quit" {
                if clipboard.changeCount == ownedClipboard {
                    clipboard.clearContents()
                    if !saved.isEmpty { clipboard.writeObjects(saved) }
                }
                app.terminate(nil); return
            }
            if command == "activate" { reply(["activated": running.activate(options: [])]); return }
            if command == "watch" { reply(["trusted": targetPermission(false)]); return }
            if command == "cancel" { targetCancel(token); reply(["canceled": true]); return }
            if command == "clipboard-change" {
                clipboard.clearContents(); clipboard.setString("Newer synthetic clipboard", forType: .string)
                ownedClipboard = clipboard.changeCount; reply(["changed": true]); return
            }
            if command == "clipboard-check" { reply(["preserved": clipboard.string(forType: .string) == "Newer synthetic clipboard"]); return }
            if command == "clipboard-transcript" { reply(["copied": clipboard.string(forType: .string) == text]); return }
            if command == "send" {
                let bytes = Array(text.utf8)
                let result = bytes.withUnsafeBufferPointer { targetSend(token, $0.baseAddress, $0.count) }
                if result == DeliveryResult.dispatched.rawValue || result == DeliveryResult.copied.rawValue { ownedClipboard = clipboard.changeCount }
                reply(["result": result]); return
            }
            guard NSWorkspace.shared.frontmostApplication?.processIdentifier == pid else { reply(["error": "fixture not foreground"]); return }
            if command == "arm" {
                var status: Int32 = 1; token = targetCapture(&status)
                reply(["status": status, "armed": token != 0]); return
            }
            if command == "selection" {
                if let snapshot = try? FocusSnapshot.capture(), let range = try? snapshot.selection() {
                    reply(["location": range.location, "length": range.length])
                } else { reply(["unavailable": true]) }
                return
            }
            if command.hasPrefix("click ") {
                let parts = command.split(separator: " ")
                guard parts.count == 3, let x = Double(parts[1]), let y = Double(parts[2]) else { exit(64) }
                let point = CGPoint(x: x, y: y)
                for type in [CGEventType.leftMouseDown, .leftMouseUp] {
                    CGEvent(mouseEventSource: nil, mouseType: type, mouseCursorPosition: point, mouseButton: .left)?.postToPid(pid)
                }
                reply(["clicked": true]); return
            }
            reply(["error": "unknown command"])
        }
        reply(["ready": true])
        DispatchQueue.global().async {
            while let command = readLine() {
                let finished = DispatchSemaphore(value: 0)
                DispatchQueue.main.async { handle(command); finished.signal() }
                finished.wait()
            }
            DispatchQueue.main.async { handle("quit") }
        }
        app.run()
    }
}
