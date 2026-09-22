import AppKit
import ApplicationServices
import Carbon

enum VerifiedPaste {
    static func send(_ text: String, to pid: pid_t, clipboardVersion: Int, verify: () -> Bool) -> DeliveryResult {
        // Construct both events before touching the clipboard. Target the current
        // verified process, never send Enter, and never retry a dispatched paste.
        guard CGPreflightPostEventAccess(), !IsSecureEventInputEnabled(),
              let source = CGEventSource(stateID: .privateState),
              let down = CGEvent(keyboardEventSource: source, virtualKey: 9, keyDown: true),
              let up = CGEvent(keyboardEventSource: source, virtualKey: 9, keyDown: false) else {
            return copy(text, clipboardVersion: clipboardVersion)
        }
        down.flags = .maskCommand
        up.flags = .maskCommand
        let pasteboard = NSPasteboard.general
        var attempt = DeliveryAttempt()
        return attempt.paste(text: text, expectedClipboard: clipboardVersion,
            clipboard: { pasteboard.changeCount }, stage: { value in
                let owned = pasteboard.clearContents()
                guard pasteboard.setString(value, forType: .string) else { return nil }
                return owned
            }, verify: { CGPreflightPostEventAccess() && !IsSecureEventInputEnabled() && verify() }, dispatch: {
                down.postToPid(pid)
                up.postToPid(pid)
            })
        // Leave the transcript on the clipboard. There is no receipt that could
        // safely authorize restoring it while the destination reads the paste.
    }

    static func copy(_ text: String, clipboardVersion: Int) -> DeliveryResult {
        let pasteboard = NSPasteboard.general
        var attempt = DeliveryAttempt()
        return attempt.paste(text: text, expectedClipboard: clipboardVersion,
            clipboard: { pasteboard.changeCount }, stage: { value in
                let owned = pasteboard.clearContents()
                guard pasteboard.setString(value, forType: .string) else { return nil }
                return owned
            }, verify: { false }, dispatch: {})
    }
}
