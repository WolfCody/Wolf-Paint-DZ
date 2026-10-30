package com.dzekiwolf.wolfpaintdz

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import com.dzekiwolf.wolfpaintdz.ui.gallery.GalleryScreen
import com.dzekiwolf.wolfpaintdz.ui.theme.WolfPaintTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            WolfPaintTheme {
                GalleryScreen()
            }
        }
    }
}
