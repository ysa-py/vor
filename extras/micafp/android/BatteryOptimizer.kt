package com.shield.battery

import android.Manifest
import android.annotation.SuppressLint
import android.app.AlarmManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.os.Build
import android.os.PowerManager
import android.util.Log
import androidx.work.*
import java.util.concurrent.TimeUnit

/**
 * BatteryOptimizer — Battery Optimization for Android 14+
 *
 * Monitors battery level, charging state, and screen on/off to determine the
 * optimal power state. Uses adaptive strategies to balance functionality with
 * battery life:
 *   - SCREEN_ON: all services active (full VPN + NAN + acoustic + probing)
 *   - SCREEN_OFF_LIGHT: reduce probe interval, disable acoustic listening
 *   - SCREEN_OFF_DEEP: minimal activity, VPN tunnel only
 *   - CHARGING: maximum capability (all services + frequent probing)
 *
 * Integrates WorkManager for periodic background tasks, maintains a
 * ForegroundService with low-priority notification for VPN tunnel, and
 * requests BATTERY_OPTIMIZATIONS exemption. Communicates power state to
 * the Rust daemon via JNI.
 */
class BatteryOptimizer : BroadcastReceiver() {

    companion object {
        private const val TAG = "BatteryOptimizer"

        // Battery thresholds
        private const val ULTRA_LOW_BATTERY_THRESHOLD = 15
        private const val LOW_BATTERY_THRESHOLD = 25
        private const val MEDIUM_BATTERY_THRESHOLD = 50

        // WorkManager tags
        private const val WORK_TAG_PROBE = "battery_adaptive_probe"
        private const val WORK_TAG_HEALTH_CHECK = "battery_health_check"
        private const val WORK_TAG_ENDPOINT_REFRESH = "battery_endpoint_refresh"

        // Alarm actions
        const val ACTION_CRITICAL_ALARM = "com.shield.battery.CRITICAL_ALARM"
        const val ACTION_PERIODIC_ALARM = "com.shield.battery.PERIODIC_ALARM"

        // JNI native methods
        init {
            System.loadLibrary("shield_native")
        }

        @JvmStatic
        external fun nativeSetPowerState(state: Int, batteryLevel: Int)

        @JvmStatic
        external fun nativeSetProbeInterval(intervalMs: Long)

        @JvmStatic
        external fun nativeSetAcousticChannelEnabled(enabled: Boolean)

        @JvmStatic
        external fun nativeSetNanEnabled(enabled: Boolean)
    }

    /**
     * Power states ordered by resource consumption (0 = highest, 3 = lowest).
     */
    enum class PowerState(val code: Int, val label: String) {
        SCREEN_ON(0, "screen_on"),
        SCREEN_OFF_LIGHT(1, "screen_off_light"),
        SCREEN_OFF_DEEP(2, "screen_off_deep"),
        CHARGING(3, "charging");

        companion object {
            fun fromCode(code: Int): PowerState =
                entries.find { it.code == code } ?: SCREEN_OFF_LIGHT
        }
    }

    // Current state
    private var currentPowerState = PowerState.SCREEN_ON
    private var batteryLevel: Int = 100
    private var isCharging: Boolean = false
    private var isScreenOn: Boolean = true
    private var isLowPowerMode: Boolean = false
    private var isDeviceIdle: Boolean = false
    private var context: Context? = null

    // Adaptive intervals (in milliseconds)
    private data class AdaptiveIntervals(
        val probeInterval: Long,
        val healthCheckInterval: Long,
        val endpointRefreshInterval: Long,
        val acousticEnabled: Boolean,
        val nanEnabled: Boolean
    )

    private val intervalsByState = mapOf(
        PowerState.SCREEN_ON to AdaptiveIntervals(
            probeInterval = 30_000L,           // 30 seconds
            healthCheckInterval = 120_000L,     // 2 minutes
            endpointRefreshInterval = 300_000L, // 5 minutes
            acousticEnabled = true,
            nanEnabled = true
        ),
        PowerState.SCREEN_OFF_LIGHT to AdaptiveIntervals(
            probeInterval = 120_000L,           // 2 minutes
            healthCheckInterval = 300_000L,     // 5 minutes
            endpointRefreshInterval = 600_000L, // 10 minutes
            acousticEnabled = false,
            nanEnabled = true
        ),
        PowerState.SCREEN_OFF_DEEP to AdaptiveIntervals(
            probeInterval = 600_000L,           // 10 minutes
            healthCheckInterval = 900_000L,     // 15 minutes
            endpointRefreshInterval = 1_800_000L, // 30 minutes
            acousticEnabled = false,
            nanEnabled = false
        ),
        PowerState.CHARGING to AdaptiveIntervals(
            probeInterval = 15_000L,            // 15 seconds
            healthCheckInterval = 60_000L,      // 1 minute
            endpointRefreshInterval = 180_000L, // 3 minutes
            acousticEnabled = true,
            nanEnabled = true
        )
    )

    // ---------------------------------------------------------------
    // Initialization
    // ---------------------------------------------------------------

    fun initialize(ctx: Context) {
        context = ctx.applicationContext
        registerReceivers()
        readInitialBatteryState()
        requestBatteryOptimizationExemption(ctx)
        scheduleAdaptiveWork()
        Log.d(TAG, "BatteryOptimizer initialized, current state: ${currentPowerState.label}")
    }

    private fun registerReceivers() {
        val ctx = context ?: return
        val filter = IntentFilter().apply {
            addAction(Intent.ACTION_BATTERY_CHANGED)
            addAction(Intent.ACTION_SCREEN_ON)
            addAction(Intent.ACTION_SCREEN_OFF)
            addAction(PowerManager.ACTION_DEVICE_IDLE_MODE_CHANGED)
            addAction(ACTION_CRITICAL_ALARM)
            addAction(ACTION_PERIODIC_ALARM)
        }
        ctx.registerReceiver(this, filter)
    }

    @SuppressLint("UnspecifiedRegisterReceiverFlag")
    private fun readInitialBatteryState() {
        val ctx = context ?: return

        // Read current battery level
        val bm = ctx.getSystemService(Context.BATTERY_SERVICE) as? android.os.BatteryManager
        if (bm != null) {
            batteryLevel = bm.getIntProperty(android.os.BatteryManager.BATTERY_PROPERTY_CAPACITY)
            isCharging = bm.isCharging
        }

        // Check screen state
        val pm = ctx.getSystemService(Context.POWER_SERVICE) as? PowerManager
        if (pm != null) {
            isScreenOn = pm.isInteractive
            isDeviceIdle = pm.isDeviceIdleMode
        }

        // Check low power mode on API 21+
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.LOLLIPOP) {
            isLowPowerMode = pm?.isPowerSaveMode ?: false
        }

        evaluatePowerState()
    }

    // ---------------------------------------------------------------
    // BroadcastReceiver
    // ---------------------------------------------------------------

    override fun onReceive(ctx: Context, intent: Intent) {
        context = ctx.applicationContext
        when (intent.action) {
            Intent.ACTION_BATTERY_CHANGED -> {
                batteryLevel = intent.getIntExtra(android.os.BatteryManager.EXTRA_LEVEL, 100)
                val status = intent.getIntExtra(android.os.BatteryManager.EXTRA_STATUS, -1)
                isCharging = status == android.os.BatteryManager.BATTERY_STATUS_CHARGING ||
                        status == android.os.BatteryManager.BATTERY_STATUS_FULL
                evaluatePowerState()
            }

            Intent.ACTION_SCREEN_ON -> {
                isScreenOn = true
                evaluatePowerState()
            }

            Intent.ACTION_SCREEN_OFF -> {
                isScreenOn = false
                evaluatePowerState()
            }

            PowerManager.ACTION_DEVICE_IDLE_MODE_CHANGED -> {
                val pm = ctx.getSystemService(Context.POWER_SERVICE) as? PowerManager
                isDeviceIdle = pm?.isDeviceIdleMode ?: false
                evaluatePowerState()
            }

            ACTION_CRITICAL_ALARM -> {
                Log.d(TAG, "Critical alarm fired during Doze mode")
                performCriticalKeepalive()
            }

            ACTION_PERIODIC_ALARM -> {
                Log.d(TAG, "Periodic alarm fired")
                performPeriodicHealthCheck()
            }
        }
    }

    // ---------------------------------------------------------------
    // Power State Evaluation
    // ---------------------------------------------------------------

    private fun evaluatePowerState() {
        val previousState = currentPowerState

        currentPowerState = when {
            isCharging -> PowerState.CHARGING
            isScreenOn -> PowerState.SCREEN_ON
            batteryLevel <= ULTRA_LOW_BATTERY_THRESHOLD || isDeviceIdle -> PowerState.SCREEN_OFF_DEEP
            batteryLevel <= LOW_BATTERY_THRESHOLD || isLowPowerMode -> PowerState.SCREEN_OFF_LIGHT
            !isScreenOn -> PowerState.SCREEN_OFF_LIGHT
            else -> PowerState.SCREEN_ON
        }

        if (currentPowerState != previousState) {
            Log.d(TAG, "Power state changed: ${previousState.label} → ${currentPowerState.label} " +
                    "(battery: $batteryLevel%, charging: $isCharging, screen: $isScreenOn)")
            applyAdaptiveStrategy()
            notifyRustDaemon()
            scheduleAdaptiveWork()
            scheduleDozeCompatibleAlarms()
        }
    }

    // ---------------------------------------------------------------
    // Adaptive Strategy Application
    // ---------------------------------------------------------------

    private fun applyAdaptiveStrategy() {
        val intervals = intervalsByState[currentPowerState] ?: return

        // Update Rust daemon with new parameters
        nativeSetPowerState(currentPowerState.code, batteryLevel)
        nativeSetProbeInterval(intervals.probeInterval)
        nativeSetAcousticChannelEnabled(intervals.acousticEnabled)
        nativeSetNanEnabled(intervals.nanEnabled)

        Log.d(TAG, "Applied ${currentPowerState.label} strategy: " +
                "probe=${intervals.probeInterval}ms, acoustic=${intervals.acousticEnabled}, nan=${intervals.nanEnabled}")
    }

    private fun notifyRustDaemon() {
        nativeSetPowerState(currentPowerState.code, batteryLevel)
    }

    // ---------------------------------------------------------------
    // WorkManager Integration
    // ---------------------------------------------------------------

    private fun scheduleAdaptiveWork() {
        val ctx = context ?: return
        val intervals = intervalsByState[currentPowerState] ?: return

        // Periodic probe
        val probeRequest = PeriodicWorkRequestBuilder<BatteryAdaptiveWorker>(
            intervals.probeInterval, TimeUnit.MILLISECONDS,
            intervals.probeInterval / 2, TimeUnit.MILLISECONDS
        )
            .setConstraints(
                Constraints.Builder()
                    .setRequiredNetworkType(NetworkType.CONNECTED)
                    .build()
            )
            .setInputData(
                workDataOf("task_type" to "probe", "power_state" to currentPowerState.code)
            )
            .build()

        WorkManager.getInstance(ctx).enqueueUniquePeriodicWork(
            WORK_TAG_PROBE,
            ExistingPeriodicWorkPolicy.REPLACE,
            probeRequest
        )

        // Health check
        val healthRequest = PeriodicWorkRequestBuilder<BatteryAdaptiveWorker>(
            intervals.healthCheckInterval, TimeUnit.MILLISECONDS,
            intervals.healthCheckInterval / 2, TimeUnit.MILLISECONDS
        )
            .setInputData(
                workDataOf("task_type" to "health_check")
            )
            .build()

        WorkManager.getInstance(ctx).enqueueUniquePeriodicWork(
            WORK_TAG_HEALTH_CHECK,
            ExistingPeriodicWorkPolicy.REPLACE,
            healthRequest
        )

        // Endpoint refresh
        val endpointRequest = PeriodicWorkRequestBuilder<BatteryAdaptiveWorker>(
            intervals.endpointRefreshInterval, TimeUnit.MILLISECONDS,
            intervals.endpointRefreshInterval / 2, TimeUnit.MILLISECONDS
        )
            .setConstraints(
                Constraints.Builder()
                    .setRequiredNetworkType(NetworkType.CONNECTED)
                    .build()
            )
            .setInputData(
                workDataOf("task_type" to "endpoint_refresh")
            )
            .build()

        WorkManager.getInstance(ctx).enqueueUniquePeriodicWork(
            WORK_TAG_ENDPOINT_REFRESH,
            ExistingPeriodicWorkPolicy.REPLACE,
            endpointRequest
        )

        Log.d(TAG, "Adaptive WorkManager tasks scheduled for state: ${currentPowerState.label}")
    }

    // ---------------------------------------------------------------
    // Doze Mode Compatibility
    // ---------------------------------------------------------------

    private fun scheduleDozeCompatibleAlarms() {
        val ctx = context ?: return
        val alarmManager = ctx.getSystemService(Context.ALARM_SERVICE) as? AlarmManager ?: return

        // Critical keepalive alarm — uses setAndAllowWhileIdle for Doze compatibility
        val criticalIntent = PendingIntent.getBroadcast(
            ctx, 0,
            Intent(ACTION_CRITICAL_ALARM),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val criticalInterval = when (currentPowerState) {
            PowerState.SCREEN_OFF_DEEP -> 15 * 60_000L // 15 minutes
            PowerState.SCREEN_OFF_LIGHT -> 5 * 60_000L // 5 minutes
            else -> return // No need for Doze-compatible alarms when screen is on or charging
        }

        val nextTrigger = System.currentTimeMillis() + criticalInterval
        alarmManager.setAndAllowWhileIdle(
            AlarmManager.RTC_WAKEUP,
            nextTrigger,
            criticalIntent
        )

        Log.d(TAG, "Doze-compatible alarm scheduled for ${criticalInterval / 1000}s from now")
    }

    // ---------------------------------------------------------------
    // Battery Optimization Exemption
    // ---------------------------------------------------------------

    private fun requestBatteryOptimizationExemption(ctx: Context) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            val pm = ctx.getSystemService(Context.POWER_SERVICE) as? PowerManager
            val packageName = ctx.packageName

            if (pm != null && !pm.isIgnoringBatteryOptimizations(packageName)) {
                Log.d(TAG, "App is not whitelisted from battery optimizations, requesting exemption")
                // Note: In production, you'd show a dialog explaining why before launching this intent
                try {
                    val intent = Intent(
                        android.provider.Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS
                    ).apply {
                        data = android.net.Uri.parse("package:$packageName")
                        addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                    }
                    ctx.startActivity(intent)
                } catch (e: Exception) {
                    Log.e(TAG, "Could not request battery optimization exemption: ${e.message}")
                }
            } else {
                Log.d(TAG, "App is already whitelisted from battery optimizations")
            }
        }
    }

    // ---------------------------------------------------------------
    // Critical Keepalive (during Doze)
    // ---------------------------------------------------------------

    private fun performCriticalKeepalive() {
        Log.d(TAG, "Performing critical keepalive during Doze mode")
        // Maintain VPN tunnel liveness with minimal network activity
        // The Rust daemon handles the actual keepalive packet
        nativeSetPowerState(PowerState.SCREEN_OFF_DEEP.code, batteryLevel)
    }

    // ---------------------------------------------------------------
    // Periodic Health Check
    // ---------------------------------------------------------------

    private fun performPeriodicHealthCheck() {
        Log.d(TAG, "Performing periodic health check")
        // Check VPN tunnel health, reconnect if necessary
        // The Rust daemon will handle reconnection logic
    }

    // ---------------------------------------------------------------
    // Public API
    // ---------------------------------------------------------------

    fun getCurrentPowerState(): PowerState = currentPowerState

    fun getBatteryLevel(): Int = batteryLevel

    fun isCharging(): Boolean = isCharging

    fun shouldReduceAnimations(): Boolean =
        currentPowerState == PowerState.SCREEN_OFF_DEEP ||
                batteryLevel <= ULTRA_LOW_BATTERY_THRESHOLD ||
                isLowPowerMode

    fun isUltraLowPower(): Boolean = batteryLevel <= ULTRA_LOW_BATTERY_THRESHOLD && !isCharging

    fun getProbeInterval(): Long =
        (intervalsByState[currentPowerState]?.probeInterval ?: 30_000L)

    // ---------------------------------------------------------------
    // Foreground Service Notification
    // ---------------------------------------------------------------

    fun buildVpnForegroundNotification(ctx: Context): android.app.Notification {
        val channelId = "shield_vpn_channel"
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = android.app.NotificationChannel(
                channelId,
                "Shield Tunnel Service",
                android.app.NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "VPN tunnel is active"
                setShowBadge(false)
            }
            val nm = ctx.getSystemService(Context.NOTIFICATION_SERVICE) as android.app.NotificationManager
            nm.createNotificationChannel(channel)
        }

        val stateText = when (currentPowerState) {
            PowerState.SCREEN_ON -> "Active — Full service"
            PowerState.SCREEN_OFF_LIGHT -> "Background — Reduced"
            PowerState.SCREEN_OFF_DEEP -> "Deep sleep — Minimal"
            PowerState.CHARGING -> "Charging — Full service"
        }

        return androidx.core.app.NotificationCompat.Builder(ctx, channelId)
            .setContentTitle("Shield Active")
            .setContentText(stateText)
            .setSmallIcon(android.R.drawable.ic_lock_lock)
            .setPriority(androidx.core.app.NotificationCompat.PRIORITY_LOW)
            .setOngoing(true)
            .setShowWhen(false)
            .build()
    }

    // ---------------------------------------------------------------
    // Cleanup
    // ---------------------------------------------------------------

    fun shutdown() {
        val ctx = context ?: return
        try {
            ctx.unregisterReceiver(this)
        } catch (_: Exception) { }

        WorkManager.getInstance(ctx).cancelUniqueWork(WORK_TAG_PROBE)
        WorkManager.getInstance(ctx).cancelUniqueWork(WORK_TAG_HEALTH_CHECK)
        WorkManager.getInstance(ctx).cancelUniqueWork(WORK_TAG_ENDPOINT_REFRESH)

        Log.d(TAG, "BatteryOptimizer shutdown complete")
    }
}

// ---------------------------------------------------------------
// WorkManager Worker
// ---------------------------------------------------------------

class BatteryAdaptiveWorker(
    context: Context,
    workerParams: WorkerParameters
) : CoroutineWorker(context, workerParams) {

    override suspend fun doWork(): Result {
        val taskType = inputData.getString("task_type") ?: "probe"
        val powerStateCode = inputData.getInt("power_state", 1)

        Log.d("BatteryAdaptiveWorker", "Executing task: $taskType, powerState: $powerStateCode")

        when (taskType) {
            "probe" -> {
                // Trigger Rust daemon to probe endpoints
                BatteryOptimizer.nativeSetPowerState(powerStateCode, -1)
            }
            "health_check" -> {
                // Verify VPN tunnel is still alive
                BatteryOptimizer.nativeSetProbeInterval(0) // signal health check
            }
            "endpoint_refresh" -> {
                // Refresh endpoint list from Rust daemon
                BatteryOptimizer.nativeSetNanEnabled(true) // trigger refresh
            }
        }

        return Result.success()
    }
}
