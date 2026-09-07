// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "LogiaTargetIdentity",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "target-probe", targets: ["TargetProbe"]),
        .executable(name: "target-fixture", targets: ["TargetFixture"]),
    ],
    targets: [
        .target(name: "TargetProbeCore"),
        .executableTarget(name: "TargetProbe", dependencies: ["TargetProbeCore"]),
        .executableTarget(name: "TargetFixture"),
        .testTarget(name: "TargetProbeTests", dependencies: ["TargetProbeCore"]),
    ]
)
