#define _GNU_SOURCE
#include "gpu-import.h"
#include <epoxy/gl.h>
#include <epoxy/egl.h>
#include <dlfcn.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include <unistd.h>
#include <errno.h>
#include <linux/dma-buf.h>
#include <sys/ioctl.h>
#include <poll.h>
#include <fcntl.h>

typedef struct {
    int32_t fd, sync_fd;
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
static guint pending_producer_imports;
static int delayed_producer_fd = -1;

typedef struct {
    void *tile;
    int fd;
    GdkGLContext *context;
    GLuint texture, memory;
    GLsync sync;
    gboolean published;
    gboolean reservation_sync;
} ImportOwner;

static void release_import(gpointer data) {
    ImportOwner *owner = data;
    if (owner->reservation_sync && owner->published) {
        /* GDK reference retirement is distinct from GPU retirement. This probe
         * waits on the dma-buf reservation's readers before freeing memory.
         * A streaming pool must enqueue retirement without blocking GTK. */
        struct dma_buf_export_sync_file fence = { .flags = DMA_BUF_SYNC_WRITE, .fd = -1 };
        int result;
        do { result = ioctl(owner->fd, DMA_BUF_IOCTL_EXPORT_SYNC_FILE, &fence); } while (result < 0 && errno == EINTR);
        if (result < 0 || fence.fd < 0) {
            g_printerr("dmabuf_retirement_error=EXPORT_SYNC_FILE errno=%d retaining_allocation\n", errno);
            return;
        }
        struct pollfd wait = { .fd = fence.fd, .events = POLLIN };
        gint64 deadline = g_get_monotonic_time() + 10000000;
        do {
            gint64 remaining = deadline - g_get_monotonic_time();
            result = remaining > 0 ? poll(&wait, 1, (remaining + 999) / 1000) : 0;
        } while (result < 0 && errno == EINTR);
        close(fence.fd);
        if (result <= 0 || !(wait.revents & POLLIN) || (wait.revents & (POLLERR | POLLNVAL))) {
            g_printerr("dmabuf_retirement_error=consumer_fence_wait result=%d events=%x retaining_allocation\n", result, wait.revents);
            return;
        }
        g_print("dmabuf_retirement=reservation_readers_complete\n");
    }
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

/* An explicit GPU wait plus an owned GL copy avoids depending on the driver's
 * implicit DMA-BUF waits. This remains an opt-in immutable-image diagnostic. */
static GdkTexture *import_dmabuf_egl(GdkDisplay *display, ImportOwner *owner, ExportInfo *info) {
    GError *error = NULL;
    EGLDisplay egl_display = EGL_NO_DISPLAY;
    EGLImage image = EGL_NO_IMAGE;
    EGLSyncKHR fence = EGL_NO_SYNC_KHR;
    GLuint external = 0, framebuffer = 0, vao = 0, vertex = 0, fragment = 0, program = 0;
    GdkTexture *result = NULL;
    owner->context = gdk_display_create_gl_context(display, &error);
    if (!owner->context) goto done;
    gdk_gl_context_set_allowed_apis(owner->context, GDK_GL_API_GLES);
    gdk_gl_context_set_required_version(owner->context, 3, 0);
    if (!gdk_gl_context_realize(owner->context, &error)) { g_clear_object(&owner->context); goto done; }
    gdk_gl_context_make_current(owner->context);
    egl_display = eglGetCurrentDisplay();
    if (egl_display == EGL_NO_DISPLAY
        || !epoxy_has_egl_extension(egl_display, "EGL_ANDROID_native_fence_sync")
        || !epoxy_has_egl_extension(egl_display, "EGL_KHR_wait_sync")
        || !epoxy_has_egl_extension(egl_display, "EGL_EXT_image_dma_buf_import_modifiers")
        || !epoxy_has_gl_extension("GL_OES_EGL_image_external_essl3")) {
        g_printerr("dmabuf_egl_error=required EGL/GLES extensions unavailable\n"); goto done;
    }
    if (info->sync_fd >= 0) {
        EGLint attributes[] = { EGL_SYNC_NATIVE_FENCE_FD_ANDROID, info->sync_fd, EGL_NONE };
        fence = eglCreateSyncKHR(egl_display, EGL_SYNC_NATIVE_FENCE_ANDROID, attributes);
        info->sync_fd = -1; /* The native-fence extension takes descriptor ownership. */
        if (fence == EGL_NO_SYNC_KHR || !eglWaitSyncKHR(egl_display, fence, 0)) {
            g_printerr("dmabuf_egl_error=native_fence_wait error=%x\n", eglGetError()); goto done;
        }
        g_print("dmabuf_sync=explicit_egl_server_wait no_cpu_wait=true\n");
    }
    EGLint attributes[] = { EGL_IMAGE_PRESERVED_KHR, EGL_TRUE,
        EGL_WIDTH, info->width, EGL_HEIGHT, info->height,
        EGL_LINUX_DRM_FOURCC_EXT, info->fourcc,
        EGL_DMA_BUF_PLANE0_FD_EXT, info->fd, EGL_DMA_BUF_PLANE0_OFFSET_EXT, info->offset,
        EGL_DMA_BUF_PLANE0_PITCH_EXT, info->stride,
        EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT, (EGLint)info->modifier,
        EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT, (EGLint)(info->modifier >> 32), EGL_NONE };
    image = eglCreateImageKHR(egl_display, EGL_NO_CONTEXT, EGL_LINUX_DMA_BUF_EXT, NULL, attributes);
    if (image == EGL_NO_IMAGE) { g_printerr("dmabuf_egl_error=image error=%x\n", eglGetError()); goto done; }
    while (glGetError() != GL_NO_ERROR) {}
    glGenTextures(1, &external);
    glActiveTexture(GL_TEXTURE0);
    glBindTexture(GL_TEXTURE_EXTERNAL_OES, external);
    glEGLImageTargetTexture2DOES(GL_TEXTURE_EXTERNAL_OES, image);
    glTexParameteri(GL_TEXTURE_EXTERNAL_OES, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_EXTERNAL_OES, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_EXTERNAL_OES, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_EXTERNAL_OES, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    glGenTextures(1, &owner->texture);
    glBindTexture(GL_TEXTURE_2D, owner->texture);
    glTexStorage2D(GL_TEXTURE_2D, 1, GL_RGBA8, info->width, info->height);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
    glGenFramebuffers(1, &framebuffer);
    glBindFramebuffer(GL_FRAMEBUFFER, framebuffer);
    glFramebufferTexture2D(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, owner->texture, 0);
    if (glCheckFramebufferStatus(GL_FRAMEBUFFER) != GL_FRAMEBUFFER_COMPLETE) {
        g_printerr("dmabuf_egl_error=incomplete framebuffer\n"); goto done;
    }
    const char *vertex_source = "#version 300 es\nout vec2 uv; void main() { vec2 p=vec2((gl_VertexID<<1)&2,gl_VertexID&2); uv=p; gl_Position=vec4(p*2.0-1.0,0,1); }";
    const char *fragment_source = "#version 300 es\n#extension GL_OES_EGL_image_external_essl3 : require\nprecision highp float; uniform samplerExternalOES source; in vec2 uv; out vec4 pixel; void main(){pixel=texture(source,uv);}";
    vertex = glCreateShader(GL_VERTEX_SHADER); fragment = glCreateShader(GL_FRAGMENT_SHADER);
    glShaderSource(vertex, 1, &vertex_source, NULL); glCompileShader(vertex);
    glShaderSource(fragment, 1, &fragment_source, NULL); glCompileShader(fragment);
    program = glCreateProgram(); glAttachShader(program, vertex); glAttachShader(program, fragment); glLinkProgram(program);
    GLint linked = GL_FALSE; glGetProgramiv(program, GL_LINK_STATUS, &linked);
    if (!linked) {
        char log[1024]; glGetProgramInfoLog(program, sizeof(log), NULL, log);
        g_printerr("dmabuf_egl_error=shader %s\n", log); goto done;
    }
    glUseProgram(program); glUniform1i(glGetUniformLocation(program, "source"), 0);
    glGenVertexArrays(1, &vao); glBindVertexArray(vao);
    glViewport(0, 0, info->width, info->height); glDrawArrays(GL_TRIANGLES, 0, 3);
    GLenum status = glGetError();
    if (status != GL_NO_ERROR) { g_printerr("dmabuf_egl_error=GPUcopy error=%x\n", status); goto done; }
    owner->sync = glFenceSync(GL_SYNC_GPU_COMMANDS_COMPLETE, 0);
    if (!owner->sync) { g_printerr("dmabuf_egl_error=GL publication fence unavailable\n"); goto done; }
    glFlush();
    GdkGLTextureBuilder *builder = gdk_gl_texture_builder_new();
    gdk_gl_texture_builder_set_context(builder, owner->context);
    gdk_gl_texture_builder_set_id(builder, owner->texture);
    gdk_gl_texture_builder_set_width(builder, info->width);
    gdk_gl_texture_builder_set_height(builder, info->height);
    gdk_gl_texture_builder_set_sync(builder, owner->sync);
    result = gdk_gl_texture_builder_build(builder, release_import, owner);
    g_object_unref(builder);
    if (result) {
        owner->published = TRUE; imports_created++;
        g_print("gpu_import=DMA_BUF_EGL_EXPLICIT texture=%s extra_gpu_copies=1 no_readback=true\n", G_OBJECT_TYPE_NAME(result));
    }
done:
    if (owner->context && gdk_gl_context_get_current() == owner->context) {
        if (program) glDeleteProgram(program);
        if (vertex) glDeleteShader(vertex);
        if (fragment) glDeleteShader(fragment);
        if (vao) glDeleteVertexArrays(1, &vao);
        if (framebuffer) glDeleteFramebuffers(1, &framebuffer);
        if (external) glDeleteTextures(1, &external);
    }
    if (image != EGL_NO_IMAGE) eglDestroyImageKHR(egl_display, image);
    if (fence != EGL_NO_SYNC_KHR) eglDestroySyncKHR(egl_display, fence);
    if (info->sync_fd >= 0) close(info->sync_fd);
    if (error) { g_printerr("dmabuf_egl_error=%s\n", error->message); g_clear_error(&error); }
    if (!result) release_import(owner);
    gdk_gl_context_clear_current();
    return result;
}

static gboolean ensure_gpu(void) {
    if (gpu) return TRUE;
    const char *path = g_getenv("PAX_PROBE_GPU_LIBRARY");
    if (!path) return FALSE;
    const char *transport = g_getenv("PAX_PROBE_GPU_TRANSPORT");
    if (g_strcmp0(transport, "dmabuf-egl") == 0 && g_strcmp0(g_getenv("PAX_PROBE_DMABUF_SYNC"), "1") != 0) {
        g_printerr("gpu_library_error=dmabuf-egl requires PAX_PROBE_DMABUF_SYNC=1\n"); return FALSE;
    }
    if (!library) {
        library = dlopen(path, RTLD_NOW | RTLD_LOCAL);
        if (!library) { g_printerr("gpu_library_error=%s\n", dlerror()); return FALSE; }
        gpu_new = dlsym(library, "pax_probe_gpu_new");
        gpu_drop = dlsym(library, "pax_probe_gpu_drop");
        gpu_tile = dlsym(library, "pax_probe_gpu_tile");
        gpu_tile_drop = dlsym(library, "pax_probe_gpu_tile_drop");
        gpu_live_allocations = dlsym(library, "pax_probe_gpu_live_allocations");
        uint32_t (*abi_version)(void) = dlsym(library, "pax_probe_gpu_abi_version");
        if (!gpu_new || !gpu_drop || !gpu_tile || !gpu_tile_drop || !gpu_live_allocations || !abi_version || abi_version() != 2) {
            g_printerr("gpu_library_error=missing or incompatible probe exports; rebuild both probe halves\n");
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
        gint64 import_start = g_get_monotonic_time();
        owner->reservation_sync = g_strcmp0(g_getenv("PAX_PROBE_DMABUF_SYNC"), "1") == 0;
        if (info.sync_fd >= 0) {
            if (width == 81 && height == 49 && g_strcmp0(g_getenv("PAX_PROBE_SYNC_DELAY_TEST"), "1") == 0) {
                if (delayed_producer_fd >= 0) close(delayed_producer_fd);
                delayed_producer_fd = fcntl(info.sync_fd, F_DUPFD_CLOEXEC, 0);
            }
            struct pollfd status = { .fd = info.sync_fd, .events = POLLIN };
            int ready = poll(&status, 1, 0);
            if (ready == 0) pending_producer_imports++;
            g_print("dmabuf_producer_pending_at_import=%d\n", ready == 0);
            if (g_strcmp0(g_getenv("PAX_PROBE_GPU_TRANSPORT"), "dmabuf-egl") == 0) {
                owner->reservation_sync = FALSE;
                return import_dmabuf_egl(display, owner, &info);
            }
            struct dma_buf_import_sync_file fence = { .flags = DMA_BUF_SYNC_WRITE, .fd = info.sync_fd };
            int result;
            do { result = ioctl(info.fd, DMA_BUF_IOCTL_IMPORT_SYNC_FILE, &fence); } while (result < 0 && errno == EINTR);
            int error_number = errno;
            close(info.sync_fd);
            if (result < 0) {
                g_printerr("dmabuf_sync_error=IMPORT_SYNC_FILE errno=%d message=%s\n", error_number, g_strerror(error_number));
                release_import(owner); return NULL;
            }
            g_print("dmabuf_sync=producer_sync_file_imported_as_write_fence\n");
        }
        if (g_strcmp0(g_getenv("PAX_PROBE_GPU_TRANSPORT"), "dmabuf-egl") == 0) {
            owner->reservation_sync = FALSE;
            return import_dmabuf_egl(display, owner, &info);
        }
        gint64 reservation_done = g_get_monotonic_time();
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
        g_print("dmabuf_import_timing reservation_us=%" G_GINT64_FORMAT " builder_us=%" G_GINT64_FORMAT "\n",
            reservation_done - import_start, g_get_monotonic_time() - reservation_done);
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
guint pax_probe_pending_producer_imports(void) { return pending_producer_imports; }
int pax_probe_delayed_producer_ready(void) {
    if (delayed_producer_fd < 0) return -1;
    struct pollfd status = { .fd = delayed_producer_fd, .events = POLLIN };
    int result = poll(&status, 1, 0);
    if (result < 0 || (status.revents & (POLLERR | POLLNVAL))) return -1;
    return result > 0 && (status.revents & POLLIN) ? 1 : 0;
}
gsize pax_probe_import_live_allocations(void) { return gpu_live_allocations ? gpu_live_allocations() : G_MAXSIZE; }

void pax_probe_gpu_shutdown(void) {
    if (delayed_producer_fd >= 0) { close(delayed_producer_fd); delayed_producer_fd = -1; }
    if (gpu) { gpu_drop(gpu); gpu = NULL; }
    /* GDK can retain textures beyond window destruction. Keep the code mapped
     * so deferred resource callbacks cannot jump into an unloaded library. */
}
