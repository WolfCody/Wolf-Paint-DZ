package com.dzekiwolf.wolfpaintdz.core

/**
 * Exceptions thrown by the Rust engine through JNI.
 *
 * The Rust side instantiates the nested classes by JVM name
 * (`com/dzekiwolf/wolfpaintdz/core/RpxNativeException$InvalidArgument`, ...) using the `(String)`
 * constructor. Do not rename or move them without changing `exception_class` in
 * crates/core-ffi/src/lib.rs. R8 keep rules are in app/proguard-rules.pro.
 */
sealed class RpxNativeException(message: String) : RuntimeException(message) {
    class InvalidArgument(message: String) : RpxNativeException(message)
    class CanvasTooLarge(message: String) : RpxNativeException(message)
    class InvalidHandle(message: String) : RpxNativeException(message)
    class NotInitialized(message: String) : RpxNativeException(message)
    class Unsupported(message: String) : RpxNativeException(message)
    class Io(message: String) : RpxNativeException(message)
    class Internal(message: String) : RpxNativeException(message)

    /** A Rust panic was caught at the JNI boundary. The process keeps running. */
    class NativePanic(message: String) : RpxNativeException(message)
}
