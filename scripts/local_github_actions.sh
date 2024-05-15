cargo test --verbose --workspace --exclude pax-chassis-macos --exclude pax-chassis-common --exclude pax-chassis-ios
cargo fmt -- --check
cargo clippy -- -D warnings
