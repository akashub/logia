import Testing
@testable import TargetProbeCore

struct TargetEngineTests {
    @Test func enginePolicyDoesNotTreatEveryAppAsABrowser() {
        #expect(TargetEngine.identify(bundle: "com.google.Chrome", electronFramework: false) == .chromium)
        #expect(TargetEngine.identify(bundle: "com.google.Chrome.canary", electronFramework: false) == .chromium)
        #expect(TargetEngine.identify(bundle: "com.microsoft.VSCode", electronFramework: true) == .electron)
        #expect(TargetEngine.identify(bundle: "com.google.Chromeish", electronFramework: false) == .native)
        #expect(TargetEngine.identify(bundle: "com.apple.TextEdit", electronFramework: false) == .native)
    }
    @Test func activationIsBoundedAndDoesNotRestartDebounce() {
        var throttle = PreparationThrottle()
        let first = ProcessIdentity(pid: 1, startedSeconds: 1, startedMicroseconds: 0)
        let claims = [throttle.claim(first, now: 0), throttle.claim(first, now: 1),
                      throttle.claim(first, now: 2.4), throttle.claim(first, now: 11)]
        #expect(claims == [true, false, false, true])
        let reusedPID = ProcessIdentity(pid: 1, startedSeconds: 2, startedMicroseconds: 0)
        #expect(throttle.claim(reusedPID, now: 12) == true)
        for pid in 2...34 { #expect(throttle.claim(ProcessIdentity(pid: Int32(pid), startedSeconds: 1, startedMicroseconds: 0), now: 13) == true) }
        #expect(throttle.claim(reusedPID, now: 14) == true, "old identities are evicted rather than retained without a bound")
    }
}
