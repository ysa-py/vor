package com.vor.licensemanager.issuer

import android.content.Intent
import android.os.Bundle
import android.provider.Settings
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.biometric.BiometricManager
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationRail
import androidx.compose.material3.NavigationRailItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Create
import androidx.compose.material.icons.filled.List
import androidx.compose.material.icons.filled.Lock
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import com.vor.licensemanager.MainActivity
import com.vor.licensemanager.R

/**
 * Vor License Manager — ISSUER build variant entry point (see
 * license/SPEC.md, "On-device issuer").
 *
 * What this activity guarantees:
 *  - a lock-screen gate (BiometricPrompt / device PIN) on EVERY app open and
 *    after every return from the background — the issuer UI never renders
 *    without a recent successful authentication;
 *  - zero network by construction (the app has no INTERNET permission; the
 *    only permission the issuer variant adds is USE_BIOMETRIC, injected by
 *    the androidx.biometric AAR manifest);
 *  - the signing seed exists only as an Android-Keystore-wrapped blob that
 *    requires a fresh keyguard authentication to unwrap (IssuerVault).
 *
 * The public VERIFIER build (MainActivity, unchanged) never contains any of
 * this — enforced at build time by the verifyVerifierPurity task.
 */
class IssuerActivity : FragmentActivity() {

    private val vault by lazy { IssuerVault(this) }
    private val ops by lazy { IssuerOps(this, vault) }

    /** Set when the activity goes to the background; forces a re-gate on return. */
    private var rearmOnStart = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        val lifecycleObserver = LifecycleEventObserver { _, event ->
            when (event) {
                Lifecycle.Event.ON_STOP -> rearmOnStart = true
                Lifecycle.Event.ON_START -> if (rearmOnStart) gateState = GateState.Locked
                else -> {}
            }
        }
        lifecycle.addObserver(lifecycleObserver)

        setContent {
            MaterialTheme {
                Surface(modifier = Modifier.fillMaxSize()) {
                    IssuerRoot()
                }
            }
        }
    }

    private var gateState by mutableStateOf(GateState.Locked)

    private enum class GateState {
        Locked,
        NeedLock,
        Open,
    }

    @Composable
    private fun IssuerRoot() {
        val keyguard = remember { getSystemService(android.app.KeyguardManager::class.java) }
        var version by remember { mutableStateOf(0) }
        val hasSeed = remember(version) { vault.hasSeed() }

        when (gateState) {
            GateState.NeedLock -> NeedLockScreen()
            GateState.Locked -> LockedScreen(onUnlock = { gateState = GateState.Open })
            GateState.Open -> IssuerApp(
                ops = ops,
                keyVersion = version,
                onKeyChanged = { version++ },
            )
        }
        // Initial state assessment (once per composition; no looping).
        androidx.compose.runtime.LaunchedEffect(Unit) {
            if (gateState == GateState.Locked) {
                gateState = if (keyguard.isDeviceSecure) GateState.Locked else GateState.NeedLock
            }
        }
    }

    @Composable
    private fun LockedScreen(onUnlock: () -> Unit) {
        var error by remember { mutableStateOf<String?>(null) }
        // Auto-show the system prompt on entry.
        androidx.compose.runtime.LaunchedEffect(Unit) { runGate() }
        Column(
            modifier = Modifier
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
                .padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
        ) {
            Text(
                stringResource(R.string.issuer_gate_title),
                style = MaterialTheme.typography.headlineMedium,
                fontWeight = FontWeight.Bold,
                textAlign = TextAlign.Center,
            )
            Spacer(Modifier.height(8.dp))
            Text(
                stringResource(R.string.issuer_gate_locked_body),
                style = MaterialTheme.typography.bodyMedium,
                textAlign = TextAlign.Center,
                modifier = Modifier.padding(horizontal = 8.dp),
            )
            error?.let {
                Spacer(Modifier.height(12.dp))
                Text(it, color = MaterialTheme.colorScheme.error, textAlign = TextAlign.Center)
            }
            Spacer(Modifier.height(20.dp))
            Button(onClick = { runGate() }) { Text(stringResource(R.string.issuer_unlock)) }
        }
    }

    private fun runGate() {
        if (!BiometricGate.canAuthenticate(this)) {
            gateState = GateState.NeedLock
            return
        }
        BiometricGate.authenticate(this, onSuccess = { gateState = GateState.Open })
    }

    @Composable
    private fun NeedLockScreen() {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
                .padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
        ) {
            Text(
                stringResource(R.string.issuer_need_lock_title),
                style = MaterialTheme.typography.headlineSmall,
                fontWeight = FontWeight.Bold,
                textAlign = TextAlign.Center,
            )
            Spacer(Modifier.height(8.dp))
            Text(
                stringResource(R.string.issuer_need_lock_body),
                style = MaterialTheme.typography.bodyMedium,
                textAlign = TextAlign.Center,
            )
            Spacer(Modifier.height(20.dp))
            Button(onClick = {
                runCatching { startActivity(Intent(Settings.ACTION_SECURITY_SETTINGS)) }
            }) { Text(stringResource(R.string.issuer_need_lock_action)) }
        }
    }
}

private data class IssuerTab(val labelRes: Int, val icon: ImageVector?, val iconRes: Int = 0)

private val TABS = listOf(
    IssuerTab(R.string.issuer_tab_issue, Icons.Filled.Create),
    IssuerTab(R.string.issuer_tab_batch, Icons.Filled.List),
    IssuerTab(R.string.issuer_tab_history, null, R.drawable.ic_issuer_history),
    IssuerTab(R.string.issuer_tab_keys, Icons.Filled.Lock),
)

@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
@Composable
private fun IssuerApp(
    ops: IssuerOps,
    keyVersion: Int,
    onKeyChanged: () -> Unit,
) {
    var tab by remember { mutableStateOf(0) }
    var historyVersion by remember { mutableStateOf(0) }
    var tiersVersion by remember { mutableStateOf(0) }
    var showTiers by remember { mutableStateOf(false) }
    var showAbout by remember { mutableStateOf(false) }
    var openVerifier by remember { mutableStateOf(false) }
    val context = androidx.compose.ui.platform.LocalContext.current

    val tiers = remember(tiersVersion) { IssuerStore.loadTiers(context) }
    val history = remember(historyVersion) { IssuerStore.loadHistory(context) }

    // Adaptive shell: phones keep the Material bottom bar; medium/expanded
    // widths (landscape phones, tablets, foldables unfolded, desktops) get a
    // side navigation rail instead, so content keeps the full screen height.
    BoxWithConstraints {
        val expanded = maxWidth >= 600.dp
        if (expanded) {
            Scaffold(
                topBar = { IssuerTopBar(onOpenVerifier = { openVerifier = true }, onOpenAbout = { showAbout = true }) },
            ) { padding ->
                Row(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(padding)
                        .imePadding(),
                ) {
                    IssuerNavRail(tab = tab, onSelect = { tab = it })
                    Box(Modifier.weight(1f)) {
                        IssuerTabContent(
                            tab = tab,
                            tiers = tiers,
                            history = history,
                            ops = ops,
                            keyVersion = keyVersion,
                            onKeyChanged = onKeyChanged,
                            onHistoryChanged = { historyVersion++ },
                            onOpenTiers = { showTiers = true },
                        )
                    }
                }
            }
        } else {
            Scaffold(
                topBar = { IssuerTopBar(onOpenVerifier = { openVerifier = true }, onOpenAbout = { showAbout = true }) },
                bottomBar = { IssuerBottomBar(tab = tab, onSelect = { tab = it }) },
            ) { padding ->
                Box(
                    Modifier
                        .padding(padding)
                        .imePadding(),
                ) {
                    IssuerTabContent(
                        tab = tab,
                        tiers = tiers,
                        history = history,
                        ops = ops,
                        keyVersion = keyVersion,
                        onKeyChanged = onKeyChanged,
                        onHistoryChanged = { historyVersion++ },
                        onOpenTiers = { showTiers = true },
                    )
                }
            }
        }
    }

    if (showTiers) {
        TiersDialog(
            tiers = tiers,
            onDismiss = { showTiers = false },
            onSave = { updated ->
                IssuerStore.saveTiers(context, updated)
                tiersVersion++
                showTiers = false
            },
        )
    }

    if (showAbout) {
        AboutIssuerDialog(onDismiss = { showAbout = false })
    }

    if (openVerifier) {
        AlertDialog(
            onDismissRequest = { openVerifier = false },
            title = { Text(stringResource(R.string.issuer_verify_action)) },
            text = { Text(stringResource(R.string.issuer_verify_body)) },
            confirmButton = {
                TextButton(onClick = {
                    openVerifier = false
                    runCatching {
                        context.startActivity(Intent(context, MainActivity::class.java))
                    }
                }) { Text(stringResource(R.string.issuer_verify_action)) }
            },
            dismissButton = {
                TextButton(onClick = { openVerifier = false }) {
                    Text(stringResource(R.string.issuer_cancel))
                }
            },
        )
    }
}

@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
@Composable
private fun IssuerTopBar(onOpenVerifier: () -> Unit, onOpenAbout: () -> Unit) {
    TopAppBar(
        title = { Text(stringResource(R.string.issuer_app_title), maxLines = 1) },
        actions = {
            TextButton(onClick = onOpenVerifier) {
                Text(stringResource(R.string.issuer_verify_action), maxLines = 1)
            }
            TextButton(onClick = onOpenAbout) {
                Text(stringResource(R.string.issuer_about_action), maxLines = 1)
            }
        },
    )
}

@Composable
private fun IssuerBottomBar(tab: Int, onSelect: (Int) -> Unit) {
    NavigationBar {
        TABS.forEachIndexed { index, item ->
            NavigationBarItem(
                selected = tab == index,
                onClick = { onSelect(index) },
                icon = { IssuerTabIcon(item) },
                label = { Text(stringResource(item.labelRes), maxLines = 1) },
            )
        }
    }
}

@Composable
private fun IssuerNavRail(tab: Int, onSelect: (Int) -> Unit) {
    NavigationRail {
        TABS.forEachIndexed { index, item ->
            NavigationRailItem(
                selected = tab == index,
                onClick = { onSelect(index) },
                icon = { IssuerTabIcon(item) },
                label = { Text(stringResource(item.labelRes)) },
            )
        }
    }
}

@Composable
private fun IssuerTabIcon(item: IssuerTab) {
    if (item.icon != null) {
        Icon(item.icon, contentDescription = stringResource(item.labelRes))
    } else {
        Icon(
            androidx.compose.ui.res.painterResource(item.iconRes),
            contentDescription = stringResource(item.labelRes),
        )
    }
}

/** Tab body with a light cross-fade/slide so tab switches feel alive. */
@Composable
private fun IssuerTabContent(
    tab: Int,
    tiers: List<String>,
    history: List<IssuerStore.IssuedEntry>,
    ops: IssuerOps,
    keyVersion: Int,
    onKeyChanged: () -> Unit,
    onHistoryChanged: () -> Unit,
    onOpenTiers: () -> Unit,
) {
    // Captured ONCE in a composable context; the callbacks below are plain
    // (non-composable) lambdas invoked after user actions.
    val context = androidx.compose.ui.platform.LocalContext.current
    AnimatedContent(
        targetState = tab,
        transitionSpec = {
            (fadeIn(animationSpec = tween(200)) +
                slideInHorizontally(animationSpec = tween(200)) { it / 16 })
                .togetherWith(fadeOut(animationSpec = tween(140)))
        },
        label = "issuer-tab",
    ) { currentTab ->
        when (currentTab) {
            0 -> IssueFormScreen(
                localTiers = tiers,
                onSign = { request, onDone ->
                    ops.sign(request) { issued ->
                        if (issued != null) onHistoryChanged()
                        onDone(issued)
                    }
                },
                onOpenTiers = onOpenTiers,
            )
            1 -> BatchScreen(
                onSignBatch = { rows, onDone ->
                    ops.signBatch(rows) { issued ->
                        if (issued != null) onHistoryChanged()
                        onDone(issued)
                    }
                },
            )
            2 -> HistoryScreen(
                history = history,
                onDelete = { entry ->
                    IssuerStore.removeHistoryEntry(context, entry)
                    onHistoryChanged()
                },
                onClearAll = {
                    IssuerStore.clearHistory(context)
                    onHistoryChanged()
                },
            )
            else -> KeysScreen(
                ops = ops,
                keyVersion = keyVersion,
                onKeyChanged = {
                    onKeyChanged()
                    onHistoryChanged()
                },
            )
        }
    }
}

@Composable
private fun TiersDialog(tiers: List<String>, onDismiss: () -> Unit, onSave: (List<String>) -> Unit) {
    var working by remember { mutableStateOf(tiers) }
    var newTier by remember { mutableStateOf("") }
    androidx.compose.material3.AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.issuer_tiers_dialog_title)) },
        text = {
            Column(Modifier.heightIn(max = 420.dp)) {
                Text(stringResource(R.string.issuer_tiers_dialog_hint), style = MaterialTheme.typography.bodySmall)
                Spacer(Modifier.height(8.dp))
                Column(
                    Modifier
                        .weight(1f, fill = false)
                        .verticalScroll(rememberScrollState()),
                ) {
                    working.forEach { tier ->
                        androidx.compose.foundation.layout.Row(
                            verticalAlignment = Alignment.CenterVertically,
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Text(tier, Modifier.weight(1f))
                            TextButton(onClick = { working = working - tier }) {
                                Text(stringResource(R.string.issuer_tiers_remove), color = MaterialTheme.colorScheme.error)
                            }
                        }
                    }
                }
                Spacer(Modifier.height(8.dp))
                androidx.compose.material3.OutlinedTextField(
                    value = newTier,
                    onValueChange = { newTier = it },
                    label = { Text(stringResource(R.string.issuer_add_tier)) },
                    singleLine = true,
                )
                Spacer(Modifier.height(8.dp))
                androidx.compose.material3.TextButton(
                    onClick = {
                        val candidate = newTier.trim()
                        if (candidate.isNotEmpty() && candidate !in working && candidate.length <= 64) {
                            working = working + candidate
                            newTier = ""
                        }
                    },
                ) { Text(stringResource(R.string.issuer_tiers_add)) }
            }
        },
        confirmButton = {
            TextButton(onClick = { onSave(working) }) { Text(stringResource(R.string.issuer_close_ok)) }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text(stringResource(R.string.issuer_cancel)) }
        },
    )
}

@Composable
private fun AboutIssuerDialog(onDismiss: () -> Unit) {
    androidx.compose.material3.AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.issuer_about_title)) },
        text = {
            Column {
                Text(stringResource(R.string.issuer_about_body), style = MaterialTheme.typography.bodySmall)
            }
        },
        confirmButton = {
            TextButton(onClick = onDismiss) { Text(stringResource(R.string.issuer_close)) }
        },
    )
}
