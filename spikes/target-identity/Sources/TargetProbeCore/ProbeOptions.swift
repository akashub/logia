import Foundation

public struct ProbeOptions {
    public let caseName: String
    public let captureDelayMS: Int
    public let recheckDelayMS: Int
    public let output: URL

    public enum Invalid: Error { case arguments }

    public static func parse(_ arguments: [String]) throws -> ProbeOptions {
        let allowed = Set(["--case", "--capture-delay-ms", "--recheck-delay-ms", "--output"])
        guard arguments.count.isMultiple(of: 2) else { throw Invalid.arguments }
        var values: [String: String] = [:]
        for index in stride(from: 0, to: arguments.count, by: 2) {
            let key = arguments[index]
            guard allowed.contains(key), values[key] == nil else { throw Invalid.arguments }
            values[key] = arguments[index + 1]
        }
        guard let name = values["--case"], !name.isEmpty, name.utf8.count <= 64,
              name.utf8.allSatisfy({ (97...122).contains($0) || (48...57).contains($0) || $0 == 45 }),
              let path = values["--output"], !path.isEmpty,
              let capture = Int(values["--capture-delay-ms"] ?? "3000"),
              let recheck = Int(values["--recheck-delay-ms"] ?? "3000"),
              (0...60000).contains(capture), (0...60000).contains(recheck)
        else { throw Invalid.arguments }
        return ProbeOptions(caseName: name, captureDelayMS: capture, recheckDelayMS: recheck,
                            output: URL(fileURLWithPath: path))
    }
}
