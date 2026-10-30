package com.rpxpaint.app.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class DocumentInfoTest {
    // Same array as the Rust test `wire_layout_is_stable` (8000x8000 vector document).
    private val wire = longArrayOf(1, 1, 8000, 8000, 256, 32, 32, 1024, 0, 256_000_000)

    @Test
    fun decodesTheLayoutProducedByRust() {
        val info = DocumentInfo.fromWire(wire)
        assertEquals(DocumentMode.VECTOR, info.mode)
        assertEquals(8000, info.width)
        assertEquals(8000, info.height)
        assertEquals(256, info.tileSize)
        assertEquals(32, info.tilesX)
        assertEquals(1024L, info.tilesTotal)
        assertEquals(0L, info.tilesAllocated)
        assertEquals(256_000_000L, info.denseRgbaBytes)
    }

    @Test
    fun rejectsWrongLength() {
        assertThrows(IllegalArgumentException::class.java) { DocumentInfo.fromWire(wire.copyOf(9)) }
    }

    @Test
    fun rejectsUnknownVersion() {
        val bad = wire.copyOf().also { it[0] = 2 }
        assertThrows(IllegalArgumentException::class.java) { DocumentInfo.fromWire(bad) }
    }

    @Test
    fun rejectsUnknownMode() {
        val bad = wire.copyOf().also { it[1] = 7 }
        assertThrows(IllegalArgumentException::class.java) { DocumentInfo.fromWire(bad) }
    }

    @Test
    fun modeWireValuesMatchRust() {
        assertEquals(0, DocumentMode.RASTER.wireValue)
        assertEquals(1, DocumentMode.VECTOR.wireValue)
    }
}
