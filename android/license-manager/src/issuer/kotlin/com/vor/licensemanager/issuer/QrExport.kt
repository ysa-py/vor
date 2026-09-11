package com.vor.licensemanager.issuer

import android.graphics.Bitmap
import com.vor.licensemanager.QrCodec
import java.io.OutputStream
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream

/**
 * Batch/QR export helpers — one token per line in a plain text file, or a
 * ZIP of per-license QR PNGs (plus the tokens.txt inside for convenience).
 * All writes go to streams the caller obtained from the system file picker
 * (SAF) — no storage permission, no direct filesystem access.
 */
object QrExport {

    /** Compress a QR bitmap as PNG into [output]; true on success. */
    fun writePng(bitmap: Bitmap, output: OutputStream): Boolean =
        bitmap.compress(Bitmap.CompressFormat.PNG, 100, output)

    /**
     * Build a QR PNG for each token and zip them together with tokens.txt.
     * File names come from [com.vor.license.issuer.BatchCsv.qrFileName].
     */
    fun writeQrZip(tokens: List<Pair<String, String>>, output: OutputStream): Boolean {
        ZipOutputStream(output.buffered()).use { zip ->
            zip.putNextEntry(ZipEntry("tokens.txt"))
            zip.write(tokens.joinToString("\n") { it.second }.toByteArray(Charsets.UTF_8))
            zip.closeEntry()
            tokens.forEach { (id, token) ->
                val bitmap = QrCodec.encode(token) ?: return false
                zip.putNextEntry(ZipEntry(com.vor.license.issuer.BatchCsv.qrFileName(id)))
                if (!writePng(bitmap, zip)) return false
                zip.closeEntry()
            }
        }
        return true
    }
}
