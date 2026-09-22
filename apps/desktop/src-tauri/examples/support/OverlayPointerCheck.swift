import AppKit
import ApplicationServices

// This helper only posts pointer events inside the owned smoke process's panel.
// It never opens a microphone, writes a field, or inspects unrelated app content.
func fail(_ message: String) -> Never {
    fputs("FAIL: \(message)\n", stderr)
    exit(1)
}
let args = CommandLine.arguments
if args.count != 4 { fail("Expected overlay PID, fixture PID, action") }
let overlayPID = Int32(args[1])!
let fixturePID = Int32(args[2])!
let action = args[3]
guard AXIsProcessTrusted() else { fail("Synthetic pointer checks need Accessibility permission") }
let expectedForeground = action == "edit" ? overlayPID : fixturePID
guard NSWorkspace.shared.frontmostApplication?.processIdentifier == expectedForeground else { fail("Expected owned app not foreground") }
let fixture = AXUIElementCreateApplication(fixturePID)
func attribute(_ element: AXUIElement, _ name: CFString) -> CFTypeRef? {
    var value: CFTypeRef?
    guard AXUIElementCopyAttributeValue(element, name, &value) == .success else { return nil }
    return value
}
guard let focused = attribute(fixture, kAXFocusedUIElementAttribute as CFString) else { fail("No fixture field") }
let field = focused as! AXUIElement
let beforeValue = attribute(field, kAXValueAttribute as CFString)
let beforeSelection = attribute(field, kAXSelectedTextRangeAttribute as CFString)
// Activation animates the panel's presented frame. Wait for its intended
// geometry before computing pointer coordinates; never click a transient frame.
var panelBounds: [String: CGFloat]?
for _ in 0..<30 {
    let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
    let info = windows.first { ($0[kCGWindowOwnerPID as String] as? Int32) == overlayPID && ($0[kCGWindowLayer as String] as? Int) == 3 }
    if let bounds = info?[kCGWindowBounds as String] as? [String: CGFloat], bounds["Width"] == 464, bounds["Height"] == 120 {
        panelBounds = bounds; break
    }
    usleep(50_000)
}
guard let bounds = panelBounds, let x = bounds["X"], let y = bounds["Y"] else {
    let owned = (CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? [])
        .filter { ($0[kCGWindowOwnerPID as String] as? Int32) == overlayPID }
        .map { "layer=\($0[kCGWindowLayer as String] ?? "?") bounds=\($0[kCGWindowBounds as String] ?? "?")" }
    fail("No visible 464x120 floating panel owned by smoke process: \(owned)")
}
let offset: CGPoint
switch action {
case "stop": offset = CGPoint(x: 60, y: 90)
case "cancel": offset = CGPoint(x: 170, y: 90)
case "copy": offset = CGPoint(x: 280, y: 90)
case "scroll", "edit": offset = CGPoint(x: 200, y: 38)
default: fail("Unknown pointer action")
}
let point = CGPoint(x: x + offset.x, y: y + offset.y)
let source = CGEventSource(stateID: .hidSystemState)
CGEvent(mouseEventSource: source, mouseType: .mouseMoved, mouseCursorPosition: point, mouseButton: .left)?.post(tap: .cghidEventTap)
usleep(100_000)
if action == "scroll" {
    let scroll = CGEvent(scrollWheelEvent2Source: source, units: .pixel, wheelCount: 1, wheel1: -160, wheel2: 0, wheel3: 0)
    scroll?.location = point
    scroll?.post(tap: .cghidEventTap)
} else {
    CGEvent(mouseEventSource: source, mouseType: .leftMouseDown, mouseCursorPosition: point, mouseButton: .left)?.post(tap: .cghidEventTap)
    usleep(50_000)
    CGEvent(mouseEventSource: source, mouseType: .leftMouseUp, mouseCursorPosition: point, mouseButton: .left)?.post(tap: .cghidEventTap)
}
usleep(300_000)
if action == "edit" {
    guard NSWorkspace.shared.frontmostApplication?.processIdentifier == overlayPID else { fail("Editing did not focus owned overlay; will not send a key") }
    let down = CGEvent(keyboardEventSource: source, virtualKey: 0, keyDown: true)
    let up = CGEvent(keyboardEventSource: source, virtualKey: 0, keyDown: false)
    var character: UniChar = 33
    down?.keyboardSetUnicodeString(stringLength: 1, unicodeString: &character)
    up?.keyboardSetUnicodeString(stringLength: 1, unicodeString: &character)
    down?.post(tap: .cghidEventTap)
    up?.post(tap: .cghidEventTap)
    usleep(300_000)
}
guard NSWorkspace.shared.frontmostApplication?.processIdentifier == expectedForeground,
      let after = attribute(fixture, kAXFocusedUIElementAttribute as CFString), CFEqual(focused, after),
      let value = attribute(field, kAXValueAttribute as CFString), let beforeValue, CFEqual(beforeValue, value),
      let selection = attribute(field, kAXSelectedTextRangeAttribute as CFString), let beforeSelection,
      CFEqual(beforeSelection, selection) else { fail("\(action) changed fixture focus, content or selection") }
print("PASS: actual \(action) pointer preserves app, field, content and selection")
