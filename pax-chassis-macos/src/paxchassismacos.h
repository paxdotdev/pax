#include <stdint.h>
//#import "CoreGraphics/CoreGraphics.h"

const struct PaxChassisMacosBridgeContainer * pax_init(void (swiftLoggerCallback)(const char*));
const void* pax_tick(const struct PaxChassisMacosBridgeContainer * container, const void * ctx, const float width, const float height);
