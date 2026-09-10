package com.unifiedshield.ui

import androidx.compose.animation.*
import androidx.compose.animation.core.*
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.unifiedshield.TunnelManager
import com.unifiedshield.UnifiedShieldStore
import com.unifiedshield.WarTimeResilienceEngine
import com.unifiedshield.autopilot.AutoPilotEngine
import com.unifiedshield.doctor.LeakTestMonitor
import com.unifiedshield.profile.ProfileManager
import com.unifiedshield.ui.components.EnterpriseStatusPill
import com.unifiedshield.ui.components.StatusPillType
import com.unifiedshield.ui.center.SecurityCenterScreen
import com.unifiedshield.ui.hub.SecurityStatusHubScreen
import com.unifiedshield.ui.intel.ProtocolIntelligenceScreen
import com.unifiedshield.ui.license.AccountLicenseScreen
import com.unifiedshield.ui.netmap.NetworkMapScreen
import com.unifiedshield.ui.providers.AiApiPanelScreen
import com.unifiedshield.ui.providers.ConnectionDoctorScreen

enum class MainNavSection(val labelFa: String, val icon: ImageVector) {
    DASHBOARD("داشبورد", Icons.Default.Shield),
    PROTECTION("حفاظت", Icons.Default.AltRoute),
    INTELLIGENCE("هوش مصنوعی", Icons.Default.Psychology),
    DNS_NETWORK("شبکه و DNS", Icons.Default.Hub),
    SETTINGS_SECURITY("امنیت و تنظیمات", Icons.Default.Settings)
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MainScreen(
    onConnectClick: () -> Unit,
    onDisconnectClick: () -> Unit
) {
    val context = LocalContext.current
    val tunnelManager = remember { TunnelManager.getInstance(context) }
    val tunnelStats by tunnelManager.stats.collectAsState()
    val warEngine = remember { WarTimeResilienceEngine.getInstance() }
    val warState by warEngine.warState.collectAsState()
    val store = remember { UnifiedShieldStore.getInstance(context) }
    val storeState by store.state.collectAsState()
    val profileManager = remember { ProfileManager.getInstance(context) }

    var currentSection by remember { mutableStateOf(MainNavSection.DASHBOARD) }
    var dashboardSubTab by remember { mutableIntStateOf(0) }
    var protectionSubTab by remember { mutableIntStateOf(0) }
    var intelligenceSubTab by remember { mutableIntStateOf(0) }
    var dnsNetworkSubTab by remember { mutableIntStateOf(0) }
    var settingsSubTab by remember { mutableIntStateOf(0) }

    var showVoiceDialog by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .background(MaterialTheme.colorScheme.surface)
            ) {
                TopAppBar(
                    title = {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween,
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Box(
                                    modifier = Modifier
                                        .size(36.dp)
                                        .clip(RoundedCornerShape(8.dp))
                                        .background(MaterialTheme.colorScheme.surfaceVariant),
                                    contentAlignment = Alignment.Center
                                ) {
                                    Image(
                                        painter = painterResource(id = com.unifiedshield.R.drawable.ic_micafp_shield_logo),
                                        contentDescription = "UnifiedShield Logo",
                                        modifier = Modifier.size(24.dp),
                                        contentScale = ContentScale.Fit
                                    )
                                }
                                Spacer(modifier = Modifier.width(10.dp))
                                Column {
                                    Text(
                                        "MICAFP",
                                        style = MaterialTheme.typography.titleMedium,
                                        fontWeight = FontWeight.Bold,
                                        color = MaterialTheme.colorScheme.onSurface
                                    )
                                    Text(
                                        "سامانه یکپارچه حفاظت ضد DPI و حریم خصوصی",
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        fontSize = 10.5.sp
                                    )
                                }
                            }

                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(8.dp)
                            ) {
                                EnterpriseStatusPill(
                                    text = if (tunnelStats.connected) "محافظت‌شده" else "آماده اتصال",
                                    type = if (tunnelStats.connected) StatusPillType.SUCCESS else StatusPillType.INFO
                                )

                                IconButton(
                                    onClick = { showVoiceDialog = true },
                                    modifier = Modifier
                                        .size(36.dp)
                                        .clip(CircleShape)
                                        .background(MaterialTheme.colorScheme.surfaceVariant)
                                        .testTag("topbar_voice_command_btn")
                                ) {
                                    Icon(
                                        imageVector = Icons.Default.Mic,
                                        contentDescription = "فرمان صوتی هوشمند",
                                        tint = MaterialTheme.colorScheme.onSurface,
                                        modifier = Modifier.size(18.dp)
                                    )
                                }
                            }
                        }
                    },
                    colors = TopAppBarDefaults.topAppBarColors(
                        containerColor = MaterialTheme.colorScheme.surface,
                        titleContentColor = MaterialTheme.colorScheme.onSurface
                    )
                )

                // Sub-Tabs for Section Navigation
                val subTabs: List<Pair<String, ImageVector>> = when (currentSection) {
                    MainNavSection.DASHBOARD -> listOf(
                        "وضعیت و اتصال" to Icons.Default.Shield,
                        "پایش فنی MICAFP" to Icons.Default.Tune,
                        "میدان ذرات" to Icons.Default.ScatterPlot,
                        "مرکز وضعیت امنیتی" to Icons.Default.HealthAndSafety
                    )
                    MainNavSection.PROTECTION -> listOf(
                        "هسته دوحالته (v70)" to Icons.Default.AltRoute,
                        "تونل‌ها" to Icons.Default.Dns,
                        "استتار و ضد DPI" to Icons.Default.VisibilityOff,
                        "پروتکل‌های پیشرفته" to Icons.Default.ElectricBolt,
                        "تغییر هسته (CoreSwitcher)" to Icons.Default.Tune,
                        "هوش پروتکل" to Icons.Default.Insights
                    )
                    MainNavSection.INTELLIGENCE -> listOf(
                        "موتور هوش مصنوعی" to Icons.Default.Psychology,
                        "پویشگر خودکار" to Icons.Default.Troubleshoot,
                        "نقشه حرارتی DPI" to Icons.Default.Sensors,
                        "هوش تهدیدات" to Icons.Default.Security,
                        "تشخیص تفکیکی DPI" to Icons.Default.BugReport,
                        "نمودارهای تله‌متری" to Icons.Default.Timeline,
                        "کادر API هوش مصنوعی" to Icons.Default.Api
                    )
                    MainNavSection.DNS_NETWORK -> listOf(
                        "WhiteDNS" to Icons.Default.Radar,
                        "MasterDNS (9X)" to Icons.Default.Bolt,
                        "StormDNS" to Icons.Default.Storm,
                        "CottenDNS" to Icons.Default.Hub,
                        "اینترنت ملی" to Icons.Default.Lan,
                        "رله اینترانت ایران" to Icons.Default.DeviceHub,
                        "نقشه شبکه" to Icons.Default.Map
                    )
                    MainNavSection.SETTINGS_SECURITY -> listOf(
                        "امنیت و کیل‌سوئیچ" to Icons.Default.Lock,
                        "ابزارها و عیب‌یابی" to Icons.Default.Construction,
                        "تنظیمات عمومی" to Icons.Default.Settings,
                        "مرکز امنیت" to Icons.Default.VerifiedUser,
                        "پزشک اتصال" to Icons.Default.MedicalServices,
                        "حساب و لایسنس" to Icons.Default.Key
                    )
                }

                val activeSubIndex = when (currentSection) {
                    MainNavSection.DASHBOARD -> dashboardSubTab
                    MainNavSection.PROTECTION -> protectionSubTab
                    MainNavSection.INTELLIGENCE -> intelligenceSubTab
                    MainNavSection.DNS_NETWORK -> dnsNetworkSubTab
                    MainNavSection.SETTINGS_SECURITY -> settingsSubTab
                }

                ScrollableTabRow(
                    selectedTabIndex = activeSubIndex.coerceIn(0, subTabs.size - 1),
                    edgePadding = 12.dp,
                    containerColor = MaterialTheme.colorScheme.surface,
                    contentColor = MaterialTheme.colorScheme.primary,
                    divider = {
                        HorizontalDivider(color = MaterialTheme.colorScheme.outline)
                    }
                ) {
                    subTabs.forEachIndexed { index, (title, icon) ->
                        val isSelected = activeSubIndex == index
                        Tab(
                            selected = isSelected,
                            onClick = {
                                when (currentSection) {
                                    MainNavSection.DASHBOARD -> dashboardSubTab = index
                                    MainNavSection.PROTECTION -> protectionSubTab = index
                                    MainNavSection.INTELLIGENCE -> intelligenceSubTab = index
                                    MainNavSection.DNS_NETWORK -> dnsNetworkSubTab = index
                                    MainNavSection.SETTINGS_SECURITY -> settingsSubTab = index
                                }
                            },
                            text = {
                                Row(
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                                    modifier = Modifier.padding(vertical = 8.dp)
                                ) {
                                    Icon(
                                        icon,
                                        contentDescription = null,
                                        modifier = Modifier.size(15.dp),
                                        tint = if (isSelected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant
                                    )
                                    Text(
                                        title,
                                        style = MaterialTheme.typography.labelMedium,
                                        fontWeight = if (isSelected) FontWeight.SemiBold else FontWeight.Normal,
                                        color = if (isSelected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant
                                    )
                                }
                            },
                            modifier = Modifier.testTag("sub_nav_tab_${currentSection.name}_$index")
                        )
                    }
                }

                // Panic Alert Strip (Calm warning styling)
                if (storeState.isPanicModeActive) {
                    Surface(
                        color = MaterialTheme.colorScheme.errorContainer,
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable {
                                currentSection = MainNavSection.SETTINGS_SECURITY
                                settingsSubTab = 0
                            }
                            .testTag("panic_alert_strip")
                    ) {
                        Row(
                            modifier = Modifier.padding(horizontal = 14.dp, vertical = 8.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(6.dp)
                            ) {
                                Icon(Icons.Default.Warning, contentDescription = null, tint = MaterialTheme.colorScheme.error, modifier = Modifier.size(16.dp))
                                Text(
                                    "حالت اضطراری فعال است: هسته محافظ (${storeState.activeCore}) فعال است",
                                    color = MaterialTheme.colorScheme.onErrorContainer,
                                    style = MaterialTheme.typography.bodySmall,
                                    fontWeight = FontWeight.Medium
                                )
                            }
                            Text("مدیریت", color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.labelSmall, fontWeight = FontWeight.Bold)
                        }
                    }
                }
            }
        },
        bottomBar = {
            NavigationBar(
                containerColor = MaterialTheme.colorScheme.surface,
                tonalElevation = 0.dp,
                modifier = Modifier
                    .border(
                        BorderStroke(1.dp, MaterialTheme.colorScheme.outline)
                    )
                    .testTag("main_bottom_nav")
            ) {
                MainNavSection.values().forEach { section ->
                    val isSelected = currentSection == section
                    NavigationBarItem(
                        selected = isSelected,
                        onClick = { currentSection = section },
                        icon = {
                            Icon(
                                imageVector = section.icon,
                                contentDescription = section.labelFa,
                                modifier = Modifier.size(20.dp)
                            )
                        },
                        label = {
                            Text(
                                text = section.labelFa,
                                style = MaterialTheme.typography.labelSmall,
                                fontWeight = if (isSelected) FontWeight.SemiBold else FontWeight.Normal
                            )
                        },
                        colors = NavigationBarItemDefaults.colors(
                            selectedIconColor = MaterialTheme.colorScheme.primary,
                            selectedTextColor = MaterialTheme.colorScheme.primary,
                            unselectedIconColor = MaterialTheme.colorScheme.onSurfaceVariant,
                            unselectedTextColor = MaterialTheme.colorScheme.onSurfaceVariant,
                            indicatorColor = MaterialTheme.colorScheme.primaryContainer
                        ),
                        modifier = Modifier.testTag("nav_item_${section.name}")
                    )
                }
            }
        }
    ) { innerPadding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.background)
                .padding(innerPadding)
        ) {
            AnimatedContent(
                targetState = currentSection to when (currentSection) {
                    MainNavSection.DASHBOARD -> dashboardSubTab
                    MainNavSection.PROTECTION -> protectionSubTab
                    MainNavSection.INTELLIGENCE -> intelligenceSubTab
                    MainNavSection.DNS_NETWORK -> dnsNetworkSubTab
                    MainNavSection.SETTINGS_SECURITY -> settingsSubTab
                },
                transitionSpec = {
                    fadeIn(animationSpec = tween(180)) togetherWith fadeOut(animationSpec = tween(150))
                },
                label = "main_screen_tab_transition"
            ) { (section, subTab) ->
                when (section) {
                    MainNavSection.DASHBOARD -> {
                        when (subTab) {
                            0 -> StatusCard(
                                isConnected = tunnelStats.connected,
                                currentCore = tunnelStats.currentCore,
                                ispName = tunnelStats.activeIsp,
                                uploadSpeed = "${tunnelStats.uploadSpeedKbps} KB/s",
                                downloadSpeed = "${tunnelStats.downloadSpeedKbps} KB/s",
                                dpiScore = tunnelStats.dpiScore,
                                onConnectClick = onConnectClick,
                                onDisconnectClick = onDisconnectClick
                            )
                            1 -> MicafpQuantumDashboardPanel(isConnected = tunnelStats.connected)
                            2 -> Column(
                                modifier = Modifier
                                    .fillMaxSize()
                                    .padding(16.dp),
                                verticalArrangement = Arrangement.spacedBy(14.dp)
                            ) {
                                Quantum3DParticleCanvas(
                                    isConnected = tunnelStats.connected,
                                    // A2 follow-up: now wired to TunnelManager's real
                                    // TCP-connect-timing probe (see TunnelStats.latencyMs).
                                    latencyMs = tunnelStats.latencyMs,
                                    evasionActive = true,
                                    modifier = Modifier.fillMaxWidth().height(360.dp)
                                )
                            }
                            // ── MICAFP Directive v6 / B3.1 — Security Status Hub (additive) ──
                            3 -> SecurityStatusHubScreen(
                                isConnected = tunnelStats.connected,
                                currentCore = tunnelStats.currentCore,
                                downloadKbps = tunnelStats.downloadSpeedKbps,
                                dpiScore = tunnelStats.dpiScore,
                                latencyMs = tunnelStats.latencyMs
                            )
                        }
                    }
                    MainNavSection.PROTECTION -> {
                        when (subTab) {
                            0 -> DualModeTransportScreen()
                            1 -> TunnelsScreen()
                            2 -> ObfuscationScreen()
                            3 -> NovelProtocolsScreen()
                            4 -> CoreSwitcher(
                                currentCore = tunnelStats.currentCore,
                                onCoreSelected = { core ->
                                    tunnelManager.performHotSwap("Manual CoreSwitcher selection")
                                }
                            )
                            // ── MICAFP Directive v6 / B3.2 — Protocol Intelligence (additive) ──
                            5 -> ProtocolIntelligenceScreen()
                        }
                    }
                    MainNavSection.INTELLIGENCE -> {
                        when (subTab) {
                            0 -> AiEngineScreen()
                            1 -> AutoScannerScreen()
                            2 -> DPIHeatmapPanel()
                            3 -> ThreatIntelPanel()
                            4 -> DpiDiagnosticScreen()
                            5 -> DiagnosticTelemetrySection()
                            // ── MICAFP Directive v6 / C3 + B.8-B.10 — AI API Panel (additive) ──
                            6 -> AiApiPanelScreen()
                        }
                    }
                    MainNavSection.DNS_NETWORK -> {
                        when (subTab) {
                            0 -> WhiteDnsScreen()
                            1 -> MasterDnsScreen(profileManager = profileManager)
                            2 -> StormDnsScreen()
                            3 -> CottenDnsScreen()
                            4 -> IntranetScreen()
                            5 -> IranIntranetRelayPanel()
                            // ── MICAFP Directive v6 / B3.4 — Network Map (additive) ──
                            6 -> NetworkMapScreen()
                        }
                    }
                    MainNavSection.SETTINGS_SECURITY -> {
                        when (subTab) {
                            0 -> SecurityScreen()
                            1 -> AdvancedToolsScreen()
                            2 -> SettingsScreen()
                            // ── MICAFP Directive v6 / B3.3, B3.6 + advanced features (additive) ──
                            3 -> SecurityCenterScreen()
                            4 -> ConnectionDoctorScreen()
                            5 -> AccountLicenseScreen()
                        }
                    }
                }
            }
        }

        if (showVoiceDialog) {
            VoiceCommandDialog(
                onDismissRequest = { showVoiceDialog = false },
                onConnectRequested = onConnectClick,
                onDisconnectRequested = onDisconnectClick
            )
        }
    }
}
