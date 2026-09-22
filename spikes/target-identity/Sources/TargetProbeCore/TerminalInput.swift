import Foundation

enum TerminalInput {
    // Never submit a line or send terminal editor controls through dictation.
    // Keep unsupported text for explicit Copy rather than silently changing it.
    static func allows(_ text: String) -> Bool {
        text.rangeOfCharacter(from: .controlCharacters.union(.newlines)) == nil
    }
}
