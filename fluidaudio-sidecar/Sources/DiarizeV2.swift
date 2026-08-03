import CoreML
import Foundation
import FluidAudio
import SenkoFbank

// Self-contained speaker diarization, faithful to senko: dense 1.5s/0.6s
// subsegments → vendored Kaldi fbank (SenkoFbank C++) → CAM++ CoreML (192-d)
// → ported spectral+eigenvalue-gap clustering. No Python, no UMAP.

struct DiarV2Segment: Codable {
    let start: Double
    let end: Double
    let speaker: Int
    var text: String = ""
}

struct DiarV2Output: Codable {
    let speakerCount: Int
    let segments: [DiarV2Segment]
    let centroids: [[Float]]  // per speaker id, 192-d L2-normalized (for cross-recording / identity)
    /// Split mode: the speaker id that is the local mic ("me"); nil for mix mode.
    var meSpeaker: Int? = nil
}

/// Assign each ASR token to the speaker whose diarization segment covers its
/// midpoint, then group consecutive same-speaker tokens into text segments.
/// Mirrors mergeAsrWithDiarization but on our DiarV2Segment speaker timeline.
func mergeTokensWithSpeakers(
    tokenTimings: [TokenTiming],
    diar: [DiarV2Segment]
) -> [DiarV2Segment] {
    guard !diar.isEmpty, !tokenTimings.isEmpty else { return diar }
    // Resolve BOTH the speaker and the diar-segment index, so text blocks break
    // on segment boundaries (pauses) too — otherwise a single-speaker channel
    // (split mode) collapses into one giant unreadable block.
    func locate(at t: Double) -> (spk: Int, seg: Int) {
        for (i, s) in diar.enumerated() where t >= s.start && t <= s.end {
            return (s.speaker, i)
        }
        var bestI = 0
        var bestD = Double.greatestFiniteMagnitude
        for (i, s) in diar.enumerated() {
            let d = abs((s.start + s.end) / 2 - t)
            if d < bestD {
                bestD = d
                bestI = i
            }
        }
        return (diar[bestI].speaker, bestI)
    }
    var out: [DiarV2Segment] = []
    var curSpk = -1
    var curSeg = -1
    var curTokens: [String] = []
    var segStart = 0.0
    var segEnd = 0.0
    var prevEnd = 0.0
    for tk in tokenTimings {
        let mid = (tk.startTime + tk.endTime) / 2.0
        // A speaker/segment switch is only allowed on a WORD boundary —
        // Parakeet marks word starts with a leading space (" Так"; continuations
        // are bare "жа"/"вид"). Otherwise sub-word tokens straddling a boundary
        // get split across speakers ("Д"/"жавид"). Punctuation-only tokens also
        // stick to the current block — they close the sentence just spoken.
        let isWordStart = tk.token.hasPrefix(" ") || tk.token.hasPrefix("\u{2581}")
        let hasWordChar = tk.token.contains { $0.isLetter || $0.isNumber }
        // A real pause (>2s) also breaks the block — keeps a long single-speaker
        // stretch (merge-bridged diar segments) readable instead of one wall.
        let pauseBreak = curSpk != -1 && isWordStart && hasWordChar
            && tk.startTime - prevEnd > 2.0
        let (spk, seg): (Int, Int) =
            (curSpk != -1 && (!isWordStart || !hasWordChar)) ? (curSpk, curSeg) : locate(at: mid)
        if spk != curSpk || seg != curSeg || pauseBreak {
            if !curTokens.isEmpty {
                out.append(DiarV2Segment(
                    start: segStart, end: segEnd, speaker: curSpk, text: joinTokens(curTokens)))
            }
            curSpk = spk
            curSeg = seg
            curTokens = [tk.token]
            segStart = tk.startTime
            segEnd = tk.endTime
        } else {
            curTokens.append(tk.token)
            segEnd = tk.endTime
        }
        prevEnd = tk.endTime
    }
    if !curTokens.isEmpty {
        out.append(DiarV2Segment(
            start: segStart, end: segEnd, speaker: curSpk, text: joinTokens(curTokens)))
    }
    return out
}

// senko normal mode.
private let subWindowSec = 1.5
private let subShiftSec = 0.6
private let sampleRate = 16_000
private let camppFrames = 150
private let camppBatch = 16
private let melBins = 80
private let embDim = 192

/// Resolve a diarization model: env override → bundled next to the sidecar →
/// dev build dir → senko-clone fallback. Keeps the sidecar self-contained.
/// Resolve a diarization model. Order: env override (the app's Rust side sets
/// these from its resource dir / dev tree) → next to the sidecar binary →
/// the repo Models dir (CLI runs from .build/release). NO home-dir fallback:
/// a missing model must fail loudly, not silently depend on a dev machine.
private func resolveModel(env: String, names: [String]) -> String? {
    let fm = FileManager.default
    if let p = ProcessInfo.processInfo.environment[env], fm.fileExists(atPath: p) {
        return p
    }
    let exeDir = URL(fileURLWithPath: CommandLine.arguments.first ?? "")
        .resolvingSymlinksInPath().deletingLastPathComponent()
    for name in names {
        let candidates = [
            exeDir.appendingPathComponent("Models/\(name)"),         // staged next to binary
            exeDir.appendingPathComponent("../Models/\(name)"),
            exeDir.appendingPathComponent("../../Models/\(name)"),   // .build/release → repo Models
            exeDir.appendingPathComponent("../../../Models/\(name)"), // .build/arm64…/release (symlink-resolved)
        ]
        for c in candidates where fm.fileExists(atPath: c.path) { return c.path }
    }
    return nil
}

/// Prefer the precompiled .mlmodelc (no runtime MLModel.compileModel cost).
private func camppPath() -> String? {
    resolveModel(
        env: "NBP_CAMPP",
        names: ["camplusplus_batch16.mlmodelc", "camplusplus_batch16.mlpackage"])
}

private func pyannotePath() -> String? {
    resolveModel(env: "NBP_PYANNOTE", names: ["pyannote_segmentation.mlmodelc"])
}

/// senko _merge_segments (diarizer.py:855) ported 1:1: (1) merge adjacent
/// same-speaker segments with gap ≤4s, (2) drop segments ≤0.78s, stitching
/// neighbours when they share a speaker.
private func mergeSenkoSegments(_ segs: [DiarV2Segment]) -> [DiarV2Segment] {
    guard !segs.isEmpty else { return [] }
    var merged: [DiarV2Segment] = []
    var cur = segs[0]
    for s in segs.dropFirst() {
        if cur.speaker == s.speaker && s.start - cur.end <= 4.0 {
            cur = DiarV2Segment(start: cur.start, end: s.end, speaker: cur.speaker)
        } else {
            merged.append(cur)
            cur = s
        }
    }
    merged.append(cur)

    var i = 0
    while i < merged.count {
        if merged[i].end - merged[i].start <= 0.78 {
            if i > 0, i < merged.count - 1, merged[i - 1].speaker == merged[i + 1].speaker {
                merged[i - 1] = DiarV2Segment(
                    start: merged[i - 1].start, end: merged[i + 1].end, speaker: merged[i - 1].speaker)
                merged.remove(at: i + 1)
            }
            merged.remove(at: i)
        } else {
            i += 1
        }
    }
    return merged
}

struct DiarPipelineError: Error, LocalizedError {
    let message: String
    var errorDescription: String? { message }
}

struct TimeRange {
    let start: Double
    let end: Double
}

/// Diarization-only result (no ASR text yet).
struct DiarPipelineResult {
    let segments: [DiarV2Segment]
    let centroids: [[Float]]
    let speakerCount: Int
    /// Raw VAD speech regions (pre-merge) — actual voice activity, unlike
    /// `segments` whose gaps are bridged by merge_segments.
    let speechRegions: [TimeRange]
}

/// The senko diarization pipeline (VAD → subsegments → fbank → CAM++ →
/// spectral → merge) as a reusable core. Throws instead of exiting so the
/// transcribe path can degrade gracefully; `progress` receives (stage, pct).
func runDiarPipeline(
    wavPath: String,
    progress: (String, Int) -> Void
) async throws -> DiarPipelineResult {
    progress("Loading CAM++", 0)
    guard let camppLocation = camppPath() else {
        throw DiarPipelineError(message: "CAM++ model not found (set NBP_CAMPP or stage Models/)")
    }
    let modelURL = URL(fileURLWithPath: camppLocation)
    // Precompiled .mlmodelc loads directly; .mlpackage needs a one-off compile.
    let compiledURL =
        camppLocation.hasSuffix(".mlmodelc")
        ? modelURL
        : try await MLModel.compileModel(at: modelURL)
    let campp = try MLModel(contentsOf: compiledURL)

    progress("Resampling", 8)
    let samples = try AudioConverter().resampleAudioFile(path: wavPath)
    let total = Double(samples.count) / Double(sampleRate)

    // pyannote VAD (senko's exact Swift/CoreML code) — subsegments only on
    // speech, so silence doesn't spawn phantom speakers or skew durations.
    progress("VAD", 12)
    guard let pyannoteLocation = pyannotePath() else {
        throw DiarPipelineError(message: "pyannote VAD model not found (set NBP_PYANNOTE or stage Models/)")
    }
    let vad = VADProcessor()
    guard vad.loadModel(at: pyannoteLocation) else {
        throw DiarPipelineError(message: "VAD model load failed")
    }
    let speech = samples.withUnsafeBufferPointer { bp in
        vad.processAudioSamples(bp.baseAddress!, sampleCount: samples.count)
    }

    // Dense subsegments within speech regions (seconds); C++ fbank ×16000.
    var starts: [Double] = []
    var subsegFlat: [Float] = []
    for seg in speech {
        var subStart = seg.start
        while subStart + subWindowSec < seg.end {
            starts.append(subStart)
            subsegFlat.append(Float(subStart))
            subsegFlat.append(Float(subStart + subWindowSec))
            subStart += subShiftSec
        }
        // senko tail (diarizer.py:538): cover the region's end, incl. regions
        // shorter than one window. Clamp start ≥0 for the C++ fbank.
        if subStart < seg.end {
            subStart = max(0, min(seg.end - subWindowSec, subStart))
            starts.append(subStart)
            subsegFlat.append(Float(subStart))
            subsegFlat.append(Float(seg.end))
        }
    }
    let nSub = starts.count
    if nSub < 2 { throw DiarPipelineError(message: "Too little speech to diarize") }

    progress("Fbank", 20)
    let handle = create_fbank_extractor()
    defer { destroy_fbank_extractor(handle) }
    var feats = samples.withUnsafeBufferPointer { sp in
        subsegFlat.withUnsafeMutableBufferPointer { ssp in
            extract_fbank_features_from_memory(
                handle, sp.baseAddress, samples.count, ssp.baseAddress, nSub)
        }
    }
    defer { free_fbank_features(&feats) }
    guard let data = feats.data, let framesPer = feats.frames_per_subsegment,
        let offsets = feats.subsegment_offsets
    else { throw DiarPipelineError(message: "fbank returned null") }
    let fdim = Int(feats.feature_dim)

    // Pad/truncate each subsegment to camppFrames×melBins, batch through CAM++.
    progress("Embedding", 30)
    var embeddings: [[Float]] = []
    embeddings.reserveCapacity(nSub)
    var b = 0
    while b < nSub {
        let n = min(camppBatch, nSub - b)
        let arr = try MLMultiArray(
            shape: [NSNumber(value: camppBatch), NSNumber(value: camppFrames),
                NSNumber(value: melBins)], dataType: .float32)
        let ptr = arr.dataPointer.bindMemory(
            to: Float.self, capacity: camppBatch * camppFrames * melBins)
        memset(ptr, 0, camppBatch * camppFrames * melBins * MemoryLayout<Float>.size)
        for j in 0..<n {
            let si = b + j
            let fr = Int(framesPer[si])
            let off = Int(offsets[si])
            let use = min(fr, camppFrames)
            for f in 0..<use {
                for m in 0..<melBins {
                    ptr[j * camppFrames * melBins + f * melBins + m] =
                        data[off + f * fdim + m]
                }
            }
        }
        let prov = try MLDictionaryFeatureProvider(dictionary: [
            "input_features": MLFeatureValue(multiArray: arr)
        ])
        let out = try await campp.prediction(from: prov)
        guard let embArr = out.featureValue(for: "embeddings")?.multiArrayValue else {
            throw DiarPipelineError(message: "CAM++ produced no embeddings")
        }
        let ep = embArr.dataPointer.bindMemory(to: Float.self, capacity: camppBatch * embDim)
        for j in 0..<n {
            var v = [Float](repeating: 0, count: embDim)
            for d in 0..<embDim { v[d] = ep[j * embDim + d] }
            embeddings.append(v)
        }
        b += n
        progress("Embedding", 30 + Int(Double(b) / Double(nSub) * 45))
    }

    progress("Clustering", 78)
    let labels = SpeakerClustering.cluster(embeddings: embeddings, starts: starts)

    var segs: [DiarV2Segment] = []
    if !labels.isEmpty {
        var cur = labels[0]
        var segStart = starts[0]
        for i in 1..<labels.count where labels[i] != cur {
            segs.append(DiarV2Segment(start: segStart, end: starts[i], speaker: cur))
            cur = labels[i]
            segStart = starts[i]
        }
        segs.append(DiarV2Segment(start: segStart, end: total, speaker: cur))
    }
    segs = mergeSenkoSegments(segs)

    // Per-speaker centroids (mean embedding, L2-normalized) for identity /
    // cross-recording matching (e.g. mix-vs-system to find "me").
    let uniqSpk = Set(labels).sorted()
    var centroids: [[Float]] = []
    for sp in uniqSpk {
        let idxs = labels.indices.filter { labels[$0] == sp }
        var c = [Float](repeating: 0, count: embDim)
        for i in idxs { for d in 0..<embDim { c[d] += embeddings[i][d] } }
        let inv = 1.0 / Float(max(1, idxs.count))
        for d in 0..<embDim { c[d] *= inv }
        var nrm: Float = 0
        for x in c { nrm += x * x }
        nrm = nrm.squareRoot() + 1e-9
        centroids.append(c.map { $0 / nrm })
    }

    return DiarPipelineResult(
        segments: segs, centroids: centroids, speakerCount: uniqSpk.count,
        speechRegions: speech.map { TimeRange(start: $0.start, end: $0.end) })
}

/// Load Parakeet v3 (cache-first) and transcribe a wav → token timings.
func runAsrPass(wavPath: String) async throws -> [TokenTiming] {
    let asrModels: AsrModels
    if let cached = try? await AsrModels.loadFromCache(version: .v3) {
        asrModels = cached
    } else {
        asrModels = try await AsrModels.downloadAndLoad(version: .v3)
    }
    let asr = AsrManager(config: .default)
    try await asr.loadModels(asrModels)
    var decoderState = TdtDecoderState.make(decoderLayers: await asr.decoderLayerCount)
    let result = try await asr.transcribe(
        URL(fileURLWithPath: wavPath), decoderState: &decoderState, language: nil)
    return result.tokenTimings ?? []
}

/// Source-split diarization for call recordings that kept both stems:
/// `raw_system` (remote voices, echo-free by design) is diarized normally;
/// `raw_mic` is the local speaker ("me") by channel identity. Each stem gets
/// its OWN ASR pass on clean audio — overlap survives, and no mix noise.
func runDiarizeV2Split(systemWav: String, micWav: String) async {
    let fm = FileManager.default
    guard fm.fileExists(atPath: systemWav), fm.fileExists(atPath: micWav) else {
        writeError("Split mode needs both system and mic wav paths")
    }
    do {
        // Remote speakers from the clean system stem.
        let sys = try await runDiarPipeline(wavPath: systemWav) { stage, pct in
            writeProgress(stage, Int(Double(pct) * 0.35))  // 0-35
        }

        // Local mic: diarize too, then ECHO-CHECK — without headphones the mic
        // hears the speakers playing the remote voices, so mic clusters whose
        // centroid matches a system speaker are echo, not "me". Keep the
        // dominant NON-echo cluster as the local speaker.
        func cos(_ a: [Float], _ b: [Float]) -> Float {
            var dot: Float = 0, na: Float = 0, nb: Float = 0
            for i in 0..<min(a.count, b.count) {
                dot += a[i] * b[i]
                na += a[i] * a[i]
                nb += b[i] * b[i]
            }
            return dot / (na.squareRoot() * nb.squareRoot() + 1e-9)
        }
        let echoThreshold: Float = 0.80
        let meId = sys.speakerCount
        var meSegments: [DiarV2Segment] = []
        var meCentroid: [Float] = []
        do {
            let mic = try await runDiarPipeline(wavPath: micWav) { stage, pct in
                writeProgress(stage, 35 + Int(Double(pct) * 0.2))  // 35-55
            }
            var dur: [Int: Double] = [:]
            for s in mic.segments { dur[s.speaker, default: 0] += s.end - s.start }
            // Drop echo clusters (acoustically identical to a remote speaker).
            var candidates: [Int] = []
            for (spk, _) in dur {
                guard spk < mic.centroids.count else { continue }
                let maxSys = sys.centroids.map { cos(mic.centroids[spk], $0) }.max() ?? 0
                if maxSys < echoThreshold {
                    candidates.append(spk)
                } else {
                    FileHandle.standardError.write(
                        Data("mic cluster \(spk) dropped as echo (cos \(maxSys))\n".utf8))
                }
            }
            if let dominant = candidates.max(by: { (dur[$0] ?? 0) < (dur[$1] ?? 0) }) {
                meSegments = mic.segments
                    .filter { $0.speaker == dominant }
                    .map { DiarV2Segment(start: $0.start, end: $0.end, speaker: meId) }
                meCentroid = mic.centroids[dominant]
            }

            // TIME-BASED echo guard on top of the cluster check: without
            // headphones a dense two-way call can fuse "me" and loud remote
            // echo into ONE mic cluster (mixed centroid slips under the cosine
            // threshold). Echo is, by definition, simultaneous with remote
            // speech — so trim every "me" segment to the parts where the
            // system channel is SILENT. Honest trade-off: local speech spoken
            // OVER a remote voice is dropped too (headphones make this moot).
            if !meSegments.isEmpty {
                let sysIntervals = sys.speechRegions.map { ($0.start, $0.end) }.sorted { $0.0 < $1.0 }
                var trimmed: [DiarV2Segment] = []
                for seg in meSegments {
                    var cursor = seg.start
                    for (s, e) in sysIntervals {
                        if e <= cursor { continue }
                        if s >= seg.end { break }
                        if s > cursor {
                            trimmed.append(
                                DiarV2Segment(start: cursor, end: min(s, seg.end), speaker: meId))
                        }
                        cursor = max(cursor, e)
                        if cursor >= seg.end { break }
                    }
                    if cursor < seg.end {
                        trimmed.append(DiarV2Segment(start: cursor, end: seg.end, speaker: meId))
                    }
                }
                // Slivers shorter than 0.5s are boundary noise, not utterances.
                meSegments = trimmed.filter { $0.end - $0.start >= 0.5 }
            }
        } catch {
            // A silent mic (listener-only call) must not kill the whole job.
            FileHandle.standardError.write(Data("mic diar skipped: \(error)\n".utf8))
        }

        // Per-stem ASR on CLEAN audio: token ownership is decided by channel,
        // not by acoustics — zero cross-speaker text bleed.
        writeProgress("Transcribing", 60)
        let sysTokens = try await runAsrPass(wavPath: systemWav)
        writeProgress("Transcribing", 75)
        let micTokens = meSegments.isEmpty ? [] : ((try? await runAsrPass(wavPath: micWav)) ?? [])
        writeProgress("Finalizing", 90)

        var segments = mergeTokensWithSpeakers(tokenTimings: sysTokens, diar: sys.segments)
        if !meSegments.isEmpty {
            // STRICT: only mic tokens inside a "me" segment (±0.3s) belong to the
            // local speaker — everything else the mic heard is speaker echo whose
            // text already comes from the clean system-stem ASR.
            let slack = 0.3
            let meTokens = micTokens.filter { tk in
                let mid = (tk.startTime + tk.endTime) / 2.0
                return meSegments.contains { mid >= $0.start - slack && mid <= $0.end + slack }
            }
            segments += mergeTokensWithSpeakers(tokenTimings: meTokens, diar: meSegments)
        }
        segments.sort { $0.start < $1.start }

        var centroids = sys.centroids
        centroids.append(meCentroid.isEmpty ? [Float](repeating: 0, count: embDim) : meCentroid)

        let outObj = DiarV2Output(
            speakerCount: sys.speakerCount + 1,
            segments: segments,
            centroids: centroids,
            meSpeaker: meId)
        writeProgress("Complete", 100)
        let enc = JSONEncoder()
        enc.outputFormatting = [.sortedKeys]
        let json = try enc.encode(outObj)
        FileHandle.standardOutput.write(json)
        FileHandle.standardOutput.write(Data("\n".utf8))
        try? FileHandle.standardOutput.close()
        try? FileHandle.standardError.close()
        exit(0)
    } catch {
        writeError(error.localizedDescription)
    }
}

/// Standalone `--diarize-v2` subcommand: diar pipeline + its own ASR pass for
/// "who said what" (used when a recording is diarized without/after transcribe).
func runDiarizeV2(wavPath: String) async {
    guard FileManager.default.fileExists(atPath: wavPath) else {
        writeError("File not found: \(wavPath)")
    }
    do {
        let diar = try await runDiarPipeline(wavPath: wavPath, progress: writeProgress)

        // ASR (Parakeet) + merge → text per speaker segment ("who said what").
        writeProgress("Transcribing", 82)
        let timings = try await runAsrPass(wavPath: wavPath)
        let textSegs = mergeTokensWithSpeakers(tokenTimings: timings, diar: diar.segments)

        let outObj = DiarV2Output(
            speakerCount: diar.speakerCount, segments: textSegs, centroids: diar.centroids)
        writeProgress("Complete", 100)
        let enc = JSONEncoder()
        enc.outputFormatting = [.sortedKeys]
        let json = try enc.encode(outObj)
        FileHandle.standardOutput.write(json)
        FileHandle.standardOutput.write(Data("\n".utf8))
        try? FileHandle.standardOutput.close()
        try? FileHandle.standardError.close()
        exit(0)
    } catch {
        writeError(error.localizedDescription)
    }
}
