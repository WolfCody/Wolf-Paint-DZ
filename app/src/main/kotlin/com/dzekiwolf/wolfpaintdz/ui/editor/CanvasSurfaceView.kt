package com.dzekiwolf.wolfpaintdz.ui.editor

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.PixelFormat
import android.view.MotionEvent
import android.view.SurfaceHolder
import android.view.SurfaceView
import com.dzekiwolf.wolfpaintdz.core.DocumentHandle
import com.dzekiwolf.wolfpaintdz.core.NativeEngine
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import java.util.concurrent.atomic.AtomicBoolean

/**
 * SurfaceView that asks the Rust engine to composite the layers and blits the result.
 * All engine calls run on a dedicated background thread to avoid blocking the UI thread.
 */
class CanvasSurfaceView(context: Context) : SurfaceView(context), SurfaceHolder.Callback {

    var engine: NativeEngine? = null
    var documentHandle: DocumentHandle? = null
    var onGestureStart: (() -> Unit)? = null

    private val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())
    private val dirty = AtomicBoolean(false)
    private val checkerPaint = Paint()
    private val bitmapPaint = Paint(Paint.FILTER_BITMAP_FLAG)

    init {
        holder.addCallback(this)
        holder.setFormat(PixelFormat.OPAQUE)
        setZOrderOnTop(false)
    }

    // ── lifecycle ─────────────────────────────────────────────────────────────
    override fun surfaceCreated(h: SurfaceHolder) { requestRedraw() }
    override fun surfaceChanged(h: SurfaceHolder, format: Int, width: Int, height: Int) { requestRedraw() }
    override fun surfaceDestroyed(h: SurfaceHolder) {}

    fun destroy() { scope.cancel() }

    // ── rendering ─────────────────────────────────────────────────────────────
    fun requestRedraw() {
        dirty.set(true)
        scope.launch { drawFrame() }
    }

    private fun drawFrame() {
        val h = holder ?: return
        val eng = engine ?: return
        val doc = documentHandle ?: return
        val canvas = try { h.lockCanvas() ?: return } catch (e: Exception) { return }
        try {
            val w = canvas.width; val sh = canvas.height
            drawCheckerboard(canvas, w, sh)
            try {
                val bytes = eng.renderToBuffer(doc, w, sh)
                val bmp = Bitmap.createBitmap(w, sh, Bitmap.Config.ARGB_8888)
                // convert RGBA8 (Rust) → ARGB8 (Android Bitmap)
                val argb = IntArray(w * sh)
                for (i in argb.indices) {
                    val r = bytes[i * 4].toInt() and 0xFF
                    val g = bytes[i * 4 + 1].toInt() and 0xFF
                    val b = bytes[i * 4 + 2].toInt() and 0xFF
                    val a = bytes[i * 4 + 3].toInt() and 0xFF
                    argb[i] = (a shl 24) or (r shl 16) or (g shl 8) or b
                }
                bmp.setPixels(argb, 0, w, 0, 0, w, sh)
                canvas.drawBitmap(bmp, 0f, 0f, bitmapPaint)
                bmp.recycle()
            } catch (_: Exception) {}
        } finally {
            h.unlockCanvasAndPost(canvas)
        }
    }

    private fun drawCheckerboard(canvas: Canvas, w: Int, h: Int) {
        val cell = 20
        for (row in 0..h / cell) {
            for (col in 0..w / cell) {
                checkerPaint.color = if ((row + col) % 2 == 0) Color.LTGRAY else Color.WHITE
                canvas.drawRect(
                    (col * cell).toFloat(), (row * cell).toFloat(),
                    ((col + 1) * cell).toFloat(), ((row + 1) * cell).toFloat(),
                    checkerPaint)
            }
        }
    }
}
