import Foundation

public enum DeliveryResult: Int32 {
    case sent = 0, copy = 1, uncertain = 2, consumed = 3, invalid = 4, dispatched = 5, clipboardChanged = 6
    case copied = 7, copyRequired = 8
}

public struct DeliveryAttempt {
    private var consumed = false
    public init() {}
    public mutating func cancel() { consumed = true }
    public mutating func paste(text: String, expectedClipboard: Int, clipboard: () -> Int,
                               stage: (String) -> Int?, verify: () -> Bool, dispatch: () -> Void) -> DeliveryResult {
        guard !consumed else { return .consumed }
        consumed = true
        guard !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              !text.utf8.contains(0), text.utf8.count <= 65_536 else { return .invalid }
        let initiallyFocused = verify()
        guard clipboard() == expectedClipboard else { return .clipboardChanged }
        guard let ownedVersion = stage(text) else { return .copyRequired }
        let focused = initiallyFocused && verify()
        guard clipboard() == ownedVersion else { return .clipboardChanged }
        // Recovery already owns the staged text. Never ask an asynchronous UI
        // fallback to write it again after this ownership check.
        guard focused else { return .copied }
        dispatch()
        return .dispatched // A posted key is not a receipt from the destination.
    }
    public mutating func send(text: String, verify: () -> TargetVerdict, write: (String) -> Bool) -> DeliveryResult {
        guard !consumed else { return .consumed }
        consumed = true // Includes failed or ambiguous writes. Never retry.
        guard !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              !text.utf8.contains(0), text.utf8.count <= 65_536 else { return .invalid }
        guard verify() == .same else { return .copy }
        return write(text) ? .sent : .uncertain
    }
}
