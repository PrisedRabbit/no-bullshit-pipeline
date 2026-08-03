import Foundation
import Accelerate

// Speaker clustering ported from 3D-Speaker / senko (cluster_cpu.py): spectral
// clustering with automatic speaker-count via the eigenvalue gap — zero manual
// threshold.
//
// KNOWN DIVERGENCES from senko (deliberate, documented):
//  - senko switches to UMAP+HDBSCAN for audio ≥20 min; we run spectral for the
//    whole file (validated: identical speaker count on a 35-min 6-speaker call)
//    and only fall back to sliding windows + centroid stitching when the
//    subsegment count would make the O(N³) eigendecomposition expensive.
//  - senko splits speaker-change boundaries at the midpoint of overlapping
//    subsegments; we cut at the next subsegment start (boundaries can sit
//    ~0.45s earlier). ASR-token attribution smooths most of this out.
//  - senko renumbers speakers by total speaking time; we keep cluster order.
//    Stable identity lives in the uid+centroid layer above, not the index.
//
// Input: per-subsegment embeddings (any dim — WeSpeaker 256-d or CAM++ 192-d).
// Output: an integer speaker label per subsegment.

enum SpeakerClustering {

    // Tunables mirror senko's spectral.yaml — hardcoded (no user knobs).
    static let pval: Double = 0.012
    static let minPnum = 6
    static let minNumSpks = 1
    static let maxNumSpks = 15
    static let merCos = 0.875
    static let minClusterSize = 4
    // Cap on the spectral matrix size: N×N doubles + O(N³) dsyev. 4500 subsegs
    // ≈ 45 min of dense speech ≈ 160 MB matrix — fine; beyond that, window.
    static let maxSpectralN = 4500
    static let windowSeconds: Double = 1800      // 30-min windows when capped
    static let stitchCos = 0.80                  // cross-window same-speaker threshold

    // MARK: - Public entry

    /// Cluster `embeddings` (row per subsegment) with start times `starts` (seconds).
    /// Returns a label per subsegment. Handles any length via sliding windows.
    static func cluster(embeddings: [[Float]], starts: [Double]) -> [Int] {
        let n = embeddings.count
        if n < 10 { return Array(repeating: 0, count: n) }

        // Whole-file spectral while the eigendecomposition stays affordable;
        // the cap is on subsegment COUNT (the real cost), not wall-clock length.
        if n <= maxSpectralN {
            return postProcess(spectral(embeddings), embeddings)
        }
        return slidingWindow(embeddings: embeddings, starts: starts)
    }

    // MARK: - Spectral clustering (single window)

    /// senko path: cosine affinity → p-pruning → symmetrize → unnormalized
    /// Laplacian → eigh → eigenvalue-gap → KMeans on the top-k eigenvectors.
    static func spectral(_ X: [[Float]]) -> [Int] {
        let n = X.count
        if n < 2 { return Array(repeating: 0, count: n) }
        let Xn = l2normRows(X)                       // [n][d] double, L2-normalized
        var A = cosineMatrix(Xn)                      // [n*n] row-major
        pPrune(&A, n: n)
        symmetrize(&A, n: n)
        zeroDiagonal(&A, n: n)
        let L = laplacian(A, n: n)                    // unnormalized D - A
        guard let (vals, vecs) = eigh(L, n: n) else { // ascending eigenvalues
            // LAPACK failed to converge — degrade to one speaker rather than
            // clustering on garbage eigenvectors.
            FileHandle.standardError.write(Data("dsyev failed; single-speaker fallback\n".utf8))
            return Array(repeating: 0, count: n)
        }
        // Clamp to n: the gap search range can exceed the eigenvector count on
        // small windows, which would over-read `vecs` when slicing top-k.
        let k = min(speakerCountFromGap(vals), n)
        // spectral embedding = first k eigenvectors (columns) → [n][k]
        var emb = [[Double]](repeating: [Double](repeating: 0, count: k), count: n)
        for i in 0..<n { for c in 0..<k { emb[i][c] = vecs[i * n + c] } }
        return kmeans(emb, k: k)
    }

    /// The zero-friction core: pick k at the largest gap in the smallest
    /// eigenvalues (senko getEigenGaps / argmax + minNumSpks).
    static func speakerCountFromGap(_ vals: [Double]) -> Int {
        let lo = minNumSpks - 1
        let hi = min(maxNumSpks + 1, vals.count)
        if hi - lo < 2 { return 1 }
        let slice = Array(vals[lo..<hi])
        var gaps = [Double]()
        for i in 0..<(slice.count - 1) { gaps.append(slice[i + 1] - slice[i]) }
        var best = 0
        for i in 1..<gaps.count where gaps[i] > gaps[best] { best = i }
        return best + minNumSpks
    }

    // MARK: - Sliding window for long audio

    static func slidingWindow(embeddings: [[Float]], starts: [Double]) -> [Int] {
        // Partition subsegments into consecutive time windows.
        var windows: [[Int]] = []
        let t0 = starts.first ?? 0
        var cur: [Int] = []
        var winEnd = t0 + windowSeconds
        for i in 0..<embeddings.count {
            if starts[i] > winEnd && !cur.isEmpty {
                windows.append(cur); cur = []
                winEnd = starts[i] + windowSeconds
            }
            cur.append(i)
        }
        if !cur.isEmpty { windows.append(cur) }

        // Cluster each window; collect (globalLabel) with per-window centroids.
        var labels = [Int](repeating: 0, count: embeddings.count)
        var globalCentroids: [[Double]] = []          // centroid per global speaker
        for win in windows {
            let sub = win.map { embeddings[$0] }
            let local = postProcess(spectral(sub), sub)
            // map each local cluster to a global speaker by centroid cosine
            let locUniq = Array(Set(local)).sorted()
            var locCentroid: [Int: [Double]] = [:]
            for lc in locUniq {
                let idxs = local.enumerated().filter { $0.element == lc }.map { $0.offset }
                locCentroid[lc] = meanRows(idxs.map { l2norm(doubleRow(sub[$0])) })
            }
            var locToGlobal: [Int: Int] = [:]
            for lc in locUniq {
                let c = locCentroid[lc]!
                var bestG = -1; var bestS = stitchCos
                for (g, gc) in globalCentroids.enumerated() {
                    let s = cosine(c, gc)
                    if s > bestS { bestS = s; bestG = g }
                }
                if bestG >= 0 {
                    locToGlobal[lc] = bestG
                    // update running centroid (simple mean of two)
                    globalCentroids[bestG] = meanRows([globalCentroids[bestG], c])
                } else {
                    locToGlobal[lc] = globalCentroids.count
                    globalCentroids.append(c)
                }
            }
            for (j, gi) in win.enumerated() { labels[gi] = locToGlobal[local[j]]! }
        }
        return labels
    }

    // MARK: - Post-processing (senko: filter minor + merge_by_cos)

    static func postProcess(_ labels0: [Int], _ X: [[Float]]) -> [Int] {
        var labels = filterMinor(labels0, X)
        labels = mergeByCos(labels, X)
        return labels
    }

    static func filterMinor(_ labels: [Int], _ X: [[Float]]) -> [Int] {
        var out = labels
        let uniq = Array(Set(out)).sorted()
        let size = Dictionary(uniqueKeysWithValues: uniq.map { c in (c, out.filter { $0 == c }.count) })
        let minor = Set(uniq.filter { size[$0]! < minClusterSize })
        if minor.isEmpty { return out }
        let major = uniq.filter { size[$0]! >= minClusterSize }
        if major.isEmpty { return Array(repeating: 0, count: out.count) }
        let centers = major.map { c -> [Double] in
            meanRows(out.enumerated().filter { $0.element == c }.map { doubleRow(X[$0.offset]) })  // senko: raw mean (cosine normalizes)
        }
        for i in 0..<out.count where minor.contains(out[i]) {
            let x = doubleRow(X[i])  // senko: raw (cosine normalizes)
            var bg = 0; var bs = -2.0
            for (j, ctr) in centers.enumerated() { let s = cosine(x, ctr); if s > bs { bs = s; bg = j } }
            out[i] = major[bg]
        }
        return out
    }

    static func mergeByCos(_ labels: [Int], _ X: [[Float]]) -> [Int] {
        var out = labels
        while true {
            let uniq = Array(Set(out)).sorted()
            if uniq.count < 2 { break }
            let centers = uniq.map { c -> [Double] in
                meanRows(out.enumerated().filter { $0.element == c }.map { doubleRow(X[$0.offset]) })  // senko: raw mean (cosine normalizes)
            }
            var bi = -1, bj = -1; var best = merCos
            for a in 0..<uniq.count { for b in (a + 1)..<uniq.count {
                let s = cosine(centers[a], centers[b])
                if s >= best { best = s; bi = a; bj = b }
            } }
            if bi < 0 { break }
            let from = uniq[bj], to = uniq[bi]
            for i in 0..<out.count where out[i] == from { out[i] = to }
        }
        // renumber 0..k-1
        let uniq = Array(Set(out)).sorted()
        let remap = Dictionary(uniqueKeysWithValues: uniq.enumerated().map { ($1, $0) })
        return out.map { remap[$0]! }
    }

    // MARK: - KMeans (Lloyd)

    static func kmeans(_ X: [[Double]], k: Int, iters: Int = 100) -> [Int] {
        let n = X.count
        if k <= 1 || n <= k { return Array(0..<n).map { min($0, k - 1) } }
        // deterministic init: spread initial centroids across the data
        var centers = (0..<k).map { X[$0 * n / k] }
        var labels = [Int](repeating: 0, count: n)
        for _ in 0..<iters {
            var changed = false
            for i in 0..<n {
                var bj = 0; var bd = Double.greatestFiniteMagnitude
                for j in 0..<k {
                    var d = 0.0
                    for c in 0..<X[i].count { let e = X[i][c] - centers[j][c]; d += e * e }
                    if d < bd { bd = d; bj = j }
                }
                if labels[i] != bj { labels[i] = bj; changed = true }
            }
            for j in 0..<k {
                let members = (0..<n).filter { labels[$0] == j }
                if members.isEmpty { continue }
                centers[j] = meanRows(members.map { X[$0] })
            }
            if !changed { break }
        }
        return labels
    }

    // MARK: - Linear algebra helpers

    static func doubleRow(_ v: [Float]) -> [Double] { v.map(Double.init) }

    static func l2norm(_ v: [Double]) -> [Double] {
        var s = 0.0; for x in v { s += x * x }
        let n = s.squareRoot() + 1e-9
        return v.map { $0 / n }
    }

    static func l2normRows(_ X: [[Float]]) -> [[Double]] { X.map { l2norm(doubleRow($0)) } }

    static func meanRows(_ rows: [[Double]]) -> [Double] {
        guard let d = rows.first?.count, !rows.isEmpty else { return [] }
        var acc = [Double](repeating: 0, count: d)
        for r in rows { for i in 0..<d { acc[i] += r[i] } }
        return acc.map { $0 / Double(rows.count) }
    }

    static func cosine(_ a: [Double], _ b: [Double]) -> Double {
        var dot = 0.0, na = 0.0, nb = 0.0
        for i in 0..<a.count { dot += a[i] * b[i]; na += a[i] * a[i]; nb += b[i] * b[i] }
        return dot / ((na.squareRoot() * nb.squareRoot()) + 1e-9)
    }

    /// Row-major n×n cosine similarity of L2-normalized rows (dot products).
    static func cosineMatrix(_ Xn: [[Double]]) -> [Double] {
        let n = Xn.count
        var M = [Double](repeating: 0, count: n * n)
        for i in 0..<n { for j in 0..<n {
            var d = 0.0; let a = Xn[i], b = Xn[j]
            for c in 0..<a.count { d += a[c] * b[c] }
            M[i * n + j] = d
        } }
        return M
    }

    /// senko p_pruning: per row, zero the smallest `(1-pval)*n` similarities
    /// (≈98.8%), keeping only the top ~1.2% (min_pnum floor). NOTE: senko's
    /// `n_elems` name is misleading — it's the ZERO count, not the keep count.
    /// This heavy sparsification is what gives the affinity graph clean block
    /// structure and thus clear eigenvalue gaps.
    static func pPrune(_ A: inout [Double], n: Int) {
        let nZero = min(Int((1.0 - pval) * Double(n)), n - minPnum)
        if nZero <= 0 { return }
        for i in 0..<n {
            let row = Array(A[i * n ..< i * n + n])
            let order = row.enumerated().sorted { $0.element < $1.element }.map { $0.offset }
            for z in 0..<nZero { A[i * n + order[z]] = 0 }
        }
    }

    static func symmetrize(_ A: inout [Double], n: Int) {
        for i in 0..<n { for j in (i + 1)..<n {
            let v = 0.5 * (A[i * n + j] + A[j * n + i])
            A[i * n + j] = v; A[j * n + i] = v
        } }
    }

    static func zeroDiagonal(_ A: inout [Double], n: Int) {
        for i in 0..<n { A[i * n + i] = 0 }
    }

    /// Unnormalized Laplacian L = D - A (D = diag of abs row sums).
    static func laplacian(_ A: [Double], n: Int) -> [Double] {
        var L = A.map { -$0 }
        for i in 0..<n {
            var deg = 0.0
            for j in 0..<n { deg += abs(A[i * n + j]) }
            L[i * n + i] = deg - A[i * n + i]   // A diag already 0 → deg
        }
        return L
    }

    /// Symmetric eigendecomposition via LAPACK dsyev. Returns eigenvalues
    /// (ascending) and eigenvectors as row-major [n*n] where column c is the
    /// c-th eigenvector (eigvecs[i*n + c] = component i of eigenvector c).
    static func eigh(_ M: [Double], n: Int) -> (vals: [Double], vecs: [Double])? {
        var a = M                                   // LAPACK is column-major; M symmetric so layout ok
        var jobz: Int8 = 0x56                       // 'V' — eigenvalues + vectors
        var uplo: Int8 = 0x55                       // 'U'
        var N = Int32(n)
        var lda = Int32(n)
        var w = [Double](repeating: 0, count: n)
        var info = Int32(0)
        var wkopt = 0.0
        var lwork = Int32(-1)
        dsyev_(&jobz, &uplo, &N, &a, &lda, &w, &wkopt, &lwork, &info)
        guard info == 0, wkopt.isFinite, wkopt >= 1 else { return nil }
        lwork = Int32(wkopt)
        var work = [Double](repeating: 0, count: Int(lwork))
        dsyev_(&jobz, &uplo, &N, &a, &lda, &w, &work, &lwork, &info)
        guard info == 0 else { return nil }         // no convergence / bad arg
        // a now holds eigenvectors column-major: a[i + c*n] = comp i of vec c.
        // Repack to row-major vecs[i*n + c].
        var vecs = [Double](repeating: 0, count: n * n)
        for c in 0..<n { for i in 0..<n { vecs[i * n + c] = a[i + c * n] } }
        return (w, vecs)
    }
}
