package com.dzekiwolf.wolfpaintdz.core

/** Typed view of the engine used by the UI layer. Lets view models be tested with a fake. */
interface NativeEngine {
    val version: String

    @Throws(RpxNativeException::class)
    fun createDocument(name: String, mode: DocumentMode, width: Int, height: Int): DocumentHandle

    @Throws(RpxNativeException::class)
    fun documentInfo(handle: DocumentHandle): DocumentInfo

    @Throws(RpxNativeException::class)
    fun documentName(handle: DocumentHandle): String

    @Throws(RpxNativeException::class)
    fun closeDocument(handle: DocumentHandle)

    @Throws(RpxNativeException::class)
    fun openDocuments(): List<DocumentHandle>
}
