package com.dzekiwolf.wolfpaintdz

import android.app.Application
import com.dzekiwolf.wolfpaintdz.core.EngineState
import java.io.File

class WolfPaintApplication : Application() {
    lateinit var engineState: EngineState
        private set

    override fun onCreate() {
        super.onCreate()
        engineState = EngineState.start(File(filesDir, "engine"))
    }
}
