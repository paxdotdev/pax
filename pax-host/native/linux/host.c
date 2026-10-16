#define _GNU_SOURCE
#include "../common/pax-desktop.h"
#include "assets.h"
#include "gpu-import.h"
#include "native-scene.h"
#include <dlfcn.h>
#include <glib-unix.h>
#include <math.h>
#include <signal.h>
#include <string.h>
#include <sys/syscall.h>
#include <unistd.h>

typedef struct {
#define API(result, name, args) result(*name) args
  API(uint32_t, abi, (void));
  API(PaxDesktopEngine *, init,
      (float, float, const uint8_t *, size_t, const uint8_t *, size_t));
  API(void, free_buffer, (PaxDesktopBuffer));
  API(PaxDesktopBuffer, initialize_gpu, (PaxDesktopEngine *));
  API(PaxDesktopBuffer, tick, (PaxDesktopEngine *, float, float, float));
  API(PaxDesktopBuffer, plan, (PaxDesktopEngine *, uint32_t, float));
  API(PaxDesktopBuffer, set_surfaces,
      (PaxDesktopEngine *, const uint8_t *, size_t));
  API(PaxDesktopBuffer, render, (PaxDesktopEngine *));
  API(PaxDesktopBuffer, interrupt,
      (PaxDesktopEngine *, const uint8_t *, size_t));
  API(PaxDesktopBuffer, release,
      (PaxDesktopEngine *, uint64_t, const uint8_t *, size_t));
  API(PaxDesktopBuffer, poll, (PaxDesktopEngine *));
  API(void, activate, (PaxDesktopEngine *));
  API(PaxDesktopBuffer, load_image,
      (PaxDesktopEngine *, const uint8_t *, size_t, const uint8_t *, size_t,
       uint32_t, uint32_t));
  API(void, set_property_waker, (PaxDesktopEngine *, uint64_t, PaxDesktopWake));
  API(bool, has_pending_properties, (PaxDesktopEngine *));
  API(void, set_shutdown_waker, (uint64_t, PaxDesktopWake));
  API(uint64_t, pump_shutdowns, (void));
  API(void, request,
      (PaxDesktopEngine *, const uint8_t *, size_t, void *, PaxDesktopReply));
  API(void, begin_close, (PaxDesktopEngine *));
  API(bool, finish_close, (PaxDesktopEngine *));
#undef API
} Cartridge;
typedef struct {
  gint references;
  void *context;
  void (*notify)(void *, uint32_t);
  void (*release)(void *);
} Observer;
typedef struct Session {
  gint references;
  Cartridge api;
  PaxDesktopEngine *engine;
  PaxNativeScene *scene;
  GtkWindow *window;
  GtkWidget *parent;
  PaxLinuxGpu *gpu;
  GMainLoop *loop;
  GMainContext *main;
  guint tick_id, poll_id, diagnostic_id, signal_id;
  guint64 token, frames, blocked, ticks, changed_frames, clean_frames;
  guint test_retain_ms;
  guint32 status, delivered_status;
  gint64 last_close_report;
  Observer *observer;
  char *title, *registry;
  double width, height, min_width, min_height;
  gboolean resizable, failed, standalone, ever_mounted;
  JsonArray *surfaces, *pending_messages, *pending_owners;
  char **assets;
} Session;
static Session *active;
static char *configuration, *last_error;
static guint64 next_session_token = 1;
static GMainContext *ui_context;
static int fail(const char *message) {
  g_free(last_error);
  last_error = g_strdup(message);
  g_printerr("Pax Linux: %s\n", message);
  return -1;
}
static gboolean main_thread(void) { return syscall(SYS_gettid) == getpid(); }
static Session *session_ref(Session *session) {
  session->references++;
  return session;
}
static void observer_unref(Observer *observer) {
  if (observer && --observer->references == 0) {
    observer->release(observer->context);
    g_free(observer);
  }
}
static void session_unref(Session *session) {
  if (!session || --session->references)
    return;
  observer_unref(session->observer);
  if (session->surfaces)
    json_array_unref(session->surfaces);
  g_clear_pointer(&session->pending_messages, json_array_unref);
  g_clear_pointer(&session->pending_owners, json_array_unref);
  pax_linux_gpu_unref(session->gpu);
  if (session->window)
    g_object_unref(session->window);
  if (session->loop)
    g_main_loop_unref(session->loop);
  g_main_context_unref(session->main);
  g_strfreev(session->assets);
  g_free(session->title);
  g_free(session->registry);
  g_free(session);
}
typedef struct {
  Observer *observer;
  Session *session;
  guint32 status;
} Notice;
static gboolean notice_main(gpointer data) {
  Notice *notice = data;
  notice->observer->notify(notice->observer->context, notice->status);
  notice->session->delivered_status = notice->status;
  if (notice->status == 4 && notice->session->loop)
    g_main_loop_quit(notice->session->loop);
  return G_SOURCE_REMOVE;
}
static void notice_free(gpointer data) {
  Notice *notice = data;
  observer_unref(notice->observer);
  session_unref(notice->session);
  g_free(notice);
}
static void notify(Session *session) {
  if (!session->observer) {
    session->delivered_status = session->status;
    if (session->status == 4 && session->loop)
      g_main_loop_quit(session->loop);
    return;
  }
  Notice *notice = g_new0(Notice, 1);
  notice->session = session_ref(session);
  notice->observer = session->observer;
  notice->observer->references++;
  notice->status = session->status;
  GSource *source = g_idle_source_new();
  g_source_set_callback(source, notice_main, notice, notice_free);
  g_source_attach(source, session->main);
  g_source_unref(source);
}
static JsonObject *response_raw(Session *session, PaxDesktopBuffer buffer) {
  JsonParser *parser = json_parser_new();
  GError *error = NULL;
  JsonObject *object = NULL;
  if (buffer.data &&
      json_parser_load_from_data(parser, (const char *)buffer.data, buffer.len,
                                 &error) &&
      JSON_NODE_HOLDS_OBJECT(json_parser_get_root(parser)))
    object =
        json_object_ref(json_node_get_object(json_parser_get_root(parser)));
  session->api.free_buffer(buffer);
  g_object_unref(parser);
  if (!object) {
    fail(error ? error->message : "invalid desktop cartridge response");
    g_clear_error(&error);
    return NULL;
  }
  return object;
}
static JsonObject *response(Session *session, PaxDesktopBuffer buffer) {
  JsonObject *object = response_raw(session, buffer);
  if (!object)
    return NULL;
  if (!json_object_has_member(object, "ok") ||
      !json_object_get_boolean_member(object, "ok")) {
    fail(json_object_has_member(object, "error")
             ? json_object_get_string_member(object, "error")
             : "desktop operation failed");
    json_object_unref(object);
    return NULL;
  }
  return object;
}
static gboolean discard_response(Session *session, PaxDesktopBuffer buffer) {
  JsonObject *object = response(session, buffer);
  if (object)
    json_object_unref(object);
  return object != NULL;
}
static char *array_json(JsonArray *array) {
  JsonNode *node = json_node_new(JSON_NODE_ARRAY);
  json_node_set_array(node, array);
  char *text = json_to_string(node, FALSE);
  json_node_free(node);
  return text;
}
static void close_session(Session *session);
static gboolean close_idle(gpointer data) {
  close_session(data);
  return G_SOURCE_REMOVE;
}
static void schedule_close(Session *session) {
  GSource *source = g_idle_source_new();
  g_source_set_callback(source, close_idle, session_ref(session),
                        (GDestroyNotify)session_unref);
  g_source_attach(source, session->main);
  g_source_unref(source);
}
static void scene_error(gpointer data, const char *error) {
  Session *session = data;
  session->failed = TRUE;
  fail(error);
  schedule_close(session);
}
static gboolean scene_interrupt(gpointer data, const char *json, gsize length) {
  Session *session = data;
  if (session->status >= 3)
    return FALSE;
  JsonObject *result =
      response(session, session->api.interrupt(session->engine,
                                               (const uint8_t *)json, length));
  if (!result) {
    session->failed = TRUE;
    schedule_close(session);
    return FALSE;
  }
  gboolean prevent = json_object_get_boolean_member(result, "preventDefault");
  json_object_unref(result);
  return prevent;
}
static gboolean release_frame(Session *session, guint64 token, int completion) {
  if (completion == -2)
    return FALSE;
  char *json =
      completion >= 0
          ? g_strdup_printf("{\"consumed\":true,\"syncFd\":%d}", completion)
          : g_strdup("{\"consumed\":false}");
  gboolean ok = discard_response(
      session, session->api.release(session->engine, token,
                                    (const uint8_t *)json, strlen(json)));
  g_free(json);
  if (!ok && completion >= 0)
    close(completion);
  return ok;
}
static gboolean update_surfaces(Session *session, float dpr, guint layers) {
  JsonArray *surfaces = json_array_new();
  g_clear_pointer(&session->pending_owners, json_array_unref);
  session->pending_owners = json_array_new();
  for (guint layer = 0; layer < layers; layer++) {
    JsonObject *result =
        response(session, session->api.plan(session->engine, layer, dpr));
    if (!result) {
      json_array_unref(surfaces);
      return FALSE;
    }
    JsonNode *plan_node = json_object_get_member(result, "plan");
    JsonNode *owner = json_object_get_member(result, "owner");
    json_array_add_int_element(
        session->pending_owners,
        owner && !JSON_NODE_HOLDS_NULL(owner) ? json_node_get_int(owner) : -1);
    if (plan_node && !JSON_NODE_HOLDS_NULL(plan_node)) {
      JsonObject *plan = json_node_get_object(plan_node);
      if (json_object_get_boolean_member(plan, "active")) {
        JsonArray *tiles = json_object_get_array_member(plan, "surfaces");
        for (guint i = 0; i < json_array_get_length(tiles); i++) {
          JsonObject *tile = json_array_get_object_element(tiles, i);
          double width = json_object_get_double_member(tile, "width"),
                 height = json_object_get_double_member(tile, "height");
          if (width <= 0 || height <= 0)
            continue;
          JsonObject *surface = json_object_new();
          json_object_set_int_member(surface, "layer", layer);
          json_object_set_string_member(
              surface, "key", json_object_get_string_member(tile, "key"));
          char *signature = g_strdup_printf(
              "%s@%.6g:%u:%u",
              json_object_get_string_member(tile, "hostSignature"), dpr,
              (guint)ceil(width * dpr), (guint)ceil(height * dpr));
          json_object_set_string_member(surface, "hostSignature", signature);
          g_free(signature);
          json_object_set_double_member(
              surface, "originX", json_object_get_double_member(tile, "left"));
          json_object_set_double_member(
              surface, "originY", json_object_get_double_member(tile, "top"));
          json_object_set_double_member(surface, "logicalWidth", width);
          json_object_set_double_member(surface, "logicalHeight", height);
          json_object_set_int_member(surface, "width", ceil(width * dpr));
          json_object_set_int_member(surface, "height", ceil(height * dpr));
          JsonArray *scale = json_array_new();
          json_array_add_double_element(scale, dpr);
          json_array_add_double_element(scale, dpr);
          json_object_set_array_member(surface, "dpr", scale);
          json_object_set_int_member(
              surface, "replayPriority",
              json_object_get_int_member(tile, "replayPriority"));
          json_array_add_object_element(surfaces, surface);
        }
      }
    }
    json_object_unref(result);
  }
  char *registry = array_json(surfaces);
  gboolean ok = TRUE;
  if (g_strcmp0(registry, session->registry) != 0) {
    ok = discard_response(session,
                          session->api.set_surfaces(session->engine,
                                                    (const uint8_t *)registry,
                                                    strlen(registry)));
    if (ok) {
      g_free(session->registry);
      session->registry = g_strdup(registry);
    }
  }
  g_free(registry);
  if (session->surfaces)
    json_array_unref(session->surfaces);
  session->surfaces = surfaces;
  return ok;
}
static JsonArray *prepare_messages(Session *session, JsonArray *messages,
                                   GError **error) {
  JsonArray *native = json_array_new();
  for (guint i = 0; i < json_array_get_length(messages); i++) {
    JsonObject *message = json_array_get_object_element(messages, i);
    if (!json_object_has_member(message, "ImageLoad")) {
      json_array_add_object_element(native, json_object_ref(message));
      continue;
    }
    JsonObject *patch = json_object_get_object_member(message, "ImageLoad");
    const char *path = json_object_get_string_member(patch, "path");
    char *filename = pax_linux_asset_path(session->assets, path, error);
    if (!filename) {
      json_array_unref(native);
      return NULL;
    }
    GdkPixbuf *decoded = gdk_pixbuf_new_from_file(filename, error);
    g_free(filename);
    if (!decoded) {
      json_array_unref(native);
      return NULL;
    }
    GdkPixbuf *rgba = gdk_pixbuf_add_alpha(decoded, FALSE, 0, 0, 0);
    g_object_unref(decoded);
    int width = gdk_pixbuf_get_width(rgba),
        height = gdk_pixbuf_get_height(rgba),
        stride = gdk_pixbuf_get_rowstride(rgba);
    gsize size = (gsize)width * height * 4;
    if (width > 16384 || height > 16384 || size > 256 * 1024 * 1024) {
      g_object_unref(rgba);
      json_array_unref(native);
      g_set_error(error, G_IO_ERROR, G_IO_ERROR_NO_SPACE,
                  "Pax image exceeds native upload budget");
      return NULL;
    }
    guint8 *packed = g_malloc(size);
    const guint8 *pixels = gdk_pixbuf_read_pixels(rgba);
    for (int row = 0; row < height; row++)
      memcpy(packed + (gsize)row * width * 4, pixels + (gsize)row * stride,
             width * 4);
    gboolean ok = discard_response(
        session,
        session->api.load_image(session->engine, (const uint8_t *)path,
                                strlen(path), packed, size, width, height));
    g_free(packed);
    g_object_unref(rgba);
    if (!ok) {
      json_array_unref(native);
      return NULL;
    }
  }
  return native;
}
static gboolean release_test_reference(gpointer data) {
  (void)data;
  return G_SOURCE_REMOVE;
}
static void retain_for_test(Session *session, GdkTexture *texture) {
  if (!session->test_retain_ms)
    return;
  // Explicit test-only slow consumer: an ordinary GDK reference keeps the
  // actual logical quota charged. It supplies no GPU completion evidence.
  GSource *source = g_timeout_source_new(session->test_retain_ms);
  g_source_set_callback(source, release_test_reference, g_object_ref(texture),
                        g_object_unref);
  g_source_attach(source, session->main);
  g_source_unref(source);
}
static gboolean frame_tick(GtkWidget *widget, GdkFrameClock *clock,
                           gpointer data) {
  (void)clock;
  Session *session = data;
  if (session->status >= 3)
    return G_SOURCE_REMOVE;
  int width = gtk_widget_get_width(widget),
      height = gtk_widget_get_height(widget);
  float dpr = gtk_widget_get_scale_factor(widget);
  if (width <= 0 || height <= 0)
    return G_SOURCE_CONTINUE;
  GError *error = NULL;
  // Retain one engine tick until all of its surface generations are ready.
  // Advancing again under pressure would mix native geometry with older pixels.
  if (!session->pending_messages) {
    JsonObject *tick = response(
        session, session->api.tick(session->engine, width, height, dpr));
    if (!tick)
      goto failed;
    session->ticks++;
    session->pending_messages = prepare_messages(
        session, json_object_get_array_member(tick, "messages"), &error);
    guint layers = json_object_get_int_member(tick, "layerCount");
    json_object_unref(tick);
    if (!session->pending_messages || !update_surfaces(session, dpr, layers))
      goto failed;
  }
  if (!pax_linux_gpu_capacity(session->gpu, session->surfaces, &error)) {
    if (error)
      goto failed;
    session->blocked++;
    return G_SOURCE_CONTINUE;
  }
  JsonObject *render = response(session, session->api.render(session->engine));
  if (!render)
    goto failed;
  if (json_object_get_boolean_member(render, "blocked")) {
    session->blocked++;
    json_object_unref(render);
    return G_SOURCE_CONTINUE;
  }
  JsonNode *frame_node = json_object_get_member(render, "frame");
  JsonArray *frames = frame_node && !JSON_NODE_HOLDS_NULL(frame_node)
                          ? json_object_get_array_member(
                                json_node_get_object(frame_node), "frames")
                          : NULL;
  GPtrArray *textures = g_ptr_array_new_with_free_func(g_object_unref);
  gboolean imported = TRUE;
  for (guint i = 0; frames && i < json_array_get_length(frames); i++) {
    JsonObject *frame = json_array_get_object_element(frames, i);
    guint64 token = json_object_get_int_member(frame, "token");
    if (!imported) {
      close(json_object_get_int_member(frame, "fd"));
      int sync_fd = json_object_get_int_member(frame, "syncFd");
      if (sync_fd >= 0)
        close(sync_fd);
      release_frame(session, token, -1);
      continue;
    }
    int completion = -1;
    GdkTexture *texture =
        pax_linux_gpu_import(session->gpu, frame, &completion, &error);
    gboolean released = release_frame(session, token, completion);
    if (!texture || !released)
      imported = FALSE;
    if (texture) {
      retain_for_test(session, texture);
      g_ptr_array_add(textures, texture);
    }
  }
  if (!imported) {
    g_ptr_array_unref(textures);
    json_object_unref(render);
    goto failed;
  }
  // No main-context dispatch occurs within this commit. Native controls and
  // every changed GPU tile become visible in the same GTK frame.
  for (guint i = 0; i < json_array_get_length(session->pending_owners); i++)
    pax_native_scene_layer_owner(
        session->scene, i,
        json_array_get_int_element(session->pending_owners, i));
  if (!pax_native_scene_apply(session->scene, session->pending_messages,
                              &error)) {
    g_ptr_array_unref(textures);
    json_object_unref(render);
    goto failed;
  }
  pax_native_scene_prune_tiles(session->scene, session->surfaces);
  for (guint i = 0; frames && i < json_array_get_length(frames); i++)
    pax_native_scene_texture(
        session->scene,
        json_object_get_object_member(json_array_get_object_element(frames, i),
                                      "surface"),
        g_ptr_array_index(textures, i));
  g_ptr_array_unref(textures);
  gboolean changed = frames && json_array_get_length(frames);
  json_object_unref(render);
  g_clear_pointer(&session->pending_messages, json_array_unref);
  g_clear_pointer(&session->pending_owners, json_array_unref);
  session->frames++;
  if (changed)
    session->changed_frames++;
  else
    session->clean_frames++;
  if (session->status == 1) {
    session->api.activate(session->engine);
    session->status = 2;
    notify(session);
  }
  return G_SOURCE_CONTINUE;
failed:
  if (error) {
    fail(error->message);
    g_clear_error(&error);
  }
  session->failed = TRUE;
  session->tick_id = 0;
  close_session(session);
  return G_SOURCE_REMOVE;
}
static gboolean diagnostic_report(gpointer data) {
  Session *session = data;
  if (session->status >= 3 || !session->scene) {
    session->diagnostic_id = 0;
    return G_SOURCE_REMOVE;
  }
  const char *path = g_getenv("PAX_NATIVE_TEST_REPORT");
  if (!path)
    return G_SOURCE_REMOVE;
  JsonObject *report = pax_native_scene_report(session->scene);
  json_object_set_int_member(report, "frames", session->frames);
  json_object_set_int_member(report, "ticks", session->ticks);
  json_object_set_int_member(report, "changedFrames", session->changed_frames);
  json_object_set_int_member(report, "cleanFrames", session->clean_frames);
  json_object_set_boolean_member(report, "pendingCommit",
                                 session->pending_messages != NULL);
  json_object_set_int_member(report, "backpressure", session->blocked);
  json_object_set_int_member(report, "glLiveImages",
                             pax_linux_gpu_live(session->gpu));
  json_object_set_int_member(report, "glLiveBytes",
                             pax_linux_gpu_bytes(session->gpu));
  JsonObject *gpu = response_raw(session, session->api.poll(session->engine));
  if (gpu)
    json_object_set_object_member(report, "runtime", gpu);
  JsonNode *root = json_node_new(JSON_NODE_OBJECT);
  json_node_take_object(root, report);
  char *json = json_to_string(root, FALSE);
  g_file_set_contents(path, json, -1, NULL);
  g_free(json);
  json_node_free(root);
  // Explicit test requests only: normal frame delivery never downloads pixels.
  const char *capture = g_getenv("PAX_NATIVE_TEST_SCREENSHOT");
  if (capture) {
    char *request = g_strconcat(capture, ".request", NULL);
    if (g_file_test(request, G_FILE_TEST_IS_REGULAR)) {
      GtkWidget *widget = GTK_WIDGET(session->scene);
      GdkPaintable *paintable = gtk_widget_paintable_new(widget);
      GtkSnapshot *snapshot = gtk_snapshot_new();
      int width = gtk_widget_get_width(widget),
          height = gtk_widget_get_height(widget);
      gdk_paintable_snapshot(paintable, GDK_SNAPSHOT(snapshot), width, height);
      GskRenderNode *node = gtk_snapshot_free_to_node(snapshot);
      if (node) {
        GskRenderer *renderer =
            gtk_native_get_renderer(gtk_widget_get_native(widget));
        graphene_rect_t bounds = GRAPHENE_RECT_INIT(0, 0, width, height);
        GdkTexture *texture =
            gsk_renderer_render_texture(renderer, node, &bounds);
        if (texture && gdk_texture_save_to_png(texture, capture))
          unlink(request);
        g_clear_object(&texture);
        gsk_render_node_unref(node);
      }
      g_object_unref(paintable);
    }
    g_free(request);
  }
  return G_SOURCE_CONTINUE;
}
static gboolean terminate_session(gpointer data) {
  close_session(data);
  return G_SOURCE_CONTINUE;
}
static gboolean poll_session(gpointer data) {
  Session *session = data;
  if (session->engine) {
    JsonObject *result =
        response_raw(session, session->api.poll(session->engine));
    if (!result) {
      session->failed = TRUE;
      if (session->status < 3)
        close_session(session);
      return G_SOURCE_CONTINUE;
    }
    if (!json_object_get_boolean_member(result, "ok")) {
      session->failed = TRUE;
      const char *message = json_object_get_string_member(result, "error");
      if (message)
        fail(message);
      if (session->status < 3)
        close_session(session);
    }
    gboolean drained = json_object_get_boolean_member(result, "drained");
    json_object_unref(result);
    if (session->status >= 3 && drained &&
        session->api.finish_close(session->engine))
      session->engine = NULL;
  }
  const char *report_path = g_getenv("PAX_NATIVE_TEST_REPORT");
  gint64 now = g_get_monotonic_time();
  if (report_path && session->status >= 3 &&
      now - session->last_close_report > 250 * G_TIME_SPAN_MILLISECOND) {
    session->last_close_report = now;
    JsonObject *report = json_object_new();
    json_object_set_boolean_member(report, "closing", TRUE);
    json_object_set_boolean_member(report, "engineRetired", !session->engine);
    json_object_set_int_member(report, "glLiveImages",
                               pax_linux_gpu_live(session->gpu));
    json_object_set_int_member(report, "glLiveBytes",
                               pax_linux_gpu_bytes(session->gpu));
    if (session->engine) {
      JsonObject *runtime =
          response_raw(session, session->api.poll(session->engine));
      if (runtime)
        json_object_set_object_member(report, "runtime", runtime);
    }
    JsonNode *root = json_node_new(JSON_NODE_OBJECT);
    json_node_take_object(root, report);
    char *json = json_to_string(root, FALSE);
    g_file_set_contents(report_path, json, -1, NULL);
    g_free(json);
    json_node_free(root);
  }
  guint64 shutdowns = session->api.pump_shutdowns();
  if (session->status >= 3 && !session->engine && shutdowns == 0 &&
      (!session->standalone || pax_linux_gpu_live(session->gpu) == 0)) {
    g_print("PAX_LINUX_CLOSED frames=%" G_GUINT64_FORMAT
            " backpressure=%" G_GUINT64_FORMAT
            " gl_refs=%u gl_bytes=%" G_GUINT64_FORMAT "\n",
            session->frames, session->blocked, pax_linux_gpu_live(session->gpu),
            pax_linux_gpu_bytes(session->gpu));
    session->status = 4;
    notify(session);
    session->poll_id = 0;
    if (active == session) {
      active = NULL;
      session_unref(session);
    }
    return G_SOURCE_REMOVE;
  }
  return G_SOURCE_CONTINUE;
}
static gboolean wake_main(gpointer data) {
  guint64 token = *(guint64 *)data;
  if (active && active->token == token && active->status < 3 && active->scene)
    gtk_widget_queue_draw(GTK_WIDGET(active->scene));
  return G_SOURCE_REMOVE;
}
static void engine_wake(uint64_t token) {
  if (!ui_context)
    return;
  guint64 *copy = g_new(guint64, 1);
  *copy = token;
  GSource *source = g_idle_source_new();
  g_source_set_callback(source, wake_main, copy, g_free);
  g_source_attach(source, ui_context);
  g_source_unref(source);
}
static gboolean load_cartridge(Session *session, const char *path) {
  void *library = dlopen(path, RTLD_NOW | RTLD_LOCAL);
  if (!library) {
    fail(dlerror());
    return FALSE;
  }
  // Process-pinned: runtime TLS, callbacks and retained GL owners may outlive a
  // mount.
#define LOAD(field, symbol)                                                    \
  do {                                                                         \
    *(void **)(&session->api.field) = dlsym(library, symbol);                  \
    if (!session->api.field) {                                                 \
      fail("Missing desktop cartridge symbol " symbol);                        \
      return FALSE;                                                            \
    }                                                                          \
  } while (0)
  LOAD(abi, "pax_cartridge_abi_version");
  LOAD(init, "pax_init_entry");
  LOAD(free_buffer, "pax_desktop_free_buffer");
  LOAD(initialize_gpu, "pax_desktop_initialize_gpu");
  LOAD(tick, "pax_desktop_tick");
  LOAD(plan, "pax_desktop_plan");
  LOAD(set_surfaces, "pax_desktop_set_surfaces");
  LOAD(render, "pax_desktop_render");
  LOAD(interrupt, "pax_desktop_interrupt");
  LOAD(release, "pax_desktop_release");
  LOAD(poll, "pax_desktop_poll");
  LOAD(activate, "pax_desktop_activate");
  LOAD(load_image, "pax_desktop_load_image");
  LOAD(set_property_waker, "pax_desktop_set_property_waker");
  LOAD(has_pending_properties, "pax_desktop_has_pending_properties");
  LOAD(set_shutdown_waker, "pax_desktop_set_shutdown_waker");
  LOAD(pump_shutdowns, "pax_desktop_pump_shutdowns");
  LOAD(request, "pax_desktop_request");
  LOAD(begin_close, "pax_desktop_begin_close");
  LOAD(finish_close, "pax_desktop_finish_close");
#undef LOAD
  if (session->api.abi() != 4) {
    fail("Incompatible desktop cartridge ABI (expected 4)");
    return FALSE;
  }
  return TRUE;
}
__attribute__((visibility("default"))) uint32_t pax_host_version(void) {
  return 2;
}
__attribute__((visibility("default"))) int32_t
pax_host_configure(const uint8_t *bytes, size_t length) {
  if (!main_thread() || active)
    return fail("Configure Linux Pax on the main thread after the previous "
                "mount closes");
  if (!bytes || length > 1048576)
    return fail("Invalid Linux host configuration");
  JsonParser *parser = json_parser_new();
  GError *error = NULL;
  gboolean valid =
      json_parser_load_from_data(parser, (const char *)bytes, length, &error) &&
      JSON_NODE_HOLDS_OBJECT(json_parser_get_root(parser));
  if (!valid) {
    fail(error ? error->message : "Invalid Linux host configuration");
    g_clear_error(&error);
    g_object_unref(parser);
    return -1;
  }
  g_free(configuration);
  configuration = g_strndup((const char *)bytes, length);
  g_object_unref(parser);
  return 0;
}
__attribute__((visibility("default"))) void *
pax_host_prepare_with_data(const uint8_t *data, size_t length) {
  if (!main_thread() || active || length > 65536 || (!data && length)) {
    fail("Prepare Linux Pax once on its main thread with at most 64 KiB "
         "startup data");
    return NULL;
  }
  // GTK4.14's legacy GL renderer misrecords external sampler bindings. The
  // supported EGL path is qualified with the new GL renderer explicitly.
  if (!g_getenv("GSK_RENDERER"))
    g_setenv("GSK_RENDERER", "ngl", FALSE);
  if (g_strcmp0(g_getenv("GSK_RENDERER"), "ngl") != 0) {
    fail("Linux native Pax currently requires GSK_RENDERER=ngl");
    return NULL;
  }
  if (!gtk_init_check()) {
    fail("GTK could not open the Linux display");
    return NULL;
  }
  JsonParser *parser = json_parser_new();
  GError *error = NULL;
  if (!configuration ||
      !json_parser_load_from_data(parser, configuration, -1, &error)) {
    fail("Linux host was not configured");
    g_clear_error(&error);
    g_object_unref(parser);
    return NULL;
  }
  JsonObject *config = json_node_get_object(json_parser_get_root(parser));
  if (!json_object_has_member(config, "cartridge")) {
    fail("Linux host configuration needs a cartridge library path");
    g_object_unref(parser);
    return NULL;
  }
  Session *session = g_new0(Session, 1);
  session->references = 1;
  session->width = 900;
  session->height = 600;
  session->resizable = TRUE;
  if (g_getenv("PAX_NATIVE_TEST_REPORT")) {
    const char *hold = g_getenv("PAX_NATIVE_TEST_RETAIN_MS");
    if (hold) {
      char *end = NULL;
      guint64 duration = g_ascii_strtoull(hold, &end, 10);
      if (!*hold || !end || *end || duration > 5000) {
        fail(
            "Diagnostic GDK retention must be between 0 and 5000 milliseconds");
        g_object_unref(parser);
        g_free(session);
        return NULL;
      }
      session->test_retain_ms = duration;
    }
  }
  session->main = g_main_context_ref_thread_default();
  session->token = next_session_token++;
  if (!ui_context)
    ui_context = g_main_context_ref(session->main);
  if (!load_cartridge(session,
                      json_object_get_string_member(config, "cartridge"))) {
    g_object_unref(parser);
    session_unref(session);
    return NULL;
  }
  const char *entry = json_object_has_member(config, "entry")
                          ? json_object_get_string_member(config, "entry")
                          : "";
  session->title = g_strdup(json_object_has_member(config, "title")
                                ? json_object_get_string_member(config, "title")
                                : "Pax");
  if (json_object_has_member(config, "assets")) {
    JsonArray *roots = json_object_get_array_member(config, "assets");
    session->assets = g_new0(char *, json_array_get_length(roots) + 1);
    for (guint i = 0; i < json_array_get_length(roots); i++)
      session->assets[i] = g_strdup(json_array_get_string_element(roots, i));
  }
  session->engine = session->api.init(0, 0, (const uint8_t *)entry,
                                      strlen(entry), data, length);
  g_object_unref(parser);
  if (!session->engine) {
    fail("Desktop cartridge initializer failed");
    session_unref(session);
    return NULL;
  }
  active = session_ref(session);
  session->api.set_property_waker(session->engine, session->token, engine_wake);
  session->api.set_shutdown_waker(session->token, engine_wake);
  GSource *poll = g_timeout_source_new(8);
  g_source_set_callback(poll, poll_session, session_ref(session),
                        (GDestroyNotify)session_unref);
  session->poll_id = g_source_attach(poll, session->main);
  g_source_unref(poll);
  return session;
}
__attribute__((visibility("default"))) int32_t pax_host_window_options(
    void *handle, const uint8_t *title, size_t length, double width,
    double height, double min_width, double min_height, bool resizable) {
  Session *session = handle;
  if (!main_thread() || session != active || session->status != 0 ||
      length > 65536 || !isfinite(width) || !isfinite(height) || width <= 0 ||
      height <= 0 || min_width < 0 || min_height < 0)
    return fail("Invalid Linux window options");
  if (title && length) {
    g_free(session->title);
    session->title = g_strndup((const char *)title, length);
  }
  session->width = width;
  session->height = height;
  session->min_width = min_width;
  session->min_height = min_height;
  session->resizable = resizable;
  return 0;
}
static int mount_session(Session *session, GtkWidget *parent) {
  if (!main_thread() || session != active || session->status != 0)
    return fail("Prepared Linux Pax must be mounted once on the main thread");
  GError *error = NULL;
  session->gpu = pax_linux_gpu_new(gtk_widget_get_display(parent), &error);
  if (!session->gpu) {
    int result = fail(error ? error->message : "Linux GPU import unavailable");
    g_clear_error(&error);
    return result;
  }
  if (!discard_response(session, session->api.initialize_gpu(session->engine)))
    return -1;
  GtkWidget *scene =
      pax_native_scene_new(scene_interrupt, scene_error, session);
  session->scene = (PaxNativeScene *)scene;
  pax_native_scene_assets(session->scene, session->assets);
  session->parent = parent;
  gtk_widget_set_hexpand(scene, TRUE);
  gtk_widget_set_vexpand(scene, TRUE);
  if (GTK_IS_BOX(parent))
    gtk_box_append(GTK_BOX(parent), scene);
  else if (GTK_IS_WINDOW(parent) && !gtk_window_get_child(GTK_WINDOW(parent)))
    gtk_window_set_child(GTK_WINDOW(parent), scene);
  else {
    g_object_ref_sink(scene);
    g_object_unref(scene);
    session->scene = NULL;
    return fail("Linux mount requires a GtkBox or an empty GtkWindow");
  }
  session->ever_mounted = TRUE;
  session->status = 1;
  if (g_getenv("PAX_NATIVE_TEST_REPORT"))
    session->diagnostic_id = g_timeout_add(250, diagnostic_report, session);
  notify(session);
  session->tick_id =
      gtk_widget_add_tick_callback(scene, frame_tick, session, NULL);
  return 0;
}
__attribute__((visibility("default"))) int32_t
pax_host_mount(void *handle, void *parent, void *controller) {
  (void)controller;
  if (!GTK_IS_WIDGET(parent))
    return fail("Invalid GTK mount container");
  return mount_session(handle, parent);
}
static gboolean window_close(GtkWindow *window, gpointer data) {
  (void)window;
  close_session(data);
  return TRUE;
}
__attribute__((visibility("default"))) int32_t pax_host_run(void *handle) {
  Session *session = handle;
  if (!main_thread() || session != active || session->status != 0)
    return fail("Run Linux Pax on its main thread");
  session->standalone = TRUE;
  session->window = GTK_WINDOW(gtk_window_new());
  g_object_ref_sink(session->window);
  gtk_window_set_title(session->window, session->title);
  gtk_window_set_default_size(session->window, session->width, session->height);
  gtk_window_set_resizable(session->window, session->resizable);
  gtk_widget_set_size_request(GTK_WIDGET(session->window), session->min_width,
                              session->min_height);
  g_signal_connect(session->window, "close-request", G_CALLBACK(window_close),
                   session);
  if (mount_session(session, GTK_WIDGET(session->window)) != 0) {
    close_session(session);
    return -1;
  }
  session->signal_id = g_unix_signal_add(SIGTERM, terminate_session, session);
  session->loop = g_main_loop_new(session->main, FALSE);
  gtk_window_present(session->window);
  g_main_loop_run(session->loop);
  return session->failed ? -1 : 0;
}
static void close_session(Session *session) {
  if (!session || session->status >= 3)
    return;
  session->status = 3;
  g_clear_pointer(&session->pending_messages, json_array_unref);
  g_clear_pointer(&session->pending_owners, json_array_unref);
  if (session->diagnostic_id) {
    g_source_remove(session->diagnostic_id);
    session->diagnostic_id = 0;
  }
  if (session->signal_id) {
    g_source_remove(session->signal_id);
    session->signal_id = 0;
  }
  notify(session);
  session->api.begin_close(session->engine);
  if (session->scene) {
    if (session->tick_id)
      gtk_widget_remove_tick_callback(GTK_WIDGET(session->scene),
                                      session->tick_id);
    session->tick_id = 0;
    pax_native_scene_stop(session->scene);
    if (GTK_IS_WINDOW(session->parent))
      gtk_window_set_child(GTK_WINDOW(session->parent), NULL);
    else if (GTK_IS_BOX(session->parent))
      gtk_box_remove(GTK_BOX(session->parent), GTK_WIDGET(session->scene));
    session->scene = NULL;
  }
  if (session->window) {
    // destroy drops GTK's internal toplevel reference. Our own reference must
    // also retire now, so the renderer can release its final texture cache.
    gtk_window_destroy(session->window);
    g_clear_object(&session->window);
  }
  session->parent = NULL;
}
__attribute__((visibility("default"))) void pax_host_close(void *handle) {
  if (!main_thread())
    return;
  Session *session = handle;
  if (!session)
    return;
  gboolean prelaunch = !session->ever_mounted && !session->loop;
  session_ref(session);
  close_session(session);
  if (prelaunch) {
    // Prepared values have no running GTK loop. Drive acknowledged shutdown on
    // its owner thread, without treating a timeout as permission to destroy it.
    gint64 deadline = g_get_monotonic_time() + 15 * G_TIME_SPAN_SECOND;
    while (session->delivered_status != 4 &&
           g_get_monotonic_time() < deadline) {
      if (!g_main_context_iteration(session->main, FALSE))
        g_usleep(1000);
    }
    if (session->delivered_status != 4) {
      session->failed = TRUE;
      fail("Prepared Linux shutdown did not drain within 15 seconds; ownership "
           "is retained and the GTK main context must continue pumping");
    }
  }
  session_unref(session);
}
__attribute__((visibility("default"))) void pax_host_release(void *handle) {
  if (main_thread())
    session_unref(handle);
}
__attribute__((visibility("default"))) void
pax_host_observe(void *handle, void *context,
                 void (*callback)(void *, uint32_t), void (*release)(void *)) {
  Session *session = handle;
  Observer *observer = g_new0(Observer, 1);
  observer->references = 1;
  observer->context = context;
  observer->notify = callback;
  observer->release = release;
  observer_unref(session->observer);
  session->observer = observer;
  notify(session);
}
typedef struct {
  void *context;
  PaxDesktopReply reply;
  uint32_t status;
  GBytes *bytes;
} Reply;
static gboolean reply_main(gpointer data) {
  Reply *reply = data;
  gsize length;
  const guint8 *bytes = g_bytes_get_data(reply->bytes, &length);
  reply->reply(reply->context, reply->status, bytes, length);
  return G_SOURCE_REMOVE;
}
static void reply_free(gpointer data) {
  Reply *reply = data;
  g_bytes_unref(reply->bytes);
  g_free(reply);
}
static void reply_received(void *context, uint32_t status, const uint8_t *data,
                           size_t length) {
  Reply *reply = context;
  reply->status = status;
  reply->bytes = g_bytes_new(data, length);
  GSource *source = g_idle_source_new();
  g_source_set_callback(source, reply_main, reply, reply_free);
  g_source_attach(source, ui_context);
  g_source_unref(source);
}
__attribute__((visibility("default"))) void
pax_host_request(void *handle, const uint8_t *data, size_t length,
                 void *context, PaxDesktopReply callback) {
  Session *session = handle;
  Reply *reply = g_new0(Reply, 1);
  reply->context = context;
  reply->reply = callback;
  if (!main_thread() || session->status >= 3 || length > 65536) {
    const char *error = "application is closed or request exceeds 64 KiB";
    reply_received(reply, 1, (const uint8_t *)error, strlen(error));
    return;
  }
  session->api.request(session->engine, data, length, reply, reply_received);
}
__attribute__((visibility("default"))) size_t pax_host_error(uint8_t *buffer,
                                                             size_t capacity) {
  const char *error =
      last_error ? last_error : "Pax Linux host operation failed";
  size_t length = strlen(error);
  if (buffer && capacity)
    memcpy(buffer, error, MIN(capacity, length));
  return length;
}
