package com.unifiedshield.ui.components

import androidx.compose.animation.*
import androidx.compose.animation.core.*
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
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.unifiedshield.ui.theme.EnterpriseColors
import com.unifiedshield.ui.theme.LocalEnterpriseSpacing

enum class StatusPillType {
    SUCCESS,
    WARNING,
    CRITICAL,
    INFO,
    NEUTRAL_SCANNING
}

/**
 * EnterpriseStatusPill — Minimal, calm status pill matching Stripe/Linear design.
 * Clear indicator dot, legible label, zero neon/siren animations for normal states.
 */
@Composable
fun EnterpriseStatusPill(
    text: String,
    type: StatusPillType = StatusPillType.SUCCESS,
    modifier: Modifier = Modifier,
    isPulsating: Boolean = false,
    icon: ImageVector? = null,
    testTag: String = "enterprise_status_pill"
) {
    val isDark = MaterialTheme.colorScheme.background == EnterpriseColors.DarkBackground

    val (bgColor, textColor, dotColor, borderColor) = when (type) {
        StatusPillType.SUCCESS -> Quadruple(
            if (isDark) Color(0xFF064E3B).copy(alpha = 0.55f) else Color(0xFFECFDF5),
            if (isDark) Color(0xFFD1FAE5) else Color(0xFF065F46),
            EnterpriseColors.Success,
            if (isDark) Color(0xFF059669).copy(alpha = 0.40f) else Color(0xFFA7F3D0)
        )
        StatusPillType.WARNING -> Quadruple(
            if (isDark) Color(0xFF451A03).copy(alpha = 0.55f) else Color(0xFFFFFBEB),
            if (isDark) Color(0xFFFEF3C7) else Color(0xFF92400E),
            EnterpriseColors.Warning,
            if (isDark) Color(0xFFD97706).copy(alpha = 0.40f) else Color(0xFFFDE68A)
        )
        StatusPillType.CRITICAL -> Quadruple(
            if (isDark) Color(0xFF450A0A).copy(alpha = 0.55f) else Color(0xFFFEF2F2),
            if (isDark) Color(0xFFFEE2E2) else Color(0xFF991B1B),
            EnterpriseColors.Critical,
            if (isDark) Color(0xFFDC2626).copy(alpha = 0.40f) else Color(0xFFFECACA)
        )
        StatusPillType.INFO -> Quadruple(
            if (isDark) Color(0xFF082F49).copy(alpha = 0.55f) else Color(0xFFF0F9FF),
            if (isDark) Color(0xFFE0F2FE) else Color(0xFF075985),
            EnterpriseColors.Info,
            if (isDark) Color(0xFF0284C7).copy(alpha = 0.40f) else Color(0xFFBAE6FD)
        )
        StatusPillType.NEUTRAL_SCANNING -> Quadruple(
            if (isDark) Color(0xFF1E1B4B).copy(alpha = 0.55f) else Color(0xFFEEF2FF),
            if (isDark) Color(0xFFE0E7FF) else Color(0xFF3730A3),
            EnterpriseColors.NeutralScanning,
            if (isDark) Color(0xFF4F46E5).copy(alpha = 0.40f) else Color(0xFFC7D2FE)
        )
    }

    Surface(
        color = bgColor,
        shape = RoundedCornerShape(14.dp),
        border = androidx.compose.foundation.BorderStroke(1.dp, borderColor),
        modifier = modifier.testTag(testTag)
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(5.dp)
        ) {
            if (icon != null) {
                Icon(
                    imageVector = icon,
                    contentDescription = null,
                    tint = textColor,
                    modifier = Modifier.size(11.dp)
                )
            } else {
                Box(
                    modifier = Modifier
                        .size(6.dp)
                        .clip(CircleShape)
                        .background(dotColor)
                )
            }

            Text(
                text = text,
                color = textColor,
                fontSize = 10.5.sp,
                fontWeight = FontWeight.SemiBold,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis
            )
        }
    }
}

private data class Quadruple<A, B, C, D>(val first: A, val second: B, val third: C, val fourth: D)

/**
 * CyberHudCard / EnterpriseCard — Flat, calm, minimalist card with 1px neutral border.
 */
@Composable
fun CyberHudCard(
    modifier: Modifier = Modifier,
    borderColor: Color = MaterialTheme.colorScheme.outline,
    backgroundColor: Color = MaterialTheme.colorScheme.surface,
    content: @Composable ColumnScope.() -> Unit
) {
    Card(
        colors = CardDefaults.cardColors(containerColor = backgroundColor),
        shape = RoundedCornerShape(12.dp),
        border = androidx.compose.foundation.BorderStroke(1.dp, borderColor),
        elevation = CardDefaults.cardElevation(defaultElevation = 0.dp),
        modifier = modifier
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(14.dp),
            content = content
        )
    }
}

/**
 * MetricCard — Clean, tabular numeric card with high readability and restrained accent.
 */
@Composable
fun MetricCard(
    title: String,
    value: String,
    subtitle: String? = null,
    icon: ImageVector? = null,
    iconTint: Color = MaterialTheme.colorScheme.primary,
    badgeText: String? = null,
    badgeType: StatusPillType = StatusPillType.SUCCESS,
    modifier: Modifier = Modifier,
    onClick: (() -> Unit)? = null,
    testTag: String = "metric_card"
) {
    Card(
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surface
        ),
        shape = RoundedCornerShape(12.dp),
        border = androidx.compose.foundation.BorderStroke(1.dp, MaterialTheme.colorScheme.outline),
        elevation = CardDefaults.cardElevation(defaultElevation = 0.dp),
        modifier = modifier
            .testTag(testTag)
            .then(if (onClick != null) Modifier.clickable { onClick() } else Modifier)
    ) {
        Column(
            modifier = Modifier.padding(12.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(6.dp)
                ) {
                    if (icon != null) {
                        Icon(
                            imageVector = icon,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.size(15.dp)
                        )
                    }
                    Text(
                        text = title,
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        fontWeight = FontWeight.Medium
                    )
                }

                if (badgeText != null) {
                    EnterpriseStatusPill(
                        text = badgeText,
                        type = badgeType
                    )
                }
            }

            Spacer(modifier = Modifier.height(6.dp))

            AnimatedContent(
                targetState = value,
                transitionSpec = {
                    fadeIn(animationSpec = tween(180)) togetherWith fadeOut(animationSpec = tween(150))
                },
                label = "metric_value_anim"
            ) { targetValue ->
                Text(
                    text = targetValue,
                    style = MaterialTheme.typography.titleLarge,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.onSurface,
                    fontFamily = FontFamily.SansSerif
                )
            }

            if (subtitle != null) {
                Spacer(modifier = Modifier.height(2.dp))
                Text(
                    text = subtitle,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    fontSize = 10.5.sp
                )
            }
        }
    }
}

/**
 * SectionHeader — Clean section header with optional action button.
 */
@Composable
fun SectionHeader(
    title: String,
    subtitle: String? = null,
    actionText: String? = null,
    onActionClick: (() -> Unit)? = null,
    icon: ImageVector? = null,
    modifier: Modifier = Modifier,
    testTag: String = "section_header"
) {
    val spacing = LocalEnterpriseSpacing.current

    Row(
        modifier = modifier
            .fillMaxWidth()
            .padding(vertical = spacing.xs)
            .testTag(testTag),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            modifier = Modifier.weight(1f)
        ) {
            if (icon != null) {
                Icon(
                    imageVector = icon,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.size(18.dp)
                )
            }
            Column {
                Text(
                    text = title,
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                    color = MaterialTheme.colorScheme.onSurface
                )
                if (subtitle != null) {
                    Text(
                        text = subtitle,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
        }

        if (actionText != null && onActionClick != null) {
            TextButton(
                onClick = onActionClick,
                contentPadding = PaddingValues(horizontal = 8.dp, vertical = 2.dp),
                modifier = Modifier.height(28.dp)
            ) {
                Text(
                    text = actionText,
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.primary,
                    fontWeight = FontWeight.SemiBold
                )
            }
        }
    }
}

/**
 * EmptyState — Clean empty state indicator.
 */
@Composable
fun EmptyState(
    title: String,
    description: String,
    icon: ImageVector = Icons.Default.Inbox,
    actionButtonText: String? = null,
    onActionClick: (() -> Unit)? = null,
    modifier: Modifier = Modifier,
    testTag: String = "empty_state"
) {
    Column(
        modifier = modifier
            .fillMaxWidth()
            .padding(24.dp)
            .testTag(testTag),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f),
            modifier = Modifier.size(40.dp)
        )
        Spacer(modifier = Modifier.height(10.dp))
        Text(
            text = title,
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.SemiBold,
            color = MaterialTheme.colorScheme.onSurface
        )
        Spacer(modifier = Modifier.height(4.dp))
        Text(
            text = description,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = androidx.compose.ui.text.style.TextAlign.Center
        )
        if (actionButtonText != null && onActionClick != null) {
            Spacer(modifier = Modifier.height(12.dp))
            OutlinedButton(
                onClick = onActionClick,
                shape = RoundedCornerShape(8.dp),
                contentPadding = PaddingValues(horizontal = 14.dp, vertical = 6.dp)
            ) {
                Text(actionButtonText, style = MaterialTheme.typography.labelMedium)
            }
        }
    }
}
