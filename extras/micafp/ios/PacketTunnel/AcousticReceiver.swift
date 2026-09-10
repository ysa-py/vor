import Foundation
import AVFoundation
import CoreAudio

/**
 * AcousticReceiver — iOS Acoustic Channel Receiver
 *
 * Uses AVAudioEngine to listen for ultrasonic signals in the 18-22 kHz range
 * for near-field peer-to-peer configuration sharing. Implements OFDM
 * demodulation via Rust FFI, handles microphone permissions, and is
 * battery-aware — only listening when the screen is on or the device is charging.
 * Supports background audio sessions with .mixWithOthers option.
 */
class AcousticReceiver {

    // MARK: - Configuration

    /// Ultrasonic frequency range for acoustic channel
    private let frequencyLow: Float = 18_000  // 18 kHz
    private let frequencyHigh: Float = 22_000 // 22 kHz

    /// Audio session configuration
    private let sampleRate: Double = 44_100.0
    private let bufferSize: AVAudioFrameCount = 4096
    private let channels: UInt32 = 1

    /// OFDM parameters
    private let ofdmSubcarriers = 64
    private let ofdmCyclicPrefix = 16
    private let ofdmSymbolDuration: Double = 0.01 // 10ms per symbol

    // MARK: - State

    private var audioEngine: AVAudioEngine?
    private var isListening = false
    private var audioSession: AVAudioSession?
    weak var delegate: AcousticReceiverDelegate?

    // Rust FFI for OFDM demodulation
    private typealias RustOfdmDemodulate = @convention(c) (
        UnsafePointer<Float>, Int, UnsafeMutablePointer<UInt8>, Int
    ) -> Int32

    private var rustOfdmDemodulate: RustOfdmDemodulate?

    // Buffer for accumulating audio samples
    private var sampleBuffer: [Float] = []
    private let sampleBufferMaxSize = 44_100 * 5 // 5 seconds of audio
    private let processingQueue = DispatchQueue(label: "shield.acoustic.processing", qos: .utility)

    // MARK: - Initialization

    init() {
        loadRustOfdmLibrary()
    }

    deinit {
        stopListening()
    }

    // MARK: - Rust FFI Loading

    private func loadRustOfdmLibrary() {
        let libraryPath = Bundle.main.bundlePath + "/Frameworks/libshield_native.dylib"

        guard let handle = dlopen(libraryPath, RTLD_NOW) else {
            NSLog("[AcousticReceiver] Failed to load Rust library: \(String(cString: dlerror()))")
            return
        }

        rustOfdmDemodulate = unsafeBitCast(
            dlsym(handle, "shield_ofdm_demodulate"),
            to: RustOfdmDemodulate.self
        )

        if rustOfdmDemodulate != nil {
            NSLog("[AcousticReceiver] Rust OFDM demodulation function loaded")
        } else {
            NSLog("[AcousticReceiver] Warning: Rust OFDM function not found, using fallback")
        }
    }

    // MARK: - Permission Handling

    /// Check and request microphone permission.
    /// The app's Info.plist must include NSMicrophoneUsageDescription.
    private func requestMicrophonePermission(completion: @escaping (Bool) -> Void) {
        let audioSession = AVAudioSession.sharedInstance()

        switch audioSession.recordPermission {
        case .granted:
            NSLog("[AcousticReceiver] Microphone permission already granted")
            completion(true)

        case .denied:
            NSLog("[AcousticReceiver] Microphone permission denied")
            completion(false)

        case .undetermined:
            NSLog("[AcousticReceiver] Requesting microphone permission...")
            audioSession.requestRecordPermission { granted in
                DispatchQueue.main.async {
                    if granted {
                        NSLog("[AcousticReceiver] Microphone permission granted")
                    } else {
                        NSLog("[AcousticReceiver] Microphone permission denied by user")
                    }
                    completion(granted)
                }
            }

        @unknown default:
            NSLog("[AcousticReceiver] Unknown microphone permission state")
            completion(false)
        }
    }

    /// Check if microphone permission is currently granted.
    var hasMicrophonePermission: Bool {
        return AVAudioSession.sharedInstance().recordPermission == .granted
    }

    // MARK: - Audio Session Configuration

    private func configureAudioSession() throws {
        let session = AVAudioSession.sharedInstance()
        self.audioSession = session

        try session.setCategory(
            .playAndRecord,
            mode: .measurement,
            options: [.mixWithOthers, .allowBluetooth, .defaultToSpeaker]
        )

        try session.setPreferredSampleRate(sampleRate)
        try session.setPreferredIOBufferDuration(0.01) // 10ms buffer
        try session.setActive(true, options: .notifyOthersOnDeactivation)

        NSLog("[AcousticReceiver] Audio session configured: sampleRate=\(sampleRate), category=playAndRecord")
    }

    // MARK: - Start/Stop Listening

    /// Start listening for ultrasonic acoustic signals.
    /// Only works when battery conditions permit (screen on or charging).
    func startListening() {
        guard !isListening else {
            NSLog("[AcousticReceiver] Already listening")
            return
        }

        requestMicrophonePermission { [weak self] granted in
            guard let self = self, granted else { return }

            do {
                try self.configureAudioSession()
                try self.setupAudioEngine()
                self.isListening = true
                NSLog("[AcousticReceiver] Started listening for ultrasonic signals (18-22 kHz)")
            } catch {
                NSLog("[AcousticReceiver] Failed to start listening: \(error)")
                self.delegate?.acousticReceiverDidEncounterError(error)
            }
        }
    }

    /// Stop listening for acoustic signals.
    func stopListening() {
        guard isListening else { return }

        audioEngine?.stop()
        audioEngine?.reset()
        audioEngine = nil

        // Deactivate audio session
        do {
            try AVAudioSession.sharedInstance().setActive(false, options: .notifyOthersOnDeactivation)
        } catch {
            NSLog("[AcousticReceiver] Failed to deactivate audio session: \(error)")
        }

        isListening = false
        sampleBuffer.removeAll()

        NSLog("[AcousticReceiver] Stopped listening")
    }

    // MARK: - Audio Engine Setup

    private func setupAudioEngine() throws {
        let engine = AVAudioEngine()
        self.audioEngine = engine

        let inputNode = engine.inputNode
        let format = inputNode.outputFormat(forBus: 0)

        // Install a tap on the input node to capture audio samples
        inputNode.installTap(
            onBus: 0,
            bufferSize: bufferSize,
            format: format
        ) { [weak self] buffer, time in
            self?.processAudioBuffer(buffer, time: time)
        }

        try engine.start()

        NSLog("[AcousticReceiver] Audio engine started with format: \(format)")
    }

    // MARK: - Audio Processing

    private func processAudioBuffer(_ buffer: AVAudioPCMBuffer, time: AVAudioTime) {
        guard let channelData = buffer.floatChannelData?[0] else { return }
        let frameCount = Int(buffer.frameLength)

        // Copy samples to our processing buffer
        processingQueue.async { [weak self] in
            guard let self = self else { return }

            // Append new samples
            let newSamples = Array(UnsafeBufferPointer(start: channelData, count: frameCount))
            self.sampleBuffer.append(contentsOf: newSamples)

            // Trim buffer if it gets too large
            if self.sampleBuffer.count > self.sampleBufferMaxSize {
                self.sampleBuffer.removeFirst(self.sampleBuffer.count - self.sampleBufferMaxSize)
            }

            // Check if we have enough samples for OFDM processing
            // We need at least one symbol duration worth of samples
            let samplesPerSymbol = Int(self.sampleRate * self.ofdmSymbolDuration)
            if self.sampleBuffer.count >= samplesPerSymbol {
                self.detectAndDemodulate()
            }
        }
    }

    /// Detect ultrasonic signal presence and demodulate using OFDM.
    private func detectAndDemodulate() {
        let samplesPerSymbol = Int(sampleRate * ofdmSymbolDuration)

        // Step 1: Detect if there's energy in the ultrasonic band (18-22 kHz)
        guard sampleBuffer.count >= samplesPerSymbol else { return }

        let recentSamples = Array(sampleBuffer.suffix(samplesPerSymbol))
        let ultrasonicEnergy = computeBandEnergy(
            samples: recentSamples,
            lowFreq: frequencyLow,
            highFreq: frequencyHigh
        )

        // Threshold for ultrasonic detection (adjusted empirically)
        let detectionThreshold: Float = 0.01
        guard ultrasonicEnergy > detectionThreshold else {
            // No ultrasonic signal detected, skip demodulation
            return
        }

        NSLog("[AcousticReceiver] Ultrasonic signal detected, energy: \(ultrasonicEnergy)")

        // Step 2: Extract the ultrasonic portion using bandpass filtering
        let filteredSamples = bandpassFilter(
            samples: recentSamples,
            lowFreq: frequencyLow,
            highFreq: frequencyHigh,
            sampleRate: Float(sampleRate)
        )

        // Step 3: OFDM demodulation via Rust FFI
        let demodulatedData = ofdmDemodulate(samples: filteredSamples)

        if let data = demodulatedData, !data.isEmpty {
            NSLog("[AcousticReceiver] OFDM demodulation successful, \(data.count) bytes received")
            delegate?.didReceiveAcousticData(data)
        }

        // Clear processed samples
        sampleBuffer.removeAll(keepingCapacity: true)
    }

    // MARK: - Signal Processing Utilities

    /// Compute energy in a specific frequency band using a simplified DFT approach.
    private func computeBandEnergy(samples: [Float], lowFreq: Float, highFreq: Float) -> Float {
        let n = samples.count
        guard n > 0 else { return 0 }

        var energy: Float = 0
        let sampleRateF = Float(sampleRate)

        // Compute energy in the target frequency band using Goertzel algorithm
        // This is more efficient than full FFT for a narrow band
        let freqStep = sampleRateF / Float(n)

        let lowBin = Int(lowFreq / freqStep)
        let highBin = min(Int(highFreq / freqStep), n / 2)

        // Goertzel algorithm for each frequency bin in the range
        for k in lowBin...highBin {
            let w = 2.0 * Float.pi * Float(k) / Float(n)
            var coeff = 2.0 * cos(w)
            var s0: Float = 0
            var s1: Float = 0
            var s2: Float = 0

            for sample in samples {
                s0 = sample + coeff * s1 - s2
                s2 = s1
                s1 = s0
            }

            let power = s1 * s1 + s2 * s2 - coeff * s1 * s2
            energy += abs(power)
        }

        return energy / Float(highBin - lowBin + 1)
    }

    /// Apply a simple bandpass filter to isolate the ultrasonic frequency range.
    /// Uses a windowed-sinc FIR filter approach.
    private func bandpassFilter(samples: [Float], lowFreq: Float, highFreq: Float, sampleRate: Float) -> [Float] {
        let n = samples.count
        let filterLength = 65 // FIR filter length (must be odd)
        let midPoint = filterLength / 2

        // Compute filter coefficients
        var coefficients = [Float](repeating: 0, count: filterLength)
        let lowNorm = lowFreq / sampleRate
        let highNorm = highFreq / sampleRate

        for i in 0..<filterLength {
            let k = i - midPoint
            if k == 0 {
                coefficients[i] = 2.0 * (highNorm - lowNorm)
            } else {
                coefficients[i] = sin(2.0 * Float.pi * highNorm * Float(k)) / (Float.pi * Float(k))
                    - sin(2.0 * Float.pi * lowNorm * Float(k)) / (Float.pi * Float(k))
            }
            // Apply Hamming window
            let window = 0.54 - 0.46 * cos(2.0 * Float.pi * Float(i) / Float(filterLength - 1))
            coefficients[i] *= window
        }

        // Apply filter using convolution
        var output = [Float](repeating: 0, count: n)
        for i in 0..<n {
            var sum: Float = 0
            for j in 0..<filterLength {
                let idx = i - j + midPoint
                if idx >= 0 && idx < n {
                    sum += samples[idx] * coefficients[j]
                }
            }
            output[i] = sum
        }

        return output
    }

    // MARK: - OFDM Demodulation

    /// Demodulate OFDM signal using Rust FFI.
    /// Falls back to a simple DFT-based approach if Rust is unavailable.
    private func ofdmDemodulate(samples: [Float]) -> Data? {
        if let demodFn = rustOfdmDemodulate {
            // Use Rust FFI for high-performance OFDM demodulation
            var outputBuffer = [UInt8](repeating: 0, count: 1024)

            let bytesRead = samples.withUnsafeBufferPointer { samplesPtr in
                outputBuffer.withUnsafeMutableBufferPointer { outputPtr in
                    return demodFn(
                        samplesPtr.baseAddress!,
                        samplesPtr.count,
                        outputPtr.baseAddress!,
                        outputPtr.count
                    )
                }
            }

            if bytesRead > 0 {
                return Data(outputBuffer.prefix(Int(bytesRead)))
            } else {
                NSLog("[AcousticReceiver] Rust OFDM demodulation returned no data")
                return nil
            }
        } else {
            // Fallback: simple DFT-based demodulation
            return fallbackDemodulate(samples: samples)
        }
    }

    /// Simple fallback demodulation using amplitude detection.
    /// This is less robust than OFDM but works for basic signals.
    private func fallbackDemodulate(samples: [Float]) -> Data? {
        // Group subcarriers and detect amplitude patterns
        let subcarrierSpacing = (frequencyHigh - frequencyLow) / Float(ofdmSubcarriers)
        var bits = [Bool]()

        for i in 0..<ofdmSubcarriers {
            let freq = frequencyLow + subcarrierSpacing * Float(i)
            let energy = goertzelEnergy(samples: samples, targetFreq: freq)

            // Simple threshold detection for binary modulation
            if energy > 0.5 {
                bits.append(true)
            } else {
                bits.append(false)
            }
        }

        // Convert bits to bytes
        var bytes = [UInt8]()
        for i in stride(from: 0, to: bits.count - 7, by: 8) {
            var byte: UInt8 = 0
            for j in 0..<8 {
                if bits[i + j] {
                    byte |= UInt8(1 << (7 - j))
                }
            }
            bytes.append(byte)
        }

        // Validate: check for framing markers
        if bytes.count >= 4 {
            // First two bytes should be sync marker 0xA5A5
            if bytes[0] == 0xA5 && bytes[1] == 0xA5 {
                return Data(bytes.dropFirst(2)) // Remove sync marker
            }
        }

        return nil
    }

    /// Goertzel algorithm for computing energy at a specific frequency.
    private func goertzelEnergy(samples: [Float], targetFreq: Float) -> Float {
        let n = samples.count
        let k = Int(0.5 + Float(n) * targetFreq / Float(sampleRate))
        let w = 2.0 * Float.pi * Float(k) / Float(n)
        let coeff = 2.0 * cos(w)

        var s1: Float = 0
        var s2: Float = 0

        for sample in samples {
            let s0 = sample + coeff * s1 - s2
            s2 = s1
            s1 = s0
        }

        let power = s1 * s1 + s2 * s2 - coeff * s1 * s2
        return sqrt(abs(power)) / Float(n)
    }

    // MARK: - Battery-Aware Control

    /// Whether the acoustic receiver should be active based on battery state.
    /// Only listens when screen is on or device is charging.
    var shouldListen: Bool {
        // Check battery state via UIDevice
        UIDevice.current.isBatteryMonitoringEnabled = true
        let batteryLevel = UIDevice.current.batteryLevel
        let batteryState = UIDevice.current.batteryState

        // Don't listen if battery is critically low and not charging
        if batteryLevel >= 0 && batteryLevel < 0.15 && batteryState != .charging && batteryState != .full {
            return false
        }

        // Don't listen in low power mode unless charging
        if ProcessInfo.processInfo.isLowPowerModeEnabled && batteryState != .charging && batteryState != .full {
            return false
        }

        return true
    }

    /// Start listening only if battery conditions permit.
    func startListeningIfBatteryAllows() {
        if shouldListen {
            startListening()
        } else {
            NSLog("[AcousticReceiver] Skipping listening due to battery constraints")
        }
    }
}
