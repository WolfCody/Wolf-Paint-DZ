# JNI: class and native method names are looked up by symbol name from librpx_core.so.
-keep class com.dzekiwolf.wolfpaintdz.core.RustBridge {
    native <methods>;
}

# Exceptions are instantiated from Rust by name (throw_new with a (String) constructor);
# nothing in Kotlin code constructs the subclasses, so R8 would otherwise remove them.
-keep class com.dzekiwolf.wolfpaintdz.core.RpxNativeException { *; }
-keep class com.dzekiwolf.wolfpaintdz.core.RpxNativeException$* {
    <init>(java.lang.String);
}
