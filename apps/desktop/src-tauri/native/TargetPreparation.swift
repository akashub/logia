import AppKit
import ApplicationServices

enum TargetPreparation {
    private static var observer: NSObjectProtocol?
    private static var throttle = PreparationThrottle()

    static func start() {
        precondition(Thread.isMainThread)
        if observer == nil {
            observer = NSWorkspace.shared.notificationCenter.addObserver(
                forName: NSWorkspace.didActivateApplicationNotification, object: nil, queue: .main
            ) { _ in prepareFrontmost() }
        }
        prepareFrontmost()
    }

    static func prepareFrontmost() {
        precondition(Thread.isMainThread)
        guard AXIsProcessTrusted(), let running = NSWorkspace.shared.frontmostApplication,
              running.processIdentifier != ProcessInfo.processInfo.processIdentifier,
              running.isActive, !running.isTerminated,
              let identity = ProcessIdentity.read(running.processIdentifier),
              let attribute = TargetEngine.identify(running).activationAttribute else { return }
        let app = AXUIElementCreateApplication(running.processIdentifier)
        guard AXUIElementSetMessagingTimeout(app, 0.25) == .success else { return }
        // Reading AXRole also enables Chromium's basic native accessibility mode.
        var role: CFTypeRef?
        _ = AXUIElementCopyAttributeValue(app, kAXRoleAttribute as CFString, &role)
        guard NSWorkspace.shared.frontmostApplication?.processIdentifier == identity.pid,
              ProcessIdentity.read(identity.pid) == identity,
              throttle.claim(identity, now: ProcessInfo.processInfo.systemUptime) else { return }
        // Mark the attempt before writing: Chrome can return notImplemented
        // AFTER scheduling activation. Never wait or switch focus here.
        _ = AXUIElementSetAttributeValue(app, attribute as CFString, kCFBooleanTrue)
    }
}
