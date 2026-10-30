package com.rpxpaint.app.ui.gallery

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.rpxpaint.app.R
import com.rpxpaint.app.core.DocumentMode

private data class SizePreset(val width: Int, val height: Int, val tag: String)

private val presets = listOf(
    SizePreset(1080, 1920, "preset_1080x1920"),
    SizePreset(4096, 4096, "preset_4096"),
    SizePreset(8000, 8000, "preset_8000"),
)

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun NewDocumentDialog(
    isBusy: Boolean,
    error: GalleryError?,
    onDismiss: () -> Unit,
    onCreate: (name: String, mode: DocumentMode, width: Int, height: Int) -> Unit,
) {
    val defaultName = stringResource(R.string.new_document_default_name)
    var name by rememberSaveable { mutableStateOf(defaultName) }
    var mode by rememberSaveable { mutableStateOf(DocumentMode.RASTER) }
    var widthText by rememberSaveable { mutableStateOf("2048") }
    var heightText by rememberSaveable { mutableStateOf("2048") }

    val width = widthText.toIntOrNull()
    val height = heightText.toIntOrNull()
    val canCreate = !isBusy && name.isNotBlank() && width != null && width > 0 && height != null && height > 0

    AlertDialog(
        onDismissRequest = { if (!isBusy) onDismiss() },
        title = { Text(stringResource(R.string.dialog_new_title)) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    label = { Text(stringResource(R.string.field_name)) },
                    singleLine = true,
                    modifier = Modifier.testTag("field_name"),
                )
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    FilterChip(
                        selected = mode == DocumentMode.RASTER,
                        onClick = { mode = DocumentMode.RASTER },
                        label = { Text(stringResource(R.string.mode_raster)) },
                        modifier = Modifier.testTag("mode_raster"),
                    )
                    FilterChip(
                        selected = mode == DocumentMode.VECTOR,
                        onClick = { mode = DocumentMode.VECTOR },
                        label = { Text(stringResource(R.string.mode_vector)) },
                        modifier = Modifier.testTag("mode_vector"),
                    )
                }
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(
                        value = widthText,
                        onValueChange = { widthText = it.filter(Char::isDigit).take(6) },
                        label = { Text(stringResource(R.string.field_width)) },
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        modifier = Modifier
                            .weight(1f)
                            .testTag("field_width"),
                    )
                    OutlinedTextField(
                        value = heightText,
                        onValueChange = { heightText = it.filter(Char::isDigit).take(6) },
                        label = { Text(stringResource(R.string.field_height)) },
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                        modifier = Modifier
                            .weight(1f)
                            .testTag("field_height"),
                    )
                }
                FlowRow(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    presets.forEach { preset ->
                        TextButton(
                            onClick = {
                                widthText = preset.width.toString()
                                heightText = preset.height.toString()
                            },
                            modifier = Modifier.testTag(preset.tag),
                        ) { Text("${preset.width} × ${preset.height}") }
                    }
                }
                if (error != null) {
                    Text(
                        text = stringResource(
                            when (error.kind) {
                                GalleryErrorKind.InvalidInput -> R.string.error_invalid_input
                                GalleryErrorKind.CanvasTooLarge -> R.string.error_canvas_too_large
                                GalleryErrorKind.Other -> R.string.error_other
                            },
                            error.detail,
                        ),
                        color = MaterialTheme.colorScheme.error,
                        style = MaterialTheme.typography.bodySmall,
                        modifier = Modifier.testTag("create_error"),
                    )
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = { if (width != null && height != null) onCreate(name, mode, width, height) },
                enabled = canCreate,
                modifier = Modifier.testTag("dialog_create"),
            ) { Text(stringResource(R.string.action_create)) }
        },
        dismissButton = {
            TextButton(onClick = onDismiss, enabled = !isBusy) { Text(stringResource(R.string.action_cancel)) }
        },
    )
}
