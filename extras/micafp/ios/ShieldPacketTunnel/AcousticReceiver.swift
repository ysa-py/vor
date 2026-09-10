/*
 * MICAFP-UnifiedShield-6.0
 * AcousticReceiver.swift — iOS acoustic channel receiver
 *
 * Uses AVAudioEngine for audio capture with an 18-22 kHz bandpass filter.
 * Implements OFDM demodulation by calling Rust FFI for signal processing.
 * Decodes AES-256-GCM endpoint payloads and forwards them to the Rust daemon.
 *
 * BATTERY CRITICAL on iOS:
 *   - iOS shows orange dot when microphone is active
 *   - Cannot maintain microphone in background for extended periods
 *   - Strategy: Only listen when app is foregrounded
 *   - In background: rely on other channels (NTP, push notification)
 *   - When NAIN CompleteBlackout: request background audio entitlement
 *   - Use AVAudioSession category .playAndRecord with .mixWithOthers
 *   - Set audio session active only during listening windows
 *   - Passive mode: 5s listen, 55s pause when backgrounded (iOS allows this)
 *
 * No root required. Cloudflare is NOT used.
 */

import Foundation
import AVFoundation
import CoreAudio
import os.log
import UIKit

// MARK: - Acoustic Channel Constants

enum AcousticConstants {
    // Frequency range for acoustic signaling
    static let lowerFrequency: Double = 18_000   // 18 kHz
    static let upperFrequency: Double = 22_000   // 22 kHz
    static let sampleRate: Double = 44_100

    // OFDM parameters
    static let fftSize: Int = 1024
    static let cyclicPrefixLength: Int = 128
    static let numSubcarriers: Int = 64
    static let pilotSpacing: Int = 4

    // Timing and detection
    static let detectionThreshold: Float = 0.65
    static let minSignalDuration: Double = 0.5   // Minimum signal duration in seconds

    // Battery optimization: listening windows
    static let foregroundListenDuration: Double = 10.0   // 10s listen when foregrounded
    static let foregroundPauseDuration: Double = 5.0     // 5s pause between listens
    static let backgroundListenDuration: Double = 5.0    // 5s listen when backgrounded
    static let backgroundPauseDuration: Double = 55.0    // 55s pause between listens (passive mode)
    static let blackoutListenDuration: Double = 10.0     // 10s listen during CompleteBlackout
    static let blackoutPauseDuration: Double = 20.0      // 20s pause during CompleteBlackout

    // Crypto constants (same as SMS and other channels)
    static let hmacLength: Int = 32       // HMAC-SHA256
    static let aesGcmIVLength: Int = 12   // AES-256-GCM IV
    static let aesGcmTagLength: Int = 16  // AES-256-GCM auth tag
}

// MARK: - Acoustic Payload Format
// Same as SMS channel:
// [32 bytes HMAC-SHA256] [12 bytes IV] [N bytes AES-256-GCM ciphertext + 16 byte tag]
// Decrypted: [1 byte version] [2 bytes endpoint_count] [endpoint_count × endpoint_entry]
// endpoint_entry: [1 byte type] [1 byte addr_len] [addr_len bytes address] [2 bytes port]

// MARK: - NAIN State

enum NainState {
    case normal
    case partialBlackout
    case completeBlackout
}

// MARK: - Rust FFI Interface

/// Bridge to Rust FFI functions for signal processing.
/// The actual Rust library is loaded dynamically.
class RustSignalProcessor {
    private let logger = Logger(subsystem: "org.micafp.unifiedshield", category: "RustSignalProcessor")
    private var rustLibraryHandle: UnsafeMutableRawPointer?

    // Rust FFI function types
    typealias RustOfdmDemodulateFunc = @convention(c) (
        _ samples: UnsafePointer<Float>,
        _ sampleCount: Int32,
        _ output: UnsafeMutablePointer<UInt8>,
        _ outputCapacity: Int32
    ) -> Int32

    typealias RustDetectPreambleFunc = @convention(c) (
        _ samples: UnsafePointer<Float>,
        _ sampleCount: Int32,
        _ correlation: UnsafeMutablePointer<Float>
    ) -> Bool

    typealias RustProcessAcousticPayloadFunc = @convention(c) (
        _ data: UnsafePointer<UInt8>,
        _ dataLen: Int32,
        _ endpoints: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
        _ endpointCapacity: Int32
    ) -> Int32

    private var ofdmDemodulate: RustOfdmDemodulateFunc?
    private var detectPreamble: RustDetectPreambleFunc?
    private var processAcousticPayload: RustProcessAcousticPayloadFunc?

    init() {
        loadRustLibrary()
    }

    deinit {
        if let handle = rustLibraryHandle {
            dlclose(handle)
        }
    }

    private func loadRustLibrary() {
        // Try to load the Rust shared library
        let libraryPaths = [
            "libshield_ffi",
            "@executable_path/Frameworks/libshield_ffi",
            "@loader_path/Frameworks/libshield_ffi"
        ]

        for path in libraryPaths {
            if let handle = dlopen(path, RTLD_NOW) {
                rustLibraryHandle = handle

                // Load FFI functions
                ofdmDemodulate = unsafeBitCast(
                    dlsym(handle, "shield_ofdm_demodulate"),
                    to: RustOfdmDemodulateFunc.self
                )
                detectPreamble = unsafeBitCast(
                    dlsym(handle, "shield_detect_preamble"),
                    to: RustDetectPreambleFunc.self
                )
                processAcousticPayload = unsafeBitCast(
                    dlsym(handle, "shield_process_acoustic_payload"),
                    to: RustProcessAcousticPayloadFunc.self
                )

                logger.info("Loaded Rust FFI library from \(path)")
                return
            }
        }

        logger.warning("Could not load Rust FFI library, using software fallback")
    }

    /// Detect acoustic preamble in the given samples.
    func detectPreambleSignal(in samples: [Float]) -> (detected: Bool, correlation: Float) {
        guard let detectFn = detectPreamble else {
            return fallbackPreambleDetection(in: samples)
        }

        var correlation: Float = 0
        let detected = samples.withUnsafeBufferPointer { samplePtr in
            detectFn(samplePtr.baseAddress!, Int32(samples.count), &correlation)
        }
        return (detected, correlation)
    }

    /// Demodulate OFDM signal from the given samples.
    func demodulateOFDM(samples: [Float]) -> Data? {
        guard let demodFn = ofdmDemodulate else {
            return fallbackOFDMDemodulation(samples: samples)
        }

        var outputBuffer = [UInt8](repeating: 0, count: 4096)

        let outputLen = samples.withUnsafeBufferPointer { samplePtr in
            demodFn(samplePtr.baseAddress!, Int32(samples.count), &outputBuffer, Int32(outputBuffer.count))
        }

        guard outputLen > 0 else {
            return nil
        }

        return Data(outputBuffer.prefix(Int(outputLen)))
    }

    /// Process an acoustic payload (HMAC validation, decryption, parsing).
    /// Falls back to Swift implementation if Rust FFI is not available.
    func processPayload(_ payload: Data, hmacKey: Data, aesKey: Data) -> [String]? {
        // Prefer Rust FFI for crypto operations (constant-time, audited)
        if let processFn = processAcousticPayload {
            var endpointPtrs = [UnsafeMutablePointer<CChar>?](repeating: nil, count: 64)
            let hmacKeyData = hmacKey
            let aesKeyData = aesKey

            let count = payload.withUnsafeBytes { payloadPtr in
                hmacKeyData.withUnsafeBytes { hmacKeyPtr in
                    aesKeyData.withUnsafeBytes { aesKeyPtr in
                        processFn(
                            payloadPtr.baseAddress!.assumingMemoryBound(to: UInt8.self),
                            Int32(payload.count),
                            &endpointPtrs,
                            64
                        )
                    }
                }
            }

            guard count > 0 else { return nil }

            var endpoints = [String]()
            for i in 0..<Int(count) {
                if let ptr = endpointPtrs[i] {
                    if let str = String(cString: ptr, encoding: .utf8) {
                        endpoints.append(str)
                    }
                    free(ptr)
                }
            }
            return endpoints.isEmpty ? nil : endpoints
        }

        // Fallback: Swift implementation
        return processPayloadSwift(payload, hmacKey: hmacKey, aesKey: aesKey)
    }

    // MARK: - Software Fallbacks

    /// Fallback preamble detection using simple energy detection in the target band.
    private func fallbackPreambleDetection(in samples: [Float]) -> (detected: Bool, correlation: Float) {
        guard !samples.isEmpty else { return (false, 0) }

        // Simple energy-based detection in the 18-22 kHz band
        let fftSize = AcousticConstants.fftSize
        var energy: Float = 0

        // Compute band energy using Goertzel algorithm for the target frequencies
        let targetFreqs: [Double] = [18_000, 19_000, 20_000, 21_000, 22_000]
        var maxMagnitude: Float = 0

        for freq in targetFreqs {
            let k = Int(Double(fftSize) * freq / AcousticConstants.sampleRate)
            let w = 2.0 * Double.pi * Double(k) / Double(fftSize)
            let coeff = Float(2.0 * cos(w))

            var s0: Float = 0
            var s1: Float = 0
            var s2: Float = 0

            for sample in samples {
                s0 = sample + coeff * s1 - s2
                s2 = s1
                s1 = s0
            }

            let magnitude = s1 * s1 + s2 * s2 - coeff * s1 * s2
            maxMagnitude = max(maxMagnitude, magnitude)
        }

        // Normalize
        energy = maxMagnitude / Float(samples.count)
        let detected = energy > AcousticConstants.detectionThreshold

        return (detected, energy)
    }

    /// Fallback OFDM demodulation (simplified — in practice, the Rust FFI handles this).
    private func fallbackOFDMDemodulation(samples: [Float]) -> Data? {
        // Simplified: just collect the raw demodulated bytes
        // In production, this would use a proper FFT-based OFDM demodulator
        // The Rust FFI is the primary implementation; this is a minimal fallback
        logger.warning("Using fallback OFDM demodulation — may be unreliable")
        return nil
    }

    private let logger = Logger(subsystem: "org.micafp.unifiedshield", category: "RustSignalProcessor")
}

// MARK: - Acoustic Receiver

/// iOS acoustic channel receiver that captures ultrasonic audio signals
/// in the 18-22 kHz range and decodes them into endpoint data.
///
/// Battery-optimized listening strategy:
///   - Foreground: continuous listening with short pauses
///   - Background: 5s listen, 55s pause (passive mode)
///   - CompleteBlackout: longer listening windows with shorter pauses
class AcousticReceiver {

    // MARK: - Properties

    private let logger = Logger(subsystem: "org.micafp.unifiedshield", category: "AcousticReceiver")
    private let signalProcessor = RustSignalProcessor()

    // Audio engine
    private var audioEngine: AVAudioEngine?
    private var audioPlayerNode: AVAudioPlayerNode?
    private var sampleBuffer: [Float] = []
    private let sampleBufferLock = NSLock()

    // State
    private var isListening = false
    private var nainState: NainState = .normal
    private var appState: UIApplication.State = .active

    // Listening timer
    private var listenTimer: Timer?
    private var listenPhase: ListenPhase = .idle

    // JNI / IPC bridge to Rust daemon
    private var onEndpointsReceived: (([String]) -> Void)?

    // Crypto keys
    private var hmacKey: Data?
    private var aesKey: Data?

    // Audio session management
    private var audioSessionActive = false

    // Buffer for collecting signal data
    private let maxBufferLength = Int(AcousticConstants.sampleRate * 10) // 10 seconds of audio
    private let detectionChunkSize = Int(AcousticConstants.sampleRate * 0.5) // 0.5 second chunks

    private enum ListenPhase {
        case idle
        case listening
        case paused
    }

    // MARK: - Initialization

    init() {
        setupAudioSession()
        setupAudioEngine()
    }

    deinit {
        stopListening()
        cleanupAudioEngine()
    }

    /// Set the callback for when endpoints are decoded from an acoustic signal.
    func setEndpointCallback(_ callback: @escaping ([String]) -> Void) {
        onEndpointsReceived = callback
    }

    /// Set the cryptographic keys for payload decryption.
    func setCryptoKeys(hmacKey: Data, aesKey: Data) {
        self.hmacKey = hmacKey
        self.aesKey = aesKey
    }

    // MARK: - Audio Session Management

    private func setupAudioSession() {
        let session = AVAudioSession.sharedInstance()

        do {
            try session.setCategory(
                .playAndRecord,
                mode: .measurement,
                options: [.mixWithOthers, .allowBluetooth, .defaultToSpeaker]
            )
            try session.setPreferredSampleRate(AcousticConstants.sampleRate)
            try session.setPreferredIOBufferDuration(0.01) // 10ms buffer for low latency

            logger.info("Audio session configured for acoustic receiver")
        } catch {
            logger.error("Failed to configure audio session: \(error.localizedDescription)")
        }
    }

    private func activateAudioSession() {
        guard !audioSessionActive else { return }

        do {
            try AVAudioSession.sharedInstance().setActive(true, options: .notifyOthersOnDeactivation)
            audioSessionActive = true
            logger.debug("Audio session activated")
        } catch {
            logger.error("Failed to activate audio session: \(error.localizedDescription)")
        }
    }

    private func deactivateAudioSession() {
        guard audioSessionActive else { return }

        do {
            try AVAudioSession.sharedInstance().setActive(false, options: .notifyOthersOnDeactivation)
            audioSessionActive = false
            logger.debug("Audio session deactivated")
        } catch {
            logger.error("Failed to deactivate audio session: \(error.localizedDescription)")
        }
    }

    // MARK: - Audio Engine Setup

    private func setupAudioEngine() {
        audioEngine = AVAudioEngine()

        guard let audioEngine = audioEngine else {
            logger.error("Failed to create audio engine")
            return
        }

        let inputNode = audioEngine.inputNode
        let format = inputNode.outputFormat(forBus: 0)

        // Convert to our desired format (mono, float32, 44100 Hz)
        guard let targetFormat = AVAudioFormat(
            commonFormat: .pcmFormatFloat32,
            sampleRate: AcousticConstants.sampleRate,
            channels: 1,
            interleaved: false
        ) else {
            logger.error("Failed to create target audio format")
            return
        }

        guard let converter = AVAudioConverter(from: format, to: targetFormat) else {
            logger.error("Failed to create audio format converter")
            return
        }

        // Install tap on the input node to capture audio samples
        inputNode.installTap(onBus: 0, bufferSize: 1024, format: targetFormat) { [weak self] buffer, time in
            self?.processAudioBuffer(buffer)
        }

        logger.info("Audio engine configured with input tap")
    }

    private func cleanupAudioEngine() {
        guard let audioEngine = audioEngine else { return }

        audioEngine.inputNode.removeTap(onBus: 0)
        audioEngine.stop()
        self.audioEngine = nil
    }

    // MARK: - Listening Control

    /// Start listening for acoustic signals.
    /// The listening pattern depends on the current NAIN state and app state.
    func startListening() {
        guard !isListening else { return }

        logger.info("Starting acoustic receiver")
        isListening = true
        listenPhase = .idle

        // Start the listening cycle based on current state
        beginListenWindow()
    }

    /// Stop listening for acoustic signals.
    func stopListening() {
        guard isListening else { return }

        logger.info("Stopping acoustic receiver")
        isListening = false
        listenPhase = .idle

        listenTimer?.invalidate()
        listenTimer = nil

        // Stop audio engine and deactivate session
        audioEngine?.stop()
        deactivateAudioSession()
    }

    /// Update the NAIN state, which affects listening behavior.
    func updateNainState(_ state: NainState) {
        let previousState = nainState
        nainState = state

        logger.info("NAIN state updated: \(String(describing: state))")

        // If we're currently listening, adjust the listening pattern
        if isListening && previousState != state {
            // Restart the listening cycle with new timing
            listenTimer?.invalidate()
            beginListenWindow()
        }
    }

    /// Update the app state (foreground/background).
    func updateAppState(_ state: UIApplication.State) {
        let previousState = appState
        appState = state

        logger.debug("App state updated: \(state.rawValue)")

        if isListening && previousState != state {
            listenTimer?.invalidate()

            if state != .active && nainState != .completeBlackout {
                // App moved to background — switch to passive mode
                beginPassiveListenWindow()
            } else {
                // App foregrounded or CompleteBlackout — active listening
                beginListenWindow()
            }
        }
    }

    // MARK: - Listening Window Management

    /// Begin an active listening window (foreground or CompleteBlackout).
    private func beginListenWindow() {
        guard isListening else { return }

        listenPhase = .listening
        startAudioCapture()

        let duration: TimeInterval
        switch nainState {
        case .completeBlackout:
            duration = AcousticConstants.blackoutListenDuration
        case .normal, .partialBlackout:
            duration = AcousticConstants.foregroundListenDuration
        }

        listenTimer = Timer.scheduledTimer(
            withTimeInterval: duration,
            repeats: false
        ) { [weak self] _ in
            self?.endListenWindow()
        }
    }

    /// End the current listening window and begin a pause.
    private func endListenWindow() {
        guard isListening else { return }

        listenPhase = .paused
        stopAudioCapture()

        // Process any accumulated buffer
        processAccumulatedBuffer()

        let pauseDuration: TimeInterval
        switch (appState, nainState) {
        case (_, .completeBlackout):
            pauseDuration = AcousticConstants.blackoutPauseDuration
        case (.active, _):
            pauseDuration = AcousticConstants.foregroundPauseDuration
        default:
            pauseDuration = AcousticConstants.backgroundPauseDuration
        }

        listenTimer = Timer.scheduledTimer(
            withTimeInterval: pauseDuration,
            repeats: false
        ) { [weak self] _ in
            self?.beginListenWindow()
        }
    }

    /// Begin a passive listening window (background, non-CompleteBlackout).
    private func beginPassiveListenWindow() {
        guard isListening else { return }

        listenPhase = .listening
        startAudioCapture()

        listenTimer = Timer.scheduledTimer(
            withTimeInterval: AcousticConstants.backgroundListenDuration,
            repeats: false
        ) { [weak self] _ in
            self?.endPassiveListenWindow()
        }
    }

    /// End a passive listening window.
    private func endPassiveListenWindow() {
        guard isListening else { return }

        listenPhase = .paused
        stopAudioCapture()
        processAccumulatedBuffer()

        listenTimer = Timer.scheduledTimer(
            withTimeInterval: AcousticConstants.backgroundPauseDuration,
            repeats: false
        ) { [weak self] _ in
            if self?.appState == .active || self?.nainState == .completeBlackout {
                self?.beginListenWindow()
            } else {
                self?.beginPassiveListenWindow()
            }
        }
    }

    // MARK: - Audio Capture

    private func startAudioCapture() {
        guard let audioEngine = audioEngine else { return }

        activateAudioSession()

        if !audioEngine.isRunning {
            do {
                try audioEngine.start()
                logger.debug("Audio engine started")
            } catch {
                logger.error("Failed to start audio engine: \(error.localizedDescription)")
            }
        }
    }

    private func stopAudioCapture() {
        guard let audioEngine = audioEngine else { return }

        if audioEngine.isRunning {
            audioEngine.pause()
            logger.debug("Audio engine paused")
        }

        // Deactivate session to save battery and remove orange dot
        deactivateAudioSession()
    }

    // MARK: - Audio Processing

    /// Process incoming audio buffer from the input tap.
    private func processAudioBuffer(_ buffer: AVAudioPCMBuffer) {
        guard let channelData = buffer.floatChannelData?[0] else { return }
        let frameCount = Int(buffer.frameLength)

        // Append to sample buffer
        sampleBufferLock.lock()
        sampleBuffer.append(contentsOf: UnsafeBufferPointer(start: channelData, count: frameCount))

        // Trim buffer if it gets too large
        if sampleBuffer.count > maxBufferLength {
            sampleBuffer.removeFirst(sampleBuffer.count - maxBufferLength)
        }

        // Check if we have enough data for a detection chunk
        if sampleBuffer.count >= detectionChunkSize {
            let chunk = Array(sampleBuffer.prefix(detectionChunkSize))
            sampleBuffer.removeFirst(detectionChunkSize)
            sampleBufferLock.unlock()

            // Process the chunk
            processDetectionChunk(chunk)
        } else {
            sampleBufferLock.unlock()
        }
    }

    /// Process a chunk of audio data for signal detection.
    private func processDetectionChunk(_ chunk: [Float]) {
        // Step 1: Detect preamble
        let (detected, correlation) = signalProcessor.detectPreambleSignal(in: chunk)

        guard detected else { return }

        logger.info("Acoustic preamble detected (correlation: \(correlation))")

        // Step 2: Collect more samples for the full signal
        collectFullSignal(afterPreamble: chunk)
    }

    /// Collect the full acoustic signal following a detected preamble.
    private func collectFullSignal(afterPreamble preambleChunk: [Float]) {
        // The preamble has been detected. We need to collect the rest of the signal.
        // In a real implementation, we'd continue capturing until the signal ends.
        // For now, we'll process what we have accumulated.

        sampleBufferLock.lock()
        let accumulatedSamples = sampleBuffer
        sampleBuffer.removeAll(keepingCapacity: true)
        sampleBufferLock.unlock()

        let allSamples = preambleChunk + accumulatedSamples

        // Step 3: OFDM Demodulation
        guard let demodulatedData = signalProcessor.demodulateOFDM(samples: allSamples) else {
            logger.warning("OFDM demodulation failed")
            return
        }

        logger.info("OFDM demodulated: \(demodulatedData.count) bytes")

        // Step 4: Process the payload (HMAC validation, decryption, endpoint parsing)
        guard let hmacKey = self.hmacKey, let aesKey = self.aesKey else {
            logger.error("Crypto keys not set, cannot process acoustic payload")
            return
        }

        guard let endpoints = signalProcessor.processPayload(demodulatedData, hmacKey: hmacKey, aesKey: aesKey) else {
            logger.warning("Failed to process acoustic payload")
            return
        }

        logger.info("Acoustic channel decoded: \(endpoints.count) endpoints")

        // Step 5: Forward to Rust daemon via callback
        onEndpointsReceived?(endpoints)
    }

    /// Process any accumulated buffer (called during pause windows).
    private func processAccumulatedBuffer() {
        sampleBufferLock.lock()
        let samples = sampleBuffer
        sampleBuffer.removeAll(keepingCapacity: true)
        sampleBufferLock.unlock()

        guard !samples.isEmpty else { return }

        // Try to detect and process any signals in the accumulated buffer
        // Use a sliding window approach
        let windowSize = detectionChunkSize
        let stepSize = windowSize / 2

        var offset = 0
        while offset + windowSize <= samples.count {
            let window = Array(samples[offset..<(offset + windowSize)])
            let (detected, _) = signalProcessor.detectPreambleSignal(in: window)

            if detected {
                logger.info("Detected preamble in accumulated buffer at offset \(offset)")
                let remainingSamples = Array(samples[offset...])
                collectFullSignal(afterPreamble: remainingSamples)
                break
            }

            offset += stepSize
        }
    }

    // MARK: - Swift Payload Processing (Fallback)

    /// Process acoustic payload using Swift (fallback when Rust FFI is unavailable).
    private func processPayloadSwift(_ payload: Data, hmacKey: Data, aesKey: Data) -> [String]? {
        // Validate minimum size
        guard payload.count >= AcousticConstants.hmacLength + AcousticConstants.aesGcmIVLength + AcousticConstants.aesGcmTagLength + 1 else {
            return nil
        }

        // Split payload
        let receivedHmac = payload.prefix(AcousticConstants.hmacLength)
        let ciphertext = payload.dropFirst(AcousticConstants.hmacLength)

        // Validate HMAC
        guard validateHMAC(data: ciphertext, expectedHMAC: receivedHmac, key: hmacKey) else {
            logger.warning("Acoustic payload HMAC validation failed")
            return nil
        }

        // Extract IV and encrypted data
        let iv = ciphertext.prefix(AcousticConstants.aesGcmIVLength)
        let encrypted = ciphertext.dropFirst(AcousticConstants.aesGcmIVLength)

        // Decrypt AES-256-GCM
        guard let plaintext = decryptAES256GCM(encrypted: Data(encrypted), iv: Data(iv), key: aesKey) else {
            logger.warning("AES-256-GCM decryption failed for acoustic payload")
            return nil
        }

        // Parse endpoint list
        return parseEndpointList(plaintext)
    }

    /// Validate HMAC-SHA256 for payload data.
    private func validateHMAC(data: Data, expectedHMAC: Data, key: Data) -> Bool {
        guard let hmacContext = CCHmacContext.allocate(capacity: 1) else { return false }
        defer { hmacContext.deallocate() }

        key.withUnsafeBytes { keyPtr in
            CCHmacInit(hmacContext, CCHmacAlgorithm(kCCHmacAlgSHA256), keyPtr.baseAddress, key.count)
        }

        data.withUnsafeBytes { dataPtr in
            CCHmacUpdate(hmacContext, dataPtr.baseAddress, data.count)
        }

        var computedHMAC = [UInt8](repeating: 0, count: AcousticConstants.hmacLength)
        CCHmacFinal(hmacContext, &computedHMAC)

        // Constant-time comparison
        return constantTimeEquals(Data(computedHMAC), expectedHMAC)
    }

    /// Decrypt data using AES-256-GCM.
    private func decryptAES256GCM(encrypted: Data, iv: Data, key: Data) -> Data? {
        // Use CryptoKit if available (iOS 13+)
        if #available(iOS 13.0, *) {
            return decryptWithCryptoKit(encrypted: encrypted, iv: iv, key: key)
        }

        // Fallback: Use CommonCrypto (GCM support varies)
        return decryptWithCommonCrypto(encrypted: encrypted, iv: iv, key: key)
    }

    @available(iOS 13.0, *)
    private func decryptWithCryptoKit(encrypted: Data, iv: Data, key: Data) -> Data? {
        do {
            let symmetricKey = CryptoKit.SymmetricKey(data: key)
            let sealedBox = try CryptoKit.AEAD.GCM.SealedBox(
                nonce: CryptoKit.AEAD.GCM.Nonce(data: iv),
                ciphertext: encrypted.dropLast(AcousticConstants.aesGcmTagLength),
                tag: encrypted.suffix(AcousticConstants.aesGcmTagLength)
            )
            let decrypted = try CryptoKit.AEAD.GCM.open(sealedBox, using: symmetricKey)
            return decrypted
        } catch {
            logger.error("CryptoKit AES-GCM decryption failed: \(error.localizedDescription)")
            return nil
        }
    }

    private func decryptWithCommonCrypto(encrypted: Data, iv: Data, key: Data) -> Data? {
        // CommonCrypto CCCryptorGCM is available on iOS but requires specific handling
        // This is a simplified placeholder — in production, use CryptoKit
        logger.warning("CommonCrypto GCM fallback not fully implemented — use CryptoKit")
        return nil
    }

    /// Constant-time comparison of two data buffers.
    private func constantTimeEquals(_ a: Data, _ b: Data) -> Bool {
        guard a.count == b.count else { return false }
        var result = 0
        for i in 0..<a.count {
            result |= Int(a[i]) ^ Int(b[i])
        }
        return result == 0
    }

    /// Parse decrypted endpoint list.
    /// Format: [1 byte version] [2 bytes count] [count × (1 byte type, 1 byte addr_len, addr_len bytes, 2 bytes port)]
    private func parseEndpointList(_ data: Data) -> [String]? {
        guard data.count >= 3 else { return nil }

        var offset = 0
        let version = data[offset]
        offset += 1

        guard version == 0x01 else {
            logger.warning("Unknown acoustic payload version: \(version)")
            return nil
        }

        let endpointCount = (Int(data[offset]) << 8) | Int(data[offset + 1])
        offset += 2

        guard endpointCount > 0 && endpointCount <= 64 else {
            logger.warning("Unreasonable endpoint count: \(endpointCount)")
            return nil
        }

        var endpoints = [String]()

        for _ in 0..<endpointCount {
            guard offset + 4 <= data.count else { break }

            let type = data[offset]
            offset += 1
            let addrLen = Int(data[offset])
            offset += 1

            guard addrLen > 0 && addrLen <= 128 && offset + addrLen + 2 <= data.count else { break }

            guard let address = String(data: data[offset..<(offset + addrLen)], encoding: .utf8) else { break }
            offset += addrLen

            let port = (Int(data[offset]) << 8) | Int(data[offset + 1])
            offset += 2

            let typeStr: String
            switch type {
            case 0x01: typeStr = "wg"
            case 0x02: typeStr = "ygg"
            case 0x03: typeStr = "obfs4"
            case 0x04: typeStr = "snow"
            case 0x05: typeStr = "ss"
            case 0x06: typeStr = "bridge"
            default: typeStr = "unknown"
            }

            endpoints.append("\(typeStr)://\(address):\(port)")
        }

        return endpoints.isEmpty ? nil : endpoints
    }
}

// MARK: - CryptoKit Import Helper

/// Import CryptoKit conditionally for AES-GCM operations.
/// This is used by the acoustic receiver for payload decryption.
private enum CryptoKitBridge {
    @available(iOS 13.0, *)
    static func decryptAESGCM(key: Data, iv: Data, ciphertext: Data, tag: Data) -> Data? {
        do {
            let symmetricKey = CryptoKit.SymmetricKey(data: key)
            let nonce = try CryptoKit.AEAD.GCM.Nonce(data: iv)
            let sealedBox = try CryptoKit.AEAD.GCM.SealedBox(nonce: nonce, ciphertext: ciphertext, tag: tag)
            return try CryptoKit.AEAD.GCM.open(sealedBox, using: symmetricKey)
        } catch {
            return nil
        }
    }
}
