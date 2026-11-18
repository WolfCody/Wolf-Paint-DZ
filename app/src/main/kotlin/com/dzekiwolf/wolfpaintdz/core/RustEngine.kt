package com.dzekiwolf.wolfpaintdz.core

import java.io.File

class RustEngine private constructor(override val version: String) : NativeEngine {
    override fun createDocument(name: String, mode: DocumentMode, width: Int, height: Int) =
        DocumentHandle(RustBridge.nativeCreateDocument(name, mode.wireValue, width, height))
    override fun documentInfo(h: DocumentHandle) = DocumentInfo.fromWire(RustBridge.nativeDocumentInfo(h.raw))
    override fun documentName(h: DocumentHandle) = RustBridge.nativeDocumentName(h.raw)
    override fun closeDocument(h: DocumentHandle) = RustBridge.nativeCloseDocument(h.raw)
    override fun openDocuments() = RustBridge.nativeOpenDocuments().map(::DocumentHandle)

    override fun addLayer(h: DocumentHandle, name: String) = RustBridge.nativeAddLayer(h.raw, name)
    override fun removeLayer(h: DocumentHandle, layerId: Long) = RustBridge.nativeRemoveLayer(h.raw, layerId)
    override fun setActiveLayer(h: DocumentHandle, layerId: Long) = RustBridge.nativeSetActiveLayer(h.raw, layerId)
    override fun setLayerVisible(h: DocumentHandle, layerId: Long, visible: Boolean) = RustBridge.nativeSetLayerVisible(h.raw, layerId, visible)
    override fun setLayerOpacity(h: DocumentHandle, layerId: Long, opacity: Float) = RustBridge.nativeSetLayerOpacity(h.raw, layerId, opacity)

    override fun brushSet(h: DocumentHandle, size: Float, hardness: Float, opacity: Float, flow: Float,
                          r: Int, g: Int, b: Int, eraser: Boolean) =
        RustBridge.nativeBrushSet(h.raw, size, hardness, opacity, flow, r, g, b, eraser)
    override fun strokeBegin(h: DocumentHandle, x: Float, y: Float, pressure: Float) = RustBridge.nativeStrokeBegin(h.raw, x, y, pressure)
    override fun strokePoints(h: DocumentHandle, points: FloatArray) = RustBridge.nativeStrokePoints(h.raw, points)
    override fun strokeEnd(h: DocumentHandle) = RustBridge.nativeStrokeEnd(h.raw)
    override fun strokeCancel(h: DocumentHandle) = RustBridge.nativeStrokeCancel(h.raw)

    override fun undo(h: DocumentHandle) = RustBridge.nativeUndo(h.raw)
    override fun redo(h: DocumentHandle) = RustBridge.nativeRedo(h.raw)
    override fun renderToBuffer(h: DocumentHandle, screenW: Int, screenH: Int) = RustBridge.nativeRenderToBuffer(h.raw, screenW, screenH)
    override fun viewportUpdate(h: DocumentHandle, panX: Float, panY: Float, zoom: Float, rotDeg: Float, sw: Int, sh: Int) =
        RustBridge.nativeViewportUpdate(h.raw, panX, panY, zoom, rotDeg, sw, sh)

    fun triggerDebugPanic() = RustBridge.nativeDebugPanic()

    companion object {
        @Throws(RpxNativeException::class)
        fun create(workDir: File): RustEngine {
            RustBridge.nativeInit(workDir.absolutePath)
            return RustEngine(RustBridge.nativeVersion())
        }
    }
}
