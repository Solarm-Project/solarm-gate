fn main() {
    //#[cfg(target_os = "macos")]
    println!("cargo:rustc-env=PKG_CONFIG_PATH=\"/opt/homebrew/opt/libarchive/lib/pkgconfig\"");
}
