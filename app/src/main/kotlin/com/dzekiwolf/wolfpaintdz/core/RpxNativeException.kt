package com.dzekiwolf.wolfpaintdz.core

/** Base class for all exceptions thrown by the native engine. */
sealed class RpxNativeException(message: String) : RuntimeException(message) {
    class InvalidArgument(message: String) : RpxNativeException(message)
    class CanvasTooLarge(message: String) : RpxNativeException(message)
    class InvalidHandle(message: String) : RpxNativeException(message)
    class NotInitialized(message: String) : RpxNativeException(message)
    class Unsupported(message: String) : RpxNativeException(message)
    class Io(message: String) : RpxNativeException(message)
    class Internal(message: String) : RpxNativeException(message)
    class NativePanic(message: String) : RpxNativeException(message)
}
