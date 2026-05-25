// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "fluidaudio-sidecar",
    platforms: [.macOS("15.0")],
    dependencies: [
        .package(url: "https://github.com/FluidInference/FluidAudio.git", from: "0.14.7"),
    ],
    targets: [
        .executableTarget(
            name: "fluidaudio-sidecar",
            dependencies: [
                .product(name: "FluidAudio", package: "FluidAudio"),
            ],
            path: "Sources"
        ),
    ]
)
