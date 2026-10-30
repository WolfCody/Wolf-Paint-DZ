package com.rpxpaint.app.core

import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assert.assertThrows
import org.junit.BeforeClass
import org.junit.Test

/** Runs against the real librpx_core.so on a device or emulator. */
class RustBridgeInstrumentedTest {

    companion object {
        private lateinit var engine: RustEngine

        @BeforeClass
        @JvmStatic
        fun startEngine() {
            val context = InstrumentationRegistry.getInstrumentation().targetContext
            engine = RustEngine.create(File(context.filesDir, "engine-test"))
        }
    }

    @Test
    fun versionIsReported() {
        assertTrue(engine.version, engine.version.isNotBlank())
    }

    @Test
    fun largeRasterDocumentAllocatesNoTiles() {
        val handle = engine.createDocument("Large", DocumentMode.RASTER, 8000, 8000)
        try {
            val info = engine.documentInfo(handle)
            assertEquals(8000, info.width)
            assertEquals(1024L, info.tilesTotal)
            assertEquals(0L, info.tilesAllocated)
            assertEquals(256_000_000L, info.denseRgbaBytes)
            assertEquals("Large", engine.documentName(handle))
            assertTrue(handle in engine.openDocuments())
        } finally {
            engine.closeDocument(handle)
        }
        assertTrue(handle !in engine.openDocuments())
    }

    @Test
    fun closedHandleThrowsInvalidHandle() {
        val handle = engine.createDocument("Tmp", DocumentMode.VECTOR, 10, 10)
        engine.closeDocument(handle)
        assertThrows(RpxNativeException.InvalidHandle::class.java) { engine.documentInfo(handle) }
        assertThrows(RpxNativeException.InvalidHandle::class.java) { engine.closeDocument(handle) }
    }

    @Test
    fun forgedHandleThrowsInvalidHandle() {
        assertThrows(RpxNativeException.InvalidHandle::class.java) { engine.documentInfo(DocumentHandle(-1)) }
        assertThrows(RpxNativeException.InvalidHandle::class.java) { engine.documentInfo(DocumentHandle(0)) }
    }

    @Test
    fun invalidArgumentsMapToTheRightExceptions() {
        assertThrows(RpxNativeException.InvalidArgument::class.java) {
            engine.createDocument("", DocumentMode.RASTER, 10, 10)
        }
        assertThrows(RpxNativeException.InvalidArgument::class.java) {
            engine.createDocument("x", DocumentMode.RASTER, -5, 10)
        }
        assertThrows(RpxNativeException.CanvasTooLarge::class.java) {
            engine.createDocument("x", DocumentMode.RASTER, 40_000, 40_000)
        }
    }

    @Test
    fun rustPanicBecomesAnExceptionAndTheProcessSurvives() {
        // Debug builds panic on purpose; release builds answer Unsupported.
        val error = runCatching { engine.triggerDebugPanic() }.exceptionOrNull()
        assertTrue(
            "unexpected: $error",
            error is RpxNativeException.NativePanic || error is RpxNativeException.Unsupported,
        )
        // Engine still works afterwards.
        val handle = engine.createDocument("After panic", DocumentMode.RASTER, 10, 10)
        engine.closeDocument(handle)
    }

    @Test
    fun initIsIdempotent() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        RustEngine.create(File(context.filesDir, "engine-test"))
    }
}
