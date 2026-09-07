import Foundation
import Testing
@testable import TargetProbeCore

struct ProbeOptionsTests {
    @Test func parsesBoundedDelaysAndCase() throws {
        let options = try ProbeOptions.parse(["--case", "field-switch", "--capture-delay-ms", "0", "--recheck-delay-ms", "60000", "--output", "/tmp/result.json"])
        #expect(options.caseName == "field-switch")
        #expect(options.captureDelayMS == 0)
        #expect(options.recheckDelayMS == 60000)
    }

    @Test func rejectsAmbiguousOrUnboundedArguments() {
        let invalid: [[String]] = [
            [], ["--case", "field-switch"],
            ["--case", "private window title", "--output", "/tmp/result.json"],
            ["--case", "a", "--output", "/tmp/result.json", "--capture-delay-ms", "-1"],
            ["--case", "a", "--output", "/tmp/result.json", "--recheck-delay-ms", "60001"],
            ["--case", "a", "--case", "b", "--output", "/tmp/result.json"],
            ["--case", "a", "--output", "/tmp/result.json", "--unknown", "1"],
        ]
        for arguments in invalid {
            #expect(throws: (any Error).self) { try ProbeOptions.parse(arguments) }
        }
    }

    @Test func evidenceCannotOverwriteExistingFile() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let output = directory.appendingPathComponent("result.json")
        try Data("preserve".utf8).write(to: output)
        let record = ProbeRecord(caseName: "test", verdict: .unknown, reason: "identity-unavailable", elapsedMS: 0)
        #expect(throws: (any Error).self) { try record.writeNew(to: output) }
        #expect(try String(contentsOf: output, encoding: .utf8) == "preserve")
    }

    @Test func evidenceContainsOnlyDeclaredMetadata() throws {
        let record = ProbeRecord(caseName: "test", verdict: .changed, reason: "identity-changed", elapsedMS: 12)
        let data = try JSONEncoder().encode(record)
        let object = try #require(JSONSerialization.jsonObject(with: data) as? [String: Any])
        #expect(Set(object.keys) == ["schema_version", "case", "verdict", "reason", "elapsed_ms"])
        #expect(object["verdict"] as? String == "changed")
    }
}
