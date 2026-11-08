package com.dzekiwolf.wolfpaintdz.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class DocumentInfoTest {
    // Same array as the Rust test `wire_layout_is_stable` (8000x8000 vector document, no layers).
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

    // Stage 2.1b: tilesAllocated field

    @Test
    fun tilesAllocatedIsAtSlotEight() {
        val info = DocumentInfo.fromWire(wire)
        // slot 8 in the wire array → tilesAllocated
        assertEquals(wire[8], info.tilesAllocated)
    }

    @Test
    fun tilesAllocatedInitialValueIsZero() {
        val info = DocumentInfo.fromWire(wire)
        assertEquals(0L, info.tilesAllocated)
    }

    @Test
    fun tilesAllocatedIsReadFromWire() {
        val withTiles = wire.copyOf().also { it[8] = 42L }
        val info = DocumentInfo.fromWire(withTiles)
        assertEquals(42L, info.tilesAllocated)
    }

    @Test
    fun previousFieldsPreservedWhenTilesAllocatedChanges() {
        val withTiles = wire.copyOf().also { it[8] = 7L }
        val info = DocumentInfo.fromWire(withTiles)
        // All other fields must be unchanged.
        assertEquals(DocumentMode.VECTOR, info.mode)
        assertEquals(8000, info.width)
        assertEquals(8000, info.height)
        assertEquals(256, info.tileSize)
        assertEquals(32, info.tilesX)
        assertEquals(32, info.tilesY)
        assertEquals(1024L, info.tilesTotal)
        assertEquals(256_000_000L, info.denseRgbaBytes)
    }
}
