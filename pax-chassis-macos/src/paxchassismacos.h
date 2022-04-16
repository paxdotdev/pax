#include <stdint.h>
#import "CoreGraphics/CoreGraphics.h"

// Don't forget to `cargo build --release` after updating this interface Rust-side
//typedef PaxChassisMacosBridgeContainer;
const struct PaxChassisMacosBridgeContainer* pax_init(const void * ctx);
const char* pax_tick(const struct PaxChassisMacosBridgeContainer * container, const void * ctx);
