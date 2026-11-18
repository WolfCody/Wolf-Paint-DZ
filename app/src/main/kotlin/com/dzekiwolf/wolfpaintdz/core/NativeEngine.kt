package com.dzekiwolf.wolfpaintdz.core

interface NativeEngine {
    val version: String
    @Throws(RpxNativeException::class) fun createDocument(name: String, mode: DocumentMode, width: Int, height: Int): DocumentHandle
    @Throws(RpxNativeException::class) fun documentInfo(handle: DocumentHandle): DocumentInfo
    @Throws(RpxNativeException::class) fun documentName(handle: DocumentHandle): String
    @Throws(RpxNativeException::class) fun closeDocument(handle: DocumentHandle)
    @Throws(RpxNativeException::class) fun openDocuments(): List<DocumentHandle>

    @Throws(RpxNativeException::class) fun addLayer(handle: DocumentHandle, name: String): Long
    @Throws(RpxNativeException::class) fun removeLayer(handle: DocumentHandle, layerId: Long)
    @Throws(RpxNativeException::class) fun setActiveLayer(handle: DocumentHandle, layerId: Long)
    @Throws(RpxNativeException::class) fun setLayerVisible(handle: DocumentHandle, layerId: Long, visible: Boolean)
    @Throws(RpxNativeException::class) fun setLayerOpacity(handle: DocumentHandle, layerId: Long, opacity: Float)

    @Throws(RpxNativeException::class) fun brushSet(handle: DocumentHandle, size: Float, hardness: Float,
        opacity: Float, flow: Float, r: Int, g: Int, b: Int, eraser: Boolean)
    @Throws(RpxNativeException::class) fun strokeBegin(handle: DocumentHandle, x: Float, y: Float, pressure: Float)
    @Throws(RpxNativeException::class) fun strokePoints(handle: DocumentHandle, points: FloatArray)
    @Throws(RpxNativeException::class) fun strokeEnd(handle: DocumentHandle)
    @Throws(RpxNativeException::class) fun strokeCancel(handle: DocumentHandle)

    @Throws(RpxNativeException::class) fun undo(handle: DocumentHandle): Boolean
    @Throws(RpxNativeException::class) fun redo(handle: DocumentHandle): Boolean

    @Throws(RpxNativeException::class) fun renderToBuffer(handle: DocumentHandle, screenW: Int, screenH: Int): ByteArray
    @Throws(RpxNativeException::class) fun viewportUpdate(handle: DocumentHandle,
        panX: Float, panY: Float, zoom: Float, rotDeg: Float, screenW: Int, screenH: Int)
}
