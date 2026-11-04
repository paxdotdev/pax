/* NativeImage GTK pixels, shared loads and lifetime regression.
 * Build like text-constraints.c, substituting native-images.c. Run under GTK
 * with GSK_RENDERER=gl (or ngl on GTK 4.14), including Xvfb for the CPU probe.
 * Readback below is test-only; application presentation stays native/GPU.
 */
#include "native-scene.h"
#include "assets.h"
#include <glib/gstdio.h>
#include <stdlib.h>
#include <string.h>
typedef struct { PaxNativeScene *scene; guint checks, failures, errors; } Test;
static void check(Test *test, gboolean ok, const char *name) {
  test->checks++; test->failures += !ok;
  g_print("%s %s\n", ok ? "PASS" : "FAIL", name);
}
static void scene_error(gpointer data, const char *message) {
  ((Test *)data)->errors++;
  g_printerr("native scene error: %s\n", message);
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
  if (!ok) g_printerr("patch error: %s\n", error ? error->message : "unknown");
  g_clear_error(&error); g_object_unref(parser); return ok;
}
static gint64 metric(Test *test, const char *name) {
  JsonObject *report = pax_native_scene_report(test->scene);
  gint64 value = json_object_get_int_member(report, name);
  json_object_unref(report); return value;
}
static gboolean idle(Test *test) {
  gint64 deadline = g_get_monotonic_time() + 5000000;
  do {
    pump(10);
    if (!metric(test, "nativeImagePending") && !metric(test, "nativeImageQueued")) {
      pump(100); return TRUE;
    }
  } while (g_get_monotonic_time() < deadline);
  return FALSE;
}
static GtkWidget *picture(GtkWidget *widget, guint *index) {
  if (GTK_IS_PICTURE(widget) && !(*index)--) return widget;
  for (GtkWidget *child = gtk_widget_get_first_child(widget); child;
       child = gtk_widget_get_next_sibling(child)) {
    GtkWidget *found = picture(child, index);
    if (found) return found;
  }
  return NULL;
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

int main(void) {
  gtk_init(); Test test = {0}; GError *error = NULL;
  GtkWidget *retained_node = NULL, *retained_picture = NULL;
  GtkWidget *stopped_node = NULL, *stopped_picture = NULL;
  char *directory = g_dir_make_tmp("pax-native-image-XXXXXX", &error);
  if (!directory) g_error("fixture directory: %s", error->message);
  char *raster = g_build_filename(directory, "bands.png", NULL);
  char *jpeg = g_build_filename(directory, "bands.jpg", NULL);
  char *svg = g_build_filename(directory, "alpha.svg", NULL);
  char *bad = g_build_filename(directory, "bad.png", NULL);
  char *huge = g_build_filename(directory, "huge.svg", NULL);
  GdkPixbuf *pixbuf = gdk_pixbuf_new(GDK_COLORSPACE_RGB, TRUE, 8, 80, 40);
  guchar *data = gdk_pixbuf_get_pixels(pixbuf);
  for (int y = 0; y < 40; y++) for (int x = 0; x < 80; x++) {
    guchar *p = data + y * gdk_pixbuf_get_rowstride(pixbuf) + x * 4;
    p[0] = x < 20 ? 255 : 0; p[1] = x >= 20 && x < 60 ? 255 : 0;
    p[2] = x >= 60 ? 255 : 0; p[3] = 255;
  }
  if (!gdk_pixbuf_save(pixbuf, raster, "png", &error, NULL)) g_error("fixture PNG: %s", error->message);
  GdkPixbuf *rgb = gdk_pixbuf_new(GDK_COLORSPACE_RGB, FALSE, 8, 80, 40);
  for (int y = 0; y < 40; y++) for (int x = 0; x < 80; x++)
    memcpy(gdk_pixbuf_get_pixels(rgb) + y * gdk_pixbuf_get_rowstride(rgb) + x * 3,
           data + y * gdk_pixbuf_get_rowstride(pixbuf) + x * 4, 3);
  if (!gdk_pixbuf_save(rgb, jpeg, "jpeg", &error, "quality", "100", NULL)) g_error("fixture JPEG: %s", error->message);
  g_object_unref(rgb);
  g_object_unref(pixbuf);
  g_file_set_contents(svg, "<svg xmlns='http://www.w3.org/2000/svg' width='80' height='40'><rect width='80' height='40' fill='#ff0000' fill-opacity='.5'/></svg>", -1, NULL);
  g_file_set_contents(bad, "not an image", -1, NULL);
  g_file_set_contents(huge, "<svg xmlns='http://www.w3.org/2000/svg' width='100000' height='100000'><rect width='100000' height='100000'/></svg>", -1, NULL);
  GtkWidget *window = gtk_window_new(); g_object_ref_sink(window);
  GtkWidget *scene = pax_native_scene_new(NULL, scene_error, &test);
  test.scene = (PaxNativeScene *)scene;
  char *roots[] = {directory, NULL}; pax_native_scene_assets(test.scene, roots);
  char *resolved = pax_linux_asset_path(roots, "/assets/bands.png", &error);
  check(&test, !error && !g_strcmp0(resolved, raster), "shared asset resolver maps /assets to the bundled root");
  g_free(resolved); g_clear_error(&error);
  resolved = pax_linux_asset_path(roots, raster, &error);
  check(&test, !error && !g_strcmp0(resolved, raster), "ordinary absolute local asset paths remain supported");
  g_free(resolved); g_clear_error(&error);
  char *uri = g_filename_to_uri(raster, NULL, NULL);
  resolved = pax_linux_asset_path(roots, uri, &error);
  check(&test, !error && !g_strcmp0(resolved, raster), "local file URI asset paths remain supported");
  g_free(resolved); g_free(uri); g_clear_error(&error);
  gtk_window_set_default_size(GTK_WINDOW(window), 320, 180);
  gtk_window_set_child(GTK_WINDOW(window), scene); gtk_window_present(GTK_WINDOW(window));
  check(&test, apply(&test,
    "[{\"NativeImageCreate\":{\"id\":1}},{\"NativeImageCreate\":{\"id\":2}},"
    "{\"NativeImageUpdate\":{\"id\":1,\"size_x\":80,\"size_y\":80,\"url\":\"assets/bands.png\",\"fit\":\"contain\",\"transform\":[1,0,0,1,20,20]}},"
    "{\"NativeImageUpdate\":{\"id\":2,\"size_x\":80,\"size_y\":80,\"url\":\"/assets/bands.png\",\"fit\":\"contain\",\"transform\":[1,0,0,1,120,20]}}]"), "NativeImage and both packaged asset aliases accepted");
  check(&test, idle(&test) && test.errors == 0, "asynchronous raster decoding completes without scene errors");
  guint index = 0; GtkWidget *first = picture(scene, &index); index = 1; GtkWidget *second = picture(scene, &index);
  check(&test, first && second, "creates genuine GtkPicture controls");
  if (!first || !second) goto done;
  check(&test, gtk_picture_get_paintable(GTK_PICTURE(first)) == gtk_picture_get_paintable(GTK_PICTURE(second)) &&
      metric(&test, "nativeImageSources") == 1 && metric(&test, "nativeImageBytes") == 80*40*4,
      "same resolved source shares one decoded texture");
  check(&test, pixel(scene, 25, 25, 0, 0, 0, 0) && pixel(scene, 25, 55, 255, 0, 0, 255), "contain centers proportional image with transparent letterbox");
  apply(&test, "[{\"NativeImageUpdate\":{\"id\":1,\"fit\":\"cover\"}}]"); pump(100);
  check(&test, pixel(scene, 25, 25, 0, 255, 0, 255) && pixel(scene, 105, 25, 0, 0, 0, 0), "cover crops proportional image without escaping its bounds");
  apply(&test, "[{\"NativeImageUpdate\":{\"id\":1,\"fit\":\"fill\"}}]"); pump(100);
  check(&test, pixel(scene, 25, 25, 255, 0, 0, 255), "fill stretches image to bounds");
  apply(&test, "[{\"NativeImageUpdate\":{\"id\":1,\"opacity\":0.5}}, {\"NativeMaskUpdate\":{\"id\":1,\"entries\":[{\"path\":\"M0 0 L40 0 L40 80 L0 80 Z\",\"opacity\":1,\"clips\":[]}]}}]"); pump(100);
  check(&test, pixel(scene, 25, 25, 0, 0, 0, 0) && pixel(scene, 95, 25, 0, 0, 128, 128), "native mask and node opacity apply once");
  apply(&test, "[{\"NativeMaskUpdate\":{\"id\":1,\"entries\":[{\"path\":\"M0 0 L40 0 L40 80 L0 80 Z\",\"opacity\":0.5,\"clips\":[]}]}}]"); pump(100);
  check(&test, pixel(scene, 25, 25, 64, 0, 0, 64), "fractional native mask combines with node opacity once");
  apply(&test, "[{\"NativeMaskUpdate\":{\"id\":1,\"entries\":[]}}, {\"FrameCreate\":{\"id\":3}}, {\"FrameUpdate\":{\"id\":3,\"size_x\":40,\"size_y\":40,\"transform\":[1,0,0,1,20,20],\"clip_content\":true}}, {\"NativeImageUpdate\":{\"id\":1,\"parent_frame\":3,\"opacity\":1}}]"); pump(100);
  check(&test, pixel(scene, 25, 25, 255, 0, 0, 255) && pixel(scene, 95, 25, 0, 0, 0, 0), "ancestor frame clips native image pixels");
  apply(&test, "[{\"NativeImageUpdate\":{\"id\":1,\"parent_frame\":null,\"url\":\"assets/bands.jpg\"}}, {\"FrameDelete\":3}]");
  check(&test, idle(&test) && pixel(scene, 25, 25, 255, 0, 0, 255), "JPEG RGB pixels decode into native texture");
  apply(&test, "[{\"NativeImageUpdate\":{\"id\":1,\"url\":\"assets/alpha.svg\",\"opacity\":1}}, {\"NativeMaskUpdate\":{\"id\":1,\"entries\":[]}}]");
  check(&test, idle(&test) && pixel(scene, 25, 25, 128, 0, 0, 128), "SVG decoding preserves authored transparency");
  index = 0; check(&test, picture(scene, &index) == first, "source and fit changes preserve native control identity");
  apply(&test, "[{\"NativeImageUpdate\":{\"id\":1,\"url\":\"\"}}]"); pump(100);
  check(&test, !gtk_picture_get_paintable(GTK_PICTURE(first)) && metric(&test, "nativeImageSources") == 1, "empty URL clears pixels and releases unused decoded source");
  apply(&test, "[{\"NativeImageUpdate\":{\"id\":1,\"url\":\"assets/bad.png\"}},{\"NativeImageUpdate\":{\"id\":1,\"url\":\"assets/alpha.svg\"}}]");
  check(&test, idle(&test) && test.errors == 0 && pixel(scene, 25, 25, 128, 0, 0, 128), "superseded failing decode cannot publish an error or stale pixels");
  apply(&test, "[{\"NativeImageUpdate\":{\"id\":1,\"url\":\"assets/bad.png\"}},{\"NativeImageDelete\":1},{\"NativeImageCreate\":{\"id\":1}},{\"NativeImageUpdate\":{\"id\":1,\"url\":\"assets/alpha.svg\",\"fit\":\"fill\",\"size_x\":80,\"size_y\":80,\"transform\":[1,0,0,1,20,20]}}]");
  check(&test, idle(&test) && test.errors == 0 && pixel(scene, 25, 25, 128, 0, 0, 128), "deletion and same-ID recreation reject previous completion");
  check(&test, !apply(&test, "[{\"NativeImageUpdate\":{\"id\":1,\"url\":\"https://example.invalid/image.png\"}}]"), "remote URLs report unsupported local-asset policy");
  apply(&test, "[{\"NativeImageUpdate\":{\"id\":1,\"url\":\"assets/huge.svg\"}}]");
  check(&test, idle(&test) && test.errors == 1, "oversized SVG fails before unbounded decoded allocation");
  index = 1; retained_picture = picture(scene, &index);
  g_object_ref(retained_picture);
  retained_node = g_object_ref(gtk_widget_get_parent(retained_picture));
  apply(&test, "[{\"NativeImageDelete\":1},{\"NativeImageDelete\":2}]");
  check(&test, idle(&test) && !metric(&test, "nativeImageSources") && !metric(&test, "nativeImageBytes"), "unmount releases all decoded sources even with a retained native node");
  check(&test, !gtk_picture_get_paintable(GTK_PICTURE(retained_picture)), "delete clears pixels on an externally retained GtkPicture");
  apply(&test, "[{\"NativeImageCreate\":{\"id\":100}},{\"NativeImageUpdate\":{\"id\":100,\"url\":\"assets/bands.png\",\"size_x\":40,\"size_y\":40}}]");
  check(&test, idle(&test), "retained shutdown fixture loads before stop");
  index = 0; stopped_picture = picture(scene, &index);
  g_object_ref(stopped_picture);
  stopped_node = g_object_ref(gtk_widget_get_parent(stopped_picture));
  for (guint i=0; i<32; i++) {
    char *batch = g_strdup_printf("[{\"NativeImageCreate\":{\"id\":%u}},{\"NativeImageUpdate\":{\"id\":%u,\"url\":\"assets/bad.png\"}},{\"NativeImageDelete\":%u}]", i+1, i+1, i+1);
    apply(&test, batch); g_free(batch);
  }
  check(&test, metric(&test, "nativeImagePending") <= 4, "decode worker count remains bounded during rapid mount/unmount");
done:
  pax_native_scene_stop(test.scene);
  if (stopped_picture)
    check(&test, !metric(&test, "nativeImageSources") && !metric(&test, "nativeImageBytes") &&
        !gtk_picture_get_paintable(GTK_PICTURE(stopped_picture)), "stop releases externally retained native image resources");
  gtk_window_destroy(GTK_WINDOW(window)); g_object_unref(window);
  // Retained toolkit objects outlive scene destruction; their final dispose
  // must not consult the old scene or retain decoded image allocations.
  g_clear_object(&retained_picture); g_clear_object(&retained_node);
  g_clear_object(&stopped_picture); g_clear_object(&stopped_node);
  pump(300);
  check(&test, test.errors == 1, "shutdown cancels pending work without post-stop callbacks");
  g_remove(raster); g_remove(jpeg); g_remove(svg); g_remove(bad); g_remove(huge); g_rmdir(directory);
  g_free(raster); g_free(jpeg); g_free(svg); g_free(bad); g_free(huge); g_free(directory);
  g_print("%s %u NativeImage checks; failures=%u\n", test.failures ? "FAIL" : "PASS", test.checks, test.failures);
  return test.failures ? 1 : 0;
}
