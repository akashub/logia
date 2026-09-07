import Foundation

public struct ProbeRecord: Codable {
    public let schemaVersion = 1
    public let caseName: String
    public let verdict: TargetVerdict
    public let reason: String
    public let elapsedMS: Int

    enum CodingKeys: String, CodingKey {
        case schemaVersion = "schema_version", caseName = "case", verdict, reason
        case elapsedMS = "elapsed_ms"
    }

    public init(caseName: String, verdict: TargetVerdict, reason: String, elapsedMS: Int) {
        self.caseName = caseName
        self.verdict = verdict
        self.reason = reason
        self.elapsedMS = elapsedMS
    }

    public func writeNew(to url: URL) throws {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        try encoder.encode(self).write(to: url, options: .withoutOverwriting)
    }
}
