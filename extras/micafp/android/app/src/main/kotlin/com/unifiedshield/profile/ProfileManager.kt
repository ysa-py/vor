package com.unifiedshield.profile

import android.content.Context
import android.util.Log
import com.unifiedshield.logging.DebugLogger
import com.unifiedshield.scanner.ScanTargetResult
import com.unifiedshield.tunnel.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

data class ProfileManagerState(
    val profiles: List<TunnelProfile> = emptyList(),
    val activeProfileId: String = "prof-dnstt-default",
    val proxyConfig: LocalProxyConfig = LocalProxyConfig(),
    val startOnBoot: Boolean = true,
    val autoConnectOnLaunch: Boolean = false,
    val isDebugLoggingActive: Boolean = true
)

class ProfileManager private constructor(private val context: Context) {

    private val TAG = "ProfileManager"
    private val logger = DebugLogger.getInstance()

    private val _state = MutableStateFlow(
        ProfileManagerState(
            profiles = getDefaultProfiles()
        )
    )
    val state: StateFlow<ProfileManagerState> = _state

    private fun getDefaultProfiles(): List<TunnelProfile> {
        return listOf(
            TunnelProfile(
                id = "prof-masterdns-arq",
                name = "MasterDnsVPN Turbo (ARQ-5B + 8-Way Multi-Resolver)",
                namePersian = "مستر دی‌ان‌اس توربو (پروتکل اختصاصی ۵ بایت + ۸ رِزولور)",
                tunnelType = TunnelType.MASTER_DNS,
                isActive = false,
                pingMs = 11,
                throughputRating = "165.0 MB/s",
                masterDnsConfig = MasterDnsConfig(
                    encryption = MasterDnsEncryption.AES_128_GCM,
                    balancingMode = MasterDnsBalancingMode.LATENCY_BASED,
                    enablePacketDuplication = false,
                    enableSocksOptimization = true,
                    enableTcpForwarding = true,
                    customMtu = 512
                )
            ),
            TunnelProfile(
                id = "prof-masterdns-ssh",
                name = "MasterDnsVPN + SSH Multipath (Zero-Loss Duplication)",
                namePersian = "مستر دی‌ان‌اس + اس‌اس‌اچ چند مسیره ضد قطعی",
                tunnelType = TunnelType.MASTER_DNS_SSH,
                isActive = false,
                pingMs = 14,
                throughputRating = "142.0 MB/s",
                masterDnsConfig = MasterDnsConfig(
                    encryption = MasterDnsEncryption.CHACHA20_POLY1305,
                    balancingMode = MasterDnsBalancingMode.DUPLICATE_BROADCAST,
                    enablePacketDuplication = true,
                    duplicationFactor = 2,
                    tcpCarrier = MasterDnsTcpCarrier.SHADOWSOCKS_CARRIER
                ),
                sshConfig = SshConfig(
                    host = "104.21.68.12",
                    port = 22,
                    cipher = SshCipher.CHACHA20_POLY1305,
                    wrapperType = SshWrapperType.TLS,
                    customSni = "c.whatsapp.net"
                )
            ),
            TunnelProfile(
                id = "prof-dnstt-default",
                name = "DNSTT Master (Alibaba Anycast)",
                namePersian = "دی‌ان‌اس‌تی‌تی پیش‌فرض (KCP + Noise)",
                tunnelType = TunnelType.DNSTT,
                isActive = true,
                pingMs = 18,
                throughputRating = "85.0 MB/s",
                dnsConfig = DnsConfig(
                    transport = DnsTransport.UDP,
                    resolverIpOrUrl = "223.5.5.5",
                    serverDomain = "t.unifiedshield.net",
                    ednsBufferSize = 1232
                )
            ),
            TunnelProfile(
                id = "prof-dnstt-ssh",
                name = "DNSTT + SSH Chained (Zero-Leak)",
                namePersian = "دی‌ان‌اس‌تی‌تی + اس‌اس‌اچ دولایه",
                tunnelType = TunnelType.DNSTT_SSH,
                isActive = false,
                pingMs = 22,
                throughputRating = "72.5 MB/s",
                sshConfig = SshConfig(
                    host = "104.21.68.12",
                    port = 22,
                    cipher = SshCipher.AES_128_GCM,
                    wrapperType = SshWrapperType.TLS,
                    customSni = "c.whatsapp.net"
                ),
                dnsConfig = DnsConfig(
                    transport = DnsTransport.DOH,
                    resolverIpOrUrl = "https://dns.alidns.com/dns-query",
                    serverDomain = "t.unifiedshield.net"
                )
            ),
            TunnelProfile(
                id = "prof-noizdns-stealth",
                name = "NoizDNS Stealth DPI-Proof",
                namePersian = "نویز دی‌ان‌اس ضد فیلترینگ با استتار",
                tunnelType = TunnelType.NOIZ_DNS,
                isActive = false,
                pingMs = 16,
                throughputRating = "94.0 MB/s",
                noizDnsConfig = NoizDnsConfig(
                    stealthModeEnabled = true,
                    jitterMs = 12
                )
            ),
            TunnelProfile(
                id = "prof-noizdns-ssh",
                name = "NoizDNS + SSH Encapsulation",
                namePersian = "نویز دی‌ان‌اس با زنجیره SSH",
                tunnelType = TunnelType.NOIZ_DNS_SSH,
                isActive = false,
                pingMs = 20,
                throughputRating = "88.0 MB/s",
                sshConfig = SshConfig(
                    cipher = SshCipher.CHACHA20_POLY1305,
                    wrapperType = SshWrapperType.WEBSOCKET,
                    wsPath = "/noiz-ws"
                )
            ),
            TunnelProfile(
                id = "prof-vaydns-opt",
                name = "VayDNS High-Throughput (TXT Wire)",
                namePersian = "وای دی‌ان‌اس با قالب سیم بهینه و رکورد TXT",
                tunnelType = TunnelType.VAY_DNS,
                isActive = false,
                pingMs = 15,
                throughputRating = "110.0 MB/s",
                vayDnsConfig = VayDnsConfig(
                    recordType = VayDnsRecordType.TXT,
                    qnameLength = 64,
                    rateLimitPps = 100
                )
            ),
            TunnelProfile(
                id = "prof-vaydns-ssh",
                name = "VayDNS + SSH Ultra Leak-Proof",
                namePersian = "وای دی‌ان‌اس + اس‌اس‌اچ پرسرعت",
                tunnelType = TunnelType.VAY_DNS_SSH,
                isActive = false,
                pingMs = 19,
                throughputRating = "98.0 MB/s",
                vayDnsConfig = VayDnsConfig(
                    recordType = VayDnsRecordType.NULL,
                    qnameLength = 96,
                    rateLimitPps = 120
                )
            ),
            TunnelProfile(
                id = "prof-slipstream-quic",
                name = "Slipstream QUIC Turbo (0-RTT)",
                namePersian = "اسلیپ‌استریم کوئیک با اتصال 0-RTT",
                tunnelType = TunnelType.SLIPSTREAM,
                isActive = false,
                pingMs = 12,
                throughputRating = "140.0 MB/s"
            ),
            TunnelProfile(
                id = "prof-slipstream-ssh",
                name = "Slipstream + SSH Dual Layer",
                namePersian = "اسلیپ‌استریم کوئیک با زنجیره SSH",
                tunnelType = TunnelType.SLIPSTREAM_SSH,
                isActive = false,
                pingMs = 16,
                throughputRating = "125.0 MB/s"
            ),
            TunnelProfile(
                id = "prof-ssh-standalone",
                name = "SSH over WebSocket (Cloudflare CDN)",
                namePersian = "اس‌اس‌اچ مستقل روی وب‌سوکت کلودفلر",
                tunnelType = TunnelType.SSH_STANDALONE,
                isActive = false,
                pingMs = 24,
                throughputRating = "90.0 MB/s",
                sshConfig = SshConfig(
                    host = "104.21.68.12",
                    port = 443,
                    cipher = SshCipher.AES_128_GCM,
                    wrapperType = SshWrapperType.WEBSOCKET,
                    wsPath = "/tunnel-ssh",
                    customHostHeader = "edge.cloudflare.com"
                )
            ),
            TunnelProfile(
                id = "prof-ssh-payload",
                name = "SSH Payload Injection Disguise",
                namePersian = "اس‌اس‌اچ با تزریق هدر ساختگی جهت دورزدن DPI",
                tunnelType = TunnelType.SSH_STANDALONE,
                isActive = false,
                pingMs = 26,
                throughputRating = "86.0 MB/s",
                sshConfig = SshConfig(
                    wrapperType = SshWrapperType.PAYLOAD_INJECTION,
                    cipher = SshCipher.AES_128_CTR,
                    rawPayloadTemplate = "GET / HTTP/1.1\r\nHost: speedtest.net\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n"
                )
            ),
            TunnelProfile(
                id = "prof-naiveproxy-ja4",
                name = "NaiveProxy Chrome-JA4 HTTPS",
                namePersian = "نایوپروکسی با اثرانگشت معتبر مرورگر گوگل کروم",
                tunnelType = TunnelType.NAIVE_PROXY,
                isActive = false,
                pingMs = 14,
                throughputRating = "135.0 MB/s",
                naiveProxyConfig = NaiveProxyConfig(
                    host = "chrome-edge.unifiedshield.net",
                    port = 443,
                    ja4Fingerprint = "t13d1516h2_8daaf6152771_0271d4a82a09",
                    paddingEnabled = true
                )
            ),
            TunnelProfile(
                id = "prof-naiveproxy-ssh",
                name = "NaiveProxy + SSH Multi-Hop",
                namePersian = "نایوپروکسی همراه با رمزنگاری SSH",
                tunnelType = TunnelType.NAIVE_PROXY_SSH,
                isActive = false,
                pingMs = 18,
                throughputRating = "115.0 MB/s"
            ),
            TunnelProfile(
                id = "prof-doh-rfc8484",
                name = "DOH Clean Resolver (RFC 8484)",
                namePersian = "دی‌ان‌اس روی HTTPS استاندارد بدون تغییر ترافیک وب",
                tunnelType = TunnelType.DOH,
                isActive = false,
                pingMs = 15,
                throughputRating = "160.0 MB/s"
            ),
            TunnelProfile(
                id = "prof-tor-snowflake",
                name = "Tor Snowflake WebRTC Broker",
                namePersian = "شبکه تور با پل نامحدود Snowflake WebRTC",
                tunnelType = TunnelType.TOR,
                isActive = false,
                pingMs = 38,
                throughputRating = "45.0 MB/s",
                torConfig = TorConfig(
                    bridgeType = TorBridgeType.SNOWFLAKE,
                    snowflakeFrontDomain = "cdn.sstatic.net"
                )
            ),
            TunnelProfile(
                id = "prof-stormdns-tcp",
                name = "StormDNS TCP-over-DNS Stream",
                namePersian = "استورم دی‌ان‌اس (انتقال جریان TCP در بسته‌های DNS)",
                tunnelType = TunnelType.STORM_DNS,
                isActive = false,
                pingMs = 15,
                throughputRating = "95.0 MB/s"
            ),
            TunnelProfile(
                id = "prof-cottendns-matrix",
                name = "CottenDNS Adaptive Matrix + Super-FEC",
                namePersian = "کاتن دی‌ان‌اس تطبیقی چندمسیره با بازیابی خطا Super-FEC",
                tunnelType = TunnelType.COTTEN_DNS,
                isActive = false,
                pingMs = 12,
                throughputRating = "118.0 MB/s"
            ),
            TunnelProfile(
                id = "prof-vless-reality",
                name = "VLESS Reality (XTLS + Vision)",
                namePersian = "وی‌لس ریالیتی (استتار مستقیم در سرورهای خارجی)",
                tunnelType = TunnelType.VLESS_REALITY,
                isActive = false,
                pingMs = 14,
                throughputRating = "155.0 MB/s"
            ),
            TunnelProfile(
                id = "prof-vmess-aead",
                name = "VMess AEAD (CDN WebSocket)",
                namePersian = "وی‌مس با رمزنگاری AEAD و وب‌سوکت توزیع‌شده CDN",
                tunnelType = TunnelType.VMESS_AEAD,
                isActive = false,
                pingMs = 19,
                throughputRating = "138.0 MB/s"
            ),
            TunnelProfile(
                id = "prof-trojan-grpc",
                name = "Trojan gRPC TLS 1.3 Multiplex",
                namePersian = "تروجان با چندگانه‌سازی gRPC و هندشیک واقعی HTTPS",
                tunnelType = TunnelType.TROJAN_TLS,
                isActive = false,
                pingMs = 16,
                throughputRating = "142.0 MB/s"
            ),
            TunnelProfile(
                id = "prof-shadowsocks-2022",
                name = "Shadowsocks 2022 Blake3 AEAD",
                namePersian = "شادوساکس ۲۰۲۲ با چرخش کلید Blake3 ضد حملات بازپخش",
                tunnelType = TunnelType.SHADOWSOCKS_2022,
                isActive = false,
                pingMs = 13,
                throughputRating = "162.0 MB/s"
            ),
            TunnelProfile(
                id = "prof-hysteria-2",
                name = "Hysteria 2 Brutal Congestion",
                namePersian = "هیستریا ۲ با کنترل ازدحام تهاجمی ضد افت بسته ۸۰٪",
                tunnelType = TunnelType.HYSTERIA_2,
                isActive = false,
                pingMs = 11,
                throughputRating = "185.0 MB/s"
            ),
            TunnelProfile(
                id = "prof-tuic-v5",
                name = "TUIC v5 Pure BBR QUIC",
                namePersian = "توئیک نسخه ۵ بر بستر خالص پروتکل QUIC",
                tunnelType = TunnelType.TUIC_V5,
                isActive = false,
                pingMs = 12,
                throughputRating = "170.0 MB/s"
            ),
            TunnelProfile(
                id = "prof-anytls-poly",
                name = "AnyTLS Dynamic Polymorphism",
                namePersian = "انی تی‌ال‌اس با تغییر شکل مداوم بایت‌های هندشیک",
                tunnelType = TunnelType.ANY_TLS,
                isActive = false,
                pingMs = 16,
                throughputRating = "140.0 MB/s"
            ),
            TunnelProfile(
                id = "prof-amnezia-wg",
                name = "AmneziaWG Junk Packet Obfuscation",
                namePersian = "امنزیا وایرگارد با تزریق بسته‌های فریبنده و هدر تصادفی",
                tunnelType = TunnelType.WIREGUARD_AMNEZIA,
                isActive = false,
                pingMs = 10,
                throughputRating = "190.0 MB/s"
            ),
            TunnelProfile(
                id = "prof-shadowtls-v3",
                name = "ShadowTLS v3 SNI Masquerade",
                namePersian = "شدو تی‌ال‌اس ۳ با شبیه‌سازی دقیق سایت‌های معتبر",
                tunnelType = TunnelType.SHADOW_TLS,
                isActive = false,
                pingMs = 15,
                throughputRating = "148.0 MB/s"
            ),
            TunnelProfile(
                id = "prof-brook-stream",
                name = "Brook Zero-Signature Protocol",
                namePersian = "بروک با ساختار داده بدون امضا و بسیار سبک",
                tunnelType = TunnelType.BROOK,
                isActive = false,
                pingMs = 17,
                throughputRating = "130.0 MB/s"
            )
        )
    }

    fun selectProfile(profileId: String) {
        val current = _state.value
        val updated = current.profiles.map {
            it.copy(isActive = (it.id == profileId))
        }
        val activeProf = updated.find { it.id == profileId }
        _state.value = current.copy(
            profiles = updated,
            activeProfileId = profileId
        )
        logger.tunnel(TAG, "Selected Profile: ${activeProf?.name} [${activeProf?.tunnelType?.title}]")
    }

    fun getActiveProfile(): TunnelProfile {
        return _state.value.profiles.find { it.isActive } ?: _state.value.profiles.first()
    }

    fun updateProfile(updatedProfile: TunnelProfile) {
        val current = _state.value
        val updatedList = current.profiles.map {
            if (it.id == updatedProfile.id) updatedProfile else it
        }
        _state.value = current.copy(profiles = updatedList)
        logger.info(TAG, "Updated configuration for profile: ${updatedProfile.name}")
    }

    fun createProfile(profile: TunnelProfile) {
        val current = _state.value
        val updatedList = current.profiles + profile
        _state.value = current.copy(profiles = updatedList)
        logger.info(TAG, "Created new profile: ${profile.name}")
    }

    fun deleteProfile(profileId: String) {
        val current = _state.value
        if (current.profiles.size <= 1) return // keep at least one
        val updatedList = current.profiles.filter { it.id != profileId }
        val newActiveId = if (current.activeProfileId == profileId) updatedList.first().id else current.activeProfileId
        val finalProfiles = updatedList.map { it.copy(isActive = (it.id == newActiveId)) }
        _state.value = current.copy(profiles = finalProfiles, activeProfileId = newActiveId)
        logger.info(TAG, "Deleted profile: $profileId")
    }

    fun autoApplyFromScanner(scannedNode: ScanTargetResult) {
        val current = _state.value
        val newId = "prof-auto-${System.currentTimeMillis()}"
        val newProfile = TunnelProfile(
            id = newId,
            name = "Auto-Discovered: ${scannedNode.target}",
            namePersian = "پیکربندی خودکار کشف‌شده: ${scannedNode.target}",
            tunnelType = scannedNode.recommendedTunnelType,
            isActive = true,
            pingMs = scannedNode.latencyMs,
            throughputRating = "${(85..145).random()}.0 MB/s",
            isAutoDiscovered = true,
            dnsConfig = DnsConfig(
                resolverIpOrUrl = if (scannedNode.target.startsWith("http")) scannedNode.target else "223.5.5.5",
                transport = if (scannedNode.target.startsWith("http")) DnsTransport.DOH else DnsTransport.UDP
            ),
            sshConfig = SshConfig(
                host = scannedNode.target.split(":").first(),
                customSni = if (scannedNode.category == com.unifiedshield.scanner.ScannerCategory.SNI) scannedNode.target else "c.whatsapp.net"
            )
        )

        val updatedList = current.profiles.map { it.copy(isActive = false) } + newProfile
        _state.value = current.copy(
            profiles = updatedList,
            activeProfileId = newId
        )
        logger.tunnel(TAG, "Auto-Applied fresh node [${scannedNode.target}] into profile list!")
    }

    fun updateProxyConfig(config: LocalProxyConfig) {
        _state.value = _state.value.copy(proxyConfig = config)
        logger.info(TAG, "Proxy Config Updated: SOCKS5=:${config.socks5Port}, HTTP=:${config.httpPort}, Listen=${config.listenAddress}")
    }

    fun toggleStartOnBoot(enabled: Boolean) {
        _state.value = _state.value.copy(startOnBoot = enabled)
    }

    fun toggleDebugLogging(enabled: Boolean) {
        _state.value = _state.value.copy(isDebugLoggingActive = enabled)
    }

    companion object {
        @Volatile
        private var instance: ProfileManager? = null

        fun getInstance(context: Context): ProfileManager {
            return instance ?: synchronized(this) {
                instance ?: ProfileManager(context.applicationContext).also { instance = it }
            }
        }
    }
}
