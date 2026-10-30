package com.dzekiwolf.wolfpaintdz.ui.gallery

import com.dzekiwolf.wolfpaintdz.core.DocumentHandle
import com.dzekiwolf.wolfpaintdz.core.DocumentInfo
import com.dzekiwolf.wolfpaintdz.core.DocumentMode
import com.dzekiwolf.wolfpaintdz.core.EngineState
import com.dzekiwolf.wolfpaintdz.core.NativeEngine
import com.dzekiwolf.wolfpaintdz.core.RpxNativeException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

/** Fake that follows the same rules as the Rust engine for the cases under test. */
private class FakeEngine : NativeEngine {
    override val version = "fake"
    private val docs = linkedMapOf<Long, Pair<String, DocumentInfo>>()
    private var next = 1L

    override fun createDocument(name: String, mode: DocumentMode, width: Int, height: Int): DocumentHandle {
        if (name.isBlank()) throw RpxNativeException.InvalidArgument("empty name")
        if (width > 32_768 || height > 32_768) throw RpxNativeException.CanvasTooLarge("too large")
        val info = DocumentInfo(mode, width, height, 256, (width + 255) / 256, (height + 255) / 256, 0, 0, width.toLong() * height * 4)
        val handle = next++
        docs[handle] = name to info
        return DocumentHandle(handle)
    }

    override fun documentInfo(handle: DocumentHandle) = docs[handle.raw]?.second ?: throw RpxNativeException.InvalidHandle("closed")
    override fun documentName(handle: DocumentHandle) = docs[handle.raw]?.first ?: throw RpxNativeException.InvalidHandle("closed")
    override fun closeDocument(handle: DocumentHandle) {
        docs.remove(handle.raw) ?: throw RpxNativeException.InvalidHandle("closed")
    }
    override fun openDocuments() = docs.keys.map { DocumentHandle(it) }
}

@OptIn(ExperimentalCoroutinesApi::class)
class GalleryViewModelTest {
    private val dispatcher = UnconfinedTestDispatcher()

    @Before fun setUp() = Dispatchers.setMain(dispatcher)
    @After fun tearDown() = Dispatchers.resetMain()

    private fun viewModel(engine: FakeEngine = FakeEngine()) =
        GalleryViewModel(EngineState.Ready(engine), dispatcher)

    @Test
    fun startsEmptyAndReady() = runTest {
        val vm = viewModel()
        assertTrue(vm.state.value.engine is EngineStatus.Ready)
        assertTrue(vm.state.value.documents.isEmpty())
    }

    @Test
    fun createsADocumentAndClosesTheDialog() = runTest {
        val vm = viewModel()
        vm.showCreateDialog()
        vm.createDocument("Poster", DocumentMode.RASTER, 8000, 8000)
        val state = vm.state.value
        assertEquals(1, state.documents.size)
        assertEquals("Poster", state.documents[0].name)
        assertEquals(8000, state.documents[0].info.width)
        assertFalse(state.isCreateDialogVisible)
        assertFalse(state.isBusy)
    }

    @Test
    fun invalidInputKeepsTheDialogOpenWithAnError() = runTest {
        val vm = viewModel()
        vm.showCreateDialog()
        vm.createDocument(" ", DocumentMode.RASTER, 100, 100)
        val state = vm.state.value
        assertTrue(state.isCreateDialogVisible)
        assertEquals(GalleryErrorKind.InvalidInput, state.createError?.kind)
        assertTrue(state.documents.isEmpty())
    }

    @Test
    fun tooLargeCanvasIsReportedAsSuch() = runTest {
        val vm = viewModel()
        vm.createDocument("x", DocumentMode.VECTOR, 40_000, 10)
        assertEquals(GalleryErrorKind.CanvasTooLarge, vm.state.value.createError?.kind)
    }

    @Test
    fun closingRemovesTheCard() = runTest {
        val vm = viewModel()
        vm.createDocument("a", DocumentMode.RASTER, 10, 10)
        val handle = vm.state.value.documents.single().handle
        vm.closeDocument(handle)
        assertTrue(vm.state.value.documents.isEmpty())
        vm.closeDocument(handle) // double close must not crash
        assertTrue(vm.state.value.documents.isEmpty())
    }

    @Test
    fun picksUpDocumentsAlreadyOpenInTheEngine() = runTest {
        val engine = FakeEngine()
        engine.createDocument("old", DocumentMode.RASTER, 10, 10)
        val vm = viewModel(engine)
        assertEquals(listOf("old"), vm.state.value.documents.map { it.name })
    }

    @Test
    fun failedEngineDisablesCreation() = runTest {
        val vm = GalleryViewModel(EngineState.Failed("no library"), dispatcher)
        assertEquals(EngineStatus.Failed("no library"), vm.state.value.engine)
        vm.createDocument("x", DocumentMode.RASTER, 10, 10)
        assertTrue(vm.state.value.documents.isEmpty())
        assertEquals(null, vm.state.value.createError)
    }
}
