#ifndef PAX_DESKTOP_H
#define PAX_DESKTOP_H
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

/* Desktop cartridge ABI 4. All engine calls belong to its UI thread. A wake
 * callback may run on another thread and must only enqueue a token-checked UI
 * operation. Libraries remain process-pinned while callbacks/TLS can reference
 * them. A nonempty returned buffer is owned by the cartridge until free_buffer. */
typedef struct PaxDesktopEngine PaxDesktopEngine;
typedef struct PaxDesktopBuffer { uint8_t *data; size_t len; } PaxDesktopBuffer;
typedef void (*PaxDesktopWake)(uint64_t token);
typedef void (*PaxDesktopReply)(void *context, uint32_t status, const uint8_t *data, size_t len);

uint32_t pax_cartridge_abi_version(void);
PaxDesktopEngine *pax_init_entry(float width, float height, const uint8_t *entry,
    size_t entry_len, const uint8_t *startup, size_t startup_len);
void pax_desktop_free_buffer(PaxDesktopBuffer buffer);
PaxDesktopBuffer pax_desktop_initialize_gpu(PaxDesktopEngine *engine);
/* {ok:true,messages:[existing NativeMessage JSON...],layerCount:u32}
 * or {ok:false,error:string}. */
PaxDesktopBuffer pax_desktop_tick(PaxDesktopEngine *engine, float width, float height, float dpr);
/* {ok:true,plan:LayerCanvasPlan|null,owner:u32|null}. Existing plan keys camelCase. */
PaxDesktopBuffer pax_desktop_plan(PaxDesktopEngine *engine, uint32_t layer, float dpr);
/* JSON array of DesktopSurfaceInfo: layer,key,hostSignature,originX,originY,
 * logicalWidth,logicalHeight,width,height,dpr:[x,y],replayPriority. Replaces the
 * complete current registry atomically. Empty removes all render surfaces. */
PaxDesktopBuffer pax_desktop_set_surfaces(PaxDesktopEngine *engine, const uint8_t *data, size_t len);
/* {ok:true,blocked:bool,frame:platform_descriptors|null}. No canvas pixel readback.
 * blocked:true preserves the previously committed native/GPU scene, including
 * while any active surface awaits replay for its current geometry. A nonblocked
 * result permits committing native patches once the platform descriptors'
 * presentation requirements are met (Android also waits for buffer receipt). */
PaxDesktopBuffer pax_desktop_render(PaxDesktopEngine *engine);
/* NativeInterrupt serde JSON; {ok:true,preventDefault:bool}. Image bytes use load_image. */
PaxDesktopBuffer pax_desktop_interrupt(PaxDesktopEngine *engine, const uint8_t *data, size_t len);
PaxDesktopBuffer pax_desktop_load_image(PaxDesktopEngine *engine, const uint8_t *path, size_t path_len,
    const uint8_t *rgba, size_t rgba_len, uint32_t width, uint32_t height);
/* Optional composed-window capture extension. Empty bytes/zero dimensions report failure. */
PaxDesktopBuffer pax_desktop_screenshot(PaxDesktopEngine *engine, uint32_t id,
    const uint8_t *rgba, size_t rgba_len, uint32_t width, uint32_t height);
/* Release one backend token with platform JSON completion evidence. */
PaxDesktopBuffer pax_desktop_release(PaxDesktopEngine *engine, uint64_t token, const uint8_t *data, size_t len);
/* {ok:bool,error:string|null,closing:bool,drained:bool,gpu:diagnostics}.
 * Errors are terminal for new rendering; close and releases remain available.
 * Never blocks on GPU work. Only drained:true permits finish_close destruction. */
PaxDesktopBuffer pax_desktop_poll(PaxDesktopEngine *engine);
void pax_desktop_activate(PaxDesktopEngine *engine);
void pax_desktop_set_property_waker(PaxDesktopEngine *engine, uint64_t token, PaxDesktopWake wake);
bool pax_desktop_has_pending_properties(PaxDesktopEngine *engine);
void pax_desktop_set_shutdown_waker(uint64_t token, PaxDesktopWake wake);
uint64_t pax_desktop_pump_shutdowns(void);
void pax_desktop_request(PaxDesktopEngine *engine, const uint8_t *data, size_t len,
    void *context, PaxDesktopReply reply);
/* Begin close before detaching views; stop tick/render/input, retire wake tokens,
 * release all host-owned exports. Poll until drained, then finish_close destroys
 * the engine and returns true. False preserves the engine for later polling.
 * True consumes the pointer even if destruction reports a contained panic. */
void pax_desktop_begin_close(PaxDesktopEngine *engine);
bool pax_desktop_finish_close(PaxDesktopEngine *engine);
#ifdef __cplusplus
}
#endif
#endif
