package com.rpxpaint.app.core

/**
 * Raw JNI surface. Every method is implemented in crates/core-ffi/src/jni_bridge.rs under the
 * name `Java_com_rpxpaint_app_core_RustBridge_<method>`; `scripts/check_jni_symbols.py` checks
 * that both sides agree. Use [RustEngine] from application code, not this object.
 *
 * Failures arrive as [RpxNativeException]. All methods are thread-safe.
 */
internal object RustBridge {
    private const val LIBRARY = "rpx_core" // librpx_core.so

    init {
        System.loadLibrary(LIBRARY)
    }

    @JvmStatic external fun nativeInit(workDir: String)
    @JvmStatic external fun nativeVersion(): String
    @JvmStatic external fun nativeCreateDocument(name: String, mode: Int, width: Int, height: Int): Long
    @JvmStatic external fun nativeDocumentInfo(handle: Long): LongArray
    @JvmStatic external fun nativeDocumentName(handle: Long): String
    @JvmStatic external fun nativeCloseDocument(handle: Long)
    @JvmStatic external fun nativeOpenDocuments(): LongArray
    @JvmStatic external fun nativeDebugPanic()
}
