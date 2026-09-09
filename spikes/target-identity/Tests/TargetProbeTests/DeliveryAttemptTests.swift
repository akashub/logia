import Testing
@testable import TargetProbeCore

struct DeliveryAttemptTests {
    @Test func testWritesOnceAndNeverRetriesAmbiguousResult() {
        var attempt = DeliveryAttempt()
        var writes = 0
        #expect(attempt.send(text: "A paragraph.", verify: { .same }, write: { _ in writes += 1; return false }) == .uncertain)
        #expect(attempt.send(text: "A paragraph.", verify: { .same }, write: { _ in writes += 1; return true }) == .consumed)
        #expect(writes == 1)
    }

    @Test func testChangedUnknownAndCanceledTargetsNeverWrite() {
        for verdict in [TargetVerdict.changed, .unknown] {
            var attempt = DeliveryAttempt()
            #expect(attempt.send(text: "Words", verify: { verdict }, write: { _ in Issue.record("wrong target write"); return true }) == .copy)
        }
        var canceled = DeliveryAttempt()
        canceled.cancel()
        #expect(canceled.send(text: "Words", verify: { Issue.record("verify after cancel"); return .same }, write: { _ in false }) == .consumed)
    }

    @Test func testRejectsInvalidPayloadAndPassesValidUnicodeUnchanged() {
        for text in ["", " \n", "bad\0text", String(repeating: "x", count: 65_537)] {
            var attempt = DeliveryAttempt()
            #expect(attempt.send(text: text, verify: { Issue.record("invalid payload verified"); return .same }, write: { _ in false }) == .invalid)
        }
        var attempt = DeliveryAttempt()
        let text = "Hello, café.\nAnother thought."
        var written = ""
        let result = attempt.send(text: text, verify: { .same }, write: { written = $0; return true })
        #expect(result == .sent)
        #expect(written == text)
    }
}
