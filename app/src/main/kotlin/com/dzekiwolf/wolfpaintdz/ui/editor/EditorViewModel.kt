package com.dzekiwolf.wolfpaintdz.ui.editor

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.ViewModelProvider.AndroidViewModelFactory.Companion.APPLICATION_KEY
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import com.dzekiwolf.wolfpaintdz.WolfPaintApplication
import com.dzekiwolf.wolfpaintdz.core.DocumentHandle
import com.dzekiwolf.wolfpaintdz.core.EngineState
import com.dzekiwolf.wolfpaintdz.core.NativeEngine
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

data class LayerInfo(val id: Long, val name: String, val visible: Boolean, val opacity: Float)

data class BrushState(
    val size: Float = 20f,
    val hardness: Float = 0.9f,
    val opacity: Float = 1f,
    val flow: Float = 1f,
    val r: Int = 0, val g: Int = 0, val b: Int = 0,
    val eraser: Boolean = false,
)

data class EditorUiState(
    val layers: List<LayerInfo> = emptyList(),
    val activeLayerId: Long = -1L,
    val brush: BrushState = BrushState(),
    val canUndo: Boolean = false,
    val canRedo: Boolean = false,
    val redrawTick: Long = 0L,
)

class EditorViewModel(
    private val engine: NativeEngine,
    val handle: DocumentHandle,
) : ViewModel() {

    private val _state = MutableStateFlow(EditorUiState())
    val state: StateFlow<EditorUiState> = _state.asStateFlow()

    init {
        // add a default raster layer
        viewModelScope.launch(Dispatchers.Default) {
            val id = engine.addLayer(handle, "Layer 1")
            _state.update { it.copy(layers = listOf(LayerInfo(id, "Layer 1", true, 1f)), activeLayerId = id) }
            pushBrush()
            redraw()
        }
    }

    // ── brush ──────────────────────────────────────────────────────────────────
    fun setBrush(brush: BrushState) {
        _state.update { it.copy(brush = brush) }
        viewModelScope.launch(Dispatchers.Default) { pushBrush() }
    }

    private fun pushBrush() {
        val b = _state.value.brush
        engine.brushSet(handle, b.size, b.hardness, b.opacity, b.flow, b.r, b.g, b.b, b.eraser)
    }

    // ── history ────────────────────────────────────────────────────────────────
    fun undo() = viewModelScope.launch(Dispatchers.Default) {
        val ok = engine.undo(handle)
        if (ok) redraw()
    }

    fun redo() = viewModelScope.launch(Dispatchers.Default) {
        val ok = engine.redo(handle)
        if (ok) redraw()
    }

    // ── layers ─────────────────────────────────────────────────────────────────
    fun addLayer() = viewModelScope.launch(Dispatchers.Default) {
        val idx = _state.value.layers.size + 1
        val id = engine.addLayer(handle, "Layer $idx")
        _state.update { s ->
            s.copy(layers = s.layers + LayerInfo(id, "Layer $idx", true, 1f), activeLayerId = id)
        }
        engine.setActiveLayer(handle, id)
        redraw()
    }

    fun setActiveLayer(id: Long) {
        _state.update { it.copy(activeLayerId = id) }
        viewModelScope.launch(Dispatchers.Default) { engine.setActiveLayer(handle, id) }
    }

    fun setLayerVisible(id: Long, v: Boolean) {
        _state.update { s -> s.copy(layers = s.layers.map { if (it.id == id) it.copy(visible = v) else it }) }
        viewModelScope.launch(Dispatchers.Default) { engine.setLayerVisible(handle, id, v); redraw() }
    }

    // ── redraw trigger ─────────────────────────────────────────────────────────
    fun redraw() { _state.update { it.copy(redrawTick = it.redrawTick + 1) } }
    fun onViewportChanged() { redraw() }

    companion object {
        fun factory(handle: DocumentHandle): ViewModelProvider.Factory = viewModelFactory {
            initializer {
                val app = this[APPLICATION_KEY] as WolfPaintApplication
                val engine = (app.engineState as? EngineState.Ready)?.engine
                    ?: error("engine not ready")
                EditorViewModel(engine, handle)
            }
        }
    }
}
