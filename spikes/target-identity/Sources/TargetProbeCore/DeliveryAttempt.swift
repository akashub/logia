import Foundation

public enum DeliveryResult: Int32 { case sent = 0, copy = 1, uncertain = 2, consumed = 3, invalid = 4 }

public struct DeliveryAttempt {
    private var consumed = false
    public init() {}
    public mutating func cancel() { consumed = true }
    public mutating func send(text: String, verify: () -> TargetVerdict, write: (String) -> Bool) -> DeliveryResult {
        guard !consumed else { return .consumed }
        consumed = true // Includes failed or ambiguous writes. Never retry.
        guard !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              !text.utf8.contains(0), text.utf8.count <= 65_536 else { return .invalid }
        guard verify() == .same else { return .copy }
        return write(text) ? .sent : .uncertain
    }
}
