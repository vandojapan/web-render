fn main() {
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        // AviUtl2 resolves the DLL entry points at runtime and needs no import
        // library. MSVC's localized library progress is otherwise misclassified
        // as linker_messages by rustc; keep actual linker diagnostics enabled.
        println!("cargo::rustc-link-arg-cdylib=/NOIMPLIB");
        println!("cargo::rustc-link-arg-cdylib=/NOEXP");
    }
}
