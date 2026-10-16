#ifndef PAX_LINUX_GPU_IMPORT_H
#define PAX_LINUX_GPU_IMPORT_H
#include <gtk/gtk.h>
#include <json-glib/json-glib.h>
typedef struct PaxLinuxGpu PaxLinuxGpu;
PaxLinuxGpu *pax_linux_gpu_new(GdkDisplay *display, GError **error);
void pax_linux_gpu_unref(PaxLinuxGpu *gpu);
gboolean pax_linux_gpu_capacity(PaxLinuxGpu *gpu, JsonArray *surfaces,
                                GError **error);
/* Takes both descriptor FDs. A nonnegative completion_fd transfers to caller;
 * it proves completion of the only EGL read of this Vulkan allocation. */
GdkTexture *pax_linux_gpu_import(PaxLinuxGpu *gpu, JsonObject *frame,
                                 int *completion_fd, GError **error);
guint pax_linux_gpu_live(PaxLinuxGpu *gpu);
guint64 pax_linux_gpu_bytes(PaxLinuxGpu *gpu);
#endif
