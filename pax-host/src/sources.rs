//! Canonical native SDK sources, shared with Cargo and the packaging frontend.
use include_dir::{include_dir, Dir};
pub static PAX_MACOS_INTERFACE_TEMPLATE: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/native/macos");
pub static PAX_IOS_INTERFACE_TEMPLATE: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/native/ios");
pub static PAX_SWIFT_CARTRIDGE_TEMPLATE: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/native/common/pax-swift-cartridge");
pub static PAX_SWIFT_COMMON_TEMPLATE: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/native/common/pax-swift-common");

pub const SOURCE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/native");
