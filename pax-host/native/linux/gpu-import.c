#include "gpu-import.h"
#include <epoxy/egl.h>
#include <epoxy/gl.h>
#include <unistd.h>
#define MAX_GL_IMAGES 48
#define MAX_GL_BYTES ((guint64)256 * 1024 * 1024)
struct PaxLinuxGpu {
  gint references;
  GdkGLContext *context;
  GMainContext *main;
  EGLDisplay display;
  GLuint program, vao;
  guint live;
  guint64 bytes;
  gboolean failed;
};
typedef struct {
  PaxLinuxGpu *gpu;
  GLuint texture;
  GLsync sync;
  guint64 bytes;
} ImageOwner;
static void restore_context(GdkGLContext *previous) {
  if (previous) {
    gdk_gl_context_make_current(previous);
    g_object_unref(previous);
  } else
    gdk_gl_context_clear_current();
}
static GdkGLContext *enter_context(PaxLinuxGpu *gpu) {
  GdkGLContext *previous = gdk_gl_context_get_current();
  if (previous)
    g_object_ref(previous);
  gdk_gl_context_make_current(gpu->context);
  return previous;
}
static gboolean release_image_main(gpointer data) {
  ImageOwner *owner = data;
  PaxLinuxGpu *gpu = owner->gpu;
  GdkGLContext *previous = enter_context(gpu);
  GLenum completed = glClientWaitSync(owner->sync, 0, 0);
  if (completed == GL_TIMEOUT_EXPIRED) {
    restore_context(previous);
    return G_SOURCE_CONTINUE;
  }
  if (completed == GL_WAIT_FAILED) {
    gpu->failed = TRUE;
    restore_context(previous);
    // A lost context is not evidence that the allocation is safe to reclaim.
    // Keep this owner and its quota charged; the next capacity check fails.
    return G_SOURCE_REMOVE;
  }
  // GDK has released its immutable image. GL deletion defers storage retirement
  // for already submitted readers; it must not be replaced by mutable reuse.
  if (owner->sync)
    glDeleteSync(owner->sync);
  if (owner->texture)
    glDeleteTextures(1, &owner->texture);
  glFlush();
  restore_context(previous);
  gpu->live--;
  gpu->bytes -= owner->bytes;
  pax_linux_gpu_unref(gpu);
  g_free(owner);
  return G_SOURCE_REMOVE;
}
static void release_image(gpointer data) {
  ImageOwner *owner = data;
  // GdkTexture references may retire on other threads; GL state belongs to UI.
  GSource *source = g_timeout_source_new(2);
  g_source_set_callback(source, release_image_main, owner, NULL);
  g_source_attach(source, owner->gpu->main);
  g_source_unref(source);
}
static GLuint compile_shader(GLenum type, const char *source, GError **error) {
  GLuint shader = glCreateShader(type);
  glShaderSource(shader, 1, &source, NULL);
  glCompileShader(shader);
  GLint valid = GL_FALSE;
  glGetShaderiv(shader, GL_COMPILE_STATUS, &valid);
  if (!valid) {
    char log[1024];
    glGetShaderInfoLog(shader, sizeof(log), NULL, log);
    g_set_error(error, G_IO_ERROR, G_IO_ERROR_FAILED, "Linux import shader: %s",
                log);
    glDeleteShader(shader);
    return 0;
  }
  return shader;
}
PaxLinuxGpu *pax_linux_gpu_new(GdkDisplay *display, GError **error) {
  PaxLinuxGpu *gpu = g_new0(PaxLinuxGpu, 1);
  gpu->references = 1;
  gpu->main = g_main_context_ref_thread_default();
  gpu->context = gdk_display_create_gl_context(display, error);
  if (!gpu->context)
    goto failed;
  gdk_gl_context_set_allowed_apis(gpu->context, GDK_GL_API_GLES);
  gdk_gl_context_set_required_version(gpu->context, 3, 0);
  if (!gdk_gl_context_realize(gpu->context, error))
    goto failed;
  GdkGLContext *previous = enter_context(gpu);
  gpu->display = eglGetCurrentDisplay();
  if (gpu->display == EGL_NO_DISPLAY ||
      !epoxy_has_egl_extension(gpu->display, "EGL_ANDROID_native_fence_sync") ||
      !epoxy_has_egl_extension(gpu->display, "EGL_KHR_wait_sync") ||
      !epoxy_has_egl_extension(gpu->display,
                               "EGL_EXT_image_dma_buf_import_modifiers") ||
      !epoxy_has_gl_extension("GL_OES_EGL_image_external_essl3")) {
    g_set_error(error, G_IO_ERROR, G_IO_ERROR_NOT_SUPPORTED,
                "Linux requires EGL native fences, DMA-BUF modifiers and GLES3 "
                "external images");
    restore_context(previous);
    goto failed;
  }
  const char *vertex_source = "#version 300 es\nout vec2 uv;void main(){vec2 "
                              "p=vec2((gl_VertexID<<1)&2,gl_VertexID&2);uv=p;"
                              "gl_Position=vec4(p*2.0-1.0,0,1);}";
  const char *fragment_source =
      "#version 300 es\n#extension GL_OES_EGL_image_external_essl3 : "
      "require\nprecision highp float;uniform samplerExternalOES source;in "
      "vec2 uv;out vec4 pixel;void main(){pixel=texture(source,uv);}";
  GLuint vertex = compile_shader(GL_VERTEX_SHADER, vertex_source, error),
         fragment =
             vertex ? compile_shader(GL_FRAGMENT_SHADER, fragment_source, error)
                    : 0;
  if (vertex && fragment) {
    gpu->program = glCreateProgram();
    glAttachShader(gpu->program, vertex);
    glAttachShader(gpu->program, fragment);
    glLinkProgram(gpu->program);
    GLint linked = 0;
    glGetProgramiv(gpu->program, GL_LINK_STATUS, &linked);
    if (!linked) {
      g_set_error(error, G_IO_ERROR, G_IO_ERROR_FAILED,
                  "Linux import shader link failed");
      glDeleteProgram(gpu->program);
      gpu->program = 0;
    }
  }
  if (vertex)
    glDeleteShader(vertex);
  if (fragment)
    glDeleteShader(fragment);
  if (gpu->program)
    glGenVertexArrays(1, &gpu->vao);
  restore_context(previous);
  if (!gpu->program)
    goto failed;
  return gpu;
failed:
  if (gpu->context)
    g_object_unref(gpu->context);
  g_main_context_unref(gpu->main);
  g_free(gpu);
  return NULL;
}
void pax_linux_gpu_unref(PaxLinuxGpu *gpu) {
  if (!gpu || !g_atomic_int_dec_and_test(&gpu->references))
    return;
  GdkGLContext *previous = enter_context(gpu);
  if (gpu->program)
    glDeleteProgram(gpu->program);
  if (gpu->vao)
    glDeleteVertexArrays(1, &gpu->vao);
  restore_context(previous);
  g_object_unref(gpu->context);
  g_main_context_unref(gpu->main);
  g_free(gpu);
}
gboolean pax_linux_gpu_capacity(PaxLinuxGpu *gpu, JsonArray *surfaces,
                                GError **error) {
  if (gpu->failed) {
    g_set_error(error, G_IO_ERROR, G_IO_ERROR_FAILED,
                "Linux GL copy retirement failed; allocations retained");
    return FALSE;
  }
  guint count = json_array_get_length(surfaces);
  guint64 bytes = 0;
  for (guint i = 0; i < count; i++) {
    JsonObject *surface = json_array_get_object_element(surfaces, i);
    guint64 width = json_object_get_int_member(surface, "width"),
            height = json_object_get_int_member(surface, "height");
    if (width == 0 || height == 0 || width > 16384 || height > 16384) {
      g_set_error(error, G_IO_ERROR, G_IO_ERROR_INVALID_DATA,
                  "Linux surface dimensions are invalid");
      return FALSE;
    }
    bytes += width * height * 4;
  }
  // A retained frame stays visible while its complete replacement is built.
  // Admission must reserve that second set; otherwise a large current frame
  // could permanently backpressure its own replacement.
  if (count > MAX_GL_IMAGES / 2 || bytes > MAX_GL_BYTES / 2) {
    g_set_error(error, G_IO_ERROR, G_IO_ERROR_NO_SPACE,
                "Linux frame exceeds replacement headroom (24 images/128 MiB)");
    return FALSE;
  }
  return gpu->live + count <= MAX_GL_IMAGES &&
         gpu->bytes + bytes <= MAX_GL_BYTES;
}
guint pax_linux_gpu_live(PaxLinuxGpu *gpu) { return gpu ? gpu->live : 0; }
guint64 pax_linux_gpu_bytes(PaxLinuxGpu *gpu) { return gpu ? gpu->bytes : 0; }
GdkTexture *pax_linux_gpu_import(PaxLinuxGpu *gpu, JsonObject *frame,
                                 int *completion_fd, GError **error) {
  *completion_fd = -1;
  int fd = json_object_get_int_member(frame, "fd"),
      producer_fd = json_object_get_int_member(frame, "syncFd");
  gboolean producer_complete =
      json_object_has_member(frame, "producerComplete") &&
      json_object_get_boolean_member(frame, "producerComplete");
  JsonObject *surface = json_object_get_object_member(frame, "surface");
  guint width = json_object_get_int_member(surface, "width"),
        height = json_object_get_int_member(surface, "height");
  EGLImage image = EGL_NO_IMAGE;
  EGLSyncKHR producer = EGL_NO_SYNC_KHR, finished = EGL_NO_SYNC_KHR;
  GLuint external = 0, framebuffer = 0;
  GdkTexture *texture = NULL;
  ImageOwner *owner = g_new0(ImageOwner, 1);
  owner->gpu = gpu;
  owner->bytes = (guint64)width * height * 4;
  gboolean copied = FALSE;
  GdkGLContext *previous = enter_context(gpu);
  if (fd < 0 || (producer_complete ? producer_fd != -1 : producer_fd < 0) ||
      !width || !height || gpu->live >= MAX_GL_IMAGES ||
      gpu->bytes + owner->bytes > MAX_GL_BYTES) {
    g_set_error(error, G_IO_ERROR, G_IO_ERROR_NO_SPACE,
                "invalid or unreserved Linux GPU publication");
    goto done;
  }
  EGLint sync_attributes[] = {EGL_SYNC_NATIVE_FENCE_FD_ANDROID, producer_fd,
                              EGL_NONE};
  if (!producer_complete) {
    producer = eglCreateSyncKHR(gpu->display, EGL_SYNC_NATIVE_FENCE_ANDROID,
                                sync_attributes);
    producer_fd = -1;
    if (producer == EGL_NO_SYNC_KHR ||
        !eglWaitSyncKHR(gpu->display, producer, 0)) {
      g_set_error(error, G_IO_ERROR, G_IO_ERROR_FAILED,
                  "EGL producer fence import/wait failed: %x", eglGetError());
      goto done;
    }
  }
  guint64 modifier = json_object_get_int_member(frame, "modifier");
  EGLint attributes[] = {EGL_IMAGE_PRESERVED_KHR,
                         EGL_TRUE,
                         EGL_WIDTH,
                         width,
                         EGL_HEIGHT,
                         height,
                         EGL_LINUX_DRM_FOURCC_EXT,
                         json_object_get_int_member(frame, "fourcc"),
                         EGL_DMA_BUF_PLANE0_FD_EXT,
                         fd,
                         EGL_DMA_BUF_PLANE0_OFFSET_EXT,
                         json_object_get_int_member(frame, "offset"),
                         EGL_DMA_BUF_PLANE0_PITCH_EXT,
                         json_object_get_int_member(frame, "stride"),
                         EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT,
                         (EGLint)modifier,
                         EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT,
                         (EGLint)(modifier >> 32),
                         EGL_NONE};
  image = eglCreateImageKHR(gpu->display, EGL_NO_CONTEXT, EGL_LINUX_DMA_BUF_EXT,
                            NULL, attributes);
  if (image == EGL_NO_IMAGE) {
    g_set_error(error, G_IO_ERROR, G_IO_ERROR_FAILED,
                "EGL DMA-BUF import failed: %x", eglGetError());
    goto done;
  }
  while (glGetError() != GL_NO_ERROR) {
  }
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
  glTexStorage2D(GL_TEXTURE_2D, 1, GL_RGBA8, width, height);
  glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
  glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
  glGenFramebuffers(1, &framebuffer);
  glBindFramebuffer(GL_FRAMEBUFFER, framebuffer);
  glFramebufferTexture2D(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D,
                         owner->texture, 0);
  if (glCheckFramebufferStatus(GL_FRAMEBUFFER) != GL_FRAMEBUFFER_COMPLETE) {
    g_set_error(error, G_IO_ERROR, G_IO_ERROR_FAILED,
                "EGL copy framebuffer incomplete");
    goto done;
  }
  glUseProgram(gpu->program);
  glUniform1i(glGetUniformLocation(gpu->program, "source"), 0);
  glBindVertexArray(gpu->vao);
  glViewport(0, 0, width, height);
  glDisable(GL_BLEND);
  glDisable(GL_SCISSOR_TEST);
  glDrawArrays(GL_TRIANGLES, 0, 3);
  copied = TRUE;
  // The source pool can recycle after this fence, independently of GTK's
  // lifetime for the immutable owned GL output. This is a GPU/server wait.
  finished =
      eglCreateSyncKHR(gpu->display, EGL_SYNC_NATIVE_FENCE_ANDROID, NULL);
  glFlush();
  if (finished != EGL_NO_SYNC_KHR)
    *completion_fd = eglDupNativeFenceFDANDROID(gpu->display, finished);
  if (*completion_fd < 0) {
    g_set_error(error, G_IO_ERROR, G_IO_ERROR_FAILED,
                "EGL consumer fence export failed; source allocation must "
                "remain pinned");
    goto done;
  }
  GLenum status = glGetError();
  if (status != GL_NO_ERROR) {
    g_set_error(error, G_IO_ERROR, G_IO_ERROR_FAILED, "EGL copy failed: %x",
                status);
    goto done;
  }
  owner->sync = glFenceSync(GL_SYNC_GPU_COMMANDS_COMPLETE, 0);
  glFlush();
  if (!owner->sync) {
    g_set_error(error, G_IO_ERROR, G_IO_ERROR_FAILED,
                "GL publication fence failed");
    goto done;
  }
  GdkGLTextureBuilder *builder = gdk_gl_texture_builder_new();
  gdk_gl_texture_builder_set_context(builder, gpu->context);
  gdk_gl_texture_builder_set_id(builder, owner->texture);
  gdk_gl_texture_builder_set_width(builder, width);
  gdk_gl_texture_builder_set_height(builder, height);
  gdk_gl_texture_builder_set_sync(builder, owner->sync);
  gdk_gl_texture_builder_set_format(builder, GDK_MEMORY_R8G8B8A8_PREMULTIPLIED);
  texture = gdk_gl_texture_builder_build(builder, release_image, owner);
  g_object_unref(builder);
  if (texture) {
    g_atomic_int_inc(&gpu->references);
    gpu->live++;
    gpu->bytes += owner->bytes;
  } else {
    g_set_error(error, G_IO_ERROR, G_IO_ERROR_FAILED,
                "GDK rejected the immutable GL publication");
  }
done:
  if (framebuffer)
    glDeleteFramebuffers(1, &framebuffer);
  if (external)
    glDeleteTextures(1, &external);
  if (image != EGL_NO_IMAGE)
    eglDestroyImageKHR(gpu->display, image);
  if (producer != EGL_NO_SYNC_KHR)
    eglDestroySyncKHR(gpu->display, producer);
  if (finished != EGL_NO_SYNC_KHR)
    eglDestroySyncKHR(gpu->display, finished);
  if (fd >= 0)
    close(fd);
  if (producer_fd >= 0)
    close(producer_fd);
  if (!texture) {
    if (owner->sync)
      glDeleteSync(owner->sync);
    if (owner->texture)
      glDeleteTextures(1, &owner->texture);
    g_free(owner);
  }
  restore_context(previous);
  // -2 means GPU reads were queued but no valid completion fence was obtained.
  // The host must fail closed and retain the source; consumed:false is unsafe.
  if (copied && *completion_fd < 0)
    *completion_fd = -2;
  return texture;
}
