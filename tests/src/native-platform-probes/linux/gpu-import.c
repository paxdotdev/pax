#include "gpu-import.h"
#include <epoxy/gl.h>
#include <dlfcn.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include <unistd.h>

typedef struct {
    int32_t fd;
    uint32_t width, height, stride, offset;
    uint64_t allocation_size;
    uint32_t fourcc;
    uint64_t modifier;
    uint8_t device_uuid[16], driver_uuid[16];
} ExportInfo;

static void *library, *gpu;
static void *(*gpu_new)(bool);
static void (*gpu_drop)(void *);
static void *(*gpu_tile)(const void *, uint32_t, uint32_t, double, double, double, ExportInfo *);
static void (*gpu_tile_drop)(void *);
static size_t (*gpu_live_allocations)(void);
static gboolean use_dmabuf;
static guint imports_created, imports_released;

typedef struct {
    void *tile;
    int fd;
    GdkGLContext *context;
    GLuint texture, memory;
    GLsync sync;
    gboolean published;
} ImportOwner;

static void release_import(gpointer data) {
    ImportOwner *owner = data;
    if (owner->context) {
        GdkGLContext *previous = gdk_gl_context_get_current();
        if (previous == owner->context) previous = NULL;
        if (previous) g_object_ref(previous);
        gdk_gl_context_make_current(owner->context);
        /* This first proof uses immutable exports. Wait for consumer retirement
         * before freeing Vulkan memory; streaming must replace this with fences. */
        glFinish();
        if (owner->sync) glDeleteSync(owner->sync);
        if (owner->texture) glDeleteTextures(1, &owner->texture);
        if (owner->memory) glDeleteMemoryObjectsEXT(1, &owner->memory);
        if (previous) { gdk_gl_context_make_current(previous); g_object_unref(previous); }
        else gdk_gl_context_clear_current();
        g_object_unref(owner->context);
    }
    if (owner->fd >= 0) close(owner->fd);
    if (owner->tile) gpu_tile_drop(owner->tile);
    if (owner->published) imports_released++;
    g_free(owner);
}

static gboolean ensure_gpu(void) {
    if (gpu) return TRUE;
    const char *path = g_getenv("PAX_PROBE_GPU_LIBRARY");
    if (!path) return FALSE;
    if (!library) {
        library = dlopen(path, RTLD_NOW | RTLD_LOCAL);
        if (!library) { g_printerr("gpu_library_error=%s\n", dlerror()); return FALSE; }
        gpu_new = dlsym(library, "pax_probe_gpu_new");
        gpu_drop = dlsym(library, "pax_probe_gpu_drop");
        gpu_tile = dlsym(library, "pax_probe_gpu_tile");
        gpu_tile_drop = dlsym(library, "pax_probe_gpu_tile_drop");
        gpu_live_allocations = dlsym(library, "pax_probe_gpu_live_allocations");
        if (!gpu_new || !gpu_drop || !gpu_tile || !gpu_tile_drop || !gpu_live_allocations) {
            g_printerr("gpu_library_error=missing probe exports\n");
            dlclose(library); library = NULL;
            gpu_live_allocations = NULL;
            return FALSE;
        }
    }
    use_dmabuf = g_strcmp0(g_getenv("PAX_PROBE_GPU_TRANSPORT"), "opaque-fd") != 0;
    gpu = gpu_new(use_dmabuf);
    return gpu != NULL;
}

GdkTexture *pax_probe_import_gpu_tile(GdkDisplay *display, guint width, guint height, double red, double green, double blue) {
    if (!ensure_gpu()) return NULL;
    ExportInfo info = {0};
    ImportOwner *owner = g_new0(ImportOwner, 1);
    owner->fd = -1;
    owner->tile = gpu_tile(gpu, width, height, red, green, blue, &info);
    if (!owner->tile) { g_free(owner); return NULL; }
    owner->fd = info.fd;
    if (use_dmabuf) {
        GdkDmabufTextureBuilder *builder = gdk_dmabuf_texture_builder_new();
        gdk_dmabuf_texture_builder_set_display(builder, display);
        gdk_dmabuf_texture_builder_set_width(builder, width);
        gdk_dmabuf_texture_builder_set_height(builder, height);
        gdk_dmabuf_texture_builder_set_fourcc(builder, info.fourcc);
        gdk_dmabuf_texture_builder_set_modifier(builder, info.modifier);
        gdk_dmabuf_texture_builder_set_n_planes(builder, 1);
        gdk_dmabuf_texture_builder_set_fd(builder, 0, info.fd);
        gdk_dmabuf_texture_builder_set_stride(builder, 0, info.stride);
        gdk_dmabuf_texture_builder_set_offset(builder, 0, info.offset);
        gdk_dmabuf_texture_builder_set_premultiplied(builder, TRUE);
        GError *error = NULL;
        GdkTexture *texture = gdk_dmabuf_texture_builder_build(builder, release_import, owner, &error);
        g_object_unref(builder);
        if (!texture) {
            g_printerr("dmabuf_import_error=%s\n", error ? error->message : "unknown");
            g_clear_error(&error); release_import(owner);
        } else {
            owner->published = TRUE; imports_created++;
            g_print("gpu_import=DMA_BUF texture=%s\n", G_OBJECT_TYPE_NAME(texture));
        }
        return texture;
    }
    GError *error = NULL;
    owner->context = gdk_display_create_gl_context(display, &error);
    if (!owner->context || !gdk_gl_context_realize(owner->context, &error)) {
        g_printerr("opaque_fd_context_error=%s\n", error ? error->message : "unknown");
        g_clear_error(&error); g_clear_object(&owner->context); release_import(owner); return NULL;
    }
    gdk_gl_context_make_current(owner->context);
    if (!epoxy_has_gl_extension("GL_EXT_memory_object_fd")) {
        g_printerr("opaque_fd_import_error=GL_EXT_memory_object_fd unavailable\n");
        release_import(owner); return NULL;
    }
    /* External memory handles are only defined for the same physical device.
     * A default Vulkan adapter can differ from GTK's GL adapter on hybrid GPUs. */
    while (glGetError() != GL_NO_ERROR) {}
    GLint device_count = 0;
    GLubyte uuid[GL_UUID_SIZE_EXT] = {0}, driver_uuid[GL_UUID_SIZE_EXT] = {0};
    glGetIntegerv(GL_NUM_DEVICE_UUIDS_EXT, &device_count);
    glGetUnsignedBytevEXT(GL_DRIVER_UUID_EXT, driver_uuid);
    gboolean same_device = FALSE;
    for (GLint i = 0; i < device_count; i++) {
        glGetUnsignedBytei_vEXT(GL_DEVICE_UUID_EXT, i, uuid);
        same_device |= memcmp(uuid, info.device_uuid, sizeof(uuid)) == 0;
    }
    if (glGetError() != GL_NO_ERROR || !same_device || memcmp(driver_uuid, info.driver_uuid, sizeof(driver_uuid))) {
        g_printerr("opaque_fd_import_error=Vulkan and GL device or driver UUID mismatch\n");
        release_import(owner); return NULL;
    }
    g_print("opaque_fd_identity=device_and_driver_uuid_match synchronization=immutable_probe_only\n");
    while (glGetError() != GL_NO_ERROR) {}
    glCreateMemoryObjectsEXT(1, &owner->memory);
    GLint dedicated = GL_TRUE;
    glMemoryObjectParameterivEXT(owner->memory, GL_DEDICATED_MEMORY_OBJECT_EXT, &dedicated);
    glImportMemoryFdEXT(owner->memory, info.allocation_size, GL_HANDLE_TYPE_OPAQUE_FD_EXT, owner->fd);
    GLenum status = glGetError();
    if (status != GL_NO_ERROR) {
        g_printerr("opaque_fd_import_error=glImportMemoryFdEXT:%x\n", status); release_import(owner); return NULL;
    }
    owner->fd = -1; /* GL owns the successfully imported descriptor. */
    glGenTextures(1, &owner->texture);
    glBindTexture(GL_TEXTURE_2D, owner->texture);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_TILING_EXT, GL_OPTIMAL_TILING_EXT);
    glTexStorageMem2DEXT(GL_TEXTURE_2D, 1, GL_RGBA8, width, height, owner->memory, 0);
    status = glGetError();
    if (status != GL_NO_ERROR) {
        g_printerr("opaque_fd_import_error=glTexStorageMem2DEXT:%x\n", status); release_import(owner); return NULL;
    }
    owner->sync = glFenceSync(GL_SYNC_GPU_COMMANDS_COMPLETE, 0);
    glFlush();
    GdkGLTextureBuilder *builder = gdk_gl_texture_builder_new();
    gdk_gl_texture_builder_set_context(builder, owner->context);
    gdk_gl_texture_builder_set_id(builder, owner->texture);
    gdk_gl_texture_builder_set_width(builder, width);
    gdk_gl_texture_builder_set_height(builder, height);
    gdk_gl_texture_builder_set_sync(builder, owner->sync);
    GdkTexture *texture = gdk_gl_texture_builder_build(builder, release_import, owner);
    if (!texture) {
        g_printerr("opaque_fd_import_error=GDK texture construction failed\n");
        g_object_unref(builder); release_import(owner); return NULL;
    }
    owner->published = TRUE; imports_created++;
    g_object_unref(builder);
    gdk_gl_context_clear_current();
    g_print("gpu_import=OPAQUE_FD_GL texture=%s\n", G_OBJECT_TYPE_NAME(texture));
    return texture;
}

guint pax_probe_import_created(void) { return imports_created; }
guint pax_probe_import_released(void) { return imports_released; }
gsize pax_probe_import_live_allocations(void) { return gpu_live_allocations ? gpu_live_allocations() : G_MAXSIZE; }

void pax_probe_gpu_shutdown(void) {
    if (gpu) { gpu_drop(gpu); gpu = NULL; }
    /* GDK can retain textures beyond window destruction. Keep the code mapped
     * so deferred resource callbacks cannot jump into an unloaded library. */
}
