import Testing
@testable import TargetProbeCore

struct TargetComparisonTests {
    @Test func sameWindowFieldSwitchIsNeverSame() {
        #expect(compare(process: true, window: true, field: false) == .changed)
    }

    @Test func unavailableIdentityIsUnknown() {
        for evidence: (Bool?, Bool?, Bool?) in [(nil, true, true), (true, nil, true), (true, true, nil)] {
            #expect(compare(process: evidence.0, window: evidence.1, field: evidence.2) == .unknown)
        }
    }

    @Test func allThreeIdentitiesMustMatch() {
        #expect(compare(process: true, window: true, field: true) == .same)
        #expect(compare(process: false, window: true, field: true) == .changed)
        #expect(compare(process: true, window: false, field: true) == .changed)
    }

    @Test func knownChangeRemainsChangedWhenAnotherReadFails() {
        #expect(compare(process: false, window: nil, field: nil) == .changed)
        #expect(compare(process: nil, window: true, field: false) == .changed)
    }
}
