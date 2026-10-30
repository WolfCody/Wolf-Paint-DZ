package com.dzekiwolf.wolfpaintdz.core

/** Document facts computed by Rust. Wire layout: `DocumentInfo::to_wire` in core-model. */
data class DocumentInfo(
    val mode: DocumentMode,
    val width: Int,
    val height: Int,
    val tileSize: Int,
    val tilesX: Int,
    val tilesY: Int,
    val tilesTotal: Long,
    val tilesAllocated: Long,
    val denseRgbaBytes: Long,
) {
    companion object {
        const val WIRE_VERSION = 1L
        const val WIRE_LENGTH = 10

        /** @throws IllegalArgumentException if the array does not match the agreed layout (a bridge bug). */
        fun fromWire(wire: LongArray): DocumentInfo {
            require(wire.size == WIRE_LENGTH) { "DocumentInfo wire length ${wire.size}, expected $WIRE_LENGTH" }
            require(wire[0] == WIRE_VERSION) { "DocumentInfo wire version ${wire[0]}, expected $WIRE_VERSION" }
            val mode = DocumentMode.entries.firstOrNull { it.wireValue.toLong() == wire[1] }
                ?: throw IllegalArgumentException("Unknown document mode ${wire[1]}")
            return DocumentInfo(
                mode = mode,
                width = wire[2].toInt(),
                height = wire[3].toInt(),
                tileSize = wire[4].toInt(),
                tilesX = wire[5].toInt(),
                tilesY = wire[6].toInt(),
                tilesTotal = wire[7],
                tilesAllocated = wire[8],
                denseRgbaBytes = wire[9],
            )
        }
    }
}
