fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        // The child is installed after instantiation. Ordinary Rust binaries
        // use a fixed-size table and cannot accept an additional function.
        println!("cargo:rustc-link-arg=--growable-table");
    }
}
