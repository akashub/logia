import AppKit
import AVFoundation

@_cdecl("logia_microphone_status")
public func microphoneStatus() -> Int32 {
    switch AVCaptureDevice.authorizationStatus(for: .audio) {
    case .notDetermined: return 0
    case .restricted: return 1
    case .denied: return 2
    case .authorized: return 3
    @unknown default: return 4
    }
}

// Permission requests do not create an audio session or start capture.
@_cdecl("logia_microphone_request")
public func microphoneRequest(_ completion: @escaping @convention(c) (Int32, UnsafeMutableRawPointer?) -> Void,
                              _ context: UnsafeMutableRawPointer?) {
    if microphoneStatus() != 0 { completion(microphoneStatus(), context); return }
    AVCaptureDevice.requestAccess(for: .audio) { _ in
        completion(microphoneStatus(), context)
    }
}

@_cdecl("logia_permission_settings")
public func permissionSettings(_ microphone: Bool) -> Bool {
    precondition(Thread.isMainThread)
    let pane = microphone ? "Privacy_Microphone" : "Privacy_Accessibility"
    guard let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?\(pane)") else { return false }
    return NSWorkspace.shared.open(url)
}
