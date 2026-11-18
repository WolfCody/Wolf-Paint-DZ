package com.dzekiwolf.wolfpaintdz.ui.editor

import android.view.MotionEvent
import android.view.SurfaceView
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import com.dzekiwolf.wolfpaintdz.core.DocumentHandle
import com.dzekiwolf.wolfpaintdz.core.NativeEngine

@Composable
fun EditorScreen(
    handle: DocumentHandle,
    engine: NativeEngine,
    onBack: () -> Unit,
) {
    val vm: EditorViewModel = viewModel(factory = EditorViewModel.factory(handle))
    val state by vm.state.collectAsStateWithLifecycle()
    var canvasView by remember { mutableStateOf<CanvasSurfaceView?>(null) }

    // when redrawTick changes, ask the surface to repaint
    LaunchedEffect(state.redrawTick) { canvasView?.requestRedraw() }

    Column(Modifier.fillMaxSize().background(Color(0xFF14161A))) {
        // ── top bar ──────────────────────────────────────────────────────────
        Row(
            Modifier.fillMaxWidth().height(48.dp).background(Color(0xFF1C1F25)).padding(horizontal = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconButton(onClick = onBack) {
                Text("←", color = Color.White)
            }
            Spacer(Modifier.weight(1f))
            IconButton(onClick = { vm.undo() }) { Text("↩", color = Color.White) }
            IconButton(onClick = { vm.redo() }) { Text("↪", color = Color.White) }
        }

        // ── canvas ────────────────────────────────────────────────────────────
        Box(Modifier.weight(1f).fillMaxWidth()) {
            AndroidView(
                factory = { ctx ->
                    CanvasSurfaceView(ctx).also { v ->
                        v.engine = engine
                        v.documentHandle = handle
                        canvasView = v
                        val dispatcher = InputDispatcher(engine, handle) { vm.onViewportChanged() }
                        v.setOnTouchListener { _, event ->
                            val consumed = dispatcher.dispatch(event)
                            if (consumed) vm.redraw()
                            consumed
                        }
                    }
                },
                modifier = Modifier.fillMaxSize(),
            )
        }

        // ── bottom bar: brush size + color ────────────────────────────────────
        BottomBar(state.brush, onBrushChanged = vm::setBrush)
    }
}

@Composable
private fun BottomBar(brush: BrushState, onBrushChanged: (BrushState) -> Unit) {
    var eraserMode by remember { mutableStateOf(false) }
    var showColorPicker by remember { mutableStateOf(false) }

    Surface(tonalElevation = 2.dp, color = Color(0xFF1C1F25)) {
        Row(
            Modifier.fillMaxWidth().height(60.dp).padding(horizontal = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            // brush size
            Text("Size", color = Color.White, style = MaterialTheme.typography.labelSmall)
            Slider(
                value = brush.size,
                onValueChange = { onBrushChanged(brush.copy(size = it, eraser = eraserMode)) },
                valueRange = 1f..200f,
                modifier = Modifier.weight(1f),
            )
            // opacity
            Slider(
                value = brush.opacity,
                onValueChange = { onBrushChanged(brush.copy(opacity = it, eraser = eraserMode)) },
                valueRange = 0f..1f,
                modifier = Modifier.weight(1f),
            )
            // color swatch
            Box(
                Modifier.size(32.dp).background(Color(brush.r / 255f, brush.g / 255f, brush.b / 255f)),
            ) { }
            // eraser toggle
            IconButton(onClick = { eraserMode = !eraserMode; onBrushChanged(brush.copy(eraser = !brush.eraser)) }) {
                Text(if (eraserMode) "🖌" else "⬜", color = Color.White)
            }
        }
    }
}
