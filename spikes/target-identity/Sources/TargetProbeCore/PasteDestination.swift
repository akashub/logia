import AppKit
import ApplicationServices
import Carbon

// This is a delivery-time snapshot, not a recording-start destination lock.
// No contents/titles are read. Field and range evidence are optional because
// keyboard editors need not expose AX text roles or accessibility write APIs.
struct PasteDestination {
    let pid: pid_t
    private let process: ProcessIdentity
    private let window: AXUIElement
    private let field: AXUIElement?
    private let selection: CFRange?

    static func capture() throws -> Self {
        guard AXIsProcessTrusted(), !IsSecureEventInputEnabled(),
              let running = NSWorkspace.shared.frontmostApplication,
              running.isActive, !running.isTerminated,
              running.processIdentifier != ProcessInfo.processInfo.processIdentifier,
              let identity = ProcessIdentity.read(running.processIdentifier) else { throw CaptureFailure("paste-context-unavailable") }
        let app = AXUIElementCreateApplication(identity.pid)
        let window = try AccessibilityRead.element(app, kAXFocusedWindowAttribute)
        let observedField = try PasteMetadata.element(app, kAXFocusedUIElementAttribute)
        var field = observedField
        // An AX window/application may be the only exposed ancestor of a custom
        // keyboard editor. It cannot tell us whether a text cursor exists.
        if let candidate = field, CFEqual(candidate, app) || CFEqual(candidate, window) { field = nil }
        var range: CFRange?
        if let field {
            try PasteMetadata.requirePasteCandidate(field)
            if let parent = try PasteMetadata.element(field, kAXWindowAttribute), !CFEqual(parent, window) {
                throw CaptureFailure("paste-window-mismatch")
            }
            guard try PasteMetadata.bool(field, kAXFocusedAttribute) != false else { throw CaptureFailure("paste-field-not-focused") }
            range = try PasteMetadata.selection(field)
        }
        let finalWindow = try AccessibilityRead.element(app, kAXFocusedWindowAttribute)
        let finalField = try PasteMetadata.element(app, kAXFocusedUIElementAttribute)
        guard CFEqual(window, finalWindow), Self.same(observedField, finalField),
              let front = NSWorkspace.shared.frontmostApplication,
              front.processIdentifier == identity.pid, front.isActive, !front.isTerminated,
              ProcessIdentity.read(identity.pid) == identity else { throw CaptureFailure("paste-process-changed") }
        return Self(pid: identity.pid, process: identity, window: window, field: field, selection: range)
    }

    func isCurrent() -> Bool {
        guard let fresh = try? Self.capture(), process == fresh.process, CFEqual(window, fresh.window) else { return false }
        if let field {
            guard let current = fresh.field, CFEqual(field, current) else { return false }
        }
        if let selection {
            guard let current = fresh.selection, selection.location == current.location,
                  selection.length == current.length else { return false }
        }
        return true
    }

    private static func same(_ before: AXUIElement?, _ after: AXUIElement?) -> Bool {
        switch (before, after) {
        case (nil, nil): return true
        case (let before?, let after?): return CFEqual(before, after)
        default: return false
        }
    }
}
