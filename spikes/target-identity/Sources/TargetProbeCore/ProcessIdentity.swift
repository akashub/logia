import Darwin

// Kernel start time distinguishes PID reuse without relying on Launch Services
// registration (standalone helper processes may have no NSRunningApplication date).
struct ProcessIdentity: Equatable {
    let pid: pid_t
    let startedSeconds: UInt64
    let startedMicroseconds: UInt64

    static func read(_ pid: pid_t) -> ProcessIdentity? {
        guard pid > 0 else { return nil }
        var info = proc_bsdinfo()
        let size = Int32(MemoryLayout<proc_bsdinfo>.size)
        guard proc_pidinfo(pid, PROC_PIDTBSDINFO, 0, &info, size) == size,
              info.pbi_start_tvsec > 0, info.pbi_start_tvusec < 1_000_000 else { return nil }
        return ProcessIdentity(pid: pid, startedSeconds: info.pbi_start_tvsec,
                               startedMicroseconds: info.pbi_start_tvusec)
    }
}
