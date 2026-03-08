// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "RelayMacOS",
    platforms: [
        .macOS(.v15),
    ],
    products: [
        .executable(
            name: "RelayMacOS",
            targets: ["RelayMacOS"]
        ),
    ],
    dependencies: [
        .package(url: "https://github.com/sindresorhus/Defaults", from: "9.0.0"),
        .package(url: "https://github.com/sindresorhus/LaunchAtLogin-Modern", from: "1.1.0"),
    ],
    targets: [
        .executableTarget(
            name: "RelayMacOS",
            dependencies: [
                "Defaults",
                .product(name: "LaunchAtLogin", package: "LaunchAtLogin-Modern"),
            ],
            path: "RelayApp",
            exclude: ["Resources/README.md", "Resources/Info.plist"]
        ),
    ]
)
