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
            name: "PaxSwiftCartridge",
            targets: ["PaxSwiftCartridge"]
        ),
    ],
    targets: [
        .binaryTarget(
            name: "PaxRustCartridge",
            path: "PaxRustCartridge.framework"
        ),
        .target(
            name: "PaxSwiftCartridge",
//            resources: [.process("Resources")],
            dependencies: ["PaxRustCartridge"]
        )
    ]
)
