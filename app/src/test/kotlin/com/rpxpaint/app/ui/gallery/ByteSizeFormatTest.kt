package com.rpxpaint.app.ui.gallery

import java.util.Locale
import org.junit.Assert.assertEquals
import org.junit.Test

class ByteSizeFormatTest {
    private val units = listOf("B", "KB", "MB", "GB", "TB")

    @Test
    fun spec_example_8000_square_rgba_is_256_MB() {
        assertEquals("256 MB", formatDecimalSize(256_000_000, units, Locale.US))
    }

    @Test
    fun usesOneDecimalWhenNotWhole() {
        assertEquals("4.3 GB", formatDecimalSize(4_294_967_296, units, Locale.US))
    }

    @Test
    fun smallValuesStayInBytes() {
        assertEquals("999 B", formatDecimalSize(999, units, Locale.US))
        assertEquals("0 B", formatDecimalSize(0, units, Locale.US))
    }

    @Test
    fun respectsLocaleDecimalSeparator() {
        assertEquals("4,3 GB", formatDecimalSize(4_294_967_296, units, Locale.GERMANY))
    }
}
