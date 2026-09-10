package com.vor.licensemanager

import android.graphics.Bitmap
import com.google.zxing.BarcodeFormat
import com.google.zxing.BinaryBitmap
import com.google.zxing.DecodeHintType
import com.google.zxing.EncodeHintType
import com.google.zxing.MultiFormatReader
import com.google.zxing.RGBLuminanceSource
import com.google.zxing.common.HybridBinarizer
import com.google.zxing.qrcode.QRCodeWriter

/**
 * Offline QR-code import/export of a license token (zxing core — pure Java,
 * no permissions, no network).
 *
 * EXPORT renders the token as an on-screen QR bitmap (hand a license to a
 * user without transmitting long text over a monitored channel — the other
 * side scans it with any camera/QR app).
 *
 * IMPORT decodes a token from an image picked via the system photo picker
 * (which needs NO storage permission). Both original and 90°-rotated
 * orientations are tried, since gallery images can carry EXIF rotation.
 */
object QrCodec {

    fun encode(text: String, size: Int = 512): Bitmap? {
        if (text.isEmpty()) return null
        return runCatching {
            val hints = mapOf(
                EncodeHintType.CHARACTER_SET to "UTF-8",
                EncodeHintType.MARGIN to 2,
            )
            val matrix = QRCodeWriter().encode(text, BarcodeFormat.QR_CODE, size, size, hints)
            val pixels = IntArray(size * size)
            for (y in 0 until size) {
                val offset = y * size
                for (x in 0 until size) {
                    pixels[offset + x] = if (matrix.get(x, y)) 0xFF000000.toInt() else 0xFFFFFFFF.toInt()
                }
            }
            Bitmap.createBitmap(pixels, size, size, Bitmap.Config.ARGB_8888)
        }.getOrNull()
    }

    fun decode(bitmap: Bitmap): String? {
        val width = bitmap.width
        val height = bitmap.height
        if (width <= 0 || height <= 0) return null
        val pixels = IntArray(width * height)
        bitmap.getPixels(pixels, 0, width, 0, 0, width, height)

        val hints = mapOf(
            DecodeHintType.POSSIBLE_FORMATS to listOf(BarcodeFormat.QR_CODE),
            DecodeHintType.TRY_HARDER to true,
        )

        fun tryDecode(source: com.google.zxing.LuminanceSource): String? =
            runCatching {
                val reader = MultiFormatReader().apply { setHints(hints) }
                reader.decodeWithState(BinaryBitmap(HybridBinarizer(source))).text
            }.getOrNull()

        val upright = tryDecode(RGBLuminanceSource(width, height, pixels))
        if (upright != null) return upright

        // Rotated fallback (EXIF-rotated gallery images).
        val rotated = runCatching {
            val rotatedBitmap = Bitmap.createBitmap(
                bitmap, 0, 0, width, height,
                android.graphics.Matrix().apply { postRotate(90f) }, true,
            )
            val w2 = rotatedBitmap.width
            val h2 = rotatedBitmap.height
            val p2 = IntArray(w2 * h2)
            rotatedBitmap.getPixels(p2, 0, w2, 0, 0, w2, h2)
            RGBLuminanceSource(w2, h2, p2)
        }.getOrNull() ?: return null
        return tryDecode(rotated)
    }
}
