package com.dzekiwolf.wolfpaintdz.core

/**
 * Opaque reference to a document owned by the Rust engine. The value is validated by Rust on
 * every call; a closed or forged handle produces [RpxNativeException.InvalidHandle].
 */
@JvmInline
value class DocumentHandle(val raw: Long)
