package com.unifiedshield.ui.theme

import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

// =============================================================================
// MICAFP Master Directive v6 — Section B2 Design Tokens (ADDITIVE LAYER)
// Calm enterprise-security register: Stripe Dashboard / Linear / 1Password.
// One accent (deep emerald). No neon, no glow, no particles, no scanlines.
// This file adds tokens; it does NOT modify EnterpriseColors / existing theme
// (Directive A1: zero deletion, additive only).
// =============================================================================

object MicafpTokens {
    // Background ramp (dark neutral, directive-exact)
    val BackgroundDeep = Color(0xFF0A0E14)
    val BackgroundRaised = Color(0xFF1A1F29)
    val SurfaceCard = Color(0xFF12161F)
    val SurfaceElevated = Color(0xFF1E2530)
    val BorderSubtle = Color(0xFF262D3A)
    val BorderStrong = Color(0xFF333C4D)

    // Text ramp (>= 4.5:1 on card surfaces)
    val TextPrimary = Color(0xFFF2F5F9)
    val TextSecondary = Color(0xFFB3BDCB)
    val TextMuted = Color(0xFF8C96A6)

    // The single accent — deep emerald. Use everywhere, all platforms.
    val Accent = Color(0xFF10B981)
    val AccentDeep = Color(0xFF059669)
    val AccentContainer = Color(0xFF064E3B)
    val AccentText = Color(0xFFD1FAE5)
    val AccentBorder = Color(0xFF10B981).copy(alpha = 0.32f)

    // Calm semantic (status only — never decorative)
    val Ok = Color(0xFF10B981)
    val Warn = Color(0xFFD97706)
    val Crit = Color(0xFFDC2626)
    val Info = Color(0xFF0284C7)
    val Domestic = Color(0xFF0284C7) // Domestic-only state shares calm info blue

    // Technical / numeric font (JetBrains Mono register via platform mono, tnum enforced)
    val Mono: FontFamily = FontFamily.Monospace
}

@Immutable
data class MicafpRadius(
    val sm: Dp = 6.dp,
    val md: Dp = 10.dp,   // default — medium, never pill, never sharp
    val lg: Dp = 14.dp,
    val xl: Dp = 20.dp,
    val full: Dp = 999.dp
)

@Immutable
data class MicafpMotion(
    // UI transitions 200–400ms; connection-state transitions 600–800ms (Directive B2)
    val fastMs: Int = 200,
    val standardMs: Int = 250,
    val screenMs: Int = 320,
    val connectionMs: Int = 700,
    // Standard enterprise decelerate
    val easing: CubicBezierEasing = CubicBezierEasing(0.2f, 0f, 0f, 1f)
)

@Immutable
data class MicafpSpacing(
    val xxs: Dp = 4.dp,
    val xs: Dp = 8.dp,
    val sm: Dp = 12.dp,
    val md: Dp = 16.dp,
    val lg: Dp = 24.dp,
    val xl: Dp = 32.dp
)

val LocalMicafpRadius = staticCompositionLocalOf { MicafpRadius() }
val LocalMicafpMotion = staticCompositionLocalOf { MicafpMotion() }
val LocalMicafpSpacing = staticCompositionLocalOf { MicafpSpacing() }

// Numeric / technical styles — always mono + tnum so digits never jitter (B2)
object MicafpType {
    val MonoValue = TextStyle(
        fontFamily = MicafpTokens.Mono,
        fontWeight = FontWeight.SemiBold,
        fontSize = 15.sp,
        lineHeight = 20.sp,
        fontFeatureSettings = "tnum"
    )
    val MonoValueLarge = TextStyle(
        fontFamily = MicafpTokens.Mono,
        fontWeight = FontWeight.Bold,
        fontSize = 22.sp,
        lineHeight = 28.sp,
        fontFeatureSettings = "tnum"
    )
    val MonoCaption = TextStyle(
        fontFamily = MicafpTokens.Mono,
        fontWeight = FontWeight.Normal,
        fontSize = 11.sp,
        lineHeight = 15.sp,
        letterSpacing = 0.2.sp,
        fontFeatureSettings = "tnum"
    )
    val SectionLabel = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.SemiBold,
        fontSize = 12.sp,
        lineHeight = 16.sp,
        letterSpacing = 0.6.sp
    )
}
