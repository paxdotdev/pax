// <PaxCartridge.h>
// Header to expose Rust data & logic via FFI -- if it isn't here, it doesn't exist to Swift.
//
// SEE ALSO: `pax-chassis-macos/src/lib.rs` where this logic is managed.

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

struct PaxEngineContainer *pax_init(float initial_width, float initial_height);

void pax_dealloc_engine(struct PaxEngineContainer * container);

void pax_interrupt(struct PaxEngineContainer *engine_container, const void * interrupt);

struct NativeMessageQueue *pax_tick(struct PaxEngineContainer *engine_container,
                                    void *cgContext,
                                    float width,
                                    float height,
                                    float dpr);

struct NativeMessageQueue *pax_get_layer_canvas_plan(struct PaxEngineContainer *engine_container,
                                                     uint32_t layer_id,
                                                     float dpr);

void pax_surface_registry_begin_frame(struct PaxEngineContainer *engine_container,
                                      uint32_t layer_count);

void pax_surface_registry_set_layer_active(struct PaxEngineContainer *engine_container,
                                           uint32_t layer_id,
                                           bool active);

void pax_surface_registry_register_surface(struct PaxEngineContainer *engine_container,
                                           uint32_t layer_id,
                                           const char *key_ptr,
                                           const char *host_signature_ptr,
                                           float origin_x,
                                           float origin_y,
                                           int32_t replay_priority,
                                           float logical_width,
                                           float logical_height,
                                           uint32_t surface_width,
                                           uint32_t surface_height,
                                           float dpr_x,
                                           float dpr_y,
                                           void *layer_ptr);

void pax_refresh_render_surfaces(struct PaxEngineContainer *engine_container);

void pax_render(struct PaxEngineContainer *engine_container);

struct NativeMessageQueue *pax_designtime_inspect_tree(struct PaxEngineContainer *engine_container,
                                                       int64_t max_depth);

struct NativeMessageQueue *pax_designtime_ray_cast(struct PaxEngineContainer *engine_container,
                                                   const struct InterruptBuffer *request_buffer);

struct NativeMessageQueue *pax_designtime_selector_query(struct PaxEngineContainer *engine_container,
                                                         const struct InterruptBuffer *request_buffer);

struct NativeMessageQueue *pax_designtime_replace_node(struct PaxEngineContainer *engine_container,
                                                       const struct InterruptBuffer *request_buffer);

void pax_dealloc_message_queue(struct NativeMessageQueue *queue);
