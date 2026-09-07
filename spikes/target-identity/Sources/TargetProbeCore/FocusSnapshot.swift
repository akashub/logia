import AppKit
import ApplicationServices

// Native references stay in memory. No field values, titles, URLs, or text are read.
public struct FocusSnapshot {
    private let pid: pid_t
    private let processIdentity: ProcessIdentity
    private let application: AXUIElement
    private let window: AXUIElement
    private let field: AXUIElement

    public static var hasPermission: Bool { AXIsProcessTrusted() }

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
        try AccessibilityRead.requireEditable(field)
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
        return FocusSnapshot(pid: pid, processIdentity: identity, application: app, window: window, field: field)
    }

    public func verdict(comparedTo fresh: FocusSnapshot?) -> TargetVerdict {
        guard let fresh else { return .unknown }
        guard processIdentity == fresh.processIdentity else { return .changed }
        // Retained AX handles may be stale even when a fresh lookup succeeds.
        guard (try? AccessibilityRead.requireEditable(field)) != nil,
              (try? AccessibilityRead.value(window, kAXRoleAttribute)) != nil,
              ProcessIdentity.read(pid) == processIdentity else { return .unknown }
        return compare(process: CFEqual(application, fresh.application),
                       window: CFEqual(window, fresh.window), field: CFEqual(field, fresh.field))
    }

}
