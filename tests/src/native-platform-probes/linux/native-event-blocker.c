/* EventBlocker final pixels and real X11 input. Build like text-constraints.c;
 * additionally link pkg-config x11. Run with GDK_BACKEND=x11 and xdotool on PATH.
 * Use GSK_RENDERER=ngl; Cairo cannot render this scene's general-matrix nodes.
 * Private Xvfb with LIBGL_ALWAYS_SOFTWARE=1 qualifies GTK behavior, not GPU.
 */
#include "native-scene.h"
#include <gdk/x11/gdkx.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
  PaxNativeScene *scene;
  guint checks, failures, down, up, clicks, covered_clicks, exposed_clicks;
} Test;
typedef struct { gboolean done; gint status; } Input;

static void check(Test *test, gboolean ok, const char *name) {
  test->checks++; test->failures += !ok;
  g_print("%s %s\n", ok ? "PASS" : "FAIL", name);
}
static void scene_error(gpointer data, const char *message) {
  Test *test = data; test->failures++;
  g_printerr("native scene error: %s\n", message);
}
static gboolean interrupt(gpointer data, const char *text, gsize length) {
  Test *test = data; JsonParser *parser = json_parser_new(); GError *error = NULL;
  gboolean ok = json_parser_load_from_data(parser, text, (gssize)length, &error);
  if (ok) {
    JsonObject *root = json_node_get_object(json_parser_get_root(parser));
    test->down += json_object_has_member(root, "MouseDown");
    test->up += json_object_has_member(root, "MouseUp");
    test->clicks += json_object_has_member(root, "Click");
    if (json_object_has_member(root, "FormButtonClick")) {
      gint64 id = json_object_get_int_member(json_object_get_object_member(root, "FormButtonClick"), "id");
      if (id == 10) test->covered_clicks++;
      else if (id == 11) test->exposed_clicks++;
      else scene_error(test, "unexpected button identity");
    }
  } else { scene_error(test, error->message); g_error_free(error); }
  g_object_unref(parser); return ok;
}
static void pump(guint milliseconds) {
  gint64 end = g_get_monotonic_time() + milliseconds * 1000;
  do {
    for (guint i = 0; i < 32 && g_main_context_pending(NULL); i++)
      g_main_context_iteration(NULL, FALSE);
    g_usleep(1000);
  } while (g_get_monotonic_time() < end);
}
static gboolean apply(Test *test, const char *source) {
  JsonParser *parser = json_parser_new(); GError *error = NULL;
  gboolean ok = json_parser_load_from_data(parser, source, -1, &error);
  if (ok) ok = pax_native_scene_apply(test->scene,
      json_node_get_array(json_parser_get_root(parser)), &error);
  if (!ok) { scene_error(test, error ? error->message : "patch rejected"); g_clear_error(&error); }
  g_object_unref(parser); pump(100); return ok;
}
static guint widget_count(GtkWidget *widget) {
  guint count = 1;
  for (GtkWidget *child = gtk_widget_get_first_child(widget); child;
       child = gtk_widget_get_next_sibling(child)) count += widget_count(child);
  return count;
}
static gboolean pixel(GtkWidget *scene, int x, int y, int r, int g, int b, int a) {
  GtkSnapshot *snapshot = gtk_snapshot_new();
  GdkPaintable *paintable = gtk_widget_paintable_new(scene);
  int width = gtk_widget_get_width(scene), height = gtk_widget_get_height(scene);
  gdk_paintable_snapshot(paintable, GDK_SNAPSHOT(snapshot), width, height);
  g_object_unref(paintable);
  GskRenderNode *node = gtk_snapshot_free_to_node(snapshot);
  if (!node) return FALSE;
  graphene_rect_t rect = GRAPHENE_RECT_INIT(0, 0, width, height);
  GdkTexture *texture = gsk_renderer_render_texture(
      gtk_native_get_renderer(gtk_widget_get_native(scene)), node, &rect);
  gsk_render_node_unref(node);
  if (!texture) return FALSE;
  gsize stride = (gsize)gdk_texture_get_width(texture) * 4;
  guchar *pixels = g_malloc(stride * gdk_texture_get_height(texture));
  gdk_texture_download(texture, pixels, stride);
  guchar *p = pixels + y * stride + x * 4;
#if G_BYTE_ORDER == G_LITTLE_ENDIAN
  int red = p[2], green = p[1], blue = p[0], alpha = p[3];
#else
  int red = p[1], green = p[2], blue = p[3], alpha = p[0];
#endif
  g_print("pixel %d,%d premultiplied RGBA=%d,%d,%d,%d expected=%d,%d,%d,%d\n",
      x, y, red, green, blue, alpha, r, g, b, a);
  gboolean ok = abs(red-r) <= 3 && abs(green-g) <= 3 && abs(blue-b) <= 3 && abs(alpha-a) <= 3;
  g_free(pixels); g_object_unref(texture); return ok;
}
static void input_done(GPid pid, gint status, gpointer data) {
  Input *input = data; input->status = status; input->done = TRUE; g_spawn_close_pid(pid);
}
static void click(Test *test, GtkWidget *window, GtkWidget *scene, int x, int y) {
  graphene_point_t from = GRAPHENE_POINT_INIT(x, y), point;
  if (!gtk_widget_compute_point(scene, window, &from, &point)) {
    check(test, FALSE, "input point resolves to native window"); return;
  }
  Window xid = gdk_x11_surface_get_xid(gtk_native_get_surface(GTK_NATIVE(window)));
  int scale = gtk_widget_get_scale_factor(window);
  // Focus/pointer may already be at the requested target. Do not wait for a new
  // focus/motion notification; require the actual press/release delivery below.
  char *command = g_strdup_printf("timeout 5s xdotool windowfocus %lu mousemove --window %lu %d %d sleep 0.05 click 1",
      (unsigned long)xid, (unsigned long)xid, (int)round(point.x*scale), (int)round(point.y*scale));
  gchar **argv = NULL; GError *error = NULL; GPid pid; Input result = {0};
  guint down = test->down, up = test->up, clicks = test->clicks;
  gboolean ok = g_shell_parse_argv(command, NULL, &argv, &error);
  if (ok) ok = g_spawn_async(NULL, argv, NULL, G_SPAWN_SEARCH_PATH | G_SPAWN_DO_NOT_REAP_CHILD,
      NULL, NULL, &pid, &error);
  if (ok) {
    g_child_watch_add(pid, input_done, &result);
    while (!result.done) pump(5);
    ok = g_spawn_check_wait_status(result.status, &error);
  }
  check(test, ok, "real X11 click completed");
  if (error) { g_printerr("%s\n", error->message); g_error_free(error); }
  g_strfreev(argv); g_free(command); pump(150);
  check(test, test->down == down+1 && test->up == up+1 && test->clicks == clicks+1,
      "blocker preserves one engine pointer press/release/click sequence");
}
int main(void) {
  gtk_init(); Test test = {0};
  GtkWidget *window = gtk_window_new(); g_object_ref_sink(window);
  GtkWidget *scene = pax_native_scene_new(interrupt, scene_error, &test);
  test.scene = (PaxNativeScene *)scene;
  gtk_window_set_title(GTK_WINDOW(window), "Pax EventBlocker background regression");
  gtk_window_set_default_size(GTK_WINDOW(window), 360, 180);
  gtk_window_set_child(GTK_WINDOW(window), scene); gtk_window_present(GTK_WINDOW(window));
  if (!GDK_IS_X11_DISPLAY(gtk_widget_get_display(window))) {
    check(&test, FALSE, "probe requires X11 input backend"); goto done;
  }
  if (!apply(&test,
      "[{\"ButtonCreate\":{\"id\":10}}, {\"ButtonCreate\":{\"id\":11}},"
      "{\"EventBlockerCreate\":{\"id\":20}},"
      "{\"ButtonUpdate\":{\"id\":10,\"content\":\"\",\"size_x\":100,\"size_y\":60,\"transform\":[1,0,0,1,20,20],\"color\":{\"Rgb\":[0,0,0]},\"hover_color\":{\"Rgb\":[0,0,0]},\"corner_radius\":0,\"outline_stroke_width\":0,\"z_index\":0}},"
      "{\"ButtonUpdate\":{\"id\":11,\"content\":\"Exposed\",\"size_x\":100,\"size_y\":60,\"transform\":[1,0,0,1,220,20],\"z_index\":0}},"
      "{\"EventBlockerUpdate\":{\"id\":20,\"size_x\":160,\"size_y\":100,\"transform\":[1,0,0,1,20,20],\"z_index\":1,\"background\":{\"Rgb\":[0.2,0.4,0.6]}}}]")) goto done;
  pump(300);
  GtkWidget *blocker = gtk_widget_pick(scene, 60, 50, GTK_PICK_DEFAULT);
  check(&test, GTK_IS_BOX(blocker), "native EventBlocker owns overlapping button hit");
  check(&test, pixel(scene, 150, 100, 51, 102, 153, 255), "initial authored opaque background paints native pixels");
  click(&test, window, scene, 60, 50);
  check(&test, test.covered_clicks == 0, "opaque blocker prevents underlying native button activation");
  click(&test, window, scene, 270, 50);
  check(&test, test.exposed_clicks == 1, "neighboring uncovered native button remains interactive");
  apply(&test, "[{\"EventBlockerUpdate\":{\"id\":20,\"background\":{\"Rgb\":[0.8,0.2,0.1]}}}]");
  check(&test, pixel(scene, 150, 100, 204, 51, 26, 255), "reactive solid background replaces prior color");
  apply(&test, "[{\"EventBlockerUpdate\":{\"id\":20,\"size_x\":160}}]");
  check(&test, pixel(scene, 150, 100, 204, 51, 26, 255), "omitted background preserves prior color");
  apply(&test, "[{\"EventBlockerUpdate\":{\"id\":20,\"background\":null}}]");
  check(&test, pixel(scene, 150, 100, 204, 51, 26, 255), "serialized absent background preserves prior color");
  apply(&test, "[{\"EventBlockerUpdate\":{\"id\":20,\"background\":{\"Rgba\":[0.2,0.4,0.6,0.5]}}}]");
  check(&test, pixel(scene, 150, 100, 26, 51, 77, 128), "authored background alpha applies exactly once");
  apply(&test, "[{\"EventBlockerUpdate\":{\"id\":20,\"opacity\":0.5}}]");
  check(&test, pixel(scene, 150, 100, 13, 26, 38, 64), "background alpha and node opacity each apply once");
  apply(&test, "[{\"EventBlockerUpdate\":{\"id\":20,\"opacity\":1,\"background\":{\"Rgba\":[0.2,0.4,0.6,0]}}}]");
  check(&test, pixel(scene, 150, 100, 0, 0, 0, 0), "fully transparent update clears old paint");
  check(&test, gtk_widget_pick(scene, 60, 50, GTK_PICK_DEFAULT) == blocker,
      "transparent background retains blocker identity and hit ownership");
  click(&test, window, scene, 60, 50);
  check(&test, test.covered_clicks == 0, "transparent blocker still prevents underlying native button activation");
  apply(&test, "[{\"EventBlockerUpdate\":{\"id\":20,\"background\":{\"Rgb\":[0.2,0.4,0.6]}}},"
      "{\"NativeMaskUpdate\":{\"id\":20,\"entries\":[{\"path\":\"M100 50 L150 50 L150 90 L100 90 Z\",\"opacity\":1,\"clips\":[]}]}}]");
  check(&test, pixel(scene, 140, 90, 0, 0, 0, 0), "native mask also removes EventBlocker background pixels");
  check(&test, pixel(scene, 90, 110, 51, 102, 153, 255), "native mask preserves neighboring background pixels");
  apply(&test, "[{\"NativeMaskUpdate\":{\"id\":20,\"entries\":[]}},"
      "{\"FrameCreate\":{\"id\":30}},{\"FrameUpdate\":{\"id\":30,\"size_x\":150,\"size_y\":100,\"transform\":[1,0,0,1,20,20],\"clip_content\":true,\"z_index\":2}},"
      "{\"EventBlockerUpdate\":{\"id\":20,\"parent_frame\":30}}]");
  check(&test, pixel(scene, 175, 100, 0, 0, 0, 0), "ancestor clip trims EventBlocker background");
  check(&test, pixel(scene, 150, 100, 51, 102, 153, 255), "ancestor clip preserves background interior");
  guint widgets = widget_count(scene);
  for (guint i = 0; i < 16; i++) {
    apply(&test, i % 2 ? "[{\"EventBlockerUpdate\":{\"id\":20,\"background\":{\"Rgb\":[0.2,0.4,0.6]}}}]"
        : "[{\"EventBlockerUpdate\":{\"id\":20,\"background\":{\"Rgba\":[0.8,0.2,0.1,0]}}}]");
  }
  check(&test, widget_count(scene) == widgets && gtk_widget_pick(scene, 60, 50, GTK_PICK_DEFAULT) == blocker,
      "repeated color changes preserve native widget population and identity");
  check(&test, pixel(scene, 150, 100, 51, 102, 153, 255), "repeated transparent/opaque transitions retain final paint");
  apply(&test, "[{\"EventBlockerDelete\":20},{\"FrameDelete\":30}]");
  click(&test, window, scene, 60, 50);
  check(&test, test.covered_clicks == 1, "deleting blocker restores real underlying button input");
  apply(&test, "[{\"EventBlockerCreate\":{\"id\":20}},{\"EventBlockerUpdate\":{\"id\":20,\"size_x\":160,\"size_y\":100,\"transform\":[1,0,0,1,20,20],\"z_index\":1}}]");
  check(&test, pixel(scene, 150, 100, 0, 0, 0, 0), "same-ID recreation defaults to transparent without stale paint");
done:
  pax_native_scene_stop(test.scene); gtk_window_destroy(GTK_WINDOW(window)); g_object_unref(window);
  g_print("%s %u EventBlocker checks; failures=%u\n", test.failures ? "FAIL" : "PASS", test.checks, test.failures);
  return test.failures ? 1 : 0;
}
