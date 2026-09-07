import Foundation
import Testing
@testable import TargetProbeCore

struct ProcessIdentityTests {
    @Test func currentProcessHasStableKernelIdentity() throws {
        let first = try #require(ProcessIdentity.read(ProcessInfo.processInfo.processIdentifier))
        let second = try #require(ProcessIdentity.read(ProcessInfo.processInfo.processIdentifier))
        #expect(first == second)
    }

    @Test func invalidProcessDoesNotBecomeAnIdentity() {
        #expect(ProcessIdentity.read(-1) == nil)
        #expect(ProcessIdentity.read(0) == nil)
        #expect(ProcessIdentity.read(Int32.max) == nil)
    }
}
