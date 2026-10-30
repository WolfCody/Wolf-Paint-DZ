package com.dzekiwolf.wolfpaintdz.core

import android.util.Log
import java.io.File

/** Result of starting the engine. The UI shows [Failed] instead of crashing the app. */
sealed interface EngineState {
    data class Ready(val engine: NativeEngine) : EngineState
    data class Failed(val reason: String) : EngineState

    companion object {
        private const val TAG = "rpx"

        fun start(workDir: File): EngineState = try {
            Ready(RustEngine.create(workDir))
        } catch (e: LinkageError) { // UnsatisfiedLinkError: no librpx_core.so for this ABI
            Log.e(TAG, "Native library could not be loaded", e)
            Failed(e.message ?: e.javaClass.simpleName)
        } catch (e: RpxNativeException) {
            Log.e(TAG, "Engine initialization failed", e)
            Failed(e.message ?: e.javaClass.simpleName)
        }
    }
}
