#pragma once
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

// Version 2 host ABI. Every operation runs on the process main thread. Handles
// originate from prepare, are consumed once by run/mount, and require release.
// close is idempotent; retain the observer context until its release callback.
uint32_t pax_host_version(void);
// Optional copied JSON launch configuration (entry, title, development paths).
// Call before prepare with no active mount. Maximum 1 MiB; 0 is success.
int32_t pax_host_configure(const uint8_t *data, size_t len);
void *pax_host_prepare(void);
int32_t pax_host_run(void *session);
int32_t pax_host_mount(void *session, void *view, void *controller);
void pax_host_observe(void *session, void *context,
                     void (*notify)(void *, uint32_t), void (*release)(void *));
void pax_host_close(void *session);
void pax_host_release(void *session);
size_t pax_host_error(uint8_t *buffer, size_t capacity);

// Startup/request inputs are copied before return; maximum 64 KiB per buffer.
void *pax_host_prepare_with_data(const uint8_t *data, size_t len);
// Replies run asynchronously on the main thread, exactly once. Status 0 is
// success; nonzero bytes encode a UTF-8 error. Copy borrowed reply bytes before
// returning. Retain context until reply, including after closing the mount.
void pax_host_request(void *session, const uint8_t *data, size_t len, void *context,
                      void (*reply)(void *, uint32_t, const uint8_t *, size_t));

// macOS only. Configure a fresh session before run; sizes are logical points.
int32_t pax_host_window_options(void *session, const uint8_t *title, size_t len,
    double width, double height, double min_width, double min_height, bool resizable);
