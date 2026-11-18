package com.dzekiwolf.wolfpaintdz.ui.gallery

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringArrayResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import com.dzekiwolf.wolfpaintdz.R
import com.dzekiwolf.wolfpaintdz.core.DocumentHandle
import com.dzekiwolf.wolfpaintdz.core.DocumentMode

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun GalleryScreen(
    onOpenEditor: (DocumentHandle) -> Unit,
    vm: GalleryViewModel = viewModel(factory = GalleryViewModel.Factory),
) {
    val state by vm.state.collectAsStateWithLifecycle()

    Scaffold(
        topBar = { TopAppBar(title = { Text(stringResource(R.string.gallery_title)) }) },
        floatingActionButton = {
            if (state.engine is EngineStatus.Ready) {
                ExtendedFloatingActionButton(
                    onClick = vm::showCreateDialog,
                    icon = { Icon(painterResource(R.drawable.ic_add), null) },
                    text = { Text(stringResource(R.string.action_new_document)) },
                    modifier = Modifier.testTag("new_document_fab"),
                )
            }
        },
    ) { padding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(padding),
            contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = 8.dp, bottom = 96.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            item { EngineStatusRow(state.engine) }
            if (state.documents.isEmpty()) {
                item { EmptyState(state.engine is EngineStatus.Ready) }
            } else {
                items(state.documents, key = { it.handle.raw }) { card ->
                    DocumentCardView(
                        card = card,
                        onClose = { vm.closeDocument(card.handle) },
                        onOpen = if (card.info.mode == DocumentMode.RASTER) {
                            { onOpenEditor(card.handle) }
                        } else null,
                    )
                }
            }
        }
    }

    if (state.isCreateDialogVisible) {
        NewDocumentDialog(
            isBusy = state.isBusy,
            error = state.createError,
            onDismiss = vm::dismissCreateDialog,
            onCreate = vm::createDocument,
        )
    }
}

@Composable
private fun EngineStatusRow(status: EngineStatus) {
    val ready = status is EngineStatus.Ready
    Row(
        Modifier.fillMaxWidth().testTag("engine_status"),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Box(
            Modifier.size(10.dp).clip(CircleShape)
                .background(if (ready) androidx.compose.ui.graphics.Color(0xFF3FB27F)
                             else androidx.compose.ui.graphics.Color(0xFFD64545))
        )
        Column {
            Text(stringResource(R.string.engine_title), style = MaterialTheme.typography.titleSmall)
            Text(
                when (status) {
                    is EngineStatus.Ready -> stringResource(R.string.engine_ready, status.version)
                    is EngineStatus.Failed -> stringResource(R.string.engine_failed, status.reason)
                },
                style = MaterialTheme.typography.bodySmall,
                color = if (ready) MaterialTheme.colorScheme.onSurfaceVariant else MaterialTheme.colorScheme.error,
            )
        }
    }
}

@Composable
private fun EmptyState(engineReady: Boolean) {
    Column(
        Modifier.fillMaxWidth().padding(top = 48.dp).testTag("empty_state"),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Text(stringResource(R.string.empty_title), style = MaterialTheme.typography.titleLarge)
        Text(
            stringResource(if (engineReady) R.string.empty_body else R.string.empty_body_engine_failed),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun DocumentCardView(card: DocumentCard, onClose: () -> Unit, onOpen: (() -> Unit)?) {
    val info = card.info
    val units = stringArrayResource(R.array.size_units).toList()
    val locale = LocalConfiguration.current.locales[0]
    Card(
        Modifier.fillMaxWidth().testTag("document_card"),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant),
    ) {
        Row(
            Modifier.padding(start = 16.dp, top = 12.dp, bottom = 12.dp, end = 4.dp),
            verticalAlignment = Alignment.Top,
        ) {
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(card.name, style = MaterialTheme.typography.titleMedium)
                Text(
                    if (info.mode == DocumentMode.RASTER) stringResource(R.string.mode_raster) else stringResource(R.string.mode_vector),
                    style = MaterialTheme.typography.labelLarge,
                    color = MaterialTheme.colorScheme.primary,
                )
                Text(stringResource(R.string.document_dimensions, info.width, info.height))
                if (info.mode == DocumentMode.RASTER) {
                    Text(
                        stringResource(R.string.document_tiles, info.tilesX, info.tilesY, info.tilesTotal, info.tileSize),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Text(
                        stringResource(R.string.document_allocated, info.tilesAllocated),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Text(
                        stringResource(R.string.document_dense, formatDecimalSize(info.denseRgbaBytes, units, locale)),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    Text(stringResource(R.string.document_vector_note), style = MaterialTheme.typography.bodySmall,
                         color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
            Column {
                if (onOpen != null) {
                    TextButton(onClick = onOpen) { Text("Open") }
                }
                IconButton(onClick = onClose, Modifier.testTag("close_document")) {
                    Icon(painterResource(R.drawable.ic_close), stringResource(R.string.action_close_document))
                }
            }
        }
    }
}
