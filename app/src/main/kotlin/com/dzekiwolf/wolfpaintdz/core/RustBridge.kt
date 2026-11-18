package com.dzekiwolf.wolfpaintdz.core

internal object RustBridge {
    private const val LIBRARY = "rpx_core"
    init { System.loadLibrary(LIBRARY) }

    // ── existing ─────────────────────────────────────────────────────────────
    @JvmStatic external fun nativeInit(workDir: String)
    @JvmStatic external fun nativeVersion(): String
    @JvmStatic external fun nativeCreateDocument(name: String, mode: Int, width: Int, height: Int): Long
    @JvmStatic external fun nativeDocumentInfo(handle: Long): LongArray
    @JvmStatic external fun nativeDocumentName(handle: Long): String
    @JvmStatic external fun nativeCloseDocument(handle: Long)
    @JvmStatic external fun nativeOpenDocuments(): LongArray
    @JvmStatic external fun nativeDebugPanic()

    // ── layers ───────────────────────────────────────────────────────────────
    @JvmStatic external fun nativeAddLayer(handle: Long, name: String): Long
    @JvmStatic external fun nativeRemoveLayer(handle: Long, layerId: Long)
    @JvmStatic external fun nativeSetActiveLayer(handle: Long, layerId: Long)
    @JvmStatic external fun nativeSetLayerVisible(handle: Long, layerId: Long, visible: Boolean)
    @JvmStatic external fun nativeSetLayerOpacity(handle: Long, layerId: Long, opacity: Float)

    // ── brush / stroke ────────────────────────────────────────────────────────
    @JvmStatic external fun nativeBrushSet(
        handle: Long, size: Float, hardness: Float, opacity: Float, flow: Float,
        r: Int, g: Int, b: Int, eraser: Boolean)
    @JvmStatic external fun nativeStrokeBegin(handle: Long, x: Float, y: Float, pressure: Float)
    @JvmStatic external fun nativeStrokePoints(handle: Long, points: FloatArray)
    @JvmStatic external fun nativeStrokeEnd(handle: Long)
    @JvmStatic external fun nativeStrokeCancel(handle: Long)

    // ── history ───────────────────────────────────────────────────────────────
    @JvmStatic external fun nativeUndo(handle: Long): Boolean
    @JvmStatic external fun nativeRedo(handle: Long): Boolean

    // ── rendering / viewport ─────────────────────────────────────────────────
    @JvmStatic external fun nativeRenderToBuffer(handle: Long, screenW: Int, screenH: Int): ByteArray
    @JvmStatic external fun nativeViewportUpdate(
        handle: Long, panX: Float, panY: Float, zoom: Float, rotDeg: Float, screenW: Int, screenH: Int)
}
