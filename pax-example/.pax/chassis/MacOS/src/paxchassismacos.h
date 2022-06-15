// <paxchassismacos.h>
// Header to expose Rust data & logic via FFI -- if it isn't here, it doesn't exist to Swift.
// See also: `pax-chassis-macos/src/lib.rs` where this logic is exposed

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct NativeMessageQueue {
  uint8_t *data_ptr;
  uint64_t length;
} NativeMessageQueue;

typedef struct InterruptBuffer {
  const void *data_ptr;
  uint64_t length;
} InterruptBuffer;

typedef struct PaxEngineContainer PaxEngineContainer;

struct PaxEngineContainer *pax_init(void (*logger)(const char*));

void pax_dealloc_engine(struct PaxEngineContainer * container);

void pax_interrupt(struct PaxEngineContainer *engine_container, const void * interrupt);

struct NativeMessageQueue *pax_tick(struct PaxEngineContainer *engine_container,
                                          void *cgContext,
                                          float width,
                                          float height);

void pax_dealloc_message_queue(struct NativeMessageQueue *queue);
