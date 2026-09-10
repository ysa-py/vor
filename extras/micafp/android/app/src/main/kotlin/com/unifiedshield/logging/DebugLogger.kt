package com.unifiedshield.logging

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import java.text.SimpleDateFormat
import java.util.*

enum class LogLevel(val label: String) {
    INFO("INFO"),
    WARN("WARN"),
    DPI("DPI"),
    PACKET("PACKET"),
    TUNNEL("TUNNEL"),
    SCANNER("SCANNER")
}

data class LogEntry(
    val id: Long = System.currentTimeMillis() + (0..999).random(),
    val timestamp: Long = System.currentTimeMillis(),
    val level: LogLevel,
    val tag: String,
    val message: String
) {
    val formattedTime: String
        get() = SimpleDateFormat("HH:mm:ss.SSS", Locale.getDefault()).format(Date(timestamp))
}

class DebugLogger private constructor() {

    private val maxLogEntries = 300
    private val _logs = MutableStateFlow<List<LogEntry>>(
        listOf(
            LogEntry(level = LogLevel.INFO, tag = "System", message = "UnifiedShield Tunnel Core Engine v3.2 Initialized"),
            LogEntry(level = LogLevel.TUNNEL, tag = "TunnelManager", message = "Multi-Tunnel Engine loaded: DNSTT, NoizDNS, VayDNS, Slipstream, NaiveProxy, Tor"),
            LogEntry(level = LogLevel.SCANNER, tag = "AutoScanner", message = "Auto-Scanner ready: SNI, IPv4/IPv6, EDNS0, DoH & Tor probes active"),
            LogEntry(level = LogLevel.DPI, tag = "DpiGuard", message = "Zero-leak DNS & Noise IK Cryptographic Layer Armed")
        )
    )
    val logs: StateFlow<List<LogEntry>> = _logs

    fun log(level: LogLevel, tag: String, message: String) {
        val entry = LogEntry(level = level, tag = tag, message = message)
        val currentList = _logs.value.toMutableList()
        currentList.add(0, entry) // prepend for recent first
        if (currentList.size > maxLogEntries) {
            currentList.removeAt(currentList.lastIndex)
        }
        _logs.value = currentList
    }

    fun info(tag: String, message: String) = log(LogLevel.INFO, tag, message)
    fun addLog(tag: String, message: String) = log(LogLevel.INFO, tag, message)
    fun warn(tag: String, message: String) = log(LogLevel.WARN, tag, message)
    fun dpi(tag: String, message: String) = log(LogLevel.DPI, tag, message)
    fun packet(tag: String, message: String) = log(LogLevel.PACKET, tag, message)
    fun tunnel(tag: String, message: String) = log(LogLevel.TUNNEL, tag, message)
    fun scanner(tag: String, message: String) = log(LogLevel.SCANNER, tag, message)

    fun clearLogs() {
        _logs.value = emptyList()
    }

    fun exportLogsText(): String {
        return _logs.value.joinToString("\n") { "[${it.formattedTime}] [${it.level.label}] [${it.tag}] ${it.message}" }
    }

    fun exportLogsToFile(context: android.content.Context): java.io.File? {
        return try {
            val fileName = "unifiedshield_connection_logs_${System.currentTimeMillis()}.txt"
            val file = java.io.File(context.cacheDir, fileName)
            file.writeText("=== UnifiedShield Connection & Security Diagnostics Log ===\nExport Time: ${Date()}\nTotal Entries: ${_logs.value.size}\n\n" + exportLogsText())
            file
        } catch (e: Exception) {
            null
        }
    }

    companion object {
        @Volatile
        private var instance: DebugLogger? = null

        fun getInstance(): DebugLogger {
            return instance ?: synchronized(this) {
                instance ?: DebugLogger().also { instance = it }
            }
        }
    }
}
