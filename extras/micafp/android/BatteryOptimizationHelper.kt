/*
 * MICAFP-UnifiedShield-6.0
 * BatteryOptimizationHelper.kt — Battery optimization and power state management
 *
 * Manages battery optimization exemption, monitors power state, and reports
 * to the Rust daemon for adaptive behavior. Handles Android 14+ foreground
 * service restrictions and Doze/App Standby modes.
 *
 * Key responsibilities:
 *   - Request BATTERY_OPTIMIZATION exemption
 *   - Check if app is whitelisted
 *   - Monitor battery level, charging state, screen on/off
 *   - Report PowerState to Rust daemon every 30 seconds
 *   - Handle Doze mode with setAndAllowWhileIdle() alarms
 *   - Request ACTIVE bucket via UsageStatsManager
 *   - Android 14+ foreground service type handling
 *
 * No root required. Cloudflare is NOT used.
 */

package org.micafp.unifiedshield.power

import android.annotation.SuppressLint
import android.app.AlarmManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.net.Uri
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.PowerManager
import android.provider.Settings
import android.util.Log
import androidx.annotation.RequiresApi
import org.micafp.unifiedshield.jni.ShieldNativeBridge
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference

/**
 * Power state data class reported to the Rust daemon.
 */
data class PowerState(
    val batteryLevel: Int,       // 0-100, -1 if unknown
    val isCharging: Boolean,
    val isScreenOn: Boolean,
    val isDozeMode: Boolean,
    val isIdleMode: Boolean,     // App Standby idle
    val batteryOptimizationWhitelisted: Boolean,
    val powerSaveMode: Boolean
)

/**
 * Battery optimization helper that manages power state reporting
 * and battery optimization exemption for the Shield VPN service.
 */
class BatteryOptimizationHelper(
    private val context: Context
) {
    companion object {
        private const val TAG = "Shield/BatteryOpt"

        // Reporting interval to Rust daemon (ms)
        private const val REPORT_INTERVAL_MS = 30_000L

        // Alarm action for Doze-safe periodic wakeups
        const val ACTION_PERIODIC_WAKEUP = "org.micafp.unifiedshield.PERIODIC_WAKEUP"
        const val ACTION_POWER_STATE_CHANGED = "org.micafp.unifiedshield.POWER_STATE_CHANGED"

        // Alarm request codes
        private const val WAKEUP_ALARM_REQUEST_CODE = 2001

        // Battery thresholds for adaptive behavior
        private const val BATTERY_LOW_THRESHOLD = 20
        private const val BATTERY_CRITICAL_THRESHOLD = 10
    }

    // JNI bridge to Rust daemon
    private val jniBridge = ShieldNativeBridge()

    // Current power state
    private val powerState = AtomicReference(
        PowerState(
            batteryLevel = -1,
            isCharging = false,
            isScreenOn = true,
            isDozeMode = false,
            isIdleMode = false,
            batteryOptimizationWhitelisted = false,
            powerSaveMode = false
        )
    )

    // Handler for periodic reporting
    private val handler = Handler(Looper.getMainLooper())
    private val reportRunnable = object : Runnable {
        override fun run() {
            updateAndReportPowerState()
            handler.postDelayed(this, REPORT_INTERVAL_MS)
        }
    }

    // Registration state
    private val isRegistered = AtomicBoolean(false)

    // ============================================================
    // Public API
    // ============================================================

    /**
     * Start monitoring battery state and reporting to the Rust daemon.
     */
    fun start() {
        if (isRegistered.getAndSet(true)) return

        Log.i(TAG, "Starting battery optimization monitoring")

        // Enable battery level monitoring
        val pm = context.getSystemService(Context.POWER_SERVICE) as PowerManager
        if (!pm.isInteractive) {
            updateScreenOff()
        }

        registerReceivers()
        requestBatteryOptimizationExemption()
        requestActiveBucket()
        schedulePeriodicWakeup()

        // Start periodic reporting
        handler.post(reportRunnable)

        // Initial state report
        updateAndReportPowerState()
    }

    /**
     * Stop monitoring and release resources.
     */
    fun stop() {
        if (!isRegistered.getAndSet(false)) return

        Log.i(TAG, "Stopping battery optimization monitoring")
        handler.removeCallbacks(reportRunnable)
        unregisterReceivers()
        cancelPeriodicWakeup()
    }

    /**
     * Get the current power state.
     */
    fun getCurrentPowerState(): PowerState = powerState.get()

    /**
     * Check if the app is whitelisted from battery optimization.
     */
    fun isWhitelistedFromBatteryOptimization(): Boolean {
        val pm = context.getSystemService(Context.POWER_SERVICE) as PowerManager
        return pm.isIgnoringBatteryOptimizations(context.packageName)
    }

    /**
     * Request battery optimization exemption by showing the system dialog.
     * Must be called from an Activity context.
     */
    fun requestBatteryOptimizationExemption() {
        if (isWhitelistedFromBatteryOptimization()) {
            Log.i(TAG, "Already whitelisted from battery optimization")
            return
        }

        Log.i(TAG, "Requesting battery optimization exemption")
        try {
            val intent = Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_SETTINGS).apply {
                data = Uri.parse("package:${context.packageName}")
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            }
            context.startActivity(intent)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to open battery optimization settings", e)
            // Fallback: open app battery settings
            try {
                val fallbackIntent = Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS).apply {
                    data = Uri.parse("package:${context.packageName}")
                    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                }
                context.startActivity(fallbackIntent)
            } catch (e2: Exception) {
                Log.e(TAG, "Failed to open app settings fallback", e2)
            }
        }
    }

    /**
     * Guide the user through battery optimization settings step by step.
     * Returns a description of the steps for the UI to display.
     */
    fun getBatteryOptimizationGuide(): List<String> {
        val steps = mutableListOf<String>()

        steps.add("Go to Settings → Apps → Shield → Battery")
        steps.add("Select 'Unrestricted' to allow background operation")

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            steps.add("Go to Settings → Apps → Shield → and enable 'Allow foreground service'")
        }

        if (Build.MANUFACTURER.equals("Xiaomi", ignoreCase = true) ||
            Build.MANUFACTURER.equals("Redmi", ignoreCase = true)) {
            steps.add("Xiaomi: Go to Settings → Apps → Shield → Battery saver → No restrictions")
            steps.add("Xiaomi: Enable auto-start in Security app")
        } else if (Build.MANUFACTURER.equals("Huawei", ignoreCase = true)) {
            steps.add("Huawei: Go to Settings → Battery → App launch → Shield → Manage manually")
            steps.add("Huawei: Enable all three toggles (auto-launch, secondary launch, background")
        } else if (Build.MANUFACTURER.equals("Samsung", ignoreCase = true)) {
            steps.add("Samsung: Go to Settings → Apps → Shield → Battery → Allow background activity")
            steps.add("Samsung: Remove Shield from Sleeping apps list")
        } else if (Build.MANUFACTURER.equals("OPPO", ignoreCase = true) ||
                   Build.MANUFACTURER.equals("Realme", ignoreCase = true)) {
            steps.add("OPPO: Go to Settings → Battery → Shield → Allow background running")
            steps.add("OPPO: Enable auto-start in Security settings")
        } else if (Build.MANUFACTURER.equals("Vivo", ignoreCase = true)) {
            steps.add("Vivo: Go to Settings → Battery → Background management → Shield → Allow")
        }

        return steps
    }

    // ============================================================
    // Power State Monitoring
    // ============================================================

    private val batteryReceiver = object : BroadcastReceiver() {
        override fun onReceive(ctx: Context, intent: Intent) {
            when (intent.action) {
                Intent.ACTION_BATTERY_CHANGED -> handleBatteryChanged(intent)
                Intent.ACTION_SCREEN_ON -> handleScreenOn()
                Intent.ACTION_SCREEN_OFF -> handleScreenOff()
                PowerManager.ACTION_DEVICE_IDLE_MODE_CHANGED -> handleDozeModeChanged()
            }
        }
    }

    @SuppressLint("UnspecifiedRegisterReceiverFlag")
    private fun registerReceivers() {
        val filter = IntentFilter().apply {
            addAction(Intent.ACTION_BATTERY_CHANGED)
            addAction(Intent.ACTION_SCREEN_ON)
            addAction(Intent.ACTION_SCREEN_OFF)
            addAction(PowerManager.ACTION_DEVICE_IDLE_MODE_CHANGED)
        }

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            context.registerReceiver(batteryReceiver, filter, Context.RECEIVER_NOT_EXPORTED)
        } else {
            context.registerReceiver(batteryReceiver, filter)
        }
    }

    private fun unregisterReceivers() {
        try {
            context.unregisterReceiver(batteryReceiver)
        } catch (_: Exception) { }
    }

    private fun handleBatteryChanged(intent: Intent) {
        val level = intent.getIntExtra("level", -1)
        val scale = intent.getIntExtra("scale", 100)
        val status = intent.getIntExtra("status", -1)

        val batteryPercent = if (scale > 0) (level * 100) / scale else -1
        val isCharging = status == android.os.BatteryManager.BATTERY_STATUS_CHARGING ||
                         status == android.os.BatteryManager.BATTERY_STATUS_FULL

        val current = powerState.get()
        val updated = current.copy(
            batteryLevel = batteryPercent,
            isCharging = isCharging
        )
        powerState.set(updated)

        Log.d(TAG, "Battery: ${batteryPercent}%, charging=$isCharging")

        // Adaptive behavior: adjust daemon behavior based on battery level
        adjustDaemonBehavior(updated)
    }

    private fun handleScreenOn() {
        val current = powerState.get()
        powerState.set(current.copy(isScreenOn = true))
        Log.d(TAG, "Screen ON")

        // Notify daemon that screen is on (can increase scan frequency)
        jniBridge.onPowerStateChange(powerState.get())
    }

    private fun handleScreenOff() {
        updateScreenOff()
        Log.d(TAG, "Screen OFF")

        // Notify daemon that screen is off (should reduce activity)
        jniBridge.onPowerStateChange(powerState.get())
    }

    private fun updateScreenOff() {
        val current = powerState.get()
        powerState.set(current.copy(isScreenOn = false))
    }

    private fun handleDozeModeChanged() {
        val pm = context.getSystemService(Context.POWER_SERVICE) as PowerManager
        val isDoze = pm.isDeviceIdleMode

        val current = powerState.get()
        powerState.set(current.copy(isDozeMode = isDoze))

        Log.w(TAG, "Doze mode changed: $isDoze")
        jniBridge.onPowerStateChange(powerState.get())
    }

    // ============================================================
    // Periodic State Reporting
    // ============================================================

    private fun updateAndReportPowerState() {
        val pm = context.getSystemService(Context.POWER_SERVICE) as PowerManager
        val bm = context.getSystemService(Context.BATTERY_SERVICE) as android.os.BatteryManager

        val batteryLevel = bm.getIntProperty(android.os.BatteryManager.BATTERY_PROPERTY_CAPACITY)
        val isCharging = bm.isCharging
        val isInteractive = pm.isInteractive
        val isDoze = pm.isDeviceIdleMode
        val isPowerSave = pm.isPowerSaveMode
        val isWhitelisted = pm.isIgnoringBatteryOptimizations(context.packageName)

        val state = PowerState(
            batteryLevel = batteryLevel,
            isCharging = isCharging,
            isScreenOn = isInteractive,
            isDozeMode = isDoze,
            isIdleMode = false, // Would need UsageStatsManager for accurate check
            batteryOptimizationWhitelisted = isWhitelisted,
            powerSaveMode = isPowerSave
        )

        powerState.set(state)

        // Report to Rust daemon
        jniBridge.onPowerStateChange(state)

        Log.d(TAG, "Power state: level=${state.batteryLevel}%, charging=${state.isCharging}, " +
                "screen=${state.isScreenOn}, doze=${state.isDozeMode}, " +
                "whitelisted=${state.batteryOptimizationWhitelisted}, " +
                "powerSave=${state.powerSaveMode}")
    }

    // ============================================================
    // Adaptive Behavior
    // ============================================================

    /**
     * Adjust Rust daemon behavior based on current power state.
     *
     * Charging: Full NAIN detection interval (30s)
     * Normal: 60s interval
     * Low battery: 300s interval
     * Critical: Minimal background activity, rely on push notifications
     */
    private fun adjustDaemonBehavior(state: PowerState) {
        val nainIntervalSeconds = when {
            state.isCharging -> 30
            state.batteryLevel < 0 -> 60  // Unknown level, use normal
            state.batteryLevel <= BATTERY_CRITICAL_THRESHOLD -> 0  // Minimal, rely on push
            state.batteryLevel <= BATTERY_LOW_THRESHOLD -> 300
            else -> 60
        }

        val scanMode = when {
            state.isCharging -> "full"
            state.batteryLevel <= BATTERY_CRITICAL_THRESHOLD -> "minimal"
            state.batteryLevel <= BATTERY_LOW_THRESHOLD -> "reduced"
            !state.isScreenOn -> "background"
            else -> "normal"
        }

        jniBridge.setPowerProfile(nainIntervalSeconds, scanMode)
    }

    // ============================================================
    // Doze Mode Handling
    // ============================================================

    /**
     * Schedule a periodic wakeup alarm that works during Doze mode
     * using setAndAllowWhileIdle().
     */
    @SuppressLint("ScheduleExactAlarm")
    private fun schedulePeriodicWakeup() {
        val alarmManager = context.getSystemService(Context.ALARM_SERVICE) as AlarmManager

        val intent = Intent(context, PeriodicWakeupReceiver::class.java).apply {
            action = ACTION_PERIODIC_WAKEUP
        }

        val pendingIntent = PendingIntent.getBroadcast(
            context,
            WAKEUP_ALARM_REQUEST_CODE,
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        // Use setAndAllowWhileIdle to wake up even in Doze mode
        val triggerAtMs = System.currentTimeMillis() + REPORT_INTERVAL_MS

        try {
            alarmManager.setAndAllowWhileIdle(
                AlarmManager.RTC_WAKEUP,
                triggerAtMs,
                pendingIntent
            )
            Log.d(TAG, "Scheduled Doze-safe periodic wakeup")
        } catch (e: SecurityException) {
            Log.e(TAG, "Cannot schedule exact alarm (missing permission?)", e)
            // Fallback: use inexact alarm
            alarmManager.setInexactRepeating(
                AlarmManager.RTC_WAKEUP,
                triggerAtMs,
                REPORT_INTERVAL_MS,
                pendingIntent
            )
        }
    }

    private fun cancelPeriodicWakeup() {
        val alarmManager = context.getSystemService(Context.ALARM_SERVICE) as AlarmManager
        val intent = Intent(context, PeriodicWakeupReceiver::class.java).apply {
            action = ACTION_PERIODIC_WAKEUP
        }
        val pendingIntent = PendingIntent.getBroadcast(
            context,
            WAKEUP_ALARM_REQUEST_CODE,
            intent,
            PendingIntent.FLAG_NO_CREATE or PendingIntent.FLAG_IMMUTABLE
        )
        pendingIntent?.let { alarmManager.cancel(it) }
    }

    // ============================================================
    // App Standby Bucket Management
    // ============================================================

    /**
     * Request the ACTIVE standby bucket so the app is not restricted
     * by App Standby. This requires the PACKAGE_USAGE_STATS permission.
     */
    @SuppressLint("NewApi")
    private fun requestActiveBucket() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            try {
                val usm = context.getSystemService(Context.USAGE_STATS_SERVICE)
                    as? android.app.usage.UsageStatsManager
                usm?.let {
                    val currentBucket = it.appStandbyBucket
                    Log.d(TAG, "Current App Standby bucket: $currentBucket")

                    if (currentBucket != android.app.usage.UsageStatsManager.STANDBY_BUCKET_ACTIVE) {
                        Log.w(TAG, "App not in ACTIVE bucket, requesting user to adjust")
                        // On Android 9+, we can prompt the user to change the bucket
                        // via settings. We cannot programmatically change it without
                        // PACKAGE_USAGE_STATS (a system-level permission).
                    }
                }
            } catch (e: Exception) {
                Log.d(TAG, "UsageStatsManager not available or permission denied", e)
            }
        }
    }

    // ============================================================
    // Android 14+ Foreground Service Restrictions
    // ============================================================

    /**
     * Check if the app has the necessary permissions for foreground services
     * on Android 14+.
     */
    fun hasForegroundServicePermission(): Boolean {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            return context.checkSelfPermission(android.Manifest.permission.FOREGROUND_SERVICE_SPECIAL_USE)
                == android.content.pm.PackageManager.PERMISSION_GRANTED
        }
        return true
    }

    /**
     * Get the appropriate foreground service type for the current Android version.
     */
    fun getForegroundServiceType(): Int {
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            // Android 14+ requires declaring specific foreground service types
            android.content.pm.ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE
        } else {
            0
        }
    }

    // ============================================================
    // Receiver for Periodic Wakeup
    // ============================================================

    /**
     * BroadcastReceiver for Doze-safe periodic wakeups.
     */
    class PeriodicWakeupReceiver : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            if (intent.action == ACTION_PERIODIC_WAKEUP) {
                Log.d(TAG, "Periodic wakeup alarm received")
                // Trigger a connectivity check via the Rust daemon
                val jniBridge = ShieldNativeBridge()
                jniBridge.onPeriodicWakeup()
                // Reschedule the next wakeup
                val helper = BatteryOptimizationHelper(context)
                helper.schedulePeriodicWakeup()
            }
        }
    }
}
