package com.v2rayez.app

import android.app.Application
import android.content.Context
import android.util.Log
import androidx.hilt.work.HiltWorkerFactory
import androidx.work.Configuration
import com.v2rayez.app.data.analytics.sanitizedForCrashlytics
import com.v2rayez.app.data.tor.TorController
import com.v2rayez.app.data.tor.TorState
import com.v2rayez.app.domain.model.AppSettings
import com.v2rayez.app.data.work.SubscriptionRefreshWorker
import com.v2rayez.app.domain.model.LogEntry
import com.v2rayez.app.domain.model.LogLevel
import com.v2rayez.app.domain.repository.LogRepository
import com.v2rayez.app.domain.repository.SettingsRepository
import com.v2rayez.app.ui.LocaleHelper
import dagger.hilt.android.HiltAndroidApp
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.UUID
import javax.inject.Inject

@HiltAndroidApp
class V2RayApplication : Application(), Configuration.Provider {

    @Inject lateinit var logRepository: LogRepository
    @Inject lateinit var settingsRepository: SettingsRepository
    @Inject lateinit var torController: TorController
    @Inject lateinit var iranGeoAutoConfigurator: com.v2rayez.app.data.core.IranGeoAutoConfigurator
    @Inject lateinit var packInstallCoordinator: com.v2rayez.app.data.core.PackInstallCoordinator

    /**
     * License verification clock (v1.0.4 anti-clock-rollback): device clock +
     * persisted monotonic ratchet + opportunistic trusted HTTPS time. The
     * refresh loop below feeds the ratchet a couple of times a day; the
     * verification itself never needs the network.
     */
    @Inject lateinit var licenseClock: com.v2rayez.app.data.license.LicenseClock

    @Inject lateinit var firebaseTelemetry: com.v2rayez.app.data.analytics.FirebaseTelemetry
    @Inject lateinit var hiltWorkerFactory: HiltWorkerFactory

    private val appScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    /** Lets WorkManager resolve [SubscriptionRefreshWorker]'s `@AssistedInject` deps via Hilt. */
    override val workManagerConfiguration: Configuration
        get() = Configuration.Builder().setWorkerFactory(hiltWorkerFactory).build()

    override fun attachBaseContext(base: Context) {
        val tag = LocaleHelper.savedTag(base)
        super.attachBaseContext(LocaleHelper.wrap(base, tag))
    }

    override fun onCreate() {
        // Hilt injects fields inside super.onCreate() — must call first, then arm telemetry
        // so the rest of startup (and fatals) are covered.
        super.onCreate()
        firebaseTelemetry.enableTelemetry()
        installCrashLogger()
        // Pre-create VPN notification channels so the first startForegroundService →
        // startForeground path never races channel creation on cold start (FGS 5s deadline).
        runCatching { ensureVpnNotificationChannels() }
        restoreTor()
        runCatching { SubscriptionRefreshWorker.schedule(this) }
            .onFailure { Log.w("V2RayApplication", "WorkManager schedule failed", it) }
        appScope.launch { iranGeoAutoConfigurator.applyIfNeeded() }
        packInstallCoordinator.start()
        // Trusted-time seed for the license clock (throttled internally; a
        // filtered/offline network just makes this a no-op).
        appScope.launch {
            licenseClock.refreshTrustedTimeAsync()
            while (true) {
                kotlinx.coroutines.delay(6 * 3600_000L)
                runCatching { licenseClock.refreshTrustedTimeAsync() }
            }
        }
        appScope.launch {
            runCatching {
                var previous: AppSettings? = null
                settingsRepository.settings().collect { settings ->
                    firebaseTelemetry.applyConsent(settings)
                    previous?.let { old -> logFeatureToggleChanges(old, settings) }
                    previous = settings
                }
            }
        }
    }

    private fun logFeatureToggleChanges(old: AppSettings, new: AppSettings) {
        fun changed(name: String, before: Boolean, after: Boolean) {
            if (before != after) firebaseTelemetry.logFeatureToggle(name, after)
        }
        changed("analytics", old.analyticsConsent, new.analyticsConsent)
        changed("notifications", old.notifications, new.notifications)
        changed("auto_connect", old.autoConnect, new.autoConnect)
        changed("boot_auto_connect", old.bootAutoConnect, new.bootAutoConnect)
        changed("battery_saver", old.batterySaver, new.batterySaver)
        changed("always_on", old.vpnAlwaysOn, new.vpnAlwaysOn)
        changed("lockdown", old.blockWithoutVpn, new.blockWithoutVpn)
        changed("full_device_tunnel", old.fullDeviceTunnel, new.fullDeviceTunnel)
        changed("allow_lan", old.allowLan, new.allowLan)
        changed("ipv6", old.enableIpv6, new.enableIpv6)
        changed("local_dns", old.enableLocalDns, new.enableLocalDns)
        changed("sniffing", old.enableSniffing, new.enableSniffing)
        changed("mux", old.enableMux, new.enableMux)
        changed("lan_sharing", old.enableLanSharing, new.enableLanSharing)
        changed("reduce_data", old.reduceData, new.reduceData)
        changed("tor", old.tor.enabled, new.tor.enabled)
        changed("tor_auto_rotate", old.tor.autoRotateBridges, new.tor.autoRotateBridges)
        changed("domain_front", old.domainFront.enabled, new.domainFront.enabled)
        changed("allow_insecure", old.tls.allowInsecure, new.tls.allowInsecure)
        changed("fragment", old.fragment.enabled, new.fragment.enabled)
        changed("warp", old.warp.enabled, new.warp.enabled)
        changed("fake_dns", old.dns.enableFakeDns, new.dns.enableFakeDns)
        changed("app_proxy", old.appProxy.enabled, new.appProxy.enabled)
        changed("app_proxy_bypass", old.appProxy.bypassMode, new.appProxy.bypassMode)
        changed("bypass_lan", old.routing.bypassLan, new.routing.bypassLan)
        changed("bypass_mainland", old.routing.bypassMainland, new.routing.bypassMainland)
        changed("bypass_iran", old.routing.bypassIran, new.routing.bypassIran)
        changed("block_ads", old.routing.blockAds, new.routing.blockAds)
    }

    /**
     * Pre-create VPN FG notification channels at process start so
     * [com.v2rayez.app.data.service.V2RayVpnService] can call startForeground within the
     * Android ~5s FGS deadline even on cold start.
     */
    private fun ensureVpnNotificationChannels() {
        if (android.os.Build.VERSION.SDK_INT < android.os.Build.VERSION_CODES.O) return
        val nm = getSystemService(android.app.NotificationManager::class.java) ?: return
        val channelId = "vpn_status"
        val quietId = "vpn_status_quiet"
        if (nm.getNotificationChannel(quietId) == null) {
            nm.createNotificationChannel(
                android.app.NotificationChannel(
                    quietId,
                    "VPN Status (silent)",
                    android.app.NotificationManager.IMPORTANCE_MIN
                ).apply {
                    setShowBadge(false)
                    setSound(null, null)
                    enableVibration(false)
                }
            )
        }
        val existing = nm.getNotificationChannel(channelId)
        if (existing == null || existing.importance < android.app.NotificationManager.IMPORTANCE_DEFAULT) {
            if (existing != null) nm.deleteNotificationChannel(channelId)
            nm.createNotificationChannel(
                android.app.NotificationChannel(
                    channelId,
                    "VPN Status",
                    android.app.NotificationManager.IMPORTANCE_DEFAULT
                ).apply {
                    setShowBadge(false)
                    setSound(null, null)
                    enableVibration(false)
                }
            )
        }
    }

    /**
     * If the user left Tor enabled, restore it after a short delay so Application.onCreate
     * never blocks/crashes on native Tor startup (16 KB page devices, slow storage).
     */
    private fun restoreTor() {
        appScope.launch {
            runCatching {
                kotlinx.coroutines.delay(1_500)
                val tor = settingsRepository.current().tor
                if (tor.enabled && torController.status.value.state == TorState.OFF) {
                    torController.start(tor)
                }
            }.onFailure { Log.w("V2RayApplication", "Tor restore skipped", it) }
        }
    }

    /**
     * Best-effort last-gasp logger: records the fatal exception to the in-app log stream before
     * delegating to the platform default handler, so crashes are visible in the Logs screen and
     * exported reports instead of vanishing.
     *
     * 2026-09 hardening (device crash report): the old handler evaluated
     * `sanitizedForCrashlytics(throwable)` OUTSIDE any runCatching when chaining to the
     * previous handler — a throw there would itself kill the process and replace the real
     * crash with an opaque one. The handler is now non-throwing end-to-end and always
     * chains the ORIGINAL throwable (sanitized best-effort) so the real cause survives.
     */
    private fun installCrashLogger() {
        val previous = Thread.getDefaultUncaughtExceptionHandler()
        Thread.setDefaultUncaughtExceptionHandler { thread, throwable ->
            runCatching {
                val ts = SimpleDateFormat("HH:mm:ss", Locale.US).format(Date())
                logRepository.append(
                    LogEntry(
                        id = UUID.randomUUID().toString(),
                        timestamp = ts,
                        level = LogLevel.ERROR,
                        message = "Fatal: ${throwable.message ?: throwable.javaClass.simpleName}",
                        detail = throwable.stackTraceToString().take(2000)
                    )
                )
                Log.e("V2RayApplication", "Uncaught exception on ${thread.name}", throwable)
                firebaseTelemetry.recordFatal(throwable)
            }
            // Never hand the platform a throwable produced by our OWN sanitization —
            // if scrubbing fails for any reason, chain the original untouched.
            val toReport = runCatching { sanitizedForCrashlytics(throwable) }.getOrDefault(throwable)
            if (toReport !is Throwable) {
                previous?.uncaughtException(thread, throwable)
            } else {
                previous?.uncaughtException(thread, toReport)
            }
        }
    }
}
