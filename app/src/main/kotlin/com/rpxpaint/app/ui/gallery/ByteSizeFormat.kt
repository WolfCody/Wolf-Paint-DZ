package com.rpxpaint.app.ui.gallery

import java.util.Locale

/**
 * Decimal size (1 MB = 1,000,000 B), the same convention the spec uses for
 * "8000 x 8000 RGBA is about 256 MB". [units] = B, KB, MB, GB, TB (localized by the caller).
 */
fun formatDecimalSize(bytes: Long, units: List<String>, locale: Locale): String {
    require(units.size >= 2) { "need at least two units" }
    var value = bytes.toDouble()
    var index = 0
    while (value >= 1000.0 && index < units.lastIndex) {
        value /= 1000.0
        index++
    }
    val text = if (value == Math.floor(value) || index == 0) {
        String.format(locale, "%.0f", value)
    } else {
        String.format(locale, "%.1f", value)
    }
    return "$text ${units[index]}"
}
