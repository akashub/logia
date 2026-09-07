import AppKit

// Only this synthetic window is controlled. Never opens or edits another app.
@MainActor final class Fixture {
    let window = NSWindow(contentRect: NSRect(x: 200, y: 250, width: 560, height: 340),
                          styleMask: [.titled, .closable], backing: .buffered, defer: false)
    var first = NSTextView(frame: NSRect(x: 20, y: 160, width: 520, height: 70))
    let second = NSTextView(frame: NSRect(x: 20, y: 70, width: 520, height: 70))
    let secure = NSSecureTextField(frame: NSRect(x: 20, y: 20, width: 260, height: 30))

    init() {
        window.title = "Logia — automatic field checks"
        window.isReleasedWhenClosed = false
        let label = NSTextField(labelWithString: "Synthetic fields only. Checks run automatically; please leave focus here.")
        label.frame = NSRect(x: 20, y: 255, width: 520, height: 50)
        label.lineBreakMode = .byWordWrapping
        window.contentView?.addSubview(label)
        first.string = "First synthetic text area"
        second.string = "Second synthetic text area"
        for view in [first, second] {
            view.font = .systemFont(ofSize: 17)
            view.isRichText = false
            window.contentView?.addSubview(view)
        }
        secure.placeholderString = "Synthetic password field"
        window.contentView?.addSubview(secure)
    }

    func handle(_ command: String) {
        if command == "quit" { NSApp.terminate(nil); return }
        first.isEditable = true
        switch command {
        case "first": window.makeFirstResponder(first)
        case "second": window.makeFirstResponder(second)
        case "secure": window.makeFirstResponder(secure)
        case "readonly":
            first.isEditable = false
            window.makeFirstResponder(first)
        case "recreate":
            let replacement = NSTextView(frame: first.frame)
            replacement.string = "First synthetic text area"
            replacement.isRichText = false
            first.removeFromSuperview()
            first = replacement
            window.contentView?.addSubview(first)
            window.makeFirstResponder(first)
        default: return
        }
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        // Acknowledge on a later run-loop turn so AX sees the new responder.
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) {
            FileHandle.standardOutput.write(Data("ready\n".utf8))
        }
    }
}

@main struct FixtureApp {
    @MainActor static func main() {
        let app = NSApplication.shared
        app.setActivationPolicy(.regular)
        let fixture = Fixture()
        DispatchQueue.global().async {
            while let command = readLine() {
                DispatchQueue.main.async { fixture.handle(command) }
            }
            DispatchQueue.main.async { NSApp.terminate(nil) }
        }
        app.run()
    }
}
