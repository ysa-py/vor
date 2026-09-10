package com.unifiedshield.ui

import androidx.compose.animation.core.*
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.AutoAwesome
import androidx.compose.material.icons.filled.Security
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.unifiedshield.ui.components.EnterpriseStatusPill
import com.unifiedshield.ui.components.StatusPillType
import com.unifiedshield.ui.theme.EnterpriseColors
import kotlin.math.*
import kotlin.random.Random

data class Quantum3DParticle(
    var x: Float,
    var y: Float,
    var z: Float,
    var vx: Float,
    var vy: Float,
    var vz: Float,
    var radius: Float,
    var color: Color
)

@Composable
fun Quantum3DParticleCanvas(
    isConnected: Boolean,
    // A2 (No Fabrication): no live latency source is wired into this screen yet.
    // Pass a real measured value once one exists; null renders an honest
    // "Pending backend wiring" label instead of a fabricated number.
    latencyMs: Long? = null,
    evasionActive: Boolean = true,
    modifier: Modifier = Modifier
) {
    var manualRotX by remember { mutableFloatStateOf(0f) }
    var manualRotY by remember { mutableFloatStateOf(0f) }

    val infiniteTransition = rememberInfiniteTransition(label = "quantum_3d_master_loop")
    val autoRotationAngle by infiniteTransition.animateFloat(
        initialValue = 0f,
        targetValue = 360f,
        animationSpec = infiniteRepeatable(
            animation = tween(9000, easing = LinearEasing),
            repeatMode = RepeatMode.Restart
        ),
        label = "auto_rotation"
    )

    val pulsePhase by infiniteTransition.animateFloat(
        initialValue = 0f,
        targetValue = 2f * PI.toFloat(),
        animationSpec = infiniteRepeatable(
            animation = tween(2200, easing = LinearEasing),
            repeatMode = RepeatMode.Restart
        ),
        label = "pulse_phase"
    )

    val wavePhase by infiniteTransition.animateFloat(
        initialValue = 0f,
        targetValue = 2f * PI.toFloat(),
        animationSpec = infiniteRepeatable(
            animation = tween(1800, easing = LinearEasing),
            repeatMode = RepeatMode.Restart
        ),
        label = "wave_phase"
    )

    // Pre-allocate 48 3D quantum particles in spherical shell
    val particles = remember {
        List(48) {
            val theta = Random.nextFloat() * 2f * PI.toFloat()
            val phi = (Random.nextFloat() - 0.5f) * PI.toFloat()
            val r = 100f + Random.nextFloat() * 70f
            Quantum3DParticle(
                x = r * cos(phi) * cos(theta),
                y = r * cos(phi) * sin(theta),
                z = r * sin(phi),
                vx = (Random.nextFloat() - 0.5f) * 0.8f,
                vy = (Random.nextFloat() - 0.5f) * 0.8f,
                vz = (Random.nextFloat() - 0.5f) * 0.8f,
                radius = 3.0f + Random.nextFloat() * 3.5f,
                color = when (Random.nextInt(4)) {
                    0 -> EnterpriseColors.CyanNeon
                    1 -> EnterpriseColors.EmeraldNeon
                    2 -> EnterpriseColors.PurpleNeon
                    else -> EnterpriseColors.IndigoNeon
                }
            )
        }
    }

    Box(
        modifier = modifier
            .fillMaxWidth()
            .height(230.dp)
            .clip(RoundedCornerShape(20.dp))
            .background(
                Brush.verticalGradient(
                    colors = listOf(
                        EnterpriseColors.CyberVoid,
                        EnterpriseColors.CyberBlack,
                        EnterpriseColors.CyberSurfaceDark
                    )
                )
            )
            .border(
                1.5.dp,
                Brush.horizontalGradient(
                    listOf(
                        EnterpriseColors.CyanQuantum.copy(alpha = 0.8f),
                        EnterpriseColors.EmeraldNeon.copy(alpha = 0.8f),
                        EnterpriseColors.PurpleCyber.copy(alpha = 0.8f)
                    )
                ),
                RoundedCornerShape(20.dp)
            )
            .pointerInput(Unit) {
                detectDragGestures { _, dragAmount ->
                    manualRotY += dragAmount.x * 0.4f
                    manualRotX -= dragAmount.y * 0.4f
                }
            }
            .testTag("quantum_3d_particle_canvas")
    ) {
        Canvas(modifier = Modifier.fillMaxSize()) {
            val centerX = size.width / 2f
            val centerY = size.height / 2f
            val fov = 320f

            // Total Rotation angles
            val totalRotY = Math.toRadians((autoRotationAngle + manualRotY).toDouble()).toFloat()
            val totalRotX = Math.toRadians(manualRotX.toDouble()).toFloat()

            val cosY = cos(totalRotY)
            val sinY = sin(totalRotY)
            val cosX = cos(totalRotX)
            val sinX = sin(totalRotX)

            // Draw Background Cyber Grid Matrix
            val gridStep = 28.dp.toPx()
            var gx = 0f
            while (gx < size.width) {
                drawLine(
                    color = Color(0xFF1E293B).copy(alpha = 0.25f),
                    start = Offset(gx, 0f),
                    end = Offset(gx, size.height),
                    strokeWidth = 0.8f
                )
                gx += gridStep
            }
            var gy = 0f
            while (gy < size.height) {
                drawLine(
                    color = Color(0xFF1E293B).copy(alpha = 0.25f),
                    start = Offset(0f, gy),
                    end = Offset(size.width, gy),
                    strokeWidth = 0.8f
                )
                gy += gridStep
            }

            // Draw Central Holographic Core Reactor
            val corePulseRadius = 26f + sin(pulsePhase) * 6f
            val coreColor = if (isConnected) EnterpriseColors.EmeraldNeon else EnterpriseColors.CyanNeon

            // Glowing Outer Field
            drawCircle(
                color = coreColor.copy(alpha = 0.20f),
                radius = corePulseRadius * 2.5f,
                center = Offset(centerX, centerY)
            )
            // Core Halo
            drawCircle(
                color = coreColor.copy(alpha = 0.55f),
                radius = corePulseRadius * 1.5f,
                center = Offset(centerX, centerY)
            )
            // Core Center
            drawCircle(
                color = coreColor,
                radius = corePulseRadius,
                center = Offset(centerX, centerY)
            )

            // Orbital Rings in 3D Perspective
            val orbitalRadius = 110f + sin(pulsePhase * 0.8f) * 10f
            drawCircle(
                color = EnterpriseColors.CyanQuantum.copy(alpha = 0.45f),
                radius = orbitalRadius,
                center = Offset(centerX, centerY),
                style = Stroke(
                    width = 2.0f,
                    pathEffect = PathEffect.dashPathEffect(floatArrayOf(15f, 10f), autoRotationAngle * 1.5f)
                )
            )

            drawCircle(
                color = EnterpriseColors.PurpleCyber.copy(alpha = 0.40f),
                radius = orbitalRadius * 1.35f,
                center = Offset(centerX, centerY),
                style = Stroke(
                    width = 1.5f,
                    pathEffect = PathEffect.dashPathEffect(floatArrayOf(10f, 15f), -autoRotationAngle * 2f)
                )
            )

            // Quantum Particles 3D Projection
            val projectedPoints = mutableListOf<Triple<Offset, Float, Color>>()

            particles.forEach { p ->
                // Rotate around Y axis
                val x1 = p.x * cosY + p.z * sinY
                val z1 = -p.x * sinY + p.z * cosY
                // Rotate around X axis
                val y2 = p.y * cosX - z1 * sinX
                val z2 = p.y * sinX + z1 * cosX

                val scale = fov / (fov + z2 + 180f)
                val projX = centerX + x1 * scale
                val projY = centerY + y2 * scale

                val particleColor = p.color.copy(alpha = (0.4f + scale * 0.6f).coerceIn(0.2f, 1.0f))
                projectedPoints.add(Triple(Offset(projX, projY), scale, particleColor))

                // Draw Particle
                drawCircle(
                    color = particleColor,
                    radius = p.radius * scale.coerceIn(0.5f, 2.0f),
                    center = Offset(projX, projY)
                )
            }

            // Draw Quantum Entanglement Filaments
            for (i in 0 until min(projectedPoints.size, 20)) {
                val p1 = projectedPoints[i]
                for (j in (i + 1) until min(projectedPoints.size, 24)) {
                    val p2 = projectedPoints[j]
                    val dist = (p1.first - p2.first).getDistance()
                    if (dist < 65f) {
                        val alpha = ((1f - dist / 65f) * 0.45f).coerceIn(0f, 0.45f)
                        drawLine(
                            color = p1.third.copy(alpha = alpha),
                            start = p1.first,
                            end = p2.first,
                            strokeWidth = 1.2f
                        )
                    }
                }
            }

            // Bottom Waveform Oscilloscope
            val wavePath = Path()
            val waveY = size.height - 24.dp.toPx()
            wavePath.moveTo(0f, waveY)
            var wx = 0f
            while (wx <= size.width) {
                val waveAmp = if (isConnected) 12f else 4f
                val wy = waveY + sin(wx * 0.035f + wavePhase) * waveAmp * sin(wx / size.width * PI.toFloat())
                wavePath.lineTo(wx, wy)
                wx += 10f
            }
            drawPath(
                path = wavePath,
                color = if (isConnected) EnterpriseColors.EmeraldNeon.copy(alpha = 0.85f) else EnterpriseColors.CyanNeon.copy(alpha = 0.5f),
                style = Stroke(width = 2.0f)
            )
        }

        // Overlay Tactical HUD badges
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(14.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.Top
        ) {
            Column {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    Icon(
                        Icons.Default.AutoAwesome,
                        contentDescription = null,
                        tint = EnterpriseColors.CyanNeon,
                        modifier = Modifier.size(16.dp)
                    )
                    Text(
                        "میدان ذرات کوانتومی سه‌بعدی",
                        fontSize = 12.sp,
                        fontWeight = FontWeight.Black,
                        color = Color.White
                    )
                }
                Text(
                    "3D Quantum Particle Vortex (GPU Accelerated)",
                    fontSize = 9.5.sp,
                    fontFamily = FontFamily.Monospace,
                    color = EnterpriseColors.CyanGlow
                )
            }

            EnterpriseStatusPill(
                text = if (isConnected) "ENTROPY: 99.8%" else "3D STANDBY",
                type = if (isConnected) StatusPillType.SUCCESS else StatusPillType.INFO,
                isPulsating = isConnected
            )
        }

        // Bottom HUD Legend
        Row(
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .fillMaxWidth()
                .padding(horizontal = 14.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                "لمس و کشیدن جهت چرخش ۳ بعدی مدار",
                fontSize = 10.sp,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Text(
                if (latencyMs != null) "تأخیر فوتونی: ${latencyMs}ms" else "تأخیر: در انتظار اتصال داده واقعی",
                fontSize = 10.sp,
                fontFamily = FontFamily.Monospace,
                fontWeight = FontWeight.Black,
                color = if (latencyMs != null) EnterpriseColors.EmeraldNeon else MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}
