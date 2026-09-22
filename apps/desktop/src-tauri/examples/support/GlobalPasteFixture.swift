import AppKit

// A keyboard editor with deliberately non-text AX metadata. This is not a
// simulated delivery result: it receives actual Command-V events in its process.
final class KeyboardEditor: NSView {
    var received = "", pastes = 0
    var exposesAccessibility = true
    var exposesRole = true
    var enabled = true
    override var acceptsFirstResponder: Bool { true }
    override func accessibilityRole() -> NSAccessibility.Role? { exposesRole ? .group : nil }
    override func isAccessibilityElement() -> Bool { exposesAccessibility }
    override func isAccessibilityFocused() -> Bool { window?.firstResponder === self }
    override func isAccessibilityEnabled() -> Bool { enabled }
    override func performKeyEquivalent(with event: NSEvent) -> Bool {
        guard window?.firstResponder === self, event.modifierFlags.contains(.command),
              event.charactersIgnoringModifiers?.lowercased() == "v" else { return false }
        received += NSPasteboard.general.string(forType: .string) ?? ""
        pastes += 1
        return true
    }
}

@main struct GlobalPasteFixture {
    @MainActor static func main() {
        let app = NSApplication.shared
        app.setActivationPolicy(.regular)
        let menu = NSMenu(), edit = NSMenu()
        let appItem = NSMenuItem(title: "Fixture", action: nil, keyEquivalent: "")
        appItem.submenu = NSMenu(); menu.addItem(appItem)
        let editItem = NSMenuItem(title: "Edit", action: nil, keyEquivalent: "")
        editItem.submenu = edit; menu.addItem(editItem)
        edit.addItem(withTitle: "Paste", action: #selector(NSText.paste(_:)), keyEquivalent: "v")
        app.mainMenu = menu
        let window = NSWindow(contentRect: NSRect(x: 180, y: 240, width: 500, height: 300),
                              styleMask: [.titled, .closable], backing: .buffered, defer: false)
        window.title = "Logia automated paste checks"
        window.isReleasedWhenClosed = false
        let first = NSTextView(frame: NSRect(x: 20, y: 200, width: 460, height: 70))
        let second = NSTextView(frame: NSRect(x: 20, y: 110, width: 460, height: 70))
        let custom = KeyboardEditor(frame: NSRect(x: 20, y: 20, width: 200, height: 60))
        let secure = NSSecureTextField(frame: NSRect(x: 230, y: 50, width: 120, height: 24))
        let button = NSButton(title: "No text cursor", target: nil, action: nil)
        button.frame = NSRect(x: 230, y: 15, width: 180, height: 24)
        for view in [first, second] { view.isRichText = false; view.font = .systemFont(ofSize: 16) }
        for view in [first, second, custom, secure, button] { window.contentView?.addSubview(view) }
        let other = NSWindow(contentRect: NSRect(x: 220, y: 280, width: 360, height: 150),
                             styleMask: [.titled], backing: .buffered, defer: false)
        other.isReleasedWhenClosed = false
        let third = NSTextView(frame: NSRect(x: 20, y: 20, width: 320, height: 100))
        third.isRichText = false; other.contentView?.addSubview(third)
        func reply(_ ok: Bool) { FileHandle.standardOutput.write(Data((ok ? "ok\n" : "fail\n").utf8)) }
        func handle(_ command: String) {
            if command == "quit" { app.terminate(nil); return }
            if command == "assert-first" { reply(first.string == "Before Logia café. after"); return }
            if command == "assert-second" { reply(second.string == "Logia café." && first.string == "Before OLD after"); return }
            if command == "assert-caret" { reply(first.string == "Logia café.Before OLD after"); return }
            if command == "assert-window" { reply(third.string == "Logia café." && first.string == "Before OLD after"); return }
            if command == "assert-custom" { reply(custom.received == "Logia café." && custom.pastes == 1); return }
            if command == "assert-untouched" { reply(first.string == "Before OLD after" && second.string.isEmpty && custom.pastes == 0 && secure.stringValue.isEmpty); return }
            switch command {
            case "reset":
                other.orderOut(nil); first.string = "Before OLD after"; second.string = ""; third.string = ""
                custom.received = ""; custom.pastes = 0; custom.exposesAccessibility = true; custom.exposesRole = true; secure.stringValue = ""
                custom.enabled = true
                first.isEditable = true; first.isSelectable = true
                window.makeFirstResponder(first); first.setSelectedRange(NSRange(location: 7, length: 3))
            case "second": window.makeFirstResponder(second)
            case "caret": first.setSelectedRange(NSRange(location: 0, length: 0))
            case "custom": window.makeFirstResponder(custom)
            case "hidden": custom.exposesAccessibility = false; window.makeFirstResponder(custom)
            case "no-role": custom.exposesRole = false; window.makeFirstResponder(custom)
            case "disabled": custom.enabled = false; window.makeFirstResponder(custom)
            case "secure": window.makeFirstResponder(secure)
            case "button": window.makeFirstResponder(button)
            case "nowhere": window.makeFirstResponder(nil)
            case "window": other.makeFirstResponder(third); other.makeKeyAndOrderFront(nil)
            default: reply(false); return
            }
            if command != "window" { window.makeKeyAndOrderFront(nil) }
            app.activate(ignoringOtherApps: true)
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { reply(true) }
        }
        DispatchQueue.global().async {
            while let command = readLine() { DispatchQueue.main.async { handle(command) } }
            DispatchQueue.main.async { app.terminate(nil) }
        }
        app.run()
    }
}
