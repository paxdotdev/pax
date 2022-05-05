#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct PaxEngineContainer PaxEngineContainer;
typedef struct PaxMessageQueueContainer PaxMessageQueueContainer;

typedef union NativeMessage NativeMessage;
typedef struct NativeArgsClick NativeArgsClick;
typedef struct ClippingPatch ClippingPatch;
typedef struct TextSize TextSize;
typedef struct Affine Affine;
typedef struct TextPatch TextPatch;
typedef struct TextCommand TextCommand;

struct PaxEngineContainer *pax_init(void (*logger)(const char*));

struct PaxMessageQueueContainer *pax_tick(struct PaxEngineContainer *bridge_container,
                                          void *cgContext,
                                          float width,
                                          float height);

void pax_cleanup_message_queue(union NativeMessage *queue);
