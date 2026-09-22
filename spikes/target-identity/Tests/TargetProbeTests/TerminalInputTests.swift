import Testing
@testable import TargetProbeCore

struct TerminalInputTests {
    @Test func terminalsAreExplicitlyIdentified() {
        #expect(TargetEngine.identify(bundle: "com.apple.Terminal", electronFramework: false) == .terminal)
        #expect(TargetEngine.identify(bundle: "com.apple.Terminalish", electronFramework: false) == .native)
    }
    @Test func terminalPasteCannotContainSubmissionOrControlCharacters() {
        #expect(TerminalInput.allows("Please explain this error — café."))
        for text in ["first\nsecond", "first\rsecond", "\tcomplete", "escape\u{1b}[A", "erase\u{7f}", "paragraph\u{2028}next"] {
            #expect(!TerminalInput.allows(text))
        }
    }
}
