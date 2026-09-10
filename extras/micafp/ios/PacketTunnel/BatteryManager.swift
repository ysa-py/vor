import Foundation
import UIKit
import BackgroundTasks

/**
 * BatteryManager — iOS Battery Manager
 *
 * Monitors battery level and charging state via UIDevice, detects low power mode
 * via ProcessInfo, and computes a PowerState that drives adaptive behavior
 * throughout the app. Integrates BGTaskScheduler for background processing
 * with adaptive intervals based on the current power state.
 *
 * Communicates the current power state to the Rust daemon via FFI so the
 * tunnel can adjust its activity level accordingly.
 */
class BatteryManager {

    // MARK: - Power State Definition

    enum PowerState: Int {
        case screenOn = 0         // Full activity: VPN + NAN + acoustic + probing
        case screenOffLight = 1   // Reduced: VPN + NAN, no acoustic
        case screenOffDeep = 2    // Minimal: VPN tunnel only
        case charging = 3         // Maximum: all services + frequent probing

        var label: String {
            switch self {
            case .screenOn: return "screen_on"
            case .screenOffLight: return "screen_off_light"
            case .screenOffDeep: return "screen_off_deep"
            case .charging: return "charging"
            }
        }
    }

    // MARK: - Configuration

    /// Battery threshold for ultra-low-power mode
    private let ultraLowBatteryThreshold: Float = 0.15

    /// Battery threshold for reduced activity
    private let lowBatteryThreshold: Float = 0.25

    /// Battery threshold for medium conservation
    private let mediumBatteryThreshold: Float = 0.50

    /// How often to check battery state (seconds)
    private let batteryCheckInterval: TimeInterval = 30.0

    /// Background task identifiers
    private let refreshTaskId = "com.shield.battery.refresh"
    private let processingTaskId = "com.shield.battery.processing"

    // MARK: - State

    private(set) var batteryLevel: Float = 1.0
    private(set) var isCharging: Bool = false
    private(set) var isLowPowerMode: Bool = false
    private(set) var currentPowerState: PowerState = .screenOn
    private(set) var batteryHistory: [(timestamp: Date, level: Float, state: PowerState)] = []

    weak var delegate: BatteryManagerDelegate?

    // Rust FFI
    private typealias RustSetPowerState = @convention(c) (Int32) -> Void
    private var rustSetPowerState: RustSetPowerState?

    // Monitoring timer
    private var monitoringTimer: Timer?

    // Notification observers
    private var observers: [NSObjectProtocol] = []

    // MARK: - Initialization

    init() {
        loadRustFFI()
        UIDevice.current.isBatteryMonitoringEnabled = true
        readInitialState()
    }

    deinit {
        stopMonitoring()
        UIDevice.current.isBatteryMonitoringEnabled = false
    }

    // MARK: - Rust FFI Loading

    private func loadRustFFI() {
        let libraryPath = Bundle.main.bundlePath + "/Frameworks/libshield_native.dylib"

        guard let handle = dlopen(libraryPath, RTLD_NOW) else {
            NSLog("[BatteryManager] Failed to load Rust library: \(String(cString: dlerror()))")
            return
        }

        rustSetPowerState = unsafeBitCast(
            dlsym(handle, "shield_set_power_state"),
            to: RustSetPowerState.self
        )

        if rustSetPowerState != nil {
            NSLog("[BatteryManager] Rust FFI power state function loaded")
        }
    }

    // MARK: - Start/Stop Monitoring

    /// Start monitoring battery state changes.
    func startMonitoring() {
        NSLog("[BatteryManager] Starting battery monitoring")

        // Register for battery state change notifications
        let batteryStateObserver = NotificationCenter.default.addObserver(
            forName: UIDevice.batteryStateDidChangeNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.handleBatteryStateChange()
        }
        observers.append(batteryStateObserver)

        let batteryLevelObserver = NotificationCenter.default.addObserver(
            forName: UIDevice.batteryLevelDidChangeNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.handleBatteryLevelChange()
        }
        observers.append(batteryLevelObserver)

        // Monitor low power mode changes
        let lowPowerObserver = NotificationCenter.default.addObserver(
            forName: NSNotification.Name.NSProcessInfoPowerStateDidChange,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.handleLowPowerModeChange()
        }
        observers.append(lowPowerObserver)

        // Monitor app foreground/background transitions
        let foregroundObserver = NotificationCenter.default.addObserver(
            forName: UIApplication.willEnterForegroundNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.handleAppForeground()
        }
        observers.append(foregroundObserver)

        let backgroundObserver = NotificationCenter.default.addObserver(
            forName: UIApplication.didEnterBackgroundNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.handleAppBackground()
        }
        observers.append(backgroundObserver)

        // Start periodic battery check timer
        monitoringTimer = Timer.scheduledTimer(
            withTimeInterval: batteryCheckInterval,
            repeats: true
        ) { [weak self] _ in
            self?.periodicBatteryCheck()
        }

        // Register background tasks
        registerBackgroundTasks()

        // Initial state evaluation
        evaluatePowerState()
    }

    /// Stop monitoring battery state changes.
    func stopMonitoring() {
        NSLog("[BatteryManager] Stopping battery monitoring")

        monitoringTimer?.invalidate()
        monitoringTimer = nil

        for observer in observers {
            NotificationCenter.default.removeObserver(observer)
        }
        observers.removeAll()

        BGTaskScheduler.shared.cancelAllTaskRequests()
    }

    // MARK: - Initial State

    private func readInitialState() {
        let device = UIDevice.current

        batteryLevel = device.batteryLevel >= 0 ? device.batteryLevel : 1.0

        let state = device.batteryState
        isCharging = (state == .charging || state == .full)

        isLowPowerMode = ProcessInfo.processInfo.isLowPowerModeEnabled

        NSLog("[BatteryManager] Initial state: level=\(batteryLevel), charging=\(isCharging), lowPower=\(isLowPowerMode)")
    }

    // MARK: - State Change Handlers

    private func handleBatteryStateChange() {
        let state = UIDevice.current.batteryState
        let wasCharging = isCharging
        isCharging = (state == .charging || state == .full)

        if wasCharging != isCharging {
            NSLog("[BatteryManager] Charging state changed: \(isCharging)")
            evaluatePowerState()
        }
    }

    private func handleBatteryLevelChange() {
        let newLevel = UIDevice.current.batteryLevel
        if newLevel >= 0 && newLevel != batteryLevel {
            let oldLevel = batteryLevel
            batteryLevel = newLevel

            NSLog("[BatteryManager] Battery level changed: \(oldLevel) → \(newLevel)")

            // Record battery history
            recordBatteryHistory()

            // Check for threshold crossings
            if oldLevel > ultraLowBatteryThreshold && newLevel <= ultraLowBatteryThreshold {
                NSLog("[BatteryManager] ⚠️ Ultra-low battery threshold crossed!")
            } else if oldLevel > lowBatteryThreshold && newLevel <= lowBatteryThreshold {
                NSLog("[BatteryManager] ⚠️ Low battery threshold crossed")
            }

            evaluatePowerState()
        }
    }

    private func handleLowPowerModeChange() {
        let wasLowPower = isLowPowerMode
        isLowPowerMode = ProcessInfo.processInfo.isLowPowerModeEnabled

        if wasLowPower != isLowPowerMode {
            NSLog("[BatteryManager] Low power mode changed: \(isLowPowerMode)")
            evaluatePowerState()
        }
    }

    private func handleAppForeground() {
        NSLog("[BatteryManager] App entered foreground")
        evaluatePowerState()
    }

    private func handleAppBackground() {
        NSLog("[BatteryManager] App entered background")
        evaluatePowerState()
        scheduleBackgroundTasks()
    }

    private func periodicBatteryCheck() {
        readInitialState()
        evaluatePowerState()
    }

    // MARK: - Power State Evaluation

    private func evaluatePowerState() {
        let previousState = currentPowerState
        let isAppForeground = UIApplication.shared.applicationState == .active

        // Determine power state based on all factors
        if isCharging {
            currentPowerState = .charging
        } else if isAppForeground && !isLowPowerMode {
            if batteryLevel <= ultraLowBatteryThreshold {
                currentPowerState = .screenOffDeep
            } else {
                currentPowerState = .screenOn
            }
        } else if isLowPowerMode || batteryLevel <= lowBatteryThreshold {
            if batteryLevel <= ultraLowBatteryThreshold {
                currentPowerState = .screenOffDeep
            } else {
                currentPowerState = .screenOffLight
            }
        } else {
            currentPowerState = .screenOffLight
        }

        if currentPowerState != previousState {
            NSLog("[BatteryManager] Power state: \(previousState.label) → \(currentPowerState.label) " +
                  "(battery: \(Int(batteryLevel * 100))%, charging: \(isCharging), lowPower: \(isLowPowerMode))")

            // Notify delegate
            delegate?.batteryStateDidChange(
                level: Int(batteryLevel * 100),
                isCharging: isCharging,
                powerState: currentPowerState.rawValue
            )

            // Notify Rust daemon
            notifyRustDaemon()

            // Update background task scheduling
            scheduleBackgroundTasks()
        }
    }

    // MARK: - Rust Daemon Communication

    private func notifyRustDaemon() {
        guard let setPowerState = rustSetPowerState else { return }
        setPowerState(Int32(currentPowerState.rawValue))
        NSLog("[BatteryManager] Notified Rust daemon: powerState=\(currentPowerState.rawValue)")
    }

    // MARK: - Battery History

    private func recordBatteryHistory() {
        let entry = (timestamp: Date(), level: batteryLevel, state: currentPowerState)
        batteryHistory.append(entry)

        // Keep only the last 100 entries
        if batteryHistory.count > 100 {
            batteryHistory.removeFirst(batteryHistory.count - 100)
        }
    }

    /// Get battery drain rate (percentage per hour) based on history.
    var drainRatePerHour: Float? {
        guard batteryHistory.count >= 2 else { return nil }

        let recent = batteryHistory.suffix(min(10, batteryHistory.count))
        guard let first = recent.first, let last = recent.last else { return nil }

        let timeDiff = last.timestamp.timeIntervalSince(first.timestamp)
        guard timeDiff > 0 else { return nil }

        let levelDiff = first.level - last.level
        let hoursDiff = Float(timeDiff / 3600.0)

        return levelDiff / hoursDiff
    }

    /// Estimate remaining time until battery reaches the ultra-low threshold.
    var estimatedTimeUntilUltraLow: TimeInterval? {
        guard let drainRate = drainRatePerHour, drainRate > 0 else { return nil }
        let remainingLevel = batteryLevel - ultraLowBatteryThreshold
        guard remainingLevel > 0 else { return 0 }
        let hoursRemaining = remainingLevel / drainRate
        return TimeInterval(hoursRemaining * 3600)
    }

    // MARK: - Background Task Scheduling

    private func registerBackgroundTasks() {
        // Register for BGAppRefreshTask
        BGTaskScheduler.shared.register(
            forTaskWithIdentifier: refreshTaskId,
            using: nil
        ) { [weak self] task in
            self?.handleRefreshTask(task as! BGAppRefreshTask)
        }

        // Register for BGProcessingTask
        BGTaskScheduler.shared.register(
            forTaskWithIdentifier: processingTaskId,
            using: nil
        ) { [weak self] task in
            self?.handleProcessingTask(task as! BGProcessingTask)
        }
    }

    private func scheduleBackgroundTasks() {
        // Adaptive intervals based on power state
        let (refreshInterval, processingInterval) = adaptiveIntervals()

        // Schedule app refresh task
        let refreshRequest = BGAppRefreshTaskRequest(identifier: refreshTaskId)
        refreshRequest.earliestBeginDate = Date(timeIntervalSinceNow: refreshInterval)

        do {
            try BGTaskScheduler.shared.submit(refreshRequest)
            NSLog("[BatteryManager] BGAppRefreshTask scheduled (interval: \(refreshInterval)s)")
        } catch {
            NSLog("[BatteryManager] Failed to schedule BGAppRefreshTask: \(error)")
        }

        // Schedule processing task (only when charging + idle)
        if isCharging {
            let processingRequest = BGProcessingTaskRequest(identifier: processingTaskId)
            processingRequest.earliestBeginDate = Date(timeIntervalSinceNow: processingInterval)
            processingRequest.requiresNetworkConnectivity = true
            processingRequest.requiresExternalPower = true
            processingRequest.requiresDeviceConnected = true

            do {
                try BGTaskScheduler.shared.submit(processingRequest)
                NSLog("[BatteryManager] BGProcessingTask scheduled (interval: \(processingInterval)s)")
            } catch {
                NSLog("[BatteryManager] Failed to schedule BGProcessingTask: \(error)")
            }
        }
    }

    /// Get adaptive intervals for background tasks based on current power state.
    private func adaptiveIntervals() -> (refresh: TimeInterval, processing: TimeInterval) {
        switch currentPowerState {
        case .charging:
            return (refresh: 15 * 60, processing: 30 * 60)    // 15 min / 30 min
        case .screenOn:
            return (refresh: 15 * 60, processing: 60 * 60)    // 15 min / 60 min
        case .screenOffLight:
            return (refresh: 30 * 60, processing: 120 * 60)   // 30 min / 2 hours
        case .screenOffDeep:
            return (refresh: 60 * 60, processing: 240 * 60)   // 60 min / 4 hours
        }
    }

    private func handleRefreshTask(_ task: BGAppRefreshTask) {
        // Schedule the next refresh
        scheduleBackgroundTasks()

        task.expirationHandler = {
            NSLog("[BatteryManager] BGAppRefreshTask expired")
        }

        NSLog("[BatteryManager] Handling BGAppRefreshTask")

        // Read current battery state
        readInitialState()
        evaluatePowerState()

        // Notify Rust daemon to perform lightweight health check
        notifyRustDaemon()

        task.setTaskCompleted(success: true)
    }

    private func handleProcessingTask(_ task: BGProcessingTask) {
        scheduleBackgroundTasks()

        task.expirationHandler = {
            NSLog("[BatteryManager] BGProcessingTask expired")
        }

        NSLog("[BatteryManager] Handling BGProcessingTask")

        // Heavy operations: endpoint discovery, key rotation, database maintenance
        // These run only when the device is charging and idle
        notifyRustDaemon()

        task.setTaskCompleted(success: true)
    }

    // MARK: - Public API

    /// Whether the acoustic channel should be enabled based on battery state.
    func shouldEnableAcousticChannel() -> Bool {
        switch currentPowerState {
        case .charging:
            return true
        case .screenOn:
            return batteryLevel > ultraLowBatteryThreshold
        case .screenOffLight:
            return false
        case .screenOffDeep:
            return false
        }
    }

    /// Whether NAN (WiFi Aware) should be enabled based on battery state.
    func shouldEnableNan() -> Bool {
        switch currentPowerState {
        case .charging, .screenOn:
            return batteryLevel > ultraLowBatteryThreshold
        case .screenOffLight:
            return batteryLevel > lowBatteryThreshold
        case .screenOffDeep:
            return false
        }
    }

    /// Whether animations should be reduced to save battery.
    var shouldReduceAnimations: Bool {
        return currentPowerState == .screenOffDeep ||
               batteryLevel <= ultraLowBatteryThreshold ||
               isLowPowerMode
    }

    /// Whether the device is in ultra-low-power mode (VPN only).
    var isUltraLowPower: Bool {
        return batteryLevel <= ultraLowBatteryThreshold && !isCharging
    }

    /// Current battery percentage as an integer (0-100).
    var batteryPercentage: Int {
        return max(0, min(100, Int(batteryLevel * 100)))
    }

    /// Get a summary of the current battery state for logging.
    func getStateSummary() -> String {
        return "BatteryManager: level=\(batteryPercentage)%, charging=\(isCharging), " +
               "lowPower=\(isLowPowerMode), state=\(currentPowerState.label), " +
               "drainRate=\(drainRatePerHour.map { String(format: "%.1f%%/hr", $0 * 100) } ?? "unknown")"
    }
}
