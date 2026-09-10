/*
 * MICAFP-UnifiedShield-6.0
 * BatteryManager.swift — iOS battery management and power state reporting
 *
 * Manages battery monitoring, power state reporting to the Rust daemon,
 * and adaptive behavior based on battery level. Integrates with
 * BGTaskScheduler for background processing and UIApplication lifecycle.
 *
 * Key responsibilities:
 *   - UIDevice.current.batteryLevel monitoring
 *   - UIDevice.current.batteryState monitoring
 *   - UIApplication.shared.isIdleTimerDisabled management
 *   - UIScreen.main.brightness monitoring (screen state proxy)
 *   - BGTaskScheduler for background processing
 *   - Reports power state to Rust daemon
 *   - Adaptive subsystem behavior:
 *       Charging: full NAIN detection interval (30s)
 *       Normal: 60s interval
 *       Low battery: 300s interval
 *       Critical: minimal background activity, rely on push notifications
 *   - UIApplicationDelegate lifecycle integration
 *
 * No root required. Cloudflare is NOT used.
 */

import Foundation
import UIKit
import os.log

// MARK: - Power State

/// Represents the current power state of the device, reported to the Rust daemon.
struct PowerState: Codable {
    /// Battery level as percentage (0-100), -1 if unknown
    let batteryLevel: Int
    /// Whether the device is charging or fully charged
    let isCharging: Bool
    /// Whether the screen is currently on (proxied by brightness)
    let isScreenOn: Bool
    /// Whether the device is in low power mode
    let isLowPowerMode: Bool
    /// Current NAIN detection interval in seconds
    let nainIntervalSeconds: Int
    /// Current scan mode
    let scanMode: ScanMode
    /// Timestamp of this report
    let timestamp: Date

    enum ScanMode: String, Codable {
        case full        // Charging: full NAIN detection, all channels active
        case normal      // Normal battery: standard intervals
        case reduced     // Low battery: reduced scanning, fewer channels
        case minimal     // Critical: minimal activity, rely on push
        case background  // App backgrounded: limited scanning
    }
}

// MARK: - Battery Thresholds

enum BatteryThreshold {
    static let low: Int = 20
    static let critical: Int = 10
}

// MARK: - NAIN Intervals

enum NainInterval {
    static let charging: Int = 30    // Full detection: 30 seconds
    static let normal: Int = 60      // Standard: 60 seconds
    static let lowBattery: Int = 300 // Reduced: 5 minutes
    static let critical: Int = 0     // Minimal: rely on push notifications only
}

// MARK: - Rust Daemon IPC for Power State

/// Protocol for reporting power state changes to the Rust daemon.
protocol PowerStateReporter {
    func reportPowerState(_ state: PowerState)
}

/// Default implementation using Unix domain socket IPC to the Rust daemon.
class DaemonPowerReporter: PowerStateReporter {
    private let logger = Logger(subsystem: "org.micafp.unifiedshield", category: "PowerReporter")

    // IPC socket path (same as used by the PacketTunnelProvider)
    private let daemonSocketPath = "/var/run/shield-daemon.sock"
    private var socketfd: Int32 = -1
    private let queue = DispatchQueue(label: "org.micafp.unifiedshield.power-ipc", qos: .utility)

    // IPC message type for power state
    private let MSG_TYPE_POWER_STATE: UInt32 = 0x10

    func reportPowerState(_ state: PowerState) {
        queue.async { [weak self] in
            guard let self = self else { return }

            do {
                let jsonData = try JSONEncoder().encode(state)
                self.sendPowerStateData(jsonData)
            } catch {
                self.logger.error("Failed to encode power state: \(error.localizedDescription)")
            }
        }
    }

    private func sendPowerStateData(_ data: Data) {
        // Connect if not already connected
        if socketfd < 0 {
            socketfd = socket(AF_UNIX, SOCK_STREAM, 0)
            guard socketfd >= 0 else {
                logger.error("Failed to create power state IPC socket")
                return
            }

            var addr = sockaddr_un()
            addr.sun_family = sa_family_t(AF_UNIX)
            let pathData = daemonSocketPath.utf8CString
            _ = withUnsafeMutableBytes(of: &addr.sun_path) { dest in
                pathData.withUnsafeBytes { src in
                    memcpy(dest.baseAddress, src.baseAddress, min(dest.count, src.count))
                }
            }

            let connectResult = withUnsafePointer(to: &addr) { ptr in
                ptr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockaddrPtr in
                    connect(socketfd, sockaddrPtr, socklen_t(MemoryLayout<sockaddr_un>.size))
                }
            }

            if connectResult < 0 {
                logger.error("Failed to connect to daemon for power state reporting")
                close(socketfd)
                socketfd = -1
                return
            }
        }

        // Send IPC message: [type] [length] [payload]
        let headerSize = 8
        var header = Data(capacity: headerSize)
        var msgType: UInt32 = MSG_TYPE_POWER_STATE
        var payloadLen: UInt32 = UInt32(data.count)
        header.append(contentsOf: withUnsafeBytes(of: &msgType) { Array($0) })
        header.append(contentsOf: withUnsafeBytes(of: &payloadLen) { Array($0) })

        let message = header + data
        let written = message.withUnsafeBytes { ptr in
            write(socketfd, ptr.baseAddress, message.count)
        }

        if written < 0 {
            logger.error("Failed to send power state to daemon")
            close(socketfd)
            socketfd = -1
        }
    }

    func disconnect() {
        if socketfd >= 0 {
            close(socketfd)
            socketfd = -1
        }
    }
}

// MARK: - Battery Manager

/// iOS battery management and power state reporting.
///
/// Monitors battery level, charging state, screen state, and low power mode.
/// Reports power state changes to the Rust daemon and adjusts subsystem
/// behavior based on the current power conditions.
///
/// Usage:
///   1. Call `startMonitoring()` early in the app lifecycle
///   2. Integrate with UIApplicationDelegate lifecycle methods
///   3. The manager automatically reports state changes to the Rust daemon
class BatteryManager {

    // MARK: - Properties

    private let logger = Logger(subsystem: "org.micafp.unifiedshield", category: "BatteryManager")
    private let powerReporter: PowerStateReporter

    // Monitoring state
    private var isMonitoring = false
    private var appState: UIApplication.State = .active

    // Current power state
    private(set) var currentPowerState: PowerState?

    // Reporting interval
    private let reportingInterval: TimeInterval = 30.0 // 30 seconds
    private var reportingTimer: Timer?

    // Brightness monitoring (screen on/off proxy)
    private var lastBrightness: CGFloat = 0
    private var brightnessCheckInterval: TimeInterval = 5.0
    private var brightnessTimer: Timer?

    // Background task identifiers
    private var backgroundRefreshTaskIdentifier: UIBackgroundTaskIdentifier = .invalid
    private var backgroundProcessingTaskIdentifier: UIBackgroundTaskIdentifier = .invalid

    // Callbacks for state changes
    var onPowerStateChanged: ((PowerState) -> Void)?
    var onBatteryCritical: (() -> Void)?

    // MARK: - Initialization

    init(powerReporter: PowerStateReporter? = nil) {
        self.powerReporter = powerReporter ?? DaemonPowerReporter()
    }

    deinit {
        stopMonitoring()
    }

    // MARK: - Public API

    /// Start monitoring battery state and reporting to the Rust daemon.
    func startMonitoring() {
        guard !isMonitoring else { return }

        logger.info("Starting battery monitoring")
        isMonitoring = true

        // Enable battery monitoring on UIDevice
        UIDevice.current.isBatteryMonitoringEnabled = true

        // Register for notifications
        registerNotifications()

        // Start periodic reporting
        startPeriodicReporting()

        // Start brightness monitoring
        startBrightnessMonitoring()

        // Perform initial state update
        updatePowerState()
    }

    /// Stop monitoring battery state.
    func stopMonitoring() {
        guard isMonitoring else { return }

        logger.info("Stopping battery monitoring")
        isMonitoring = false

        // Stop timers
        reportingTimer?.invalidate()
        reportingTimer = nil
        brightnessTimer?.invalidate()
        brightnessTimer = nil

        // Remove notification observers
        NotificationCenter.default.removeObserver(self)

        // Disable battery monitoring
        UIDevice.current.isBatteryMonitoringEnabled = false

        // Disconnect IPC
        if let reporter = powerReporter as? DaemonPowerReporter {
            reporter.disconnect()
        }
    }

    /// Called from UIApplicationDelegate when app enters foreground.
    func applicationDidBecomeActive() {
        appState = .active
        logger.info("App became active")

        // Re-enable idle timer disable if we need to keep the screen on
        // (only during active scanning)
        updatePowerState()
    }

    /// Called from UIApplicationDelegate when app enters background.
    func applicationWillResignActive() {
        appState = .background
        logger.info("App will resign active")

        // Allow screen to turn off normally when backgrounded
        UIApplication.shared.isIdleTimerDisabled = false

        updatePowerState()
        scheduleBackgroundTasks()
    }

    /// Called from UIApplicationDelegate when app enters foreground.
    func applicationWillEnterForeground() {
        logger.info("App will enter foreground")
        updatePowerState()
    }

    /// Called from UIApplicationDelegate when app enters background.
    func applicationDidEnterBackground() {
        appState = .background
        logger.info("App did enter background")
        scheduleBackgroundTasks()
    }

    /// Get the current battery level as a percentage (0-100), or -1 if unknown.
    func getBatteryLevel() -> Int {
        let level = UIDevice.current.batteryLevel
        return level >= 0 ? Int(level * 100) : -1
    }

    /// Check if the device is currently charging.
    func isCharging() -> Bool {
        let state = UIDevice.current.batteryState
        return state == .charging || state == .full
    }

    /// Check if low power mode is currently active.
    func isLowPowerMode() -> Bool {
        if #available(iOS 13.0, *) {
            return ProcessInfo.processInfo.isLowPowerModeEnabled
        }
        return false
    }

    /// Determine if the screen is likely on based on brightness.
    func isScreenLikelyOn() -> Bool {
        // UIScreen.main.brightness > 0 is a reasonable proxy for screen being on
        // On iOS, there's no direct API for screen on/off state
        let brightness = UIScreen.main.brightness
        return brightness > 0.01
    }

    /// Prevent the screen from auto-locking during active operations.
    func preventScreenLock() {
        UIApplication.shared.isIdleTimerDisabled = true
        logger.debug("Screen auto-lock disabled")
    }

    /// Allow the screen to auto-lock normally.
    func allowScreenLock() {
        UIApplication.shared.isIdleTimerDisabled = false
        logger.debug("Screen auto-lock enabled")
    }

    // MARK: - Power State Calculation

    /// Calculate the NAIN detection interval based on current power conditions.
    func calculateNainInterval(batteryLevel: Int, isCharging: Bool, isLowPower: Bool) -> Int {
        if isCharging {
            return NainInterval.charging
        }
        if isLowPower || batteryLevel <= BatteryThreshold.critical {
            return NainInterval.critical
        }
        if batteryLevel <= BatteryThreshold.low {
            return NainInterval.lowBattery
        }
        return NainInterval.normal
    }

    /// Calculate the scan mode based on current power conditions.
    func calculateScanMode(
        batteryLevel: Int,
        isCharging: Bool,
        isLowPower: Bool,
        isScreenOn: Bool,
        appState: UIApplication.State
    ) -> PowerState.ScanMode {
        if isCharging {
            return .full
        }
        if isLowPower || batteryLevel <= BatteryThreshold.critical {
            return .minimal
        }
        if batteryLevel <= BatteryThreshold.low {
            return .reduced
        }
        if appState != .active || !isScreenOn {
            return .background
        }
        return .normal
    }

    // MARK: - Notification Registration

    private func registerNotifications() {
        let notificationCenter = NotificationCenter.default

        // Battery level changes
        notificationCenter.addObserver(
            self,
            selector: #selector(batteryLevelDidChange),
            name: UIDevice.batteryLevelDidChangeNotification,
            object: nil
        )

        // Battery state changes (charging/discharging)
        notificationCenter.addObserver(
            self,
            selector: #selector(batteryStateDidChange),
            name: UIDevice.batteryStateDidChangeNotification,
            object: nil
        )

        // Low power mode changes
        if #available(iOS 13.0, *) {
            notificationCenter.addObserver(
                self,
                selector: #selector(lowPowerModeDidChange),
                name: ProcessInfo.powerStateDidChangeNotification,
                object: nil
            )
        }

        // App lifecycle
        notificationCenter.addObserver(
            self,
            selector: #selector(appDidBecomeActive),
            name: UIApplication.didBecomeActiveNotification,
            object: nil
        )

        notificationCenter.addObserver(
            self,
            selector: #selector(appWillResignActive),
            name: UIApplication.willResignActiveNotification,
            object: nil
        )

        notificationCenter.addObserver(
            self,
            selector: #selector(appDidEnterBackground),
            name: UIApplication.didEnterBackgroundNotification,
            object: nil
        )

        notificationCenter.addObserver(
            self,
            selector: #selector(appWillEnterForeground),
            name: UIApplication.willEnterForegroundNotification,
            object: nil
        )

        // Screen brightness changes (proxy for screen on/off)
        notificationCenter.addObserver(
            self,
            selector: #selector(screenBrightnessDidChange),
            name: UIScreen.brightnessDidChangeNotification,
            object: nil
        )
    }

    // MARK: - Notification Handlers

    @objc private func batteryLevelDidChange() {
        logger.debug("Battery level changed: \(getBatteryLevel())%")
        updatePowerState()

        // Check for critical battery
        let level = getBatteryLevel()
        if level >= 0 && level <= BatteryThreshold.critical {
            onBatteryCritical?()
        }
    }

    @objc private func batteryStateDidChange() {
        let state = UIDevice.current.batteryState
        logger.debug("Battery state changed: \(state.rawValue) (charging: \(isCharging()))")
        updatePowerState()
    }

    @objc private func lowPowerModeDidChange() {
        logger.debug("Low power mode changed: \(isLowPowerMode())")
        updatePowerState()
    }

    @objc private func appDidBecomeActive() {
        appState = .active
        updatePowerState()
    }

    @objc private func appWillResignActive() {
        appState = .background
        UIApplication.shared.isIdleTimerDisabled = false
        updatePowerState()
    }

    @objc private func appDidEnterBackground() {
        appState = .background
        scheduleBackgroundTasks()
    }

    @objc private func appWillEnterForeground() {
        updatePowerState()
    }

    @objc private func screenBrightnessDidChange() {
        let brightness = UIScreen.main.brightness
        let wasScreenOn = lastBrightness > 0.01
        let isScreenOn = brightness > 0.01
        lastBrightness = brightness

        if wasScreenOn != isScreenOn {
            logger.debug("Screen state changed: \(isScreenOn ? "ON" : "OFF") (brightness: \(brightness))")
            updatePowerState()
        }
    }

    // MARK: - Power State Update

    /// Update the current power state and report it to the Rust daemon.
    private func updatePowerState() {
        let batteryLevel = getBatteryLevel()
        let charging = isCharging()
        let screenOn = isScreenLikelyOn()
        let lowPower = isLowPowerMode()

        let nainInterval = calculateNainInterval(
            batteryLevel: batteryLevel,
            isCharging: charging,
            isLowPower: lowPower
        )

        let scanMode = calculateScanMode(
            batteryLevel: batteryLevel,
            isCharging: charging,
            isLowPower: lowPower,
            isScreenOn: screenOn,
            appState: appState
        )

        let state = PowerState(
            batteryLevel: batteryLevel,
            isCharging: charging,
            isScreenOn: screenOn,
            isLowPowerMode: lowPower,
            nainIntervalSeconds: nainInterval,
            scanMode: scanMode,
            timestamp: Date()
        )

        currentPowerState = state

        // Report to Rust daemon
        powerReporter.reportPowerState(state)

        // Notify local observers
        onPowerStateChanged?(state)

        // Manage idle timer based on state
        manageIdleTimer(state: state)

        logger.debug("Power state updated: level=\(batteryLevel)%, charging=\(charging), " +
                     "screen=\(screenOn), lowPower=\(lowPower), interval=\(nainInterval)s, " +
                     "mode=\(scanMode.rawValue)")
    }

    /// Manage the idle timer based on current power state.
    private func manageIdleTimer(state: PowerState) {
        switch state.scanMode {
        case .full, .normal:
            // When actively using the app and scanning, prevent screen lock
            if appState == .active {
                preventScreenLock()
            }
        case .reduced, .background, .minimal:
            // Allow screen to turn off to save battery
            allowScreenLock()
        }
    }

    // MARK: - Periodic Reporting

    private func startPeriodicReporting() {
        reportingTimer?.invalidate()

        reportingTimer = Timer.scheduledTimer(
            withTimeInterval: reportingInterval,
            repeats: true
        ) { [weak self] _ in
            self?.updatePowerState()
        }

        // Also run on a dispatch queue for more reliable background operation
        schedulePeriodicReportingOnQueue()
    }

    private func schedulePeriodicReportingOnQueue() {
        let queue = DispatchQueue.global(qos: .utility)

        queue.asyncAfter(deadline: .now() + reportingInterval) { [weak self] in
            guard let self = self, self.isMonitoring else { return }
            self.updatePowerState()
            self.schedulePeriodicReportingOnQueue()
        }
    }

    // MARK: - Brightness Monitoring

    private func startBrightnessMonitoring() {
        lastBrightness = UIScreen.main.brightness

        brightnessTimer?.invalidate()

        brightnessTimer = Timer.scheduledTimer(
            withTimeInterval: brightnessCheckInterval,
            repeats: true
        ) { [weak self] _ in
            guard let self = self else { return }
            let currentBrightness = UIScreen.main.brightness
            let wasScreenOn = self.lastBrightness > 0.01
            let isScreenOn = currentBrightness > 0.01

            if wasScreenOn != isScreenOn {
                self.lastBrightness = currentBrightness
                self.logger.debug("Screen state detected via brightness poll: \(isScreenOn ? "ON" : "OFF")")
                self.updatePowerState()
            }
        }
    }

    // MARK: - Background Task Scheduling

    /// Schedule BGTaskScheduler tasks for background processing.
    private func scheduleBackgroundTasks() {
        // Schedule a BGAppRefreshTask for periodic connectivity checks
        let refreshRequest = BGAppRefreshTaskRequest(identifier: "org.micafp.unifiedshield.power-check")
        refreshRequest.earliestBeginDate = Date(timeIntervalSinceNow: 15 * 60) // 15 minutes

        do {
            try BGTaskScheduler.shared.submit(refreshRequest)
            logger.debug("Scheduled background power check task")
        } catch {
            logger.warning("Failed to schedule background power check: \(error.localizedDescription)")
        }

        // Schedule a BGProcessingTask for heavy work when charging
        let processingRequest = BGProcessingTaskRequest(identifier: "org.micafp.unifiedshield.power-processing")
        processingRequest.earliestBeginDate = Date(timeIntervalSinceNow: 30 * 60) // 30 minutes
        processingRequest.requiresNetworkConnectivity = true
        processingRequest.requiresExternalPower = true  // Only when charging

        do {
            try BGTaskScheduler.shared.submit(processingRequest)
            logger.debug("Scheduled background processing task")
        } catch {
            logger.warning("Failed to schedule background processing: \(error.localizedDescription)")
        }
    }

    // MARK: - Adaptive Behavior Helpers

    /// Get recommended subsystem configuration based on current power state.
    func getRecommendedConfiguration() -> SubsystemConfiguration {
        guard let state = currentPowerState else {
            return SubsystemConfiguration.default
        }

        return SubsystemConfiguration(
            nainIntervalSeconds: state.nainIntervalSeconds,
            acousticEnabled: state.scanMode != .minimal,
            ntpEnabled: state.scanMode != .minimal || state.batteryLevel <= BatteryThreshold.critical,
            pushNotificationsEnabled: true,  // Always enabled - minimal battery impact
            nanMeshEnabled: state.scanMode == .full || state.scanMode == .normal,
            smsBootstrapEnabled: true,  // Passive - no battery impact unless received
            backgroundScanInterval: state.scanMode == .full ? 30 :
                                   state.scanMode == .normal ? 60 :
                                   state.scanMode == .reduced ? 300 : 0,
            maxConcurrentConnections: state.scanMode == .full ? 8 :
                                     state.scanMode == .normal ? 4 :
                                     state.scanMode == .reduced ? 2 : 1
        )
    }

    /// Check if the current power state allows for battery-intensive operations.
    func canPerformHeavyOperation() -> Bool {
        guard let state = currentPowerState else { return false }
        return state.isCharging || (state.batteryLevel > BatteryThreshold.low && !state.isLowPowerMode)
    }

    /// Check if acoustic listening should be active.
    func shouldListenAcoustically() -> Bool {
        guard let state = currentPowerState else { return false }
        // Don't listen acoustically in minimal mode (critical battery)
        return state.scanMode != .minimal
    }

    /// Get the recommended listening duration for the acoustic receiver.
    func getAcousticListenDuration() -> TimeInterval {
        guard let state = currentPowerState else { return 5.0 }

        switch state.scanMode {
        case .full:
            return 10.0
        case .normal:
            return 10.0
        case .reduced:
            return 5.0
        case .background:
            return 5.0   // Passive mode: 5s listen
        case .minimal:
            return 0     // No listening
        }
    }

    /// Get the recommended pause duration between acoustic listening windows.
    func getAcousticPauseDuration() -> TimeInterval {
        guard let state = currentPowerState else { return 55.0 }

        switch state.scanMode {
        case .full:
            return 5.0
        case .normal:
            return 20.0
        case .reduced:
            return 55.0
        case .background:
            return 55.0  // Passive mode: 55s pause
        case .minimal:
            return .infinity // No listening
        }
    }
}

// MARK: - Subsystem Configuration

/// Configuration for all Shield subsystems, adjusted based on power state.
struct SubsystemConfiguration {
    let nainIntervalSeconds: Int
    let acousticEnabled: Bool
    let ntpEnabled: Bool
    let pushNotificationsEnabled: Bool
    let nanMeshEnabled: Bool
    let smsBootstrapEnabled: Bool
    let backgroundScanInterval: Int
    let maxConcurrentConnections: Int

    static let `default` = SubsystemConfiguration(
        nainIntervalSeconds: NainInterval.normal,
        acousticEnabled: true,
        ntpEnabled: true,
        pushNotificationsEnabled: true,
        nanMeshEnabled: true,
        smsBootstrapEnabled: true,
        backgroundScanInterval: 60,
        maxConcurrentConnections: 4
    )
}

// MARK: - UIApplicationDelegate Integration

/// Helper class for integrating BatteryManager with UIApplicationDelegate.
///
/// Usage in AppDelegate:
/// ```
/// class AppDelegate: UIResponder, UIApplicationDelegate {
///     let batteryIntegration = AppDelegateBatteryIntegration()
///
///     func applicationDidBecomeActive(_ application: UIApplication) {
///         batteryIntegration.applicationDidBecomeActive()
///     }
///
///     func applicationWillResignActive(_ application: UIApplication) {
///         batteryIntegration.applicationWillResignActive()
///     }
/// }
/// ```
class AppDelegateBatteryIntegration {

    private let batteryManager = BatteryManager()

    init() {
        // Start monitoring immediately
        batteryManager.startMonitoring()

        // Register background task handlers
        registerBackgroundTaskHandlers()

        // Set up critical battery callback
        batteryManager.onBatteryCritical = { [weak self] in
            self?.handleCriticalBattery()
        }
    }

    // MARK: - Lifecycle Methods

    func applicationDidBecomeActive() {
        batteryManager.applicationDidBecomeActive()
    }

    func applicationWillResignActive() {
        batteryManager.applicationWillResignActive()
    }

    func applicationDidEnterBackground() {
        batteryManager.applicationDidEnterBackground()
    }

    func applicationWillEnterForeground() {
        batteryManager.applicationWillEnterForeground()
    }

    // MARK: - Background Task Registration

    private func registerBackgroundTaskHandlers() {
        // Register BGAppRefreshTask handler
        BGTaskScheduler.shared.register(
            forTaskWithIdentifier: "org.micafp.unifiedshield.power-check",
            using: nil
        ) { task in
            self.handlePowerCheckTask(task as! BGAppRefreshTask)
        }

        // Register BGProcessingTask handler
        BGTaskScheduler.shared.register(
            forTaskWithIdentifier: "org.micafp.unifiedshield.power-processing",
            using: nil
        ) { task in
            self.handleProcessingTask(task as! BGProcessingTask)
        }
    }

    private func handlePowerCheckTask(_ task: BGAppRefreshTask) {
        // Schedule the next task
        let nextRequest = BGAppRefreshTaskRequest(identifier: "org.micafp.unifiedshield.power-check")
        nextRequest.earliestBeginDate = Date(timeIntervalSinceNow: 15 * 60)
        try? BGTaskScheduler.shared.submit(nextRequest)

        task.expirationHandler = {
            task.setTaskCompleted(success: false)
        }

        // Update power state and report to daemon
        batteryManager.updatePowerState()

        task.setTaskCompleted(success: true)
    }

    private func handleProcessingTask(_ task: BGProcessingTask) {
        // Schedule next processing task
        let nextRequest = BGProcessingTaskRequest(identifier: "org.micafp.unifiedshield.power-processing")
        nextRequest.earliestBeginDate = Date(timeIntervalSinceNow: 30 * 60)
        nextRequest.requiresExternalPower = true
        nextRequest.requiresNetworkConnectivity = true
        try? BGTaskScheduler.shared.submit(nextRequest)

        task.expirationHandler = {
            task.setTaskCompleted(success: false)
        }

        // Heavy processing work when charging
        // This is when we can do things like key rotation, peer discovery, etc.
        batteryManager.updatePowerState()

        task.setTaskCompleted(success: true)
    }

    // MARK: - Critical Battery Handling

    private func handleCriticalBattery() {
        Logger(subsystem: "org.micafp.unifiedshield", category: "BatteryIntegration")
            .warning("Critical battery level detected — switching to minimal mode")

        // In critical battery mode, we rely entirely on push notifications
        // and passive channels (SMS, NTP) that don't require active scanning
    }

    // MARK: - Access

    /// Get the shared BatteryManager instance.
    var manager: BatteryManager {
        return batteryManager
    }
}
