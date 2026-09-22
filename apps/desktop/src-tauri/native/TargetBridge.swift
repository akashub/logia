import AppKit
import ApplicationServices

// Rust dispatches every entry to the main thread. No Swift object crosses the ABI.
private var sequence: UInt64 = 0
private var current: (token: UInt64, clipboardVersion: Int)?

@_cdecl("logia_target_capture")
public func targetCapture(_ status: UnsafeMutablePointer<Int32>) -> UInt64 {
    precondition(Thread.isMainThread)
    current = nil
    // Arm a session, not a field. The user's cursor may move while speaking.
    // Native clipboard ownership applies even if typing permission is absent.
    status.pointee = FocusSnapshot.hasPermission ? 0 : 2
    sequence &+= 1
    if sequence == 0 { sequence = 1 }
    current = (sequence, NSPasteboard.general.changeCount)
    return sequence
}

@_cdecl("logia_target_cancel")
public func targetCancel(_ token: UInt64) {
    precondition(Thread.isMainThread)
    if current?.token == token { current = nil }
}

@_cdecl("logia_target_send")
public func targetSend(_ token: UInt64, _ bytes: UnsafePointer<UInt8>?, _ count: Int) -> Int32 {
    precondition(Thread.isMainThread)
    guard let target = current, target.token == token else { return DeliveryResult.consumed.rawValue }
    current = nil // Consume before validation, checks, or AX. A failed send is final.
    guard count > 0, count <= 65_536, let bytes,
          let text = String(bytes: UnsafeBufferPointer(start: bytes, count: count), encoding: .utf8) else {
        return DeliveryResult.invalid.rawValue
    }
    let destination = TerminalInput.allows(text) ? try? PasteDestination.capture() : nil
    guard let destination else {
        return VerifiedPaste.copy(text, clipboardVersion: target.clipboardVersion).rawValue
    }
    return VerifiedPaste.send(text, to: destination.pid, clipboardVersion: target.clipboardVersion,
                              verify: destination.isCurrent).rawValue
}

@_cdecl("logia_target_permission")
public func targetPermission(_ prompt: Bool) -> Bool {
    precondition(Thread.isMainThread)
    let allowed = prompt
        ? AXIsProcessTrustedWithOptions([kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true] as CFDictionary)
        : AXIsProcessTrusted()
    if allowed { TargetPreparation.start() }
    return allowed
}
