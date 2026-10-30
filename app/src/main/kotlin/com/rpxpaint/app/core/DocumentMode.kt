package com.rpxpaint.app.core

/** Values match `DocumentMode::to_wire` in Rust. */
enum class DocumentMode(val wireValue: Int) {
    RASTER(0),
    VECTOR(1),
}
