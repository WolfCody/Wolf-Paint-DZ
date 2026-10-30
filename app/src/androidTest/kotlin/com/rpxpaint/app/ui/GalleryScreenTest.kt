package com.rpxpaint.app.ui

import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import com.rpxpaint.app.MainActivity
import org.junit.Rule
import org.junit.Test

/** Locale-independent UI test (uses test tags): create an 8000x8000 raster document, then close it. */
class GalleryScreenTest {
    @get:Rule
    val rule = createAndroidComposeRule<MainActivity>()

    @Test
    fun createAndCloseLargeRasterDocument() {
        rule.onNodeWithTag("engine_status").assertExists()
        rule.onNodeWithTag("new_document_fab").performClick()
        rule.onNodeWithTag("preset_8000").performClick()
        rule.onNodeWithTag("dialog_create").performClick()

        rule.waitUntil(timeoutMillis = 5_000) {
            rule.onAllNodesWithTag("document_card").fetchSemanticsNodes().isNotEmpty()
        }
        rule.onAllNodesWithTag("document_card").assertCountEquals(1)

        rule.onNodeWithTag("close_document").performClick()
        rule.waitUntil(timeoutMillis = 5_000) {
            rule.onAllNodesWithTag("document_card").fetchSemanticsNodes().isEmpty()
        }
        rule.onNodeWithTag("empty_state").assertExists()
    }
}
