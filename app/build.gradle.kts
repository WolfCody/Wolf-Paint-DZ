import com.android.build.api.artifact.SingleArtifact
import java.util.Properties
import javax.inject.Inject
import org.gradle.process.ExecOperations
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
    // No kotlin-android plugin: AGP 9 compiles Kotlin itself (built-in Kotlin).
}

// ---------------------------------------------------------------------------------------------
// Rust integration: `cargo ndk` builds librpx_core.so per ABI, the output directory is wired
// into the variant's jniLibs through the AGP Variant API (so Gradle knows the task dependency).
// ---------------------------------------------------------------------------------------------

/** NDK pinned to the AGP 9.3 default; r28+ links with 16 KB page alignment (Android 15+ devices). */
val rpxNdkVersion = "28.2.13676358"

val rpxRustAbis: Provider<List<String>> = providers.gradleProperty("rpx.rustAbis")
    .orElse("arm64-v8a,x86_64")
    .map { value -> value.split(',').map { it.trim() }.filter { it.isNotEmpty() } }

abstract class CargoNdkTask @Inject constructor(
    private val execOps: ExecOperations,
) : DefaultTask() {

    @get:Input abstract val abis: ListProperty<String>
    @get:Input abstract val release: Property<Boolean>
    @get:Input abstract val minApi: Property<Int>
    @get:Input abstract val ndkVersion: Property<String>

    @get:InputFiles
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val rustSources: ConfigurableFileCollection

    @get:Internal abstract val ndkDirectory: DirectoryProperty
    @get:Internal abstract val workspaceDir: DirectoryProperty

    @get:OutputDirectory abstract val outputDir: DirectoryProperty

    @TaskAction
    fun build() {
        val out = outputDir.get().asFile
        out.deleteRecursively()
        out.mkdirs()

        val cargoHome = System.getenv("CARGO_HOME") ?: (System.getProperty("user.home") + "/.cargo")
        val cargoBin = File(cargoHome, "bin")
        // Java resolves the executable with the *parent* PATH; IDEs started from a desktop
        // launcher often lack ~/.cargo/bin, so prefer the absolute rustup proxy when present.
        val cargoExe = listOf("cargo", "cargo.exe")
            .map { File(cargoBin, it) }
            .firstOrNull { it.isFile }
            ?.absolutePath ?: "cargo"

        val command = mutableListOf(cargoExe, "ndk")
        abis.get().forEach { command += listOf("-t", it) }
        command += listOf("-P", minApi.get().toString(), "-o", out.absolutePath)
        command += listOf("build", "--package", "core-ffi")
        if (release.get()) command += "--release"
        // Reproducible builds: once Cargo.lock is committed, refuse to silently change it.
        if (File(workspaceDir.get().asFile, "Cargo.lock").isFile) command += "--locked"

        logger.lifecycle("cargo-ndk: ${command.joinToString(" ")}")
        try {
            execOps.exec {
                workingDir = workspaceDir.get().asFile
                commandLine(command)
                environment("ANDROID_NDK_HOME", ndkDirectory.get().asFile.absolutePath)
                environment(
                    "PATH",
                    cargoBin.absolutePath + File.pathSeparator + (System.getenv("PATH") ?: ""),
                )
            }
        } catch (e: Exception) {
            throw GradleException(
                "Building the Rust engine failed. Check that rustup, the Android targets and " +
                    "cargo-ndk are installed (README.md, section 'Rust setup'). Cause: ${e.message}",
                e,
            )
        }
        val missing = abis.get().filter { abi -> !File(out, "$abi/librpx_core.so").isFile }
        if (missing.isNotEmpty()) {
            throw GradleException("cargo-ndk finished but librpx_core.so is missing for: $missing")
        }
    }
}

/** Fails the build if the *merged* manifest (after all libraries) requests INTERNET. */
abstract class VerifyNoInternetTask : DefaultTask() {

    @get:InputFile
    @get:PathSensitive(PathSensitivity.NONE)
    abstract val mergedManifest: RegularFileProperty

    @TaskAction
    fun verify() {
        val factory = javax.xml.parsers.DocumentBuilderFactory.newInstance()
        factory.isNamespaceAware = true
        factory.setFeature(javax.xml.XMLConstants.FEATURE_SECURE_PROCESSING, true)
        val document = factory.newDocumentBuilder().parse(mergedManifest.get().asFile)
        val androidNs = "http://schemas.android.com/apk/res/android"

        val permissions = mutableListOf<String>()
        for (tag in listOf("uses-permission", "uses-permission-sdk-23")) {
            val nodes = document.getElementsByTagName(tag)
            for (i in 0 until nodes.length) {
                val element = nodes.item(i) as org.w3c.dom.Element
                permissions += element.getAttributeNS(androidNs, "name")
            }
        }
        logger.lifecycle(
            "Merged manifest permissions: " + if (permissions.isEmpty()) "<none>" else permissions.joinToString(),
        )
        if ("android.permission.INTERNET" in permissions) {
            throw GradleException(
                "The merged AndroidManifest requests android.permission.INTERNET. RPX Paint is " +
                    "offline-only; find the dependency that adds it with " +
                    "'./gradlew :app:dependencies' and the Manifest Merger report.",
            )
        }
    }
}

// ---------------------------------------------------------------------------------------------

val keystoreProperties = Properties().apply {
    val file = rootProject.file("keystore.properties") // never committed, see README
    if (file.isFile) file.inputStream().use { load(it) }
}

android {
    namespace = "com.rpxpaint.app"
    compileSdk = 36
    ndkVersion = rpxNdkVersion

    defaultConfig {
        // applicationId may be changed freely; `namespace` (Kotlin package) must stay in sync
        // with the JNI function names in crates/core-ffi (see README).
        applicationId = "com.rpxpaint.app"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"

        ndk {
            abiFilters += rpxRustAbis.get()
        }
    }

    signingConfigs {
        if (keystoreProperties.isNotEmpty()) {
            create("release") {
                storeFile = rootProject.file(keystoreProperties.getProperty("storeFile"))
                storePassword = keystoreProperties.getProperty("storePassword")
                keyAlias = keystoreProperties.getProperty("keyAlias")
                keyPassword = keystoreProperties.getProperty("keyPassword")
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
            signingConfig = signingConfigs.findByName("release")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        compose = true
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_17)
    }
}

androidComponents {
    onVariants { variant ->
        val capName = variant.name.replaceFirstChar { it.uppercase() }

        val buildRust = tasks.register<CargoNdkTask>("buildRustJni$capName") {
            group = "rust"
            description = "Builds librpx_core.so for ${variant.name} with cargo-ndk"
            abis.set(rpxRustAbis)
            release.set(variant.buildType == "release")
            minApi.set(variant.minSdk.apiLevel)
            ndkVersion.set(rpxNdkVersion)
            ndkDirectory.set(androidComponents.sdkComponents.ndkDirectory)
            workspaceDir.set(rootProject.layout.projectDirectory)
            rustSources.from(
                rootProject.files("Cargo.toml", "Cargo.lock", "rust-toolchain.toml"),
                rootProject.fileTree("crates") { include("**/*.rs", "**/Cargo.toml") },
            )
        }
        variant.sources.jniLibs?.addGeneratedSourceDirectory(buildRust, CargoNdkTask::outputDir)

        val verifyNoInternet = tasks.register<VerifyNoInternetTask>("verifyNoInternet$capName") {
            group = "verification"
            description = "Fails if the merged ${variant.name} manifest requests INTERNET"
            mergedManifest.set(variant.artifacts.get(SingleArtifact.MERGED_MANIFEST))
        }
        tasks.matching { it.name == "assemble$capName" || it.name == "bundle$capName" }
            .configureEach { dependsOn(verifyNoInternet) }
    }
}

dependencies {
    val composeBom = platform(libs.androidx.compose.bom)
    implementation(composeBom)
    androidTestImplementation(composeBom)

    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.material3)
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.lifecycle.viewmodel.compose)
    implementation(libs.androidx.lifecycle.runtime.compose)
    implementation(libs.kotlinx.coroutines.android)

    testImplementation(libs.junit)
    testImplementation(libs.kotlinx.coroutines.test)

    androidTestImplementation(libs.androidx.test.ext.junit)
    androidTestImplementation(libs.androidx.test.runner)
    androidTestImplementation(libs.androidx.compose.ui.test.junit4)
    debugImplementation(libs.androidx.compose.ui.test.manifest)
}
