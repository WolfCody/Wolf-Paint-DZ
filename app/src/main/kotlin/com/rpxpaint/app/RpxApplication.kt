package com.rpxpaint.app

import android.app.Application
import com.rpxpaint.app.core.EngineState
import java.io.File

class RpxApplication : Application() {

    lateinit var engineState: EngineState
        private set

    override fun onCreate() {
        super.onCreate()
        // Engine-owned directory (tile spill and working copies in later stages).
        engineState = EngineState.start(File(filesDir, "engine"))
    }
}
