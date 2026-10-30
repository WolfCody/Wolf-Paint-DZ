package com.rpxpaint.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import com.rpxpaint.app.ui.gallery.GalleryScreen
import com.rpxpaint.app.ui.theme.RpxTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            RpxTheme {
                GalleryScreen()
            }
        }
    }
}
