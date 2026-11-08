package com.dzekiwolf.wolfpaintdz.core

import java.io.File

/** [NativeEngine] backed by librpx_core.so. */
class RustEngine private constructor(override val version: String) : NativeEngine {

    override fun createDocument(name: String, mode: DocumentMode, width: Int, height: Int): DocumentHandle =
        DocumentHandle(RustBridge.nativeCreateDocument(name, mode.wireValue, width, height))

    override fun documentInfo(handle: DocumentHandle): DocumentInfo =
        DocumentInfo.fromWire(RustBridge.nativeDocumentInfo(handle.raw))

    override fun documentName(handle: DocumentHandle): String = RustBridge.nativeDocumentName(handle.raw)

    override fun closeDocument(handle: DocumentHandle) = RustBridge.nativeCloseDocument(handle.raw)

    override fun openDocuments(): List<DocumentHandle> =
        RustBridge.nativeOpenDocuments().map { DocumentHandle(it) }

    // Stage 2.1b

    /**
     * Adds a raster layer to [handle] and returns its unique layer ID.
     * The layer is visible and fully opaque by default.
     */
    fun addLayer(handle: DocumentHandle, name: String): Long =
        RustBridge.nativeAddLayer(handle.raw, name)

    /**
     * Writes one pixel into [layerId] of [handle].
     * [colorArgb] is the standard Android `0xAARRGGBB` integer.
     * Returns `true` on success, `false` if the coordinates are outside the canvas.
     */
    fun setPixel(handle: DocumentHandle, layerId: Long, x: Int, y: Int, colorArgb: Int): Boolean =
        RustBridge.nativeSetPixel(handle.raw, layerId, x, y, colorArgb)

    /** Test hook: makes Rust panic (debug builds) so the panic-to-exception path can be verified. */
    fun triggerDebugPanic() = RustBridge.nativeDebugPanic()

    companion object {
        /**
         * Loads the library and initializes the engine. Idempotent per process.
         * @throws LinkageError if librpx_core.so is missing for this ABI.
         */
        @Throws(RpxNativeException::class)
        fun create(workDir: File): RustEngine {
            RustBridge.nativeInit(workDir.absolutePath)
            return RustEngine(RustBridge.nativeVersion())
        }
    }
}
