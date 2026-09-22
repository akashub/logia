import AppKit
import WebKit

// Create NSPanel with this style rather than mutating an NSWindow's class.
// A retained Tauri window still owns IPC/lifecycle; only its WKWebView moves.
private final class RecordingPanel: NSPanel {
    var acceptsKeyboard = false
    override var canBecomeKey: Bool { acceptsKeyboard }
    override var canBecomeMain: Bool { false }
}

private var panel: RecordingPanel?
private weak var overlayWebview: WKWebView?
private var pinnedScreen: NSScreen?
private var topEdge = false

@_cdecl("logia_overlay_attach")
public func overlayAttach(_ pointer: UnsafeMutableRawPointer) {
    precondition(Thread.isMainThread)
    guard panel == nil else { return }
    let webview = Unmanaged<WKWebView>.fromOpaque(pointer).takeUnretainedValue()
    let created = RecordingPanel(contentRect: NSRect(x: 0, y: 0, width: 38, height: 38),
                                 styleMask: [.borderless, .nonactivatingPanel],
                                 backing: .buffered, defer: false)
    created.title = "Logia overlay"
    created.isReleasedWhenClosed = false
    created.isOpaque = false
    created.backgroundColor = .clear
    created.hasShadow = false // The web card supplies its own bounded shadow.
    created.level = .floating
    created.hidesOnDeactivate = false
    created.becomesKeyOnlyIfNeeded = true
    created.isFloatingPanel = true
    created.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .ignoresCycle]
    webview.removeFromSuperview()
    webview.frame = created.contentView!.bounds
    webview.autoresizingMask = [.width, .height]
    created.contentView!.addSubview(webview)
    overlayWebview = webview
    panel = created
}

@_cdecl("logia_overlay_resize")
public func overlayResize(_ width: Double, _ height: Double, _ top: Bool) {
    precondition(Thread.isMainThread)
    guard width.isFinite, height.isFinite, (38...900).contains(width),
          (38...700).contains(height), let panel else { return }
    topEdge = top
    let screen = pinnedScreen ?? activeScreen()
    guard let screen else { return }
    let area = screen.visibleFrame
    let size = NSSize(width: min(width, area.width), height: min(height, area.height))
    let margin = 24.0
    let origin = NSPoint(x: area.midX - size.width / 2,
                         y: top ? area.maxY - size.height - margin : area.minY + margin)
    panel.setFrame(NSRect(origin: origin, size: size), display: true)
}

private func activeScreen() -> NSScreen? {
    // Pin the pointer's display for a reveal, so a live card never follows it.
    NSScreen.screens.first { NSMouseInRect(NSEvent.mouseLocation, $0.frame, false) }
        ?? NSScreen.main ?? NSScreen.screens.first
}

@_cdecl("logia_overlay_show")
public func overlayShow() -> Bool {
    precondition(Thread.isMainThread)
    guard let panel else { return false }
    overlayEditing(false)
    // A new reveal starts a new intent, even when the idle pip was visible.
    pinnedScreen = activeScreen()
    overlayResize(panel.frame.width, panel.frame.height, topEdge)
    panel.orderFrontRegardless()
    return true
}

@_cdecl("logia_overlay_editing")
public func overlayEditing(_ enabled: Bool) -> Bool {
    precondition(Thread.isMainThread)
    guard let panel else { return false }
    panel.acceptsKeyboard = enabled
    if enabled {
        // Only an explicit, idle-session edit request reaches this path.
        NSApp.activate(ignoringOtherApps: true)
        panel.makeKeyAndOrderFront(nil)
        guard let webview = overlayWebview, panel.makeFirstResponder(webview) else {
            panel.acceptsKeyboard = false
            panel.resignKey()
            return false
        }
    } else if panel.isKeyWindow {
        panel.resignKey()
    }
    return true
}

@_cdecl("logia_overlay_visible")
public func overlayVisible() -> Bool {
    precondition(Thread.isMainThread)
    return panel.map { $0.isVisible && $0.isOnActiveSpace } ?? false
}

@_cdecl("logia_overlay_hide")
public func overlayHide() {
    precondition(Thread.isMainThread)
    overlayEditing(false)
    panel?.orderOut(nil)
    pinnedScreen = nil
}
