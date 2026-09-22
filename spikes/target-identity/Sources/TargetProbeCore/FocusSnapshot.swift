import AppKit
import ApplicationServices

// Native references stay in memory. No field values, titles, URLs, or text are read.
public struct FocusSnapshot {
    private let pid: pid_t
    private let processIdentity: ProcessIdentity
    private let application: AXUIElement
    private let window: AXUIElement
    private let field: AXUIElement
    private let paste: Bool
    private let terminal: Bool
    private let clipboardVersion: Int

    public static var hasPermission: Bool { AXIsProcessTrusted() }
    public var belongsToCurrentProcess: Bool { pid == ProcessInfo.processInfo.processIdentifier }

    // Selection metadata only. Reading the text itself is unnecessary.
    public func selection() throws -> CFRange {
        try AccessibilityRead.requireEditable(field, terminal: terminal)
        if !paste {
            var settable: DarwinBoolean = false
            guard AXUIElementIsAttributeSettable(field, kAXSelectedTextAttribute as CFString, &settable) == .success,
                  settable.boolValue else { throw CaptureFailure("selected-text-not-settable") }
        }
        let value = try AccessibilityRead.value(field, kAXSelectedTextRangeAttribute)
        guard CFGetTypeID(value) == AXValueGetTypeID() else { throw CaptureFailure("selection-type") }
        let ax = unsafeDowncast(value, to: AXValue.self)
        var range = CFRange()
        guard AXValueGetType(ax) == .cfRange, AXValueGetValue(ax, .cfRange, &range),
              range.location >= 0, range.length >= 0 else { throw CaptureFailure("selection-unavailable") }
        guard !terminal || range.length == 0 else { throw CaptureFailure("terminal-output-selected") }
        return range
    }

    public func send(_ text: String, selection expected: CFRange) -> DeliveryResult {
        guard !terminal || TerminalInput.allows(text) else { return .copyRequired }
        let verify = {
            guard let fresh = try? Self.capture(), verdict(comparedTo: fresh) == .same,
                  let range = try? fresh.selection(), range.location == expected.location,
                  range.length == expected.length, stillFocused() else { return false }
            return true
        }
        if paste { return VerifiedPaste.send(text, to: pid, clipboardVersion: clipboardVersion, verify: verify) }
        var attempt = DeliveryAttempt()
        return attempt.send(text: text, verify: { verify() ? .same : .unknown }, write: {
            // Exact retained field, one selected-text write. Never AXValue or Enter.
            guard (try? AccessibilityRead.bound(field)) != nil else { return false }
            return AXUIElementSetAttributeValue(field, kAXSelectedTextAttribute as CFString, $0 as CFString) == .success
        })
    }

    private func stillFocused() -> Bool {
        guard let focusedWindow = try? AccessibilityRead.element(application, kAXFocusedWindowAttribute),
              CFEqual(window, focusedWindow),
              let focusedField = try? AccessibilityRead.element(application, kAXFocusedUIElementAttribute),
              CFEqual(field, focusedField),
              let front = NSWorkspace.shared.frontmostApplication,
              front.isActive, !front.isTerminated, front.processIdentifier == pid,
              ProcessIdentity.read(pid) == processIdentity else { return false }
        return true
    }

    public static func capture() throws -> FocusSnapshot {
        guard hasPermission else { throw CaptureFailure("accessibility-permission-missing") }
        guard let running = NSWorkspace.shared.frontmostApplication else {
            throw CaptureFailure("active-application-unavailable")
        }
        guard running.isActive else { throw CaptureFailure("active-application-not-active") }
        guard !running.isTerminated else { throw CaptureFailure("active-application-terminated") }
        // System-wide AXFocusedApplication fails on some macOS installations.
        // Active application is a candidate only: require native focused field
        // and focused window agreement, then recheck the active process.
        let pid = running.processIdentifier
        guard let identity = ProcessIdentity.read(pid) else { throw CaptureFailure("process-identity-unavailable") }
        let app = AXUIElementCreateApplication(pid)
        let timeout = AXUIElementSetMessagingTimeout(app, 0.25)
        guard timeout == .success else { throw CaptureFailure("messaging-timeout", code: timeout) }
        let field = try AccessibilityRead.element(app, kAXFocusedUIElementAttribute)
        let engine = TargetEngine.identify(running)
        try AccessibilityRead.requireEditable(field, terminal: engine == .terminal)
        guard try AccessibilityRead.value(field, kAXFocusedAttribute) as? Bool == true else {
            throw CaptureFailure("field-not-focused")
        }
        let window = try AccessibilityRead.element(field, kAXWindowAttribute)
        let focusedWindow = try AccessibilityRead.element(app, kAXFocusedWindowAttribute)
        guard CFEqual(window, focusedWindow) else { throw CaptureFailure("focused-window-mismatch") }
        guard let stillFront = NSWorkspace.shared.frontmostApplication,
              stillFront.processIdentifier == pid, ProcessIdentity.read(pid) == identity,
              stillFront.isActive, !stillFront.isTerminated else {
            throw CaptureFailure("application-changed-during-capture")
        }
        let finalField = try AccessibilityRead.element(app, kAXFocusedUIElementAttribute)
        guard CFEqual(field, finalField) else { throw CaptureFailure("field-changed-during-capture") }
        return FocusSnapshot(pid: pid, processIdentity: identity, application: app, window: window, field: field,
                             paste: engine != .native, terminal: engine == .terminal, clipboardVersion: NSPasteboard.general.changeCount)
    }

    public func verdict(comparedTo fresh: FocusSnapshot?) -> TargetVerdict {
        guard let fresh else { return .unknown }
        guard processIdentity == fresh.processIdentity else { return .changed }
        // Retained AX handles may be stale even when a fresh lookup succeeds.
        guard (try? AccessibilityRead.requireEditable(field, terminal: terminal)) != nil,
              (try? AccessibilityRead.value(window, kAXRoleAttribute)) != nil,
              ProcessIdentity.read(pid) == processIdentity else { return .unknown }
        return compare(process: CFEqual(application, fresh.application),
                       window: CFEqual(window, fresh.window), field: CFEqual(field, fresh.field))
    }

}
