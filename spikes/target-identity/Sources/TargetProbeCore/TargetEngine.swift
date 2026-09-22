import AppKit

public enum TargetEngine: Equatable {
    case native, chromium, electron, terminal

    public var activationAttribute: String? {
        switch self {
        case .native, .terminal: return nil
        case .chromium: return "AXEnhancedUserInterface"
        case .electron: return "AXManualAccessibility"
        }
    }

    public static func identify(bundle: String?, electronFramework: Bool) -> Self {
        if bundle == "com.apple.Terminal" { return .terminal }
        if electronFramework { return .electron }
        let families = ["com.google.Chrome", "com.brave.Browser", "com.microsoft.edgemac", "company.thebrowser.Browser"]
        if let bundle, families.contains(where: { bundle == $0 || bundle.hasPrefix($0 + ".") }) { return .chromium }
        return .native
    }

    public static func identify(_ app: NSRunningApplication) -> Self {
        let framework = app.bundleURL?.appendingPathComponent("Contents/Frameworks/Electron Framework.framework").path
        return identify(bundle: app.bundleIdentifier, electronFramework: framework.map { FileManager.default.fileExists(atPath: $0) } ?? false)
    }
}

// Bounded, process-start-aware cooldown: repeated activation must not restart
// Chromium's two-second debounce, but a later renderer reset can recover.
struct PreparationThrottle {
    private var attempts: [(ProcessIdentity, TimeInterval)] = []
    mutating func claim(_ identity: ProcessIdentity, now: TimeInterval) -> Bool {
        if let previous = attempts.first(where: { $0.0 == identity }), now - previous.1 < 10 { return false }
        attempts.removeAll { $0.0.pid == identity.pid }
        attempts.append((identity, now))
        if attempts.count > 32 { attempts.removeFirst(attempts.count - 32) }
        return true
    }
}
