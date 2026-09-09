import AppKit
import ApplicationServices

// Rust dispatches every entry to the main thread. No Swift object crosses the ABI.
private var sequence: UInt64 = 0
private var current: (token: UInt64, snapshot: FocusSnapshot, selection: CFRange)?

@_cdecl("logia_target_capture")
public func targetCapture(_ status: UnsafeMutablePointer<Int32>) -> UInt64 {
    precondition(Thread.isMainThread)
    current = nil
    status.pointee = 1
    guard FocusSnapshot.hasPermission else { status.pointee = 2; return 0 }
    guard let snapshot = try? FocusSnapshot.capture(), !snapshot.belongsToCurrentProcess,
          let selection = try? snapshot.selection() else { return 0 }
    sequence &+= 1
    if sequence == 0 { sequence = 1 }
    current = (sequence, snapshot, selection)
    status.pointee = 0
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
    return target.snapshot.send(text, selection: target.selection).rawValue
}

@_cdecl("logia_target_permission")
public func targetPermission(_ prompt: Bool) -> Bool {
    precondition(Thread.isMainThread)
    if !prompt { return AXIsProcessTrusted() }
    return AXIsProcessTrustedWithOptions([kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true] as CFDictionary)
}
