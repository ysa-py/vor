package com.unifiedshield

import android.content.Context
import android.util.Log
import org.json.JSONArray
import org.json.JSONObject

/**
 * JNI bridge to the Rust daemon shared library (libunifiedshield.so).
 * Provides native method declarations that correspond to the Rust FFI exports.
 */
class CoreBridge(private val context: Context) {

    companion object {
        private const val TAG = "CoreBridge"

        init {
            System.loadLibrary("unifiedshield")
        }

        // JNI native method declarations - implemented in Rust
        @JvmStatic
        private external fun nativeStartDaemon(coreId: String, obfuscationMode: String): Int

        @JvmStatic
        private external fun nativeStopDaemon(): Int

        @JvmStatic
        private external fun nativeGetStatus(): String

        @JvmStatic
        private external fun nativeSwitchCore(coreId: String): Int

        @JvmStatic
        private external fun nativeUpdateReward(peerId: String, bytesRelayed: Int): Int

        @JvmStatic
        private external fun nativeSetKillSwitch(enabled: Boolean): Int

        @JvmStatic
        private external fun nativeTriggerObfuscationMode(mode: String): Int

        @JvmStatic
        private external fun nativeConfigureSplitTunneling(excludedAppsJson: String, excludeIranianIps: Boolean): Int

        @JvmStatic
        private external fun nativeGetAvailableCores(): String

        @JvmStatic
        private external fun nativeReportIsp(ispName: String, asn: String): Int

        @JvmStatic
        private external fun nativeDestroy()
    }

    private var statusCallback: ((Map<String, Any>) -> Unit)? = null

    fun setStatusCallback(callback: ((Map<String, Any>) -> Unit)?) {
        statusCallback = callback
    }

    fun startDaemon(coreId: String, obfuscationMode: String) {
        val result = nativeStartDaemon(coreId, obfuscationMode)
        if (result != 0) {
            throw DaemonBridgeException("Failed to start daemon with core $coreId, error code: $result")
        }
        Log.i(TAG, "Daemon started with core: $coreId, mode: $obfuscationMode")
    }

    fun stopDaemon() {
        val result = nativeStopDaemon()
        if (result != 0) {
            throw DaemonBridgeException("Failed to stop daemon, error code: $result")
        }
        Log.i(TAG, "Daemon stopped")
    }

    fun getStatus(): Map<String, Any> {
        val jsonStr = nativeGetStatus()
        try {
            val json = JSONObject(jsonStr)
            return jsonToMap(json)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to parse status JSON", e)
            return mapOf("status" to "error", "message" to "Failed to parse status")
        }
    }

    fun switchCore(coreId: String) {
        val result = nativeSwitchCore(coreId)
        if (result != 0) {
            throw DaemonBridgeException("Failed to switch core to $coreId, error code: $result")
        }
        Log.i(TAG, "Switched core to: $coreId")
    }

    fun updateReward(peerId: String, bytesRelayed: Int) {
        val result = nativeUpdateReward(peerId, bytesRelayed)
        if (result != 0) {
            Log.w(TAG, "Failed to update reward, error code: $result")
        }
    }

    fun setKillSwitch(enabled: Boolean) {
        val result = nativeSetKillSwitch(enabled)
        if (result != 0) {
            throw DaemonBridgeException("Failed to set kill switch, error code: $result")
        }
        Log.i(TAG, "Kill switch set to: $enabled")
    }

    fun triggerObfuscationMode(mode: String) {
        val result = nativeTriggerObfuscationMode(mode)
        if (result != 0) {
            throw DaemonBridgeException("Failed to trigger obfuscation mode: $mode, error code: $result")
        }
        Log.i(TAG, "Obfuscation mode set to: $mode")
    }

    fun configureSplitTunneling(excludedApps: List<String>, excludeIranianIps: Boolean) {
        val jsonArray = JSONArray(excludedApps)
        val result = nativeConfigureSplitTunneling(jsonArray.toString(), excludeIranianIps)
        if (result != 0) {
            throw DaemonBridgeException("Failed to configure split tunneling, error code: $result")
        }
        Log.i(TAG, "Split tunneling configured: excludeIranianIps=$excludeIranianIps, excludedApps=${excludedApps.size}")
    }

    fun getAvailableCores(): List<Map<String, Any>> {
        val jsonStr = nativeGetAvailableCores()
        try {
            val jsonArray = JSONArray(jsonStr)
            val cores = mutableListOf<Map<String, Any>>()
            for (i in 0 until jsonArray.length()) {
                val json = jsonArray.getJSONObject(i)
                cores.add(jsonToMap(json))
            }
            return cores
        } catch (e: Exception) {
            Log.e(TAG, "Failed to parse cores JSON", e)
            return emptyList()
        }
    }

    fun reportIsp(ispName: String, asn: String) {
        val result = nativeReportIsp(ispName, asn)
        if (result != 0) {
            Log.w(TAG, "Failed to report ISP, error code: $result")
        }
    }

    fun destroy() {
        nativeDestroy()
    }

    /**
     * Called from Rust via JNI to push status updates back to Flutter.
     */
    @Suppress("unused")
    fun onStatusUpdate(jsonStr: String) {
        try {
            val json = JSONObject(jsonStr)
            val map = jsonToMap(json)
            statusCallback?.invoke(map)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to parse status update JSON", e)
        }
    }

    private fun jsonToMap(json: JSONObject): Map<String, Any> {
        val map = mutableMapOf<String, Any>()
        val keys = json.keys()
        while (keys.hasNext()) {
            val key = keys.next()
            val value = json.get(key)
            when (value) {
                is JSONObject -> map[key] = jsonToMap(value)
                is JSONArray -> map[key] = jsonArrayToList(value)
                else -> map[key] = value
            }
        }
        return map
    }

    private fun jsonArrayToList(jsonArray: JSONArray): List<Any> {
        val list = mutableListOf<Any>()
        for (i in 0 until jsonArray.length()) {
            val value = jsonArray.get(i)
            when (value) {
                is JSONObject -> list.add(jsonToMap(value))
                is JSONArray -> list.add(jsonArrayToList(value))
                else -> list.add(value)
            }
        }
        return list
    }

    class DaemonBridgeException(message: String) : Exception(message)
}
