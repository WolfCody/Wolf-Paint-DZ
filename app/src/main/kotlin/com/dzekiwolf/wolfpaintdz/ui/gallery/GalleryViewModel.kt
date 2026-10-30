package com.dzekiwolf.wolfpaintdz.ui.gallery

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.ViewModelProvider.AndroidViewModelFactory.Companion.APPLICATION_KEY
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import com.dzekiwolf.wolfpaintdz.WolfPaintApplication
import com.dzekiwolf.wolfpaintdz.core.DocumentHandle
import com.dzekiwolf.wolfpaintdz.core.DocumentInfo
import com.dzekiwolf.wolfpaintdz.core.DocumentMode
import com.dzekiwolf.wolfpaintdz.core.EngineState
import com.dzekiwolf.wolfpaintdz.core.NativeEngine
import com.dzekiwolf.wolfpaintdz.core.RpxNativeException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

data class DocumentCard(val handle: DocumentHandle, val name: String, val info: DocumentInfo)

sealed interface EngineStatus {
    data class Ready(val version: String) : EngineStatus
    data class Failed(val reason: String) : EngineStatus
}

enum class GalleryErrorKind { InvalidInput, CanvasTooLarge, Other }

data class GalleryError(val kind: GalleryErrorKind, val detail: String)

data class GalleryUiState(
    val engine: EngineStatus,
    val documents: List<DocumentCard> = emptyList(),
    val isCreateDialogVisible: Boolean = false,
    val createError: GalleryError? = null,
    val isBusy: Boolean = false,
)

/**
 * Stage 1 gallery: lists the documents currently open in the Rust engine. Persistent project
 * storage (Room + .rpx) arrives in stage 6, so documents live only as long as the process.
 * Engine calls run off the main thread.
 */
class GalleryViewModel(
    private val engineState: EngineState,
    private val dispatcher: CoroutineDispatcher = Dispatchers.Default,
) : ViewModel() {

    private val engine: NativeEngine? = (engineState as? EngineState.Ready)?.engine

    private val _state = MutableStateFlow(
        GalleryUiState(
            engine = when (engineState) {
                is EngineState.Ready -> EngineStatus.Ready(engineState.engine.version)
                is EngineState.Failed -> EngineStatus.Failed(engineState.reason)
            },
        ),
    )
    val state: StateFlow<GalleryUiState> = _state.asStateFlow()

    init {
        // The process may already hold documents (e.g. the activity was re-created).
        engine?.let { current ->
            viewModelScope.launch {
                val cards = withContext(dispatcher) {
                    try {
                        current.openDocuments().map { card(current, it) }
                    } catch (e: RpxNativeException) {
                        emptyList()
                    }
                }
                _state.update { it.copy(documents = cards) }
            }
        }
    }

    fun showCreateDialog() = _state.update { it.copy(isCreateDialogVisible = true, createError = null) }

    fun dismissCreateDialog() = _state.update { it.copy(isCreateDialogVisible = false, createError = null) }

    fun createDocument(name: String, mode: DocumentMode, width: Int, height: Int) {
        val current = engine ?: return
        viewModelScope.launch {
            _state.update { it.copy(isBusy = true, createError = null) }
            val outcome: Result<DocumentCard> = withContext(dispatcher) {
                try {
                    Result.success(card(current, current.createDocument(name, mode, width, height)))
                } catch (e: RpxNativeException) {
                    Result.failure(e)
                }
            }
            outcome.fold(
                onSuccess = { created ->
                    _state.update {
                        it.copy(isBusy = false, isCreateDialogVisible = false, documents = it.documents + created)
                    }
                },
                onFailure = { error ->
                    _state.update { it.copy(isBusy = false, createError = (error as RpxNativeException).toGalleryError()) }
                },
            )
        }
    }

    fun closeDocument(handle: DocumentHandle) {
        val current = engine ?: return
        viewModelScope.launch {
            withContext(dispatcher) {
                try {
                    current.closeDocument(handle)
                } catch (e: RpxNativeException.InvalidHandle) {
                    // Already closed: the list below is rebuilt from the engine either way.
                } catch (e: RpxNativeException) {
                    _state.update { it.copy(createError = e.toGalleryError()) }
                }
            }
            _state.update { it.copy(documents = it.documents.filterNot { card -> card.handle == handle }) }
        }
    }

    private fun card(engine: NativeEngine, handle: DocumentHandle) =
        DocumentCard(handle, engine.documentName(handle), engine.documentInfo(handle))

    private fun RpxNativeException.toGalleryError(): GalleryError = GalleryError(
        kind = when (this) {
            is RpxNativeException.InvalidArgument -> GalleryErrorKind.InvalidInput
            is RpxNativeException.CanvasTooLarge -> GalleryErrorKind.CanvasTooLarge
            else -> GalleryErrorKind.Other
        },
        detail = message.orEmpty(),
    )

    companion object {
        val Factory: ViewModelProvider.Factory = viewModelFactory {
            initializer {
                val app = this[APPLICATION_KEY] as WolfPaintApplication
                GalleryViewModel(app.engineState)
            }
        }
    }
}
