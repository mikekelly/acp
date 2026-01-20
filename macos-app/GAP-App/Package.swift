// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "GAP",
    platforms: [.macOS(.v13)],
    targets: [
        .executableTarget(
            name: "GAP",
            path: "Sources"
        )
    ]
)
