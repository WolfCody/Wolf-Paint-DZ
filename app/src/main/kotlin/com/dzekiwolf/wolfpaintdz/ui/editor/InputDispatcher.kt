package com.dzekiwolf.wolfpaintdz.ui.editor

import android.view.MotionEvent
import com.dzekiwolf.wolfpaintdz.core.DocumentHandle
import com.dzekiwolf.wolfpaintdz.core.NativeEngine

/**
 * Translates Android MotionEvents into batched Rust stroke calls.
 * Separates stylus (draws) from two-finger gestures (pan/zoom/rotate).
 */
class InputDispatcher(
    private val engine: NativeEngine,
    private val handle: DocumentHandle,
    private val onViewportChanged: () -> Unit,
) {
    private var drawing = false
    // viewport gesture state
    private var gesturePointers = mutableMapOf<Int, Pair<Float, Float>>()
    private var lastMidX = 0f; private var lastMidY = 0f
    private var lastDist = 1f; private var lastAngle = 0f

    fun dispatch(event: MotionEvent): Boolean {
        val toolType = event.getToolType(0)
        val isStylus = toolType == MotionEvent.TOOL_TYPE_STYLUS || toolType == MotionEvent.TOOL_TYPE_ERASER

        return when (event.actionMasked) {
            MotionEvent.ACTION_DOWN, MotionEvent.ACTION_POINTER_DOWN -> handleDown(event, isStylus)
            MotionEvent.ACTION_MOVE -> handleMove(event, isStylus)
            MotionEvent.ACTION_UP, MotionEvent.ACTION_POINTER_UP -> handleUp(event, isStylus)
            MotionEvent.ACTION_CANCEL -> handleCancel(); true
            else -> false
        }
    }

    private fun handleDown(event: MotionEvent, isStylus: Boolean): Boolean {
        if (event.pointerCount == 1 && !drawing) {
            drawing = true
            val pressure = event.pressure.coerceIn(0.01f, 1.0f)
            engine.strokeBegin(handle, event.x, event.y, pressure)
            return true
        }
        // second finger while drawing → cancel stroke and switch to gesture
        if (drawing) { engine.strokeCancel(handle); drawing = false }
        updateGesturePointers(event)
        return true
    }

    private fun handleMove(event: MotionEvent, isStylus: Boolean): Boolean {
        if (drawing && event.pointerCount == 1) {
            val pts = buildBatch(event)
            if (pts.isNotEmpty()) engine.strokePoints(handle, pts)
        } else if (!drawing && event.pointerCount >= 2) {
            handleGestureMove(event)
        }
        return true
    }

    private fun handleUp(event: MotionEvent, isStylus: Boolean): Boolean {
        if (drawing && event.pointerCount == 1) {
            engine.strokeEnd(handle)
            drawing = false
        }
        updateGesturePointers(event)
        return true
    }

    private fun handleCancel() {
        if (drawing) { engine.strokeCancel(handle); drawing = false }
        gesturePointers.clear()
    }

    private fun buildBatch(event: MotionEvent): FloatArray {
        val count = event.historySize + 1
        val buf = FloatArray(count * 3)
        for (h in 0 until event.historySize) {
            val base = h * 3
            buf[base]     = event.getHistoricalX(0, h)
            buf[base + 1] = event.getHistoricalY(0, h)
            buf[base + 2] = event.getHistoricalPressure(0, h).coerceIn(0.01f, 1.0f)
        }
        val base = event.historySize * 3
        buf[base] = event.x; buf[base + 1] = event.y
        buf[base + 2] = event.pressure.coerceIn(0.01f, 1.0f)
        return buf
    }

    private fun updateGesturePointers(event: MotionEvent) {
        gesturePointers.clear()
        for (i in 0 until event.pointerCount) {
            gesturePointers[event.getPointerId(i)] = Pair(event.getX(i), event.getY(i))
        }
        if (gesturePointers.size >= 2) {
            val pts = gesturePointers.values.toList()
            lastMidX = (pts[0].first + pts[1].first) / 2f
            lastMidY = (pts[0].second + pts[1].second) / 2f
            lastDist = dist(pts[0], pts[1])
            lastAngle = angle(pts[0], pts[1])
        }
    }

    private fun handleGestureMove(event: MotionEvent) {
        if (event.pointerCount < 2) return
        val pts = (0 until event.pointerCount).map { Pair(event.getX(it), event.getY(it)) }
        val midX = (pts[0].first + pts[1].first) / 2f
        val midY = (pts[0].second + pts[1].second) / 2f
        val d = dist(pts[0], pts[1])
        val a = angle(pts[0], pts[1])
        engine.viewportUpdate(handle,
            panX = midX - lastMidX, panY = midY - lastMidY,
            zoom = d / lastDist.coerceAtLeast(1f),
            rotDeg = a - lastAngle, screenW = 0, screenH = 0)
        lastMidX = midX; lastMidY = midY; lastDist = d; lastAngle = a
        onViewportChanged()
    }

    private fun dist(a: Pair<Float,Float>, b: Pair<Float,Float>): Float {
        val dx = a.first - b.first; val dy = a.second - b.second
        return Math.sqrt((dx*dx + dy*dy).toDouble()).toFloat().coerceAtLeast(1f)
    }
    private fun angle(a: Pair<Float,Float>, b: Pair<Float,Float>): Float {
        return Math.toDegrees(Math.atan2((b.second - a.second).toDouble(), (b.first - a.first).toDouble())).toFloat()
    }
}
