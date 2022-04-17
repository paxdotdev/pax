#include <stdint.h>
#import "CoreGraphics/CoreGraphics.h"

// Don't forget to `cargo build --release` after updating this interface Rust-side
const struct PaxChassisMacosBridgeContainer * pax_init();
const void* pax_tick(const struct PaxChassisMacosBridgeContainer * container, const void * ctx, const float width, const float height);
