package com.unifiedshield

import android.content.Intent
import android.os.Build
import android.os.Bundle
import androidx.annotation.NonNull
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import io.flutter.plugin.common.EventChannel

class MainActivity : FlutterActivity() {
    private val CHANNEL = "com.unifiedshield/daemon"
    private val STATUS_CHANNEL = "com.unifiedshield/status"
    private val OTA_CHANNEL = "com.unifiedshield/ota"

    private var statusEventSink: EventChannel.EventSink? = null
    private lateinit var coreBridge: CoreBridge
    private lateinit var killSwitch: KillSwitch

    override fun configureFlutterEngine(@NonNull flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        coreBridge = CoreBridge(context)
        killSwitch = KillSwitch(context)

        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "startDaemon" -> {
                        val coreId = call.argument<String>("core_id") ?: "warp"
                        val obfuscationMode = call.argument<String>("obfuscation_mode") ?: "default"
                        try {
                            coreBridge.startDaemon(coreId, obfuscationMode)
                            result.success(null)
                        } catch (e: Exception) {
                            result.error("DAEMON_ERROR", e.message, null)
                        }
                    }
                    "stopDaemon" -> {
                        try {
                            coreBridge.stopDaemon()
                            result.success(null)
                        } catch (e: Exception) {
                            result.error("DAEMON_ERROR", e.message, null)
                        }
                    }
                    "getStatus" -> {
                        try {
                            val status = coreBridge.getStatus()
                            result.success(status)
                        } catch (e: Exception) {
                            result.error("DAEMON_ERROR", e.message, null)
                        }
                    }
                    "switchCore" -> {
                        val coreId = call.argument<String>("core_id") ?: "warp"
                        try {
                            coreBridge.switchCore(coreId)
                            result.success(null)
                        } catch (e: Exception) {
                            result.error("DAEMON_ERROR", e.message, null)
                        }
                    }
                    "updateReward" -> {
                        val peerId = call.argument<String>("peer_id") ?: ""
                        val bytesRelayed = call.argument<Int>("bytes_relayed") ?: 0
                        try {
                            coreBridge.updateReward(peerId, bytesRelayed)
                            result.success(null)
                        } catch (e: Exception) {
                            result.error("DAEMON_ERROR", e.message, null)
                        }
                    }
                    "setKillSwitch" -> {
                        val enabled = call.argument<Boolean>("enabled") ?: true
                        try {
                            killSwitch.setEnabled(enabled)
                            result.success(null)
                        } catch (e: Exception) {
                            result.error("KILLSWITCH_ERROR", e.message, null)
                        }
                    }
                    "triggerObfuscationMode" -> {
                        val mode = call.argument<String>("mode") ?: "default"
                        try {
                            coreBridge.triggerObfuscationMode(mode)
                            result.success(null)
                        } catch (e: Exception) {
                            result.error("DAEMON_ERROR", e.message, null)
                        }
                    }
                    "configureSplitTunneling" -> {
                        val excludedApps = call.argument<List<String>>("excluded_apps") ?: emptyList()
                        val excludeIranianIps = call.argument<Boolean>("exclude_iranian_ips") ?: true
                        try {
                            coreBridge.configureSplitTunneling(excludedApps, excludeIranianIps)
                            result.success(null)
                        } catch (e: Exception) {
                            result.error("DAEMON_ERROR", e.message, null)
                        }
                    }
                    "getAvailableCores" -> {
                        try {
                            val cores = coreBridge.getAvailableCores()
                            result.success(cores)
                        } catch (e: Exception) {
                            result.error("DAEMON_ERROR", e.message, null)
                        }
                    }
                    "reportIsp" -> {
                        val ispName = call.argument<String>("isp_name") ?: ""
                        val asn = call.argument<String>("asn") ?: ""
                        try {
                            coreBridge.reportIsp(ispName, asn)
                            result.success(null)
                        } catch (e: Exception) {
                            result.error("DAEMON_ERROR", e.message, null)
                        }
                    }
                    "requestVpnPermission" -> {
                        try {
                            val intent = VpnService.prepare(context)
                            if (intent != null) {
                                startActivityForResult(intent, 0)
                                result.success(false)
                            } else {
                                result.success(true)
                            }
                        } catch (e: Exception) {
                            result.error("VPN_ERROR", e.message, null)
                        }
                    }
                    else -> result.notImplemented()
                }
            }

        EventChannel(flutterEngine.dartExecutor.binaryMessenger, STATUS_CHANNEL)
            .setStreamHandler(object : EventChannel.StreamHandler {
                override fun onListen(arguments: Any?, events: EventChannel.EventSink?) {
                    statusEventSink = events
                    coreBridge.setStatusCallback { data ->
                        activity.runOnUiThread {
                            events?.success(data)
                        }
                    }
                }

                override fun onCancel(arguments: Any?) {
                    statusEventSink = null
                    coreBridge.setStatusCallback(null)
                }
            })

        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, OTA_CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "installUpdate" -> {
                        val filePath = call.argument<String>("file_path") ?: ""
                        OtaUpdater.installUpdate(context, filePath, result)
                    }
                    else -> result.notImplemented()
                }
            }
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode == 0) {
            if (resultCode == RESULT_OK) {
                statusEventSink?.success(mapOf("vpn_permission" to "granted"))
            } else {
                statusEventSink?.success(mapOf("vpn_permission" to "denied"))
            }
        }
    }

    override fun onDestroy() {
        coreBridge.destroy()
        killSwitch.destroy()
        super.onDestroy()
    }
}
