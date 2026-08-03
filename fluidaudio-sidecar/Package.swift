// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "fluidaudio-sidecar",
    platforms: [.macOS("15.0")],
    dependencies: [
        .package(url: "https://github.com/FluidInference/FluidAudio.git", from: "0.14.7"),
    ],
    targets: [
        // senko's Kaldi-style fbank (CAM++ input) — vendored C++ verbatim.
        .target(
            name: "SenkoFbank",
            path: "Sources/SenkoFbank",
            cxxSettings: [
                .headerSearchPath("include"),
                .headerSearchPath("."),
                .headerSearchPath("feature"),
            ]
        ),
        .executableTarget(
            name: "fluidaudio-sidecar",
            dependencies: [
                .product(name: "FluidAudio", package: "FluidAudio"),
                "SenkoFbank",
            ],
            path: "Sources",
            exclude: ["SenkoFbank"]
        ),
    ],
    cxxLanguageStandard: .cxx20
)
