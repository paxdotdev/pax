/* Snapshot stack, nested transformed clips, and native mask final-pixel gate.
 * Build like text-constraints.c, substituting this filename. Requires GTK4.14+.
 */
#include "native-scene.h"
#include <math.h>
#include <string.h>

static guint checks, failures, stack_warnings;
static GLogWriterOutput log_writer(GLogLevelFlags level, const GLogField *fields,
                                  gsize count, gpointer data) {
  (void)data;
  for (gsize i = 0; i < count; i++)
    if (!strcmp(fields[i].key, "MESSAGE") && fields[i].value &&
        (strstr(fields[i].value, "gtk_snapshot_save") ||
         strstr(fields[i].value, "gtk_snapshot_restore")))
      stack_warnings++;
  return g_log_writer_default(level, fields, count, NULL);
}
static void check(gboolean value, const char *name) {
  checks++; failures += !value; g_print("%s %s\n", value ? "PASS" : "FAIL", name);
}
static void error(gpointer data, const char *message) {
  (void)data; failures++; g_printerr("scene error: %s\n", message);
}
static gboolean interrupt(gpointer data, const char *text, gsize length) {
  (void)data; (void)text; (void)length; return TRUE;
}
static void pump(void) {
  gint64 until = g_get_monotonic_time() + 150000;
  while (g_get_monotonic_time() < until) {
    for (guint i = 0; i < 32 && g_main_context_pending(NULL); i++)
      g_main_context_iteration(NULL, FALSE);
    g_usleep(1000);
  }
}
static void apply(GtkWidget *scene, const char *text) {
  JsonParser *parser = json_parser_new(); GError *failure = NULL;
  gboolean ok = json_parser_load_from_data(parser, text, -1, &failure);
  if (ok) ok = pax_native_scene_apply((PaxNativeScene *)scene,
      json_node_get_array(json_parser_get_root(parser)), &failure);
  check(ok, "nested clip/mask patch accepted");
  if (failure) {g_printerr("%s\n", failure->message); g_error_free(failure);}
  g_object_unref(parser); pump();
}
static GdkTexture *capture(GtkWidget *scene) {
  GtkSnapshot *snapshot = gtk_snapshot_new();
  GdkPaintable *paintable = gtk_widget_paintable_new(scene);
  gdk_paintable_snapshot(paintable, GDK_SNAPSHOT(snapshot), 300, 280);
  g_object_unref(paintable);
  GskRenderNode *node = gtk_snapshot_free_to_node(snapshot);
  if (!node) return NULL;
  graphene_rect_t bounds = GRAPHENE_RECT_INIT(0, 0, 300, 280);
  GdkTexture *texture = gsk_renderer_render_texture(
      gtk_native_get_renderer(gtk_widget_get_native(scene)), node, &bounds);
  gsk_render_node_unref(node); return texture;
}
static void pixel(GdkTexture *texture, guint x, guint y, guint alpha,
                  const char *name) {
  if (!texture) {check(FALSE, name); return;}
  gsize stride = gdk_texture_get_width(texture) * 4;
  guchar *pixels = g_malloc(stride * gdk_texture_get_height(texture));
  gdk_texture_download(texture, pixels, stride);
  guchar *p = pixels + y * stride + x * 4;
#if G_BYTE_ORDER == G_LITTLE_ENDIAN
  guint red = p[2], actual_alpha = p[3], green = p[1], blue = p[0];
#else
  guint red = p[1], actual_alpha = p[0], green = p[2], blue = p[3];
#endif
  g_print("pixel %u,%u rgba=%u,%u,%u,%u expected red/alpha=%u\n",
          x, y, red, green, blue, actual_alpha, alpha);
  check(abs((int)red-(int)alpha)<=3 && green<=3 && blue<=3 &&
        abs((int)actual_alpha-(int)alpha)<=3, name);
  g_free(pixels);
}
int main(int argc, char **argv) {
  g_log_set_writer_func(log_writer, NULL, NULL); gtk_init();
  GtkWidget *window = gtk_window_new(); g_object_ref_sink(window);
  GtkWidget *scene = pax_native_scene_new(interrupt, error, NULL);
  gtk_window_set_default_size(GTK_WINDOW(window), 300, 280);
  gtk_window_set_child(GTK_WINDOW(window), scene); gtk_window_present(GTK_WINDOW(window));
  apply(scene,
    "[{\"FrameCreate\":{\"id\":10,\"render_layer_id\":0}},"
    "{\"FrameCreate\":{\"id\":11,\"render_layer_id\":0}},"
    "{\"ButtonCreate\":{\"id\":12,\"render_layer_id\":0}},"
    "{\"FrameUpdate\":{\"id\":10,\"size_x\":200,\"size_y\":200,\"transform\":[1,0,0,1,20,20],\"clip_content\":true,\"clip_path\":\"M40 40 L200 40 L200 200 L40 200 Z\"}},"
    "{\"FrameUpdate\":{\"id\":11,\"parent_frame\":10,\"size_x\":140,\"size_y\":140,\"transform\":[1,0,0,1,40,40],\"clip_content\":true,\"clip_path\":\"M60 60 L160 60 L160 160 L60 160 Z\"}},"
    "{\"ButtonUpdate\":{\"id\":12,\"parent_frame\":11,\"size_x\":140,\"size_y\":140,\"transform\":[1,0,0,1,40,40],\"content\":\"\",\"color\":{\"Rgb\":[1,0,0]},\"hover_color\":{\"Rgb\":[1,0,0]},\"outline_stroke_width\":0,\"corner_radius\":0}},"
    "{\"NativeMaskUpdate\":{\"id\":12,\"entries\":[{\"path\":\"M40 40 L60 40 L60 60 L40 60 Z\",\"opacity\":1,\"clips\":[]},{\"path\":\"M60 60 L80 60 L80 80 L60 80 Z\",\"opacity\":0.5,\"clips\":[\"M60 60 L80 60 L80 80 L60 80 Z\"]}]}}]");
  for (guint i = 0; i < 3; i++) {
    gtk_widget_queue_draw(scene); pump();
    GdkTexture *texture = capture(scene);
    pixel(texture, 70, 70, 255, "nested world-space clips preserve native interior");
    pixel(texture, 50, 70, 0, "inner path clips native pixels outside its boundary");
    pixel(texture, 170, 70, 0, "nested clip trims the far native edge");
    pixel(texture, 90, 90, 0, "opaque punch-through removes native pixels");
    pixel(texture, 110, 110, 128, "nested fractional mask applies native alpha once");
    if (texture && argc == 2) gdk_texture_save_to_png(texture, argv[1]);
    g_clear_object(&texture);
  }
  check(stack_warnings == 0, "nested clips restore snapshot state without stack warnings");
  pax_native_scene_stop((PaxNativeScene *)scene);
  gtk_window_destroy(GTK_WINDOW(window)); g_object_unref(window); pump();
  g_print("%s %u snapshot checks; stack_warnings=%u\n",
          failures ? "FAIL" : "PASS", checks, stack_warnings);
  return failures ? 1 : 0;
}
