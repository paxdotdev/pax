#include <stdint.h>
//#import "CoreGraphics/CoreGraphics.h"


typedef struct PaxMessageQueueContainer PaxMessageQueueContainer;
typedef struct SomethingNotHere SomethingNotHere;
const struct PaxEngineContainer * pax_init(void (swiftLoggerCallback)(const char*));
const struct PaxMessageQueueContainer * pax_tick(const struct PaxEngineContainer * container, const void * ctx, const float width, const float height);
const void * pax_cleanup_message_queue(const struct PaxMessageQueueContainer * queue);
