package com.rpxpaint.app.ui.gallery

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
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
import com.rpxpaint.app.R
import com.rpxpaint.app.core.DocumentHandle
import com.rpxpaint.app.core.DocumentMode

@Composable
fun GalleryScreen(viewModel: GalleryViewModel = viewModel(factory = GalleryViewModel.Factory)) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    GalleryContent(
        state = state,
        onNewDocument = viewModel::showCreateDialog,
        onCloseDocument = viewModel::closeDocument,
    )
    if (state.isCreateDialogVisible) {
        NewDocumentDialog(
            isBusy = state.isBusy,
            error = state.createError,
            onDismiss = viewModel::dismissCreateDialog,
            onCreate = viewModel::createDocument,
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun GalleryContent(
    state: GalleryUiState,
    onNewDocument: () -> Unit,
    onCloseDocument: (DocumentHandle) -> Unit,
) {
    Scaffold(
        topBar = { TopAppBar(title = { Text(stringResource(R.string.gallery_title)) }) },
        floatingActionButton = {
            if (state.engine is EngineStatus.Ready) {
                ExtendedFloatingActionButton(
                    onClick = onNewDocument,
                    icon = { Icon(painterResource(R.drawable.ic_add), contentDescription = null) },
                    text = { Text(stringResource(R.string.action_new_document)) },
                    modifier = Modifier.testTag("new_document_fab"),
                )
            }
        },
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding),
            contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = 8.dp, bottom = 96.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            item { EngineStatusRow(state.engine) }
            if (state.documents.isEmpty()) {
                item { EmptyState(engineReady = state.engine is EngineStatus.Ready) }
            } else {
                items(state.documents, key = { it.handle.raw }) { card ->
                    DocumentCardView(card, onClose = { onCloseDocument(card.handle) })
                }
            }
        }
    }
}

@Composable
private fun EngineStatusRow(status: EngineStatus) {
    val ready = status is EngineStatus.Ready
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .testTag("engine_status"),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Box(
            modifier = Modifier
                .size(10.dp)
                .clip(CircleShape)
                .background(if (ready) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error),
        )
        Column {
            Text(stringResource(R.string.engine_title), style = MaterialTheme.typography.titleSmall)
            Text(
                text = when (status) {
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
        modifier = Modifier
            .fillMaxWidth()
            .padding(top = 48.dp)
            .testTag("empty_state"),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Text(stringResource(R.string.empty_title), style = MaterialTheme.typography.titleLarge)
        Text(
            text = stringResource(if (engineReady) R.string.empty_body else R.string.empty_body_engine_failed),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun DocumentCardView(card: DocumentCard, onClose: () -> Unit) {
    val info = card.info
    val units = stringArrayResource(R.array.size_units).toList()
    val locale = LocalConfiguration.current.locales[0]
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .testTag("document_card"),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant),
    ) {
        Row(
            modifier = Modifier.padding(start = 16.dp, top = 12.dp, bottom = 12.dp, end = 4.dp),
            verticalAlignment = Alignment.Top,
        ) {
            Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(card.name, style = MaterialTheme.typography.titleMedium)
                Text(
                    text = stringResource(
                        if (info.mode == DocumentMode.RASTER) R.string.mode_raster else R.string.mode_vector,
                    ),
                    style = MaterialTheme.typography.labelLarge,
                    color = MaterialTheme.colorScheme.primary,
                )
                Text(stringResource(R.string.document_dimensions, info.width, info.height))
                Spacer(Modifier.size(4.dp))
                if (info.mode == DocumentMode.RASTER) {
                    val small = MaterialTheme.typography.bodySmall
                    val muted = MaterialTheme.colorScheme.onSurfaceVariant
                    Text(
                        stringResource(R.string.document_tiles, info.tilesX, info.tilesY, info.tilesTotal, info.tileSize),
                        style = small,
                        color = muted,
                    )
                    Text(
                        stringResource(R.string.document_allocated, info.tilesAllocated),
                        style = small,
                        color = muted,
                    )
                    Text(
                        stringResource(R.string.document_dense, formatDecimalSize(info.denseRgbaBytes, units, locale)),
                        style = small,
                        color = muted,
                    )
                } else {
                    Text(
                        stringResource(R.string.document_vector_note),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            IconButton(onClick = onClose, modifier = Modifier.testTag("close_document")) {
                Icon(painterResource(R.drawable.ic_close), contentDescription = stringResource(R.string.action_close_document))
            }
        }
    }
}
