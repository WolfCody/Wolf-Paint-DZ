// Root build file: only declares plugin versions (from gradle/libs.versions.toml).
// AGP 9 ships Kotlin support itself ("built-in Kotlin"), so org.jetbrains.kotlin.android
// must NOT be applied. The Compose compiler plugin is still a separate plugin.
plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.kotlin.compose) apply false
}
