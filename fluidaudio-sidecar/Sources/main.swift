import Foundation
import FluidAudio

struct SpeakerSegment: Codable {
    let speakerId: String
    let startTime: Double
    let endTime: Double
    let text: String
}

struct FluidAudioOutputJSON: Codable {
    let text: String
    let speakerCount: Int
    let model: String
    let segments: [SpeakerSegment]
}

func writeError(_ message: String) -> Never {
    FileHandle.standardError.write(Data("Error: \(message)\n".utf8))
    exit(1)
}

/// Merge ASR word timings with diarization segments.
/// Each word is assigned to the speaker whose diarization segment overlaps the word's midpoint.
/// Returns (segments, speakerCount) where speakerCount reflects speakers present in the output.
func mergeAsrWithDiarization(
    tokenTimings: [TokenTiming],
    fullText: String,
    diarizationSegments: [TimedSpeakerSegment]
) -> ([SpeakerSegment], Int) {
    guard !diarizationSegments.isEmpty else {
        // No diarization — return single segment with full text
        let endTime = tokenTimings.last?.endTime ?? 0
        return ([SpeakerSegment(
            speakerId: "Speaker 1",
            startTime: 0,
            endTime: endTime,
            text: fullText
        )], 1)
    }

    guard !tokenTimings.isEmpty else {
        // No word timings — assign full text to first speaker
        return ([SpeakerSegment(
            speakerId: "Speaker 1",
            startTime: Double(diarizationSegments.first?.startTimeSeconds ?? 0),
            endTime: Double(diarizationSegments.last?.endTimeSeconds ?? 0),
            text: fullText
        )], Set(diarizationSegments.map { $0.speakerId }).count)
    }

    // Assign each word to a speaker based on its midpoint time
    struct WordWithSpeaker {
        let token: String
        let startTime: Double
        let endTime: Double
        let speakerId: String
    }

    var assignedWords: [WordWithSpeaker] = []
    let defaultSpeaker = diarizationSegments.first?.speakerId ?? "Speaker 1"

    for timing in tokenTimings {
        let midpoint = (timing.startTime + timing.endTime) / 2.0

        // Find the diarization segment that contains this word's midpoint
        var matchedSpeaker = defaultSpeaker
        for seg in diarizationSegments {
            let start = Double(seg.startTimeSeconds)
            let end = Double(seg.endTimeSeconds)
            if midpoint >= start && midpoint <= end {
                matchedSpeaker = seg.speakerId
                break
            }
        }

        assignedWords.append(WordWithSpeaker(
            token: timing.token,
            startTime: timing.startTime,
            endTime: timing.endTime,
            speakerId: matchedSpeaker
        ))
    }

    // Group consecutive words by the same speaker into segments
    var segments: [SpeakerSegment] = []
    var currentSpeaker = ""
    var currentWords: [String] = []
    var segmentStart: Double = 0
    var segmentEnd: Double = 0

    for word in assignedWords {
        if word.speakerId != currentSpeaker {
            // Flush previous segment
            if !currentWords.isEmpty {
                segments.append(SpeakerSegment(
                    speakerId: currentSpeaker,
                    startTime: segmentStart,
                    endTime: segmentEnd,
                    text: currentWords.joined(separator: " ").trimmingCharacters(in: .whitespaces)
                ))
            }
            currentSpeaker = word.speakerId
            currentWords = [word.token]
            segmentStart = word.startTime
            segmentEnd = word.endTime
        } else {
            currentWords.append(word.token)
            segmentEnd = word.endTime
        }
    }

    // Flush last segment
    if !currentWords.isEmpty {
        segments.append(SpeakerSegment(
            speakerId: currentSpeaker,
            startTime: segmentStart,
            endTime: segmentEnd,
            text: currentWords.joined(separator: " ").trimmingCharacters(in: .whitespaces)
        ))
    }

    // Normalize speaker IDs to "Speaker 1", "Speaker 2", etc.
    let uniqueSpeakers = Array(Set(segments.map { $0.speakerId })).sorted()
    var speakerMap: [String: String] = [:]
    for (i, id) in uniqueSpeakers.enumerated() {
        speakerMap[id] = "Speaker \(i + 1)"
    }

    let normalizedSegments = segments.map { seg in
        SpeakerSegment(
            speakerId: speakerMap[seg.speakerId] ?? seg.speakerId,
            startTime: seg.startTime,
            endTime: seg.endTime,
            text: seg.text
        )
    }

    // Count speakers actually present in merged output
    let outputSpeakerCount = Set(normalizedSegments.map { $0.speakerId }).count

    return (normalizedSegments, outputSpeakerCount)
}

// MARK: - Main

@main
struct FluidAudioSidecar {
    static func main() async {
        guard CommandLine.arguments.count >= 2 else {
            writeError("Usage: fluidaudio-sidecar <path-to-wav>")
        }

        let wavPath = CommandLine.arguments[1]
        let fileURL = URL(fileURLWithPath: wavPath)

        guard FileManager.default.fileExists(atPath: wavPath) else {
            writeError("File not found: \(wavPath)")
        }

        do {
            // Step 1: Initialize ASR
            let asrModels = try await AsrModels.downloadAndLoad(version: .v3)
            let asrManager = AsrManager(config: .default)
            try await asrManager.initialize(models: asrModels)

            // Step 2: Initialize offline diarizer
            let diarizerConfig = OfflineDiarizerConfig()
            let diarizerManager = OfflineDiarizerManager(config: diarizerConfig)
            try await diarizerManager.prepareModels()

            // Step 3: Run ASR
            let asrResult = try await asrManager.transcribe(fileURL, source: .system)

            // Step 4: Run diarization
            let diarizationResult = try await diarizerManager.process(fileURL)

            // Step 5: Merge results
            let timings = asrResult.tokenTimings ?? []
            let (segments, speakerCount) = mergeAsrWithDiarization(
                tokenTimings: timings,
                fullText: asrResult.text,
                diarizationSegments: diarizationResult.segments
            )

            let output = FluidAudioOutputJSON(
                text: asrResult.text,
                speakerCount: speakerCount,
                model: "parakeet-tdt-v3",
                segments: segments
            )

            let encoder = JSONEncoder()
            encoder.outputFormatting = [.sortedKeys]
            let json = try encoder.encode(output)

            FileHandle.standardOutput.write(json)
            FileHandle.standardOutput.write(Data("\n".utf8))

            asrManager.cleanup()
        } catch {
            writeError(error.localizedDescription)
        }
    }
}
