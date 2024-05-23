// swift-tools-version: 5.9
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "PaxSwiftCartridge",
    platforms: [
        .macOS(.v12),
        .iOS(.v13)
    ],
    products: [
        .library(
            name: "PaxCartridge",
            targets: ["PaxCartridge", "PaxCartridgeAssets"]
        ),
        .library(
            name: "PaxCartridgeAssets",
            targets: ["PaxCartridgeAssets"]
        ),
    ],
    targets: [
        .binaryTarget(
            name: "PaxCartridge",
            path: "PaxCartridge.xcframework"
        ),
        .target(
            name: "PaxCartridgeAssets",
            resources: [.process("Resources")]
        )
    ]
)
