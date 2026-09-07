public enum TargetVerdict: String, Codable { case same, changed, unknown }

public func compare(process: Bool?, window: Bool?, field: Bool?) -> TargetVerdict {
    let evidence = [process, window, field]
    if evidence.contains(where: { $0 == false }) { return .changed }
    if evidence.contains(where: { $0 == nil }) { return .unknown }
    return .same
}
