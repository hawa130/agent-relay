// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "RelayMacOS",
    platforms: [
        .macOS(.v15)
    ],
    products: [
        .library(
            name: "AgentRelayUI",
            targets: ["AgentRelayUI"]),
        .executable(
            name: "AgentRelay",
            targets: ["AgentRelay"])
    ],
    dependencies: [
        .package(url: "https://github.com/sindresorhus/Defaults", from: "9.0.0"),
        .package(url: "https://github.com/sindresorhus/LaunchAtLogin-Modern", from: "1.1.0")
    ],
    targets: [
        .target(
            name: "AgentRelayUI",
            dependencies: [
                "Defaults",
                .product(name: "LaunchAtLogin", package: "LaunchAtLogin-Modern")
            ],
            path: "RelayApp",
            exclude: ["Resources/README.md"],
            resources: [
                .process("Resources")
            ]),
        .executableTarget(
            name: "AgentRelay",
            dependencies: [
                "AgentRelayUI",
                "Defaults"
            ],
            path: "RelayAppExecutable",
            exclude: ["AppIcon.icon", "Resources/Info.plist"]),
        .testTarget(
            name: "RelayMacOSTests",
            dependencies: ["AgentRelayUI"],
            path: "Tests/RelayMacOSTests")
    ])
