import Foundation
import Network

/**
 * DPI (Deep Packet Inspection) detector for iOS.
 *
 * Monitors TLS handshake patterns using a sliding window algorithm.
 * When the DPI score exceeds 0.72, triggers core switching for evasion.
 */
class DpiDetector {

    private var monitoringActive = false
    private var monitorTimer: Timer?

    // Sliding window for TLS analysis
    private let windowSize = 50
    private var handshakeTimings: [Double] = []
    private var tlsFingerprintScores: [Double] = []

    // DPI indicators
    private var connectionResets = 0
    private var tlsHandshakeFailures = 0
    private var totalHandshakes = 0

    // Callback
    private var onDpiDetected: ((Double) -> Void)?

    static let dpiThreshold = 0.72

    /**
     * Start monitoring for DPI patterns.
     */
    func startMonitoring(callback: @escaping (Double) -> Void) {
        guard !monitoringActive else { return }
        monitoringActive = true
        onDpiDetected = callback

        monitorTimer = Timer.scheduledTimer(withTimeInterval: 2.0, repeats: true) { [weak self] _ in
            self?.evaluateDpiScore()
        }
    }

    /**
     * Stop monitoring.
     */
    func stopMonitoring() {
        monitoringActive = false
        monitorTimer?.invalidate()
        monitorTimer = nil
        reset()
    }

    /**
     * Record a TLS handshake timing.
     */
    func recordHandshake(timingMs: Double, success: Bool) {
        totalHandshakes += 1
        handshakeTimings.append(timingMs)

        if !success {
            tlsHandshakeFailures += 1
        }

        // Trim to window size
        if handshakeTimings.count > windowSize {
            handshakeTimings.removeFirst(handshakeTimings.count - windowSize)
        }
    }

    /**
     * Record a connection reset.
     */
    func recordConnectionReset() {
        connectionResets += 1
    }

    /**
     * Record a TLS fingerprint anomaly score.
     */
    func recordTlsFingerprintScore(_ score: Double) {
        tlsFingerprintScores.append(score)
        if tlsFingerprintScores.count > windowSize {
            tlsFingerprintScores.removeFirst(tlsFingerprintScores.count - windowSize)
        }
    }

    /**
     * Reset counters (after core switch).
     */
    func reset() {
        connectionResets = 0
        tlsHandshakeFailures = 0
        totalHandshakes = 0
        handshakeTimings.removeAll()
        tlsFingerprintScores.removeAll()
    }

    /**
     * Get current DPI score.
     */
    var currentScore: Double {
        return calculateDpiScore()
    }

    // MARK: - Private

    private func evaluateDpiScore() {
        let score = calculateDpiScore()
        if score > Self.dpiThreshold {
            onDpiDetected?(score)
        }
    }

    /**
     * Calculate DPI score using weighted factors:
     * 1. TLS handshake failure rate (0.35)
     * 2. Connection reset frequency (0.30)
     * 3. TLS fingerprint anomaly (0.20)
     * 4. Timing pattern deviation (0.15)
     */
    private func calculateDpiScore() -> Double {
        guard totalHandshakes >= 5 else { return 0.0 }

        // Factor 1: TLS handshake failure rate
        let failureRate = Double(tlsHandshakeFailures) / Double(totalHandshakes)
        let failureScore = min(1.0, failureRate * 3.0)

        // Factor 2: Connection reset frequency
        let resetRate = Double(connectionResets) / Double(max(1, totalHandshakes))
        let resetScore = min(1.0, resetRate * 5.0)

        // Factor 3: TLS fingerprint anomaly
        let fingerprintScore = tlsFingerprintScores.isEmpty ? 0.0 : tlsFingerprintScores.reduce(0, +) / Double(tlsFingerprintScores.count)

        // Factor 4: Timing deviation
        let timingScore: Double
        if handshakeTimings.count >= 10 {
            let mean = handshakeTimings.reduce(0, +) / Double(handshakeTimings.count)
            let variance = handshakeTimings.map { pow($0 - mean, 2) }.reduce(0, +) / Double(handshakeTimings.count)
            let cv = sqrt(variance) / max(1.0, mean)
            timingScore = min(1.0, cv * 2.0)
        } else {
            timingScore = 0.0
        }

        return (
            failureScore * 0.35 +
            resetScore * 0.30 +
            fingerprintScore * 0.20 +
            timingScore * 0.15
        ).clamped(to: 0.0...1.0)
    }
}

// Helper extension
extension Comparable {
    func clamped(to range: ClosedRange<Self>) -> Self {
        return min(max(self, range.lowerBound), range.upperBound)
    }
}
