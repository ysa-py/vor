package com.unifiedshield.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

// =========================================================================
// ENTERPRISE PROFESSIONAL PALETTE (Stripe / Linear / 1Password Register)
// Restrained, neutral-dominant, zero-neon, calm semantic accents
// =========================================================================

object EnterpriseColors {
    // Light Mode Neutral Surfaces
    val LightBackground = Color(0xFFF8FAFC)       // Slate 50
    val LightSurface = Color(0xFFFFFFFF)          // White
    val LightSurfaceCard = Color(0xFFFFFFFF)      // Pure White Card
    val LightSurfaceElevated = Color(0xFFF1F5F9)  // Slate 100
    val LightBorder = Color(0xFFE2E8F0)           // Slate 200
    val LightBorderSubtle = Color(0xFFCBD5E1)     // Slate 300
    val LightTextPrimary = Color(0xFF0F172A)      // Slate 900
    val LightTextSecondary = Color(0xFF475569)    // Slate 600
    val LightTextMuted = Color(0xFF64748B)        // Slate 500

    // Dark Mode Neutral Surfaces
    val DarkBackground = Color(0xFF090D16)        // Slate 950 Deep
    val DarkSurface = Color(0xFF0F172A)           // Slate 900
    val DarkSurfaceCard = Color(0xFF131C2E)       // Slate 900 subtle tint
    val DarkSurfaceElevated = Color(0xFF1E293B)   // Slate 800
    val DarkBorder = Color(0xFF1E293B)            // Slate 800
    val DarkBorderSubtle = Color(0xFF334155)      // Slate 700
    val DarkTextPrimary = Color(0xFFF8FAFC)       // Slate 50
    val DarkTextSecondary = Color(0xFF94A3B8)     // Slate 400
    val DarkTextMuted = Color(0xFF64748B)         // Slate 500

    // Calm Primary Accent (Desaturated Emerald)
    val PrimaryLight = Color(0xFF059669)          // Emerald 600
    val PrimaryDark = Color(0xFF10B981)           // Emerald 500 (Clean, non-glowing)
    val PrimaryContainerLight = Color(0xFFECFDF5) // Emerald 50
    val PrimaryContainerDark = Color(0xFF064E3B)  // Emerald 900
    val PrimaryBorder = Color(0xFF10B981).copy(alpha = 0.35f)

    // Legacy Aliases for backwards compatibility with existing views
    val CyberVoid = DarkBackground
    val CyberBlack = DarkSurface
    val CyberSurfaceDark = DarkSurfaceCard
    val CyberSurfaceCard = DarkSurfaceCard
    val CyberSurfaceElevated = DarkSurfaceElevated
    val CyberSurfaceGlass = DarkSurfaceElevated.copy(alpha = 0.85f)
    val CyberBorderSubtle = DarkBorderSubtle
    val CyberBorderGlow = DarkBorderSubtle
    val CyberBorderCyan = DarkBorderSubtle
    val CyberGridLine = DarkBorder

    val EmeraldNeon = Color(0xFF10B981)
    val EmeraldGlow = Color(0xFF059669)
    val EmeraldDeep = Color(0xFF064E3B)
    val EmeraldPrimaryDark = Color(0xFF047857)

    val CyanQuantum = Color(0xFF0284C7)           // Sky 600 (Muted, professional)
    val CyanNeon = Color(0xFF0EA5E9)              // Sky 500
    val CyanDeep = Color(0xFF0C4A6E)
    val CyanGlow = Color(0xFF38BDF8)

    val IndigoNeural = Color(0xFF4F46E5)          // Indigo 600
    val IndigoNeon = Color(0xFF6366F1)            // Indigo 500
    val IndigoDeep = Color(0xFF1E1B4B)
    val PurpleCyber = Color(0xFF7C3AED)           // Violet 600
    val PurpleNeon = Color(0xFF8B5CF6)            // Violet 500

    // Calm Semantic Status
    val Success = Color(0xFF10B981)
    val SuccessContainer = Color(0xFF064E3B)
    val SuccessText = Color(0xFFD1FAE5)

    val Warning = Color(0xFFD97706)               // Amber 600
    val WarningContainer = Color(0xFF451A03)
    val WarningText = Color(0xFFFEF3C7)

    val Critical = Color(0xFFDC2626)              // Red 600
    val CriticalContainer = Color(0xFF450A0A)
    val CriticalText = Color(0xFFFEE2E2)

    val Info = Color(0xFF0284C7)                  // Sky 600
    val InfoContainer = Color(0xFF082F49)
    val InfoText = Color(0xFFE0F2FE)

    val NeutralScanning = Color(0xFF6366F1)
    val NeutralScanningContainer = Color(0xFF1E1B4B)
    val NeutralScanningText = Color(0xFFE0E7FF)

    // Flat Professional Gradients (Subtle, never garish)
    val PrimaryActiveGradient = listOf(
        Color(0xFF059669),
        Color(0xFF047857)
    )

    val StandbyGradient = listOf(
        Color(0xFF334155),
        Color(0xFF1E293B)
    )

    val ReactorActiveGradient = PrimaryActiveGradient
    val ReactorStandbyGradient = StandbyGradient
}

// ==========================================
// 5-LEVEL ELEVATION & SPACING SYSTEM
// ==========================================

@Immutable
data class EnterpriseElevation(
    val base: Dp = 0.dp,
    val raised: Dp = 1.dp,
    val overlay: Dp = 3.dp,
    val modal: Dp = 8.dp,
    val tooltip: Dp = 12.dp
)

@Immutable
data class EnterpriseSpacing(
    val xxs: Dp = 4.dp,
    val xs: Dp = 8.dp,
    val s: Dp = 12.dp,
    val m: Dp = 16.dp,
    val l: Dp = 20.dp,
    val xl: Dp = 24.dp
)

@Immutable
data class SemanticStatusColors(
    val success: Color = EnterpriseColors.Success,
    val successContainer: Color = EnterpriseColors.SuccessContainer,
    val successText: Color = EnterpriseColors.SuccessText,
    val warning: Color = EnterpriseColors.Warning,
    val warningContainer: Color = EnterpriseColors.WarningContainer,
    val warningText: Color = EnterpriseColors.WarningText,
    val critical: Color = EnterpriseColors.Critical,
    val criticalContainer: Color = EnterpriseColors.CriticalContainer,
    val criticalText: Color = EnterpriseColors.CriticalText,
    val info: Color = EnterpriseColors.Info,
    val infoContainer: Color = EnterpriseColors.InfoContainer,
    val infoText: Color = EnterpriseColors.InfoText,
    val neutralScanning: Color = EnterpriseColors.NeutralScanning,
    val neutralScanningContainer: Color = EnterpriseColors.NeutralScanningContainer,
    val neutralScanningText: Color = EnterpriseColors.NeutralScanningText
)

val LocalEnterpriseElevation = staticCompositionLocalOf { EnterpriseElevation() }
val LocalEnterpriseSpacing = staticCompositionLocalOf { EnterpriseSpacing() }
val LocalSemanticStatus = staticCompositionLocalOf { SemanticStatusColors() }

// ==========================================
// MATERIAL 3 COLOR SCHEMES (Light & Dark)
// ==========================================

val DarkColorScheme: ColorScheme = darkColorScheme(
    primary = EnterpriseColors.PrimaryDark,
    onPrimary = Color(0xFF021B12),
    primaryContainer = EnterpriseColors.PrimaryContainerDark,
    onPrimaryContainer = Color(0xFFD1FAE5),
    secondary = EnterpriseColors.CyanNeon,
    onSecondary = Color(0xFF082F49),
    secondaryContainer = EnterpriseColors.CyanDeep,
    onSecondaryContainer = Color(0xFFBAE6FD),
    tertiary = EnterpriseColors.IndigoNeon,
    onTertiary = Color(0xFF1E1B4B),
    tertiaryContainer = EnterpriseColors.IndigoDeep,
    onTertiaryContainer = Color(0xFFC7D2FE),
    background = EnterpriseColors.DarkBackground,
    onBackground = EnterpriseColors.DarkTextPrimary,
    surface = EnterpriseColors.DarkSurface,
    onSurface = EnterpriseColors.DarkTextPrimary,
    surfaceVariant = EnterpriseColors.DarkSurfaceElevated,
    onSurfaceVariant = EnterpriseColors.DarkTextSecondary,
    outline = EnterpriseColors.DarkBorderSubtle,
    outlineVariant = EnterpriseColors.DarkBorder,
    error = EnterpriseColors.Critical,
    onError = Color(0xFFFFFFFF),
    errorContainer = EnterpriseColors.CriticalContainer,
    onErrorContainer = EnterpriseColors.CriticalText
)

val LightColorScheme: ColorScheme = lightColorScheme(
    primary = EnterpriseColors.PrimaryLight,
    onPrimary = Color(0xFFFFFFFF),
    primaryContainer = EnterpriseColors.PrimaryContainerLight,
    onPrimaryContainer = Color(0xFF065F46),
    secondary = EnterpriseColors.CyanQuantum,
    onSecondary = Color(0xFFFFFFFF),
    secondaryContainer = Color(0xFFE0F2FE),
    onSecondaryContainer = Color(0xFF075985),
    tertiary = EnterpriseColors.IndigoNeural,
    onTertiary = Color(0xFFFFFFFF),
    tertiaryContainer = Color(0xFFEEF2FF),
    onTertiaryContainer = Color(0xFF3730A3),
    background = EnterpriseColors.LightBackground,
    onBackground = EnterpriseColors.LightTextPrimary,
    surface = EnterpriseColors.LightSurface,
    onSurface = EnterpriseColors.LightTextPrimary,
    surfaceVariant = EnterpriseColors.LightSurfaceElevated,
    onSurfaceVariant = EnterpriseColors.LightTextSecondary,
    outline = EnterpriseColors.LightBorder,
    outlineVariant = EnterpriseColors.LightBorderSubtle,
    error = EnterpriseColors.Critical,
    onError = Color(0xFFFFFFFF),
    errorContainer = Color(0xFFFEE2E2),
    onErrorContainer = Color(0xFF991B1B)
)

// ==========================================
// ENTERPRISE CLEAN TYPOGRAPHY
// ==========================================

val EnterpriseTypography = Typography(
    displayLarge = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Bold,
        fontSize = 36.sp,
        lineHeight = 44.sp,
        letterSpacing = (-0.25).sp,
        fontFeatureSettings = "tnum"
    ),
    displayMedium = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Bold,
        fontSize = 28.sp,
        lineHeight = 36.sp,
        letterSpacing = (-0.15).sp,
        fontFeatureSettings = "tnum"
    ),
    displaySmall = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.SemiBold,
        fontSize = 24.sp,
        lineHeight = 32.sp,
        letterSpacing = 0.sp,
        fontFeatureSettings = "tnum"
    ),
    headlineLarge = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Bold,
        fontSize = 22.sp,
        lineHeight = 28.sp,
        letterSpacing = 0.sp,
        fontFeatureSettings = "tnum"
    ),
    headlineMedium = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.SemiBold,
        fontSize = 18.sp,
        lineHeight = 24.sp,
        letterSpacing = 0.sp,
        fontFeatureSettings = "tnum"
    ),
    headlineSmall = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.SemiBold,
        fontSize = 16.sp,
        lineHeight = 22.sp,
        letterSpacing = 0.sp,
        fontFeatureSettings = "tnum"
    ),
    titleLarge = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.SemiBold,
        fontSize = 16.sp,
        lineHeight = 22.sp,
        letterSpacing = 0.sp,
        fontFeatureSettings = "tnum"
    ),
    titleMedium = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Medium,
        fontSize = 14.sp,
        lineHeight = 20.sp,
        letterSpacing = 0.1.sp,
        fontFeatureSettings = "tnum"
    ),
    titleSmall = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Medium,
        fontSize = 13.sp,
        lineHeight = 18.sp,
        letterSpacing = 0.1.sp,
        fontFeatureSettings = "tnum"
    ),
    bodyLarge = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Normal,
        fontSize = 14.sp,
        lineHeight = 20.sp,
        letterSpacing = 0.15.sp,
        fontFeatureSettings = "tnum"
    ),
    bodyMedium = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Normal,
        fontSize = 13.sp,
        lineHeight = 18.sp,
        letterSpacing = 0.15.sp,
        fontFeatureSettings = "tnum"
    ),
    bodySmall = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Normal,
        fontSize = 12.sp,
        lineHeight = 16.sp,
        letterSpacing = 0.1.sp,
        fontFeatureSettings = "tnum"
    ),
    labelLarge = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.SemiBold,
        fontSize = 13.sp,
        lineHeight = 18.sp,
        letterSpacing = 0.1.sp,
        fontFeatureSettings = "tnum"
    ),
    labelMedium = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Medium,
        fontSize = 11.5.sp,
        lineHeight = 16.sp,
        letterSpacing = 0.15.sp,
        fontFeatureSettings = "tnum"
    ),
    labelSmall = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Medium,
        fontSize = 10.5.sp,
        lineHeight = 14.sp,
        letterSpacing = 0.2.sp,
        fontFeatureSettings = "tnum"
    )
)

@Composable
fun UnifiedShieldTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit
) {
    val colorScheme = if (darkTheme) DarkColorScheme else LightColorScheme
    val elevation = EnterpriseElevation()
    val spacing = EnterpriseSpacing()
    val semanticStatus = SemanticStatusColors()

    CompositionLocalProvider(
        LocalEnterpriseElevation provides elevation,
        LocalEnterpriseSpacing provides spacing,
        LocalSemanticStatus provides semanticStatus
    ) {
        MaterialTheme(
            colorScheme = colorScheme,
            typography = EnterpriseTypography,
            content = content
        )
    }
}
