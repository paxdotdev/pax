// <paxchassismacos.h>
// Header to expose Rust data & logic via FFI -- if it isn't here, it doesn't exist to Swift.
// Portions of this code were generated with `cbindgen --lang C`.
// See `pax-message/src/lib.rs` for more documentation.

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct NativeArgsClick {
  double x;
  double y;
} NativeArgsClick;


typedef struct TextCommand {
  const char *set_font;
  const char *set_weight;
  const char *set_fill_color;
  const char *set_stroke_color;
  const char *set_decoration;
} TextCommand;

typedef struct NativeMessageQueue {
  const struct NativeMessage *msg_ptr;
  uint64_t length;
} NativeMessageQueue;


typedef struct PaxEngineContainer PaxEngineContainer;


struct PaxEngineContainer *pax_init(void (*logger)(const char*));

struct NativeMessageQueue *pax_tick(struct PaxEngineContainer *bridge_container,
                                          void *cgContext,
                                          float width,
                                          float height);

void pax_cleanup_message_queue(struct NativeMessage *queue);
