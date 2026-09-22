import Testing
@testable import TargetProbeCore

struct PasteAttemptTests {
    @Test func clipboardOrFocusChangesNeverDispatch() {
        for change in ["before", "during-stage", "focus", "initial-focus"] {
            var attempt = DeliveryAttempt()
            var version = change == "before" ? 8 : 7, staged = 0, checks = 0, posts = 0
            let result = attempt.paste(text: "Words", expectedClipboard: 7,
                clipboard: { version }, stage: { _ in staged += 1; version = change == "during-stage" ? 9 : 8; return 8 },
                verify: { checks += 1; return change != "initial-focus" && (change != "focus" || checks == 1) }, dispatch: { posts += 1 })
            #expect(posts == 0)
            #expect(staged == (change == "before" ? 0 : 1))
            #expect(result == (change.contains("focus") ? .copied : .clipboardChanged))
        }
    }
    @Test func failedClipboardStagingDoesNotRequestAnAutomaticRetry() {
        var attempt = DeliveryAttempt()
        #expect(attempt.paste(text: "Words", expectedClipboard: 7, clipboard: { 7 },
            stage: { _ in nil }, verify: { true }, dispatch: { Issue.record("dispatched without text") }) == .copyRequired)
    }
    @Test func dispatchIsOneShotAndDoesNotClaimReceipt() {
        var attempt = DeliveryAttempt(), version = 7, posts = 0
        let result = attempt.paste(text: "Hello, café.", expectedClipboard: 7, clipboard: { version },
            stage: { text in #expect(text == "Hello, café."); version = 8; return version },
            verify: { true }, dispatch: { posts += 1 })
        #expect(result == .dispatched)
        #expect(attempt.paste(text: "Duplicate", expectedClipboard: 8, clipboard: { version },
            stage: { _ in Issue.record("restaged"); return 9 }, verify: { true }, dispatch: { posts += 1 }) == .consumed)
        #expect(posts == 1)
    }
    @Test func canceledAndInvalidPastesNeverTouchClipboard() {
        for text in ["", " \n", "bad\0text", String(repeating: "x", count: 65_537)] {
            var attempt = DeliveryAttempt()
            #expect(attempt.paste(text: text, expectedClipboard: 0, clipboard: { 0 },
                stage: { _ in Issue.record("invalid staged"); return 1 }, verify: { true }, dispatch: {}) == .invalid)
        }
        var attempt = DeliveryAttempt(); attempt.cancel()
        #expect(attempt.paste(text: "Words", expectedClipboard: 0, clipboard: { 0 },
            stage: { _ in Issue.record("canceled staged"); return 1 }, verify: { true }, dispatch: {}) == .consumed)
    }
}
