package com.rpxpaint.app.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

// Palette: graphite surfaces like a drawing tablet, cobalt for actions (ink),
// madder red reserved for errors. Own design; no third-party branding.
private val Graphite = Color(0xFF14161A)
private val Slate = Color(0xFF1C1F25)
private val SlateHigh = Color(0xFF272B33)
private val Cobalt = Color(0xFF7EA2FF)
private val CobaltDeep = Color(0xFF2F5BD8)
private val Paper = Color(0xFFF6F5F1)
private val Madder = Color(0xFFFFB4AB)
private val MadderDeep = Color(0xFFB3261E)

private val DarkColors = darkColorScheme(
    primary = Cobalt,
    onPrimary = Color(0xFF0A1B4D),
    primaryContainer = Color(0xFF1F3A8A),
    onPrimaryContainer = Color(0xFFDCE5FF),
    background = Graphite,
    onBackground = Color(0xFFE6E8EC),
    surface = Slate,
    onSurface = Color(0xFFE6E8EC),
    surfaceVariant = SlateHigh,
    onSurfaceVariant = Color(0xFFB4BAC6),
    outline = Color(0xFF6B7280),
    error = Madder,
    onError = Color(0xFF690005),
)

private val LightColors = lightColorScheme(
    primary = CobaltDeep,
    onPrimary = Color.White,
    primaryContainer = Color(0xFFDCE5FF),
    onPrimaryContainer = Color(0xFF0A1B4D),
    background = Paper,
    onBackground = Color(0xFF1A1C20),
    surface = Color.White,
    onSurface = Color(0xFF1A1C20),
    surfaceVariant = Color(0xFFE9E7E1),
    onSurfaceVariant = Color(0xFF454A54),
    outline = Color(0xFF787E8A),
    error = MadderDeep,
    onError = Color.White,
)

@Composable
fun RpxTheme(darkTheme: Boolean = isSystemInDarkTheme(), content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = if (darkTheme) DarkColors else LightColors,
        content = content,
    )
}
