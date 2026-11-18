package com.dzekiwolf.wolfpaintdz

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.*
import com.dzekiwolf.wolfpaintdz.core.DocumentHandle
import com.dzekiwolf.wolfpaintdz.core.EngineState
import com.dzekiwolf.wolfpaintdz.ui.editor.EditorScreen
import com.dzekiwolf.wolfpaintdz.ui.gallery.GalleryScreen
import com.dzekiwolf.wolfpaintdz.ui.theme.WolfPaintTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            WolfPaintTheme {
                AppNav()
            }
        }
    }
}

@Composable
private fun AppNav() {
    val app = androidx.compose.ui.platform.LocalContext.current.applicationContext as WolfPaintApplication
    var editorHandle by remember { mutableStateOf<DocumentHandle?>(null) }
    val handle = editorHandle

    if (handle != null) {
        val engine = (app.engineState as? EngineState.Ready)?.engine
        if (engine != null) {
            EditorScreen(handle = handle, engine = engine, onBack = { editorHandle = null })
        } else {
            editorHandle = null
        }
    } else {
        GalleryScreen(onOpenEditor = { editorHandle = it })
    }
}
