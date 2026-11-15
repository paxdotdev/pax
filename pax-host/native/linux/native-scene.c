#include "native-scene.h"
#include "assets.h"
#include <fontconfig/fontconfig.h>
#include <pango/pangocairo.h>
#include <math.h>
#include <string.h>

typedef struct _PaxNode PaxNode;
typedef struct {
  gint references;
  guint users;
  char *filename;
  GCancellable *cancel;
  GdkTexture *texture;
  gsize bytes;
  gboolean failed;
} NativeImage;
typedef struct {
  GWeakRef scene;
  NativeImage *image;
} NativeImageJob;
#define NATIVE_IMAGE_ASSET_LIMIT (32u * 1024u * 1024u)
#define NATIVE_IMAGE_CACHE_LIMIT (256u * 1024u * 1024u)
#define NATIVE_IMAGE_WORKERS 4

typedef struct {
  GskPath *path;
  GPtrArray *clips;
  double opacity;
} MaskEntry;
typedef struct {
  GdkSurface *surface;
  double x, y;
} WheelPointer;
typedef struct {
  GtkWidget parent;
  PaxNode *node;
} PaxInput;
typedef GtkWidgetClass PaxInputClass;
typedef GtkWidgetClass PaxNodeClass;
typedef GtkWidgetClass PaxNativeSceneClass;
struct _PaxNode {
  GtkWidget parent;
  PaxNativeScene *scene;
  guint id, layer;
  gint64 parent_id, transform_scroller;
  double authored_transform[6];
  int z;
  char *kind, *tile_key;
  GtkWidget *control, *content, *input;
  GdkTexture *texture;
  NativeImage *native_image;
  char *image_url;
  GPtrArray *mask;
  GskPath *clip_path;
  double matrix[6], width, height, radius, opacity, inner_width, inner_height;
  GdkRGBA blocker_background;
  double text_width_constraint, text_height_constraint;
  double range_min, range_max, range_step, range_value;
  guint selected_id;
  guint64 control_events;
  char **options;
  GtkStringList *options_model;
  GtkCheckButton *radio_group;
  PangoAttrList *form_attributes;
  JsonObject *form_appearance;
  JsonObject *rich_text, *link_style;
  gboolean text_editable, text_selectable, text_wrap;
  GtkCssProvider *form_css, *transparent_text_css;
  char *form_css_class, *transparent_text_class;
  float form_xalign, form_yalign;
  GtkJustification text_justification;
  GArray *snap_authored[2], *snap_points[2];
  GHashTable *snap_touches;
  guint snap_tick, snap_buttons, snap_keys;
  GtkWindow *snap_window;
  gulong snap_window_signal;
  gint64 snap_last_change, snap_started, snap_duration;
  double snap_origin[2], snap_from[2], snap_to[2];
  double requested_scroll[2];
  gboolean scroll_request_pending[2];
  int snap_direction[2];
  gboolean snap_pending, snap_animating, snap_writing, snap_scroll_active;
  guint64 latest_text_measure_generation;
  gboolean clip, updating;
};
struct _PaxNativeScene {
  GtkWidget parent;
  GHashTable *nodes, *tiles, *layer_owners, *pressed_keys, *wheel_pointers;
  PaxSceneInterrupt interrupt;
  PaxSceneError error;
  PaxSceneNavigate navigate;
  gpointer data;
  guint layer_count;
  guint64 scene_pointer_presses, scene_pointer_releases, scene_pointer_clicks;
  struct { double x, y; gboolean pressed, dragged; } pointer_buttons[3];
  guint64 text_metrics_invalidations;
  guint64 placement_reconciliations, placement_visits;
  guint font_map_serial;
  GPtrArray *pointer_focus_viewports;
  guint pointer_focus_restore_source;
  gboolean stopped, text_metrics_dirty;
  gboolean commit_open, placement_pending;
  char **assets;
  GHashTable *native_images;
  GQueue image_queue;
  guint image_pending;
  gsize image_bytes;
};
G_DEFINE_TYPE(PaxNativeScene, pax_native_scene, GTK_TYPE_WIDGET)
G_DEFINE_TYPE(PaxNode, pax_node, GTK_TYPE_WIDGET)
G_DEFINE_TYPE(PaxInput, pax_input, GTK_TYPE_WIDGET)
static gboolean has(JsonObject *o, const char *key) {
  return json_object_has_member(o, key) &&
         !JSON_NODE_HOLDS_NULL(json_object_get_member(o, key));
}
static double number(JsonObject *o, const char *key, double fallback) {
  return has(o, key) ? json_object_get_double_member(o, key) : fallback;
}
static const char *string(JsonObject *o, const char *key,
                          const char *fallback) {
  return has(o, key) ? json_object_get_string_member(o, key) : fallback;
}
static gboolean boolean(JsonObject *o, const char *key, gboolean fallback) {
  return has(o, key) ? json_object_get_boolean_member(o, key) : fallback;
}
static NativeImage *native_image_ref(NativeImage *image) {
  // GTask may release task data on a worker after its UI completion callback.
  // Pixel sources are immutable; only ownership crosses that thread boundary.
  g_atomic_int_inc(&image->references);
  return image;
}
static void native_image_unref(gpointer data) {
  NativeImage *image = data;
  if (!g_atomic_int_dec_and_test(&image->references)) return;
  g_clear_object(&image->texture);
  g_clear_object(&image->cancel);
  g_free(image->filename);
  g_free(image);
}
static void native_image_release(PaxNode *node) {
  NativeImage *image = node->native_image;
  if (!image) return;
  node->native_image = NULL;
  if (node->control)
    gtk_picture_set_paintable(GTK_PICTURE(node->control), NULL);
  if (!--image->users) {
    g_cancellable_cancel(image->cancel);
    // A queued/running decode owns its own reference and is retired separately.
    node->scene->image_bytes -= image->bytes;
    g_hash_table_remove(node->scene->native_images, image->filename);
  }
  native_image_unref(image);
}
static void native_image_size(GdkPixbufLoader *loader, int width, int height,
                               gboolean *oversized) {
  if (width <= 0 || height <= 0 || width > 16384 || height > 16384 ||
      (guint64)width * height * 4 > NATIVE_IMAGE_ASSET_LIMIT) {
    *oversized = TRUE;
    // size-prepared runs before the decoded pixel allocation. The result is
    // rejected, not displayed at reduced quality or accepted as a large image.
    gdk_pixbuf_loader_set_size(loader, 1, 1);
  }
}
static void native_image_decode(GTask *task, gpointer source, gpointer data,
                                 GCancellable *cancel) {
  (void)source;
  NativeImageJob *job = data;
  GError *error = NULL;
  GFile *file = g_file_new_for_path(job->image->filename);
  GFileInputStream *input = g_file_read(file, cancel, &error);
  g_object_unref(file);
  GdkPixbufLoader *loader = gdk_pixbuf_loader_new();
  gboolean oversized = FALSE;
  g_signal_connect(loader, "size-prepared", G_CALLBACK(native_image_size), &oversized);
  guint8 buffer[8192];
  gsize total = 0;
  while (input && !error) {
    gssize count = g_input_stream_read(G_INPUT_STREAM(input), buffer,
                                       sizeof buffer, cancel, &error);
    if (count <= 0) break;
    total += count;
    if (total > NATIVE_IMAGE_ASSET_LIMIT) {
      g_set_error_literal(&error, G_IO_ERROR, G_IO_ERROR_NO_SPACE,
                           "NativeImage encoded file exceeds 32 MiB");
      break;
    }
    if (!gdk_pixbuf_loader_write(loader, buffer, count, &error)) break;
    if (oversized) {
      g_set_error_literal(&error, G_IO_ERROR, G_IO_ERROR_NO_SPACE,
                           "NativeImage decoded pixels exceed 32 MiB or 16384 pixels per axis");
      break;
    }
  }
  g_clear_object(&input);
  GError *close_error = NULL;
  gboolean closed = gdk_pixbuf_loader_close(loader, &close_error);
  if (!error && !closed) error = g_steal_pointer(&close_error);
  g_clear_error(&close_error);
  GdkPixbuf *pixbuf = !error ? gdk_pixbuf_loader_get_pixbuf(loader) : NULL;
  if (pixbuf && !oversized &&
      gdk_pixbuf_get_byte_length(pixbuf) <= NATIVE_IMAGE_ASSET_LIMIT) {
    g_task_return_pointer(task, g_object_ref(pixbuf), g_object_unref);
  } else {
    if (!error) g_set_error_literal(&error, G_IO_ERROR, G_IO_ERROR_INVALID_DATA,
                                    "NativeImage has no valid bounded bitmap");
    g_task_return_error(task, error);
  }
  g_object_unref(loader);
}
static void native_image_job_free(gpointer data) {
  NativeImageJob *job = data;
  g_weak_ref_clear(&job->scene);
  native_image_unref(job->image);
  g_free(job);
}
static void native_image_start(PaxNativeScene *scene);
static void native_image_ready(GObject *source, GAsyncResult *result, gpointer data) {
  (void)source;
  (void)data;
  GTask *task = G_TASK(result);
  NativeImageJob *job = g_task_get_task_data(task);
  NativeImage *image = job->image;
  GError *error = NULL;
  GdkPixbuf *pixbuf = g_task_propagate_pointer(task, &error);
  PaxNativeScene *scene = g_weak_ref_get(&job->scene);
  if (scene) {
    scene->image_pending--;
    // Identity, not the URL alone, rejects a deleted/recreated request for the
    // same path. No task retains a widget or a live engine session.
    if (!scene->stopped && image->users &&
        g_hash_table_lookup(scene->native_images, image->filename) == image) {
      if (pixbuf) {
        gsize bytes = gdk_pixbuf_get_byte_length(pixbuf);
        if (bytes > NATIVE_IMAGE_CACHE_LIMIT - scene->image_bytes) {
          g_set_error_literal(&error, G_IO_ERROR, G_IO_ERROR_NO_SPACE,
                               "Mounted NativeImage pixels exceed the 256 MiB cache budget");
        } else {
          GBytes *pixels = gdk_pixbuf_read_pixel_bytes(pixbuf);
          image->texture = gdk_memory_texture_new(
              gdk_pixbuf_get_width(pixbuf), gdk_pixbuf_get_height(pixbuf),
              gdk_pixbuf_get_has_alpha(pixbuf) ? GDK_MEMORY_R8G8B8A8 : GDK_MEMORY_R8G8B8,
              pixels, gdk_pixbuf_get_rowstride(pixbuf));
          g_bytes_unref(pixels);
          image->bytes = bytes;
          scene->image_bytes += bytes;
          GHashTableIter iterator;
          gpointer value;
          g_hash_table_iter_init(&iterator, scene->nodes);
          while (g_hash_table_iter_next(&iterator, NULL, &value)) {
            PaxNode *node = value;
            if (node->native_image == image)
              gtk_picture_set_paintable(GTK_PICTURE(node->control), GDK_PAINTABLE(image->texture));
          }
        }
      }
      if (error) {
        image->failed = TRUE;
        if (scene->error) {
          char *message = g_strdup_printf("NativeImage %s: %s", image->filename, error->message);
          scene->error(scene->data, message);
          g_free(message);
        }
      }
    }
    native_image_start(scene);
    g_object_unref(scene);
  }
  g_clear_object(&pixbuf);
  g_clear_error(&error);
}
static void native_image_start(PaxNativeScene *scene) {
  while (!scene->stopped && scene->image_pending < NATIVE_IMAGE_WORKERS &&
         !g_queue_is_empty(&scene->image_queue)) {
    NativeImage *image = g_queue_pop_head(&scene->image_queue);
    if (!image->users) { native_image_unref(image); continue; }
    NativeImageJob *job = g_new0(NativeImageJob, 1);
    g_weak_ref_init(&job->scene, scene);
    job->image = image;
    GTask *task = g_task_new(NULL, image->cancel, native_image_ready, NULL);
    g_task_set_task_data(task, job, native_image_job_free);
    scene->image_pending++;
    g_task_run_in_thread(task, native_image_decode);
    g_object_unref(task);
  }
}
static void native_images_stop(PaxNativeScene *scene) {
  GHashTableIter iterator;
  gpointer value;
  g_hash_table_iter_init(&iterator, scene->native_images);
  while (g_hash_table_iter_next(&iterator, NULL, &value))
    g_cancellable_cancel(((NativeImage *)value)->cancel);
  g_queue_clear_full(&scene->image_queue, native_image_unref);
  // A toolkit/accessibility client can retain detached widgets beyond the
  // scene. Retire image references now, while node->scene is still valid.
  g_hash_table_iter_init(&iterator, scene->nodes);
  while (g_hash_table_iter_next(&iterator, NULL, &value))
    native_image_release(value);
}
static gboolean native_image_update(PaxNode *node, JsonObject *patch, GError **error) {
  if (has(patch, "fit")) {
    const char *fit = string(patch, "fit", "contain");
    GtkContentFit content_fit;
    if (!strcmp(fit, "contain")) content_fit = GTK_CONTENT_FIT_CONTAIN;
    else if (!strcmp(fit, "cover")) content_fit = GTK_CONTENT_FIT_COVER;
    else if (!strcmp(fit, "fill")) content_fit = GTK_CONTENT_FIT_FILL;
    else {
      g_set_error(error, G_IO_ERROR, G_IO_ERROR_INVALID_DATA,
                   "Unknown NativeImage fit: %s", fit);
      return FALSE;
    }
    gtk_picture_set_content_fit(GTK_PICTURE(node->control), content_fit);
  }
  if (!has(patch, "url") || !g_strcmp0(node->image_url, string(patch, "url", "")))
    return TRUE;
  const char *url = string(patch, "url", "");
  char *filename = *url ? pax_linux_asset_path(node->scene->assets, url, error) : NULL;
  if (*url && !filename) return FALSE;
  if (filename) {
    char *canonical = g_canonicalize_filename(filename, NULL);
    g_free(filename);
    filename = canonical;
  }
  if (filename && node->native_image &&
      !strcmp(filename, node->native_image->filename)) {
    g_free(node->image_url);
    node->image_url = g_strdup(url);
    g_free(filename);
    return TRUE;
  }
  native_image_release(node);
  g_free(node->image_url);
  node->image_url = g_strdup(url);
  if (!filename) return TRUE;
  NativeImage *image = g_hash_table_lookup(node->scene->native_images, filename);
  if (!image) {
    image = g_new0(NativeImage, 1);
    image->references = 1;
    image->filename = g_strdup(filename);
    image->cancel = g_cancellable_new();
    g_hash_table_insert(node->scene->native_images, g_strdup(filename), image);
    g_queue_push_tail(&node->scene->image_queue, native_image_ref(image));
  }
  image->users++;
  node->native_image = native_image_ref(image);
  if (image->texture)
    gtk_picture_set_paintable(GTK_PICTURE(node->control), GDK_PAINTABLE(image->texture));
  native_image_start(node->scene);
  g_free(filename);
  return TRUE;
}
static void mask_free(gpointer data) {
  MaskEntry *entry = data;
  gsk_path_unref(entry->path);
  g_ptr_array_unref(entry->clips);
  g_free(entry);
}
static gboolean send_object(PaxNativeScene *scene, const char *kind,
                            JsonObject *body) {
  JsonObject *outer = json_object_new();
  json_object_set_object_member(outer, kind, body);
  JsonNode *node = json_node_new(JSON_NODE_OBJECT);
  json_node_take_object(node, outer);
  char *text = json_to_string(node, FALSE);
  gboolean result = !scene->stopped && scene->interrupt &&
                    scene->interrupt(scene->data, text, strlen(text));
  g_free(text);
  json_node_free(node);
  return result;
}
static void emit_text(PaxNode *node, const char *kind, const char *text) {
  if (node->updating)
    return;
  JsonObject *body = json_object_new();
  json_object_set_int_member(body, "id", node->id);
  json_object_set_string_member(body, "text", text);
  send_object(node->scene, kind, body);
}
static void entry_changed(GtkEditable *entry, PaxNode *node) {
  emit_text(node, "FormTextboxInput", gtk_editable_get_text(entry));
}
static void entry_activate(GtkEntry *entry, PaxNode *node) {
  emit_text(node, "FormTextboxChange",
            gtk_editable_get_text(GTK_EDITABLE(entry)));
}
static void text_tag_range(GtkTextBuffer *buffer) {
  GtkTextTag *tag = gtk_text_tag_table_lookup(gtk_text_buffer_get_tag_table(buffer), "pax-style");
  if (tag) {
    GtkTextIter start, end;
    gtk_text_buffer_get_bounds(buffer, &start, &end);
    gtk_text_buffer_apply_tag(buffer, tag, &start, &end);
  }
}
static void text_changed(GtkTextBuffer *buffer, PaxNode *node) {
  // Uniform native text style also applies to insertions into an empty buffer
  // and replacements that do not carry another style patch.
  text_tag_range(buffer);
  GtkTextIter a, b;
  gtk_text_buffer_get_bounds(buffer, &a, &b);
  char *text = gtk_text_buffer_get_text(buffer, &a, &b, TRUE);
  emit_text(node,
            strcmp(node->kind, "Text") == 0 ? "TextInput" : "FormTextboxInput",
            text);
  g_free(text);
}
static void button_clicked(GtkButton *button, PaxNode *node) {
  (void)button;
  if (node->updating)
    return;
  JsonObject *body = json_object_new();
  json_object_set_int_member(body, "id", node->id);
  send_object(node->scene, "FormButtonClick", body);
}
static void checkbox_changed(GtkCheckButton *button, PaxNode *node) {
  if (node->updating)
    return;
  JsonObject *body = json_object_new();
  json_object_set_int_member(body, "id", node->id);
  json_object_set_boolean_member(body, "state",
                                 gtk_check_button_get_active(button));
  send_object(node->scene, "FormCheckboxToggle", body);
}
static double slider_value(PaxNode *node, double value) {
  double steps = floor((node->range_max - node->range_min) / node->range_step + 1e-9);
  double position = round((CLAMP(value, node->range_min, node->range_max) -
                           node->range_min) / node->range_step);
  return CLAMP(node->range_min + CLAMP(position, 0, steps) * node->range_step,
               node->range_min, node->range_max);
}
static void slider_changed(GtkRange *range, PaxNode *node) {
  if (node->updating || node->scene->stopped)
    return;
  double value = slider_value(node, gtk_range_get_value(range));
  // Accessibility clients can set a range directly, bypassing change-value.
  // Normalize that path too, without dispatching the normalization recursively.
  node->updating = TRUE;
  gtk_range_set_value(range, value);
  node->updating = FALSE;
  if (value == node->range_value)
    return;
  node->range_value = value;
  node->control_events++;
  JsonObject *body = json_object_new();
  json_object_set_int_member(body, "id", node->id);
  json_object_set_double_member(body, "value", value);
  send_object(node->scene, "FormSliderChange", body);
}
static gboolean slider_change_value(GtkRange *range, GtkScrollType scroll,
                                    double value, PaxNode *node) {
  (void)scroll;
  if (!node->scene->stopped && isfinite(value))
    gtk_range_set_value(range, slider_value(node, value));
  return TRUE;
}
static void selection_changed(PaxNode *node, guint selected, const char *kind) {
  if (node->updating || node->scene->stopped ||
      selected >= g_strv_length(node->options) || selected == node->selected_id)
    return;
  node->selected_id = selected;
  node->control_events++;
  JsonObject *body = json_object_new();
  json_object_set_int_member(body, "id", node->id);
  json_object_set_int_member(body, "selected_id", selected);
  send_object(node->scene, kind, body);
}
static void dropdown_changed(GtkDropDown *dropdown, GParamSpec *property,
                              PaxNode *node) {
  (void)property;
  selection_changed(node, gtk_drop_down_get_selected(dropdown),
                     "FormDropdownChange");
}
static void radio_changed(GtkCheckButton *button, PaxNode *node) {
  if (gtk_check_button_get_active(button))
    selection_changed(node,
                       GPOINTER_TO_UINT(g_object_get_data(G_OBJECT(button), "pax-option")),
                       "FormRadioListChange");
}
static void dropdown_setup(GtkSignalListItemFactory *factory, GtkListItem *item,
                            PaxNode *node) {
  (void)factory;
  GtkWidget *label = gtk_label_new("");
  gtk_label_set_xalign(GTK_LABEL(label), 0);
  gtk_label_set_yalign(GTK_LABEL(label), node->form_yalign);
  gtk_label_set_attributes(GTK_LABEL(label), node->form_attributes);
  gtk_list_item_set_child(item, label);
}
static void dropdown_bind(GtkSignalListItemFactory *factory, GtkListItem *item,
                           PaxNode *node) {
  (void)factory;
  GtkLabel *label = GTK_LABEL(gtk_list_item_get_child(item));
  GtkStringObject *text = GTK_STRING_OBJECT(gtk_list_item_get_item(item));
  gtk_label_set_text(label, gtk_string_object_get_string(text));
  gtk_label_set_xalign(label, node->form_xalign);
  gtk_label_set_yalign(label, node->form_yalign);
  gtk_label_set_attributes(label, node->form_attributes);
}
static GtkAdjustment *scroll_adjustment(PaxNode *node, guint axis) {
  GtkScrolledWindow *scroll = GTK_SCROLLED_WINDOW(node->control);
  return axis ? gtk_scrolled_window_get_vadjustment(scroll)
              : gtk_scrolled_window_get_hadjustment(scroll);
}
static void scroll_emit(PaxNode *node) {
  JsonObject *body = json_object_new();
  json_object_set_int_member(body, "id", node->id);
  double x = gtk_adjustment_get_value(scroll_adjustment(node, 0));
  double y = gtk_adjustment_get_value(scroll_adjustment(node, 1));
  json_object_set_double_member(body, "scroll_x", x);
  json_object_set_double_member(body, "scroll_y", y);
  json_object_set_double_member(body, "presentation_scroll_x", x);
  json_object_set_double_member(body, "presentation_scroll_y", y);
  send_object(node->scene, "ScrollerPosition", body);
}
static void snap_cancel(PaxNode *node) {
  if (node->snap_tick) {
    gtk_widget_remove_tick_callback(node->control, node->snap_tick);
    node->snap_tick = 0;
  }
  node->snap_pending = node->snap_animating = FALSE;
}
static gboolean snap_has_points(PaxNode *node) {
  return (node->snap_points[0] && node->snap_points[0]->len) ||
         (node->snap_points[1] && node->snap_points[1]->len);
}
static gboolean snap_held(PaxNode *node) {
  return node->snap_scroll_active || node->snap_buttons || node->snap_keys ||
         (node->snap_touches && g_hash_table_size(node->snap_touches));
}
static double snap_target(PaxNode *node, guint axis, double current) {
  GArray *points = node->snap_points[axis];
  if (!points || !points->len)
    return current;
  double target = g_array_index(points, double, 0);
  for (guint i = 1; i < points->len; i++) {
    double point = g_array_index(points, double, i);
    if (fabs(point - current) < fabs(target - current)) target = point;
  }
  // Small discrete wheel/key steps must advance instead of repeatedly snapping
  // back to the starting anchor. Continuous gestures choose the nearest endpoint.
  int direction = node->snap_direction[axis];
  double origin = node->snap_origin[axis];
  if (direction > 0 && current > origin + .01 && target <= origin + .01) {
    for (guint i = 0; i < points->len; i++) {
      double point = g_array_index(points, double, i);
      if (point > origin + .01) { target = point; break; }
    }
  } else if (direction < 0 && current < origin - .01 && target >= origin - .01) {
    for (guint i = points->len; i > 0; i--) {
      double point = g_array_index(points, double, i - 1);
      if (point < origin - .01) { target = point; break; }
    }
  }
  return target;
}
static gboolean snap_tick(GtkWidget *widget, GdkFrameClock *clock, gpointer data) {
  (void)widget;
  PaxNode *node = data;
  if (node->scene->stopped || !snap_has_points(node)) goto finished;
  if (node->scroll_request_pending[0] || node->scroll_request_pending[1])
    return G_SOURCE_CONTINUE;
  if (snap_held(node)) return G_SOURCE_CONTINUE;
  gint64 now = gdk_frame_clock_get_frame_time(clock);
  if (!node->snap_animating) {
    // GTK exposes no kinetic-completion signal. The alpha policy waits for
    // 150ms without native adjustment movement after all held input ends.
    if (now - node->snap_last_change < 150000) return G_SOURCE_CONTINUE;
    gboolean animations = TRUE, changed = FALSE;
    g_object_get(gtk_widget_get_settings(node->control), "gtk-enable-animations", &animations, NULL);
    for (guint axis = 0; axis < 2; axis++) {
      node->snap_from[axis] = gtk_adjustment_get_value(scroll_adjustment(node, axis));
      node->snap_to[axis] = snap_target(node, axis, node->snap_from[axis]);
      changed |= fabs(node->snap_from[axis] - node->snap_to[axis]) > .01;
    }
    if (!changed) { scroll_emit(node); goto finished; }
    node->snap_animating = TRUE;
    node->snap_started = now;
    node->snap_duration = animations ? 180000 : 0;
  }
  double fraction = node->snap_duration ?
      CLAMP((now - node->snap_started) / (double)node->snap_duration, 0, 1) : 1;
  double eased = 1 - pow(1 - fraction, 3);
  node->snap_writing = TRUE;
  for (guint axis = 0; axis < 2; axis++)
    gtk_adjustment_set_value(scroll_adjustment(node, axis),
                            node->snap_from[axis] +
                            (node->snap_to[axis] - node->snap_from[axis]) * eased);
  node->snap_writing = FALSE;
  scroll_emit(node);
  if (fraction < 1) return G_SOURCE_CONTINUE;
finished:
  node->snap_tick = 0;
  node->snap_pending = node->snap_animating = FALSE;
  node->snap_direction[0] = node->snap_direction[1] = 0;
  return G_SOURCE_REMOVE;
}
static void snap_schedule(PaxNode *node) {
  if (node->scene->stopped || !snap_has_points(node)) return;
  node->snap_pending = TRUE;
  node->snap_last_change = g_get_monotonic_time();
  if (!node->snap_tick)
    node->snap_tick = gtk_widget_add_tick_callback(node->control, snap_tick, node, NULL);
}
static int snap_compare(gconstpointer a, gconstpointer b) {
  double left = *(const double *)a, right = *(const double *)b;
  return left < right ? -1 : left > right;
}
static void snap_refresh(PaxNode *node) {
  if (!node->snap_authored[0] && !node->snap_authored[1]) return;
  gboolean changed = FALSE;
  for (guint axis = 0; axis < 2; axis++) {
    GtkAdjustment *adjustment = scroll_adjustment(node, axis);
    double limit = MAX(0, gtk_adjustment_get_upper(adjustment) -
                          gtk_adjustment_get_page_size(adjustment));
    GArray *next = g_array_new(FALSE, FALSE, sizeof(double));
    if (node->snap_authored[axis])
      for (guint i = 0; i < node->snap_authored[axis]->len; i++) {
        double point = CLAMP(g_array_index(node->snap_authored[axis], double, i), 0, limit);
        g_array_append_val(next, point);
      }
    g_array_sort(next, snap_compare);
    for (guint i = next->len; i > 1; i--)
      if (g_array_index(next, double, i-1) == g_array_index(next, double, i-2))
        g_array_remove_index(next, i-1);
    GArray *old = node->snap_points[axis];
    gboolean axis_changed = !old || old->len != next->len ||
      (next->len && memcmp(old->data, next->data, next->len * sizeof(double)));
    changed |= axis_changed;
    if (old) g_array_unref(old);
    node->snap_points[axis] = next;
  }
  if (changed) {
    snap_cancel(node);
    node->snap_direction[0] = node->snap_direction[1] = 0;
    snap_schedule(node);
  }
}
static void snap_extents_changed(GtkAdjustment *adjustment, PaxNode *node) {
  (void)adjustment;
  if (!node->scene->stopped) snap_refresh(node);
}
static void snap_input(PaxNode *node) {
  if (!node->snap_pending || node->snap_animating) {
    for (guint axis = 0; axis < 2; axis++)
      node->snap_origin[axis] = gtk_adjustment_get_value(scroll_adjustment(node, axis));
    node->snap_direction[0] = node->snap_direction[1] = 0;
  }
  if (node->snap_animating) {
    snap_cancel(node);
    // A press can interrupt without moving the adjustment. Keep the pending
    // resolution so release still settles the interrupted position.
    snap_schedule(node);
  }
  node->snap_last_change = g_get_monotonic_time();
}
static void snap_scroll_begin(GtkEventControllerScroll *controller, PaxNode *node) {
  (void)controller;
  snap_input(node);
  node->snap_scroll_active = TRUE;
  node->snap_direction[0] = node->snap_direction[1] = 0;
}
static void snap_scroll_end(GtkEventControllerScroll *controller, PaxNode *node) {
  (void)controller;
  node->snap_scroll_active = FALSE;
  if (node->snap_pending) snap_schedule(node);
}
static gboolean snap_scroll(GtkEventControllerScroll *controller, double dx,
                            double dy, PaxNode *node) {
  snap_input(node);
  GdkEvent *event = gtk_event_controller_get_current_event(GTK_EVENT_CONTROLLER(controller));
  if (event && gdk_scroll_event_get_direction(event) != GDK_SCROLL_SMOOTH) {
    if (dx) node->snap_direction[0] = dx > 0 ? 1 : -1;
    if (dy) node->snap_direction[1] = dy > 0 ? 1 : -1;
  }
  return FALSE;
}
static guint snap_key(guint key, guint *axis, int *direction) {
  *axis = 1; *direction = 0;
  switch (key) {
  case GDK_KEY_Left: case GDK_KEY_KP_Left: *axis = 0; *direction = -1; return 1;
  case GDK_KEY_Right: case GDK_KEY_KP_Right: *axis = 0; *direction = 1; return 2;
  case GDK_KEY_Up: case GDK_KEY_KP_Up: *direction = -1; return 4;
  case GDK_KEY_Down: case GDK_KEY_KP_Down: *direction = 1; return 8;
  case GDK_KEY_Page_Up: case GDK_KEY_KP_Page_Up: *direction = -1; return 16;
  case GDK_KEY_Page_Down: case GDK_KEY_KP_Page_Down: *direction = 1; return 32;
  case GDK_KEY_Home: case GDK_KEY_KP_Home: *direction = -1; return 64;
  case GDK_KEY_End: case GDK_KEY_KP_End: *direction = 1; return 128;
  default: return 0;
  }
}
static gboolean snap_event(GtkEventControllerLegacy *controller, GdkEvent *event,
                           PaxNode *node) {
  (void)controller;
  // Passive raw observation is needed for held scrollbar buttons and cancelled
  // touch sequences; claiming a competing gesture would disrupt GTK scrolling.
  gboolean was_held = snap_held(node);
  GdkEventType type = gdk_event_get_event_type(event);
  if (type == GDK_BUTTON_PRESS || type == GDK_BUTTON_RELEASE) {
    guint button = gdk_button_event_get_button(event);
    if (button > 0 && button <= 32) {
      guint mask = 1u << (button - 1);
      if (type == GDK_BUTTON_PRESS) { snap_input(node); node->snap_buttons |= mask; }
      else node->snap_buttons &= ~mask;
    }
  } else if (type == GDK_TOUCH_BEGIN || type == GDK_TOUCH_END || type == GDK_TOUCH_CANCEL) {
    GdkEventSequence *sequence = gdk_event_get_event_sequence(event);
    if (type == GDK_TOUCH_BEGIN) {
      snap_input(node);
      g_hash_table_add(node->snap_touches, sequence);
    } else g_hash_table_remove(node->snap_touches, sequence);
  } else if (type == GDK_KEY_PRESS || type == GDK_KEY_RELEASE) {
    guint axis; int direction;
    guint key = snap_key(gdk_key_event_get_keyval(event), &axis, &direction);
    if (type == GDK_KEY_PRESS && key) {
      snap_input(node); node->snap_keys |= key; node->snap_direction[axis] = direction;
    } else node->snap_keys &= ~key;
  } else if (type == GDK_GRAB_BROKEN) {
    node->snap_buttons = node->snap_keys = 0;
    node->snap_scroll_active = FALSE;
    g_hash_table_remove_all(node->snap_touches);
  }
  if (was_held && !snap_held(node) && node->snap_pending) snap_schedule(node);
  return FALSE;
}
static void snap_focus_leave(GtkEventControllerFocus *controller, PaxNode *node) {
  (void)controller;
  gboolean held_key = node->snap_keys != 0;
  node->snap_keys = 0;
  if (held_key && !snap_held(node) && node->snap_pending) snap_schedule(node);
}
static void snap_disconnect_window(PaxNode *node) {
  if (node->snap_window) {
    g_signal_handler_disconnect(node->snap_window, node->snap_window_signal);
    g_object_remove_weak_pointer(G_OBJECT(node->snap_window), (gpointer *)&node->snap_window);
    node->snap_window = NULL;
  }
  node->snap_window_signal = 0;
}
static void snap_window_active(GtkWindow *window, GParamSpec *property, PaxNode *node) {
  (void)property;
  if (gtk_window_is_active(window)) return;
  // Deactivation can deliver the final release to a different native window.
  gboolean was_held = snap_held(node);
  node->snap_buttons = node->snap_keys = 0;
  node->snap_scroll_active = FALSE;
  g_hash_table_remove_all(node->snap_touches);
  if (was_held && node->snap_pending) snap_schedule(node);
}
static void snap_map(GtkWidget *widget, PaxNode *node) {
  (void)widget;
  GtkRoot *root = gtk_widget_get_root(node->control);
  if (GTK_IS_WINDOW(root) && node->snap_window != GTK_WINDOW(root)) {
    snap_disconnect_window(node);
    node->snap_window = GTK_WINDOW(root);
    g_object_add_weak_pointer(G_OBJECT(root), (gpointer *)&node->snap_window);
    node->snap_window_signal = g_signal_connect_object(root, "notify::is-active",
                                                       G_CALLBACK(snap_window_active), node, 0);
  }
  if (node->snap_pending) snap_schedule(node);
}
static void snap_unmap(GtkWidget *widget, PaxNode *node) {
  (void)widget;
  snap_disconnect_window(node);
  gboolean pending = node->snap_pending;
  snap_cancel(node);
  // A hidden widget has no useful frame clock. Resume pending work on map,
  // keeping the current offset rather than advancing an invisible animation.
  node->snap_pending = pending;
  node->snap_direction[0] = node->snap_direction[1] = 0;
  node->snap_buttons = node->snap_keys = 0;
  node->snap_scroll_active = FALSE;
  g_hash_table_remove_all(node->snap_touches);
}
static void scroll_changed(GtkAdjustment *adjustment, PaxNode *node) {
  (void)adjustment;
  if (node->updating || node->scene->stopped || node->snap_writing ||
      node->scroll_request_pending[0] || node->scroll_request_pending[1]) return;
  if (node->snap_animating) snap_cancel(node);
  scroll_emit(node);
  snap_schedule(node);
}
static void node_matrix(PaxNode *node, graphene_matrix_t *matrix) {
  graphene_matrix_init_from_2d(matrix, node->matrix[0], node->matrix[1],
                               node->matrix[2], node->matrix[3],
                               node->matrix[4], node->matrix[5]);
}
static gboolean node_matrix_invertible(PaxNode *node) {
  for (guint i = 0; i < 6; i++)
    if (!isfinite(node->matrix[i])) return FALSE;
  double determinant = node->matrix[0] * node->matrix[3] -
                       node->matrix[1] * node->matrix[2];
  return isfinite(determinant) && fabs(determinant) >= 1e-12;
}
static gboolean path_contains(GskPath *path, double x, double y) {
  graphene_point_t p = GRAPHENE_POINT_INIT(x, y);
  return gsk_path_in_fill(path, &p, GSK_FILL_RULE_WINDING);
}
static gboolean coverage(PaxNode *node, double x, double y) {
  if (node->opacity <= 0)
    return FALSE;
  if (node->clip) {
    if (x < 0 || y < 0 || x >= node->width || y >= node->height)
      return FALSE;
    double r = MIN(node->radius, MIN(node->width, node->height) / 2);
    double cx = CLAMP(x, r, node->width - r),
           cy = CLAMP(y, r, node->height - r);
    if (r > 0 && (x - cx) * (x - cx) + (y - cy) * (y - cy) > r * r)
      return FALSE;
    if (node->clip_path) {
      double wx = node->matrix[0] * x + node->matrix[2] * y + node->matrix[4];
      double wy = node->matrix[1] * x + node->matrix[3] * y + node->matrix[5];
      if (!path_contains(node->clip_path, wx, wy))
        return FALSE;
    }
  }
  for (guint i = 0; i < node->mask->len; i++) {
    MaskEntry *entry = g_ptr_array_index(node->mask, i);
    if (entry->opacity < 1.0 || !path_contains(entry->path, x, y))
      continue;
    gboolean inside = TRUE;
    for (guint j = 0; j < entry->clips->len; j++)
      inside &= path_contains(g_ptr_array_index(entry->clips, j), x, y);
    if (inside)
      return FALSE;
  }
  return TRUE;
}
static gboolean input_contains(GtkWidget *widget, double x, double y) {
  // GTK picks children before checking their parent's contains(). This overlay
  // owns masked holes only within its allocation, never neighboring controls.
  return x >= 0 && y >= 0 && x < gtk_widget_get_width(widget) &&
         y < gtk_widget_get_height(widget) &&
         !coverage(((PaxInput *)widget)->node, x, y);
}
static void input_snapshot(GtkWidget *widget, GtkSnapshot *snapshot) {
  (void)widget;
  (void)snapshot;
}
static void pax_input_class_init(PaxInputClass *klass) {
  klass->contains = input_contains;
  klass->snapshot = input_snapshot;
}
static void pax_input_init(PaxInput *input) { (void)input; }
static void append_modifiers(JsonObject *body, GdkModifierType state) {
  JsonArray *array = json_array_new();
  if (state & GDK_SHIFT_MASK)
    json_array_add_string_element(array, "Shift");
  if (state & GDK_CONTROL_MASK)
    json_array_add_string_element(array, "Control");
  if (state & GDK_ALT_MASK)
    json_array_add_string_element(array, "Alt");
  if (state & (GDK_SUPER_MASK | GDK_META_MASK))
    json_array_add_string_element(array, "Command");
  json_object_set_array_member(body, "modifiers", array);
}
static gboolean canvas_hit(PaxNativeScene *scene, double x, double y) {
  GtkWidget *picked = gtk_widget_pick(GTK_WIDGET(scene), x, y, GTK_PICK_DEFAULT);
  for (GtkWidget *widget = picked; widget && widget != GTK_WIDGET(scene);
       widget = gtk_widget_get_parent(widget)) {
    if (G_TYPE_CHECK_INSTANCE_TYPE(widget, pax_input_get_type()))
      return TRUE;
    if (G_TYPE_CHECK_INSTANCE_TYPE(widget, pax_node_get_type())) {
      PaxNode *node = (PaxNode *)widget;
      if (node->control && !node->content)
        return FALSE;
    }
  }
  return TRUE;
}
static void wheel_pointer_free(gpointer data) {
  WheelPointer *pointer = data;
  g_clear_object(&pointer->surface);
  g_free(pointer);
}
static void remember_wheel_pointer(PaxNativeScene *scene, GdkEvent *event) {
  if (scene->stopped || !event) return;
  GdkSeat *seat = gdk_event_get_seat(event);
  GdkSurface *surface = gdk_event_get_surface(event);
  double x, y;
  if (!seat || !surface || !gdk_event_get_position(event, &x, &y)) return;
  WheelPointer *pointer = g_hash_table_lookup(scene->wheel_pointers, seat);
  if (!pointer) {
    pointer = g_new0(WheelPointer, 1);
    g_hash_table_insert(scene->wheel_pointers, g_object_ref(seat), pointer);
  }
  g_set_object(&pointer->surface, surface);
  pointer->x = x; pointer->y = y;
}
static void wheel_pointer_enter(GtkEventControllerMotion *controller,
                                double x, double y, PaxNativeScene *scene) {
  (void)x; (void)y;
  remember_wheel_pointer(scene,
      gtk_event_controller_get_current_event(GTK_EVENT_CONTROLLER(controller)));
}
static void wheel_pointer_leave(GtkEventControllerMotion *controller, PaxNativeScene *scene) {
  GdkEvent *event = gtk_event_controller_get_current_event(GTK_EVENT_CONTROLLER(controller));
  GdkSeat *seat = event ? gdk_event_get_seat(event) : NULL;
  if (seat) g_hash_table_remove(scene->wheel_pointers, seat);
  else g_hash_table_remove_all(scene->wheel_pointers);
}
static void restore_pointer_focus_reveal(PaxNativeScene *scene) {
  if (scene->pointer_focus_restore_source) {
    g_source_remove(scene->pointer_focus_restore_source);
    scene->pointer_focus_restore_source = 0;
  }
  if (!scene->pointer_focus_viewports) return;
  // Only viewports that were enabled were saved; an authored/host-disabled
  // viewport keeps its original setting. Re-enabling attaches GTK's focus
  // listener without scrolling to the already-focused widget.
  for (guint i = 0; i < scene->pointer_focus_viewports->len; i++)
    gtk_viewport_set_scroll_to_focus(g_ptr_array_index(scene->pointer_focus_viewports, i), TRUE);
  g_clear_pointer(&scene->pointer_focus_viewports, g_ptr_array_unref);
}
static gboolean restore_pointer_focus_idle(gpointer data) {
  PaxNativeScene *scene = data;
  scene->pointer_focus_restore_source = 0;
  restore_pointer_focus_reveal(scene);
  return G_SOURCE_REMOVE;
}
static void preserve_label_pointer_focus(GtkEventController *controller,
                                         GdkEvent *event, PaxNativeScene *scene) {
  GdkEventType type = gdk_event_get_event_type(event);
  if (type == GDK_KEY_PRESS || type == GDK_GRAB_BROKEN ||
      type == GDK_TOUCH_CANCEL || type == GDK_BUTTON_RELEASE || type == GDK_TOUCH_END)
    restore_pointer_focus_reveal(scene);
  if (scene->stopped || (type != GDK_BUTTON_PRESS && type != GDK_TOUCH_BEGIN)) return;
  restore_pointer_focus_reveal(scene);
  if (type == GDK_BUTTON_PRESS && gdk_button_event_get_button(event) != GDK_BUTTON_PRIMARY) return;
  double x, y, sx, sy;
  if (!gdk_event_get_position(event, &x, &y)) return;
  GtkNative *native = gtk_widget_get_native(gtk_event_controller_get_widget(controller));
  if (!native) return;
  gtk_native_get_surface_transform(native, &sx, &sy);
  graphene_point_t from = GRAPHENE_POINT_INIT(x + sx, y + sy), to;
  if (!gtk_widget_compute_point(GTK_WIDGET(native), GTK_WIDGET(scene), &from, &to)) return;
  GtkWidget *label = gtk_widget_pick(GTK_WIDGET(scene), to.x, to.y, GTK_PICK_DEFAULT);
  if (!GTK_IS_LABEL(label) || !gtk_label_get_selectable(GTK_LABEL(label)) ||
      gtk_widget_has_focus(label)) return;
  // GtkLabel focuses itself during its native press handler. GtkViewport
  // otherwise reveals the label's entire (possibly huge) bounds synchronously,
  // moving the text under the pointer before selection starts. Suspend only
  // that pointer dispatch's ancestor reveal; GTK still owns focus/selection.
  // Restore before keyboard events so Tab/programmatic focus keeps its reveal.
  for (GtkWidget *parent = gtk_widget_get_parent(label); parent && parent != GTK_WIDGET(scene);
       parent = gtk_widget_get_parent(parent)) {
    if (!GTK_IS_VIEWPORT(parent) || !gtk_viewport_get_scroll_to_focus(GTK_VIEWPORT(parent))) continue;
    GtkWidget *scroll = gtk_widget_get_parent(parent);
    GtkWidget *owner = scroll ? gtk_widget_get_parent(scroll) : NULL;
    if (!GTK_IS_SCROLLED_WINDOW(scroll) || !owner ||
        !G_TYPE_CHECK_INSTANCE_TYPE(owner, pax_node_get_type()) ||
        ((PaxNode *)owner)->control != scroll || strcmp(((PaxNode *)owner)->kind, "Scroller"))
      continue;
    if (!scene->pointer_focus_viewports)
      scene->pointer_focus_viewports = g_ptr_array_new_with_free_func(g_object_unref);
    g_ptr_array_add(scene->pointer_focus_viewports, g_object_ref(parent));
    gtk_viewport_set_scroll_to_focus(GTK_VIEWPORT(parent), FALSE);
  }
  if (scene->pointer_focus_viewports)
    scene->pointer_focus_restore_source = g_idle_add_full(G_PRIORITY_HIGH_IDLE,
        restore_pointer_focus_idle, scene, NULL);
}
static gboolean pointer_event(GtkEventControllerLegacy *controller,
                              GdkEvent *event, PaxNativeScene *scene) {
  preserve_label_pointer_focus(GTK_EVENT_CONTROLLER(controller), event, scene);
  GdkEventType type = gdk_event_get_event_type(event);
  if (type == GDK_GRAB_BROKEN || type == GDK_TOUCH_CANCEL) {
    memset(scene->pointer_buttons, 0, sizeof scene->pointer_buttons);
    if (type == GDK_GRAB_BROKEN) {
      GdkSeat *seat = gdk_event_get_seat(event);
      if (seat) g_hash_table_remove(scene->wheel_pointers, seat);
      else g_hash_table_remove_all(scene->wheel_pointers);
    }
    return FALSE;
  }
  const char *kind = NULL;
  if (type == GDK_BUTTON_PRESS)
    kind = "MouseDown";
  else if (type == GDK_BUTTON_RELEASE)
    kind = "MouseUp";
  else if (type == GDK_MOTION_NOTIFY)
    kind = "MouseMove";
  else
    return FALSE;
  if (type == GDK_BUTTON_PRESS)
    scene->scene_pointer_presses++;
  double x, y;
  if (!gdk_event_get_position(event, &x, &y))
    return FALSE;
  remember_wheel_pointer(scene, event);
  GtkWidget *widget =
      gtk_event_controller_get_widget(GTK_EVENT_CONTROLLER(controller));
  GtkNative *native = gtk_widget_get_native(widget);
  double sx, sy;
  gtk_native_get_surface_transform(native, &sx, &sy);
  graphene_point_t from = GRAPHENE_POINT_INIT(x + sx, y + sy), to;
  if (!gtk_widget_compute_point(GTK_WIDGET(native), GTK_WIDGET(scene), &from,
                                &to))
    return FALSE;
  if (type == GDK_BUTTON_PRESS && canvas_hit(scene, to.x, to.y))
    gtk_widget_grab_focus(GTK_WIDGET(scene));
  JsonObject *body = json_object_new();
  json_object_set_double_member(body, "x", to.x);
  json_object_set_double_member(body, "y", to.y);
  guint button =
      type == GDK_MOTION_NOTIFY ? 1 : gdk_button_event_get_button(event);
  json_object_set_string_member(body, "button",
                                button == 3   ? "Right"
                                : button == 2 ? "Middle"
                                              : "Left");
  append_modifiers(body, gdk_event_get_modifier_state(event));
  // A raw release is not a click: native Text selection and scrollbar drags
  // still deliver pointer events, but must not activate a surrounding Link.
  for (guint i = 0; i < G_N_ELEMENTS(scene->pointer_buttons); i++) {
    if (scene->pointer_buttons[i].pressed && gtk_drag_check_threshold(
            GTK_WIDGET(scene), (int)scene->pointer_buttons[i].x,
            (int)scene->pointer_buttons[i].y, (int)to.x, (int)to.y))
      scene->pointer_buttons[i].dragged = TRUE;
  }
  if (type == GDK_BUTTON_PRESS && button >= 1 && button <= 3) {
    scene->pointer_buttons[button - 1].x = to.x;
    scene->pointer_buttons[button - 1].y = to.y;
    scene->pointer_buttons[button - 1].pressed = TRUE;
    scene->pointer_buttons[button - 1].dragged = FALSE;
  }
  if (type == GDK_BUTTON_RELEASE) {
    scene->scene_pointer_releases++;
    gboolean click = button >= 1 && button <= 3 &&
        scene->pointer_buttons[button - 1].pressed &&
        !scene->pointer_buttons[button - 1].dragged && to.x >= 0 && to.y >= 0 &&
        to.x < gtk_widget_get_width(GTK_WIDGET(scene)) &&
        to.y < gtk_widget_get_height(GTK_WIDGET(scene));
    if (button >= 1 && button <= 3)
      scene->pointer_buttons[button - 1].pressed = FALSE;
    JsonObject *copy = click ? json_object_ref(body) : NULL;
    send_object(scene, kind, body);
    if (copy) {
      scene->scene_pointer_clicks++;
      send_object(scene, "Click", copy);
    }
  } else
    send_object(scene, kind, body);
  return FALSE;
}
static void add_pointer_controller(GtkWidget *widget, PaxNativeScene *scene) {
  GtkEventController *controller = gtk_event_controller_legacy_new();
  // Observe once before native labels/editors consume their events. Returning
  // FALSE preserves GTK selection, focus and gestures while engine handlers
  // (including a Link around native text) receive the same pointer sequence.
  gtk_event_controller_set_propagation_phase(controller, GTK_PHASE_CAPTURE);
  g_signal_connect(controller, "event", G_CALLBACK(pointer_event), scene);
  gtk_widget_add_controller(widget, controller);
}
static gboolean wheel_event(GtkEventControllerScroll *controller, double dx,
                             double dy, PaxNativeScene *scene) {
  if (scene->stopped || (!dx && !dy) || !isfinite(dx) || !isfinite(dy))
    return FALSE;
  GdkEvent *event = gtk_event_controller_get_current_event(GTK_EVENT_CONTROLLER(controller));
  double x, y;
  if (!event)
    return FALSE;
  GdkSurface *surface = gdk_event_get_surface(event);
  // GDK scroll events need not carry a position. Preserve preceding motion/enter
  // order: a live pointer query alone can see motion queued after this wheel and
  // hit a different Pax node than GTK's pointer-focus target under UI backlog.
  // Surface coordinates also survive changes to the scene's embedding offset.
  if (gdk_event_get_position(event, &x, &y)) {
    remember_wheel_pointer(scene, event);
  } else {
    GdkSeat *seat = gdk_event_get_seat(event);
    WheelPointer *pointer = seat ? g_hash_table_lookup(scene->wheel_pointers, seat) : NULL;
    if (pointer && pointer->surface == surface) {
      x = pointer->x; y = pointer->y;
    } else {
      // No prior pointer event is available (for example, a newly mapped scene).
      // Query the logical seat pointer; X11 source slave devices cannot be queried.
      GdkDevice *device = seat ? gdk_seat_get_pointer(seat) : NULL;
      if (!surface || !device || !gdk_surface_get_device_position(surface, device, &x, &y, NULL))
        return FALSE;
    }
  }
  GtkNative *native = surface ? gtk_native_get_for_surface(surface) : NULL;
  if (!native)
    return FALSE;
  double sx, sy;
  gtk_native_get_surface_transform(native, &sx, &sy);
  graphene_point_t from = GRAPHENE_POINT_INIT(x + sx, y + sy), point;
  if (!gtk_widget_compute_point(GTK_WIDGET(native), GTK_WIDGET(scene), &from, &point))
    return FALSE;
  JsonObject *body = json_object_new();
  json_object_set_double_member(body, "x", point.x);
  json_object_set_double_member(body, "y", point.y);
  json_object_set_double_member(body, "delta_x", dx);
  json_object_set_double_member(body, "delta_y", dy);
  append_modifiers(body, gdk_event_get_modifier_state(event));
  // This is raw input, not a native offset acknowledgement. Capture once above
  // all native descendants, and let GTK perform its default only if permitted.
  // BOTH_AXES retains device units and natural direction; no pixel conversion,
  // discrete rounding, or synthetic kinetic events belong in the Pax bridge.
  return send_object(scene, "Wheel", body);
}
static const char *key_name(guint key, char utf8[8]) {
  switch (key) {
  case GDK_KEY_Return:
  case GDK_KEY_KP_Enter:
    return "Enter";
  case GDK_KEY_Tab:
  case GDK_KEY_ISO_Left_Tab:
    return "Tab";
  case GDK_KEY_Escape:
    return "Escape";
  case GDK_KEY_BackSpace:
    return "Backspace";
  case GDK_KEY_Delete:
  case GDK_KEY_KP_Delete:
    return "Delete";
  case GDK_KEY_Left:
  case GDK_KEY_KP_Left:
    return "ArrowLeft";
  case GDK_KEY_Right:
  case GDK_KEY_KP_Right:
    return "ArrowRight";
  case GDK_KEY_Up:
  case GDK_KEY_KP_Up:
    return "ArrowUp";
  case GDK_KEY_Down:
  case GDK_KEY_KP_Down:
    return "ArrowDown";
  case GDK_KEY_Page_Up:
  case GDK_KEY_KP_Page_Up:
    return "PageUp";
  case GDK_KEY_Page_Down:
  case GDK_KEY_KP_Page_Down:
    return "PageDown";
  case GDK_KEY_Shift_L:
  case GDK_KEY_Shift_R:
    return "Shift";
  case GDK_KEY_Control_L:
  case GDK_KEY_Control_R:
    return "Control";
  case GDK_KEY_Alt_L:
  case GDK_KEY_Alt_R:
    return "Alt";
  case GDK_KEY_Super_L:
  case GDK_KEY_Super_R:
  case GDK_KEY_Meta_L:
  case GDK_KEY_Meta_R:
    return "Meta";
  default:
    break;
  }
  gunichar unicode = gdk_keyval_to_unicode(key);
  if (unicode && g_unichar_isprint(unicode)) {
    g_unichar_to_utf8(unicode, utf8);
    return utf8;
  }
  const char *name = gdk_keyval_name(key);
  return name ? name : "";
}
static gboolean key_pressed(GtkEventControllerKey *controller, guint key,
                            guint code, GdkModifierType state,
                            PaxNativeScene *scene) {
  (void)controller;
  // Capture observes native descendants too; their editing/shortcuts must stay
  // inside GTK rather than also dispatching through the mounted Pax tree.
  if (!gtk_widget_has_focus(GTK_WIDGET(scene)))
    return FALSE;
  JsonObject *body = json_object_new();
  char utf8[8] = {0};
  json_object_set_string_member(body, "key", key_name(key, utf8));
  json_object_set_boolean_member(
      body, "is_repeat",
      g_hash_table_contains(scene->pressed_keys, GUINT_TO_POINTER(code)));
  append_modifiers(body, state);
  if (!g_hash_table_contains(scene->pressed_keys, GUINT_TO_POINTER(code)))
    g_hash_table_insert(scene->pressed_keys, GUINT_TO_POINTER(code), json_object_ref(body));
  JsonObject *press = json_object_ref(body);
  gboolean prevent = send_object(scene, "KeyDown", body);
  gunichar unicode = gdk_keyval_to_unicode(key);
  if (!prevent && !scene->stopped && gtk_widget_has_focus(GTK_WIDGET(scene)) &&
      !(state & (GDK_CONTROL_MASK | GDK_SUPER_MASK | GDK_META_MASK)) &&
      ((unicode && g_unichar_isprint(unicode)) || key == GDK_KEY_Return || key == GDK_KEY_KP_Enter))
    prevent = send_object(scene, "KeyPress", press);
  else
    json_object_unref(press);
  return prevent;
}
static void key_released(GtkEventControllerKey *controller, guint key,
                         guint code, GdkModifierType state,
                         PaxNativeScene *scene) {
  (void)controller;
  (void)key;
  JsonObject *body = g_hash_table_lookup(scene->pressed_keys, GUINT_TO_POINTER(code));
  if (!body)
    return;
  json_object_ref(body);
  g_hash_table_remove(scene->pressed_keys, GUINT_TO_POINTER(code));
  json_object_set_boolean_member(body, "is_repeat", FALSE);
  append_modifiers(body, state);
  send_object(scene, "KeyUp", body);
}
void pax_native_scene_release_keys(PaxNativeScene *scene) {
  GtkRoot *root = gtk_widget_get_root(GTK_WIDGET(scene));
  if (!gtk_widget_get_mapped(GTK_WIDGET(scene)) ||
      (GTK_IS_WINDOW(root) && !gtk_window_is_active(GTK_WINDOW(root))))
    memset(scene->pointer_buttons, 0, sizeof scene->pointer_buttons);
  // Remove first: a KeyUp handler can itself change focus or close the mount.
  GList *keys = g_hash_table_get_values(scene->pressed_keys);
  for (GList *item = keys; item; item = item->next)
    json_object_ref(item->data);
  g_hash_table_remove_all(scene->pressed_keys);
  for (GList *item = keys; item; item = item->next) {
    JsonObject *body = item->data;
    json_object_set_boolean_member(body, "is_repeat", FALSE);
    append_modifiers(body, 0);
    send_object(scene, "KeyUp", body);
  }
  g_list_free(keys);
}
static void canvas_focus_changed(GtkEventControllerFocus *controller,
                                 GParamSpec *property, PaxNativeScene *scene) {
  (void)property;
  if (!gtk_event_controller_focus_is_focus(controller))
    pax_native_scene_release_keys(scene);
}
static void node_measure(GtkWidget *widget, GtkOrientation orientation,
                         int for_size, int *minimum, int *natural,
                         int *min_baseline, int *nat_baseline) {
  (void)for_size;
  PaxNode *node = (PaxNode *)widget;
  *minimum = *natural = (int)ceil(orientation == GTK_ORIENTATION_HORIZONTAL
                                      ? MAX(0, node->width)
                                      : MAX(0, node->height));
  if (min_baseline)
    *min_baseline = -1;
  if (nat_baseline)
    *nat_baseline = -1;
}
static void node_allocate(GtkWidget *widget, int width, int height,
                          int baseline) {
  PaxNode *node = (PaxNode *)widget;
  gboolean requested = node->scroll_request_pending[0] || node->scroll_request_pending[1];
  gboolean updating = node->updating;
  if (requested)
    node->updating = TRUE;
  if (node->control)
    gtk_widget_allocate(node->control, width, height, baseline, NULL);
  if (requested) {
    // GtkScrolledWindow establishes adjustment ranges during allocation. A
    // request applied earlier can clamp against zero or the previous extent,
    // then overwrite the engine's bound property with that premature result.
    for (guint axis = 0; axis < 2; axis++) {
      if (node->scroll_request_pending[axis])
        gtk_adjustment_set_value(scroll_adjustment(node, axis), node->requested_scroll[axis]);
      node->scroll_request_pending[axis] = FALSE;
    }
    node->updating = updating;
    if (!node->scene->stopped) {
      scroll_emit(node);
      snap_schedule(node);
    }
  }
  if (node->input)
    gtk_widget_allocate(node->input, width, height, baseline, NULL);
}
static void node_snapshot(GtkWidget *widget, GtkSnapshot *snapshot) {
  PaxNode *node = (PaxNode *)widget;
  int clips = 0;
  gboolean transformed_clip = FALSE;
  if (node->clip) {
    graphene_rect_t bounds =
        GRAPHENE_RECT_INIT(0, 0, node->width, node->height);
    if (node->radius > 0) {
      GskRoundedRect rounded;
      gsk_rounded_rect_init_from_rect(&rounded, &bounds, node->radius);
      gtk_snapshot_push_rounded_clip(snapshot, &rounded);
    } else
      gtk_snapshot_push_clip(snapshot, &bounds);
    clips++;
    if (node->clip_path) {
      graphene_matrix_t world, inverse;
      double *m = node->authored_transform;
      graphene_matrix_init_from_2d(&world, m[0], m[1], m[2], m[3], m[4], m[5]);
      if (graphene_matrix_inverse(&world, &inverse)) {
        gtk_snapshot_save(snapshot);
        transformed_clip = TRUE;
        gtk_snapshot_transform_matrix(snapshot, &inverse);
        gtk_snapshot_push_fill(snapshot, node->clip_path,
                               GSK_FILL_RULE_WINDING);
        gtk_snapshot_transform_matrix(snapshot, &world);
        clips++;
      }
    }
  }
  if (node->mask->len) {
    gtk_snapshot_push_mask(snapshot, GSK_MASK_MODE_INVERTED_ALPHA);
    for (guint i = 0; i < node->mask->len; i++) {
      MaskEntry *entry = g_ptr_array_index(node->mask, i);
      for (guint j = 0; j < entry->clips->len; j++)
        gtk_snapshot_push_fill(snapshot, g_ptr_array_index(entry->clips, j),
                               GSK_FILL_RULE_WINDING);
      GdkRGBA white = {1, 1, 1, entry->opacity};
      gtk_snapshot_append_fill(snapshot, entry->path, GSK_FILL_RULE_WINDING,
                               &white);
      for (guint j = 0; j < entry->clips->len; j++)
        gtk_snapshot_pop(snapshot);
    }
    gtk_snapshot_pop(snapshot);
  }
  if (node->blocker_background.alpha > 0) {
    // Paint in the native node's masked coordinate space. Transparency changes
    // its ink only; the retained input blocker still covers the same bounds.
    graphene_rect_t rect = GRAPHENE_RECT_INIT(0, 0, node->width, node->height);
    gtk_snapshot_append_color(snapshot, &node->blocker_background, &rect);
  }
  if (node->texture) {
    graphene_rect_t rect = GRAPHENE_RECT_INIT(0, 0, node->width, node->height);
    gtk_snapshot_append_texture(snapshot, node->texture, &rect);
  }
  if (node->control)
    gtk_widget_snapshot_child(widget, node->control, snapshot);
  if (node->mask->len)
    gtk_snapshot_pop(snapshot);
  if (transformed_clip) {
    gtk_snapshot_pop(snapshot);
    clips--;
    // The transform save belongs to the outer clip state. Restore it before
    // popping that clip, rather than crossing GTK's snapshot state boundary.
    gtk_snapshot_restore(snapshot);
  }
  while (clips--)
    gtk_snapshot_pop(snapshot);
}
static void node_dispose(GObject *object) {
  PaxNode *node = (PaxNode *)object;
  node->updating = TRUE;
  native_image_release(node);
  snap_disconnect_window(node);
  snap_cancel(node);
  if (node->form_css) {
    gtk_style_context_remove_provider_for_display(
        gtk_widget_get_display(GTK_WIDGET(node)), GTK_STYLE_PROVIDER(node->form_css));
    g_clear_object(&node->form_css);
  }
  if (node->transparent_text_css) {
    gtk_style_context_remove_provider_for_display(
        gtk_widget_get_display(GTK_WIDGET(node)), GTK_STYLE_PROVIDER(node->transparent_text_css));
    g_clear_object(&node->transparent_text_css);
  }
  while (gtk_widget_get_first_child(GTK_WIDGET(node)))
    gtk_widget_unparent(gtk_widget_get_first_child(GTK_WIDGET(node)));
  node->control = node->input = node->content = NULL;
  g_clear_object(&node->texture);
  g_clear_object(&node->options_model);
  g_clear_object(&node->radio_group);
  G_OBJECT_CLASS(pax_node_parent_class)->dispose(object);
}
static void node_finalize(GObject *object) {
  PaxNode *node = (PaxNode *)object;
  for (guint axis = 0; axis < 2; axis++) {
    g_clear_pointer(&node->snap_authored[axis], g_array_unref);
    g_clear_pointer(&node->snap_points[axis], g_array_unref);
  }
  g_clear_pointer(&node->snap_touches, g_hash_table_unref);
  g_free(node->image_url);
  g_free(node->kind);
  g_free(node->tile_key);
  g_strfreev(node->options);
  g_free(node->form_css_class);
  g_free(node->transparent_text_class);
  g_clear_pointer(&node->form_attributes, pango_attr_list_unref);
  g_clear_pointer(&node->form_appearance, json_object_unref);
  g_clear_pointer(&node->rich_text, json_object_unref);
  g_clear_pointer(&node->link_style, json_object_unref);
  g_ptr_array_unref(node->mask);
  if (node->clip_path)
    gsk_path_unref(node->clip_path);
  G_OBJECT_CLASS(pax_node_parent_class)->finalize(object);
}
static void pax_node_class_init(PaxNodeClass *klass) {
  klass->measure = node_measure;
  klass->size_allocate = node_allocate;
  klass->snapshot = node_snapshot;
  G_OBJECT_CLASS(klass)->dispose = node_dispose;
  G_OBJECT_CLASS(klass)->finalize = node_finalize;
}
static void pax_node_init(PaxNode *node) {
  node->matrix[0] = node->matrix[3] = node->opacity = 1;
  node->authored_transform[0] = node->authored_transform[3] = 1;
  node->transform_scroller = -1;
  node->text_width_constraint = node->text_height_constraint = -1;
  node->parent_id = -1;
  node->mask = g_ptr_array_new_with_free_func(mask_free);
  node->range_max = 100;
  node->range_step = 1;
  node->form_yalign = .5;
  node->options = g_new0(char *, 1);
}
static GtkWidget *scene_root(PaxNativeScene *scene) {
  return gtk_widget_get_first_child(GTK_WIDGET(scene));
}
static void scene_measure(GtkWidget *widget, GtkOrientation orientation,
                          int for_size, int *minimum, int *natural,
                          int *min_baseline, int *nat_baseline) {
  (void)widget;
  (void)orientation;
  (void)for_size;
  *minimum = 0;
  *natural = 0;
  if (min_baseline)
    *min_baseline = -1;
  if (nat_baseline)
    *nat_baseline = -1;
}
static void scene_allocate(GtkWidget *widget, int width, int height,
                           int baseline) {
  GtkWidget *root = scene_root((PaxNativeScene *)widget);
  if (root)
    gtk_widget_allocate(root, width, height, baseline, NULL);
}
static void scene_snapshot(GtkWidget *widget, GtkSnapshot *snapshot) {
  GtkWidget *root = scene_root((PaxNativeScene *)widget);
  if (root)
    gtk_widget_snapshot_child(widget, root, snapshot);
}
static void scene_system_setting_changed(GtkWidget *widget,
                                          GtkSystemSetting setting) {
  // GTK refreshes the Pango contexts throughout this subtree first. Defer the
  // engine notification until an accepted frame, including an empty batch.
  GTK_WIDGET_CLASS(pax_native_scene_parent_class)
      ->system_setting_changed(widget, setting);
  PaxNativeScene *scene = (PaxNativeScene *)widget;
  if (!scene->stopped &&
      (setting == GTK_SYSTEM_SETTING_DPI ||
       setting == GTK_SYSTEM_SETTING_FONT_NAME ||
       setting == GTK_SYSTEM_SETTING_FONT_CONFIG ||
       setting == GTK_SYSTEM_SETTING_DISPLAY))
    scene->text_metrics_dirty = TRUE;
}
static void scene_dispose(GObject *object) {
  PaxNativeScene *scene = (PaxNativeScene *)object;
  scene->stopped = TRUE;
  native_images_stop(scene);
  restore_pointer_focus_reveal(scene);
  memset(scene->pointer_buttons, 0, sizeof scene->pointer_buttons);
  g_hash_table_remove_all(scene->wheel_pointers);
  g_hash_table_remove_all(scene->tiles);
  g_hash_table_remove_all(scene->nodes);
  GtkWidget *root = scene_root(scene);
  if (root)
    gtk_widget_unparent(root);
  G_OBJECT_CLASS(pax_native_scene_parent_class)->dispose(object);
}
static void scene_finalize(GObject *object) {
  PaxNativeScene *scene = (PaxNativeScene *)object;
  g_strfreev(scene->assets);
  g_hash_table_unref(scene->nodes);
  g_hash_table_unref(scene->native_images);
  g_hash_table_unref(scene->tiles);
  g_hash_table_unref(scene->layer_owners);
  g_hash_table_unref(scene->pressed_keys);
  g_hash_table_unref(scene->wheel_pointers);
  G_OBJECT_CLASS(pax_native_scene_parent_class)->finalize(object);
}
static void scene_unmap(GtkWidget *widget) {
  PaxNativeScene *scene = (PaxNativeScene *)widget;
  restore_pointer_focus_reveal(scene);
  memset(scene->pointer_buttons, 0, sizeof scene->pointer_buttons);
  g_hash_table_remove_all(scene->wheel_pointers);
  GTK_WIDGET_CLASS(pax_native_scene_parent_class)->unmap(widget);
}
static void pax_native_scene_class_init(PaxNativeSceneClass *klass) {
  klass->unmap = scene_unmap;
  klass->measure = scene_measure;
  klass->size_allocate = scene_allocate;
  klass->snapshot = scene_snapshot;
  klass->system_setting_changed = scene_system_setting_changed;
  G_OBJECT_CLASS(klass)->dispose = scene_dispose;
  G_OBJECT_CLASS(klass)->finalize = scene_finalize;
}
static void pax_native_scene_init(PaxNativeScene *scene) {
  scene->nodes = g_hash_table_new_full(g_direct_hash, g_direct_equal, NULL,
                                       g_object_unref);
  scene->tiles =
      g_hash_table_new_full(g_str_hash, g_str_equal, g_free, g_object_unref);
  scene->layer_owners = g_hash_table_new(g_direct_hash, g_direct_equal);
  scene->pressed_keys = g_hash_table_new_full(g_direct_hash, g_direct_equal, NULL,
                                             (GDestroyNotify)json_object_unref);
  scene->wheel_pointers = g_hash_table_new_full(g_direct_hash, g_direct_equal,
                                               g_object_unref, wheel_pointer_free);
  scene->native_images = g_hash_table_new_full(g_str_hash, g_str_equal, g_free, native_image_unref);
  scene->layer_count = 1;
  GtkWidget *root = gtk_fixed_new();
  gtk_widget_set_parent(root, GTK_WIDGET(scene));
  gtk_widget_set_overflow(GTK_WIDGET(scene), GTK_OVERFLOW_HIDDEN);
  gtk_widget_set_focusable(GTK_WIDGET(scene), TRUE);
}
GtkWidget *pax_native_scene_new(PaxSceneInterrupt interrupt,
                                PaxSceneError error, gpointer data) {
  PaxNativeScene *scene = g_object_new(pax_native_scene_get_type(), NULL);
  scene->interrupt = interrupt;
  scene->error = error;
  scene->data = data;
  add_pointer_controller(GTK_WIDGET(scene), scene);
  GtkEventController *motion = gtk_event_controller_motion_new();
  gtk_event_controller_set_propagation_phase(motion, GTK_PHASE_CAPTURE);
  g_signal_connect(motion, "enter", G_CALLBACK(wheel_pointer_enter), scene);
  g_signal_connect(motion, "leave", G_CALLBACK(wheel_pointer_leave), scene);
  gtk_widget_add_controller(GTK_WIDGET(scene), motion);
  GtkEventController *wheel = gtk_event_controller_scroll_new(GTK_EVENT_CONTROLLER_SCROLL_BOTH_AXES);
  gtk_event_controller_set_static_name(wheel, "pax-wheel-input");
  gtk_event_controller_set_propagation_phase(wheel, GTK_PHASE_CAPTURE);
  g_signal_connect(wheel, "scroll", G_CALLBACK(wheel_event), scene);
  gtk_widget_add_controller(GTK_WIDGET(scene), wheel);
  GtkEventController *keys = gtk_event_controller_key_new();
  gtk_event_controller_set_propagation_phase(keys, GTK_PHASE_CAPTURE);
  g_signal_connect(keys, "key-pressed", G_CALLBACK(key_pressed), scene);
  g_signal_connect(keys, "key-released", G_CALLBACK(key_released), scene);
  gtk_widget_add_controller(GTK_WIDGET(scene), keys);
  GtkEventController *focus = gtk_event_controller_focus_new();
  g_signal_connect(focus, "notify::is-focus", G_CALLBACK(canvas_focus_changed), scene);
  gtk_widget_add_controller(GTK_WIDGET(scene), focus);
  return GTK_WIDGET(scene);
}
static void node_control(PaxNode *node, GtkWidget *control) {
  if (node->control)
    gtk_widget_unparent(node->control);
  node->control = control;
  if (node->form_css_class)
    gtk_widget_add_css_class(control, node->form_css_class);
  if (node->transparent_text_css)
    gtk_widget_add_css_class(control, node->transparent_text_class);
  gtk_widget_set_parent(control, GTK_WIDGET(node));
  if (node->input)
    gtk_widget_insert_after(node->input, GTK_WIDGET(node), control);
}
void pax_native_scene_navigation(PaxNativeScene *scene, PaxSceneNavigate navigate) {
  scene->navigate = navigate;
}
static gboolean text_link_activated(GtkLabel *label, const char *uri, PaxNode *node) {
  (void)label;
  // The host owns virtual routes and OS launches. Never let GTK reinterpret a
  // relative application route as a filesystem path.
  if (!node->scene->stopped && node->scene->navigate)
    node->scene->navigate(node->scene->data, uri);
  return TRUE;
}
static PaxNode *create_node(PaxNativeScene *scene, const char *kind,
                            JsonObject *patch, GError **error) {
  guint id = json_object_get_int_member(patch, "id");
  if (g_hash_table_contains(scene->nodes, GUINT_TO_POINTER(id))) {
    g_set_error(error, G_IO_ERROR, G_IO_ERROR_INVALID_DATA,
                "duplicate native node %u", id);
    return NULL;
  }
  PaxNode *node = g_object_new(pax_node_get_type(), NULL);
  g_object_ref_sink(node);
  node->scene = scene;
  node->id = id;
  node->kind = g_strdup(kind);
  node->parent_id = has(patch, "parent_frame")
                        ? json_object_get_int_member(patch, "parent_frame")
                        : -1;
  node->layer = number(patch, "render_layer_id", 0);
  scene->layer_count = MAX(scene->layer_count, node->layer + 1);
  if (strcmp(kind, "Frame") == 0)
    node->control = node->content = gtk_fixed_new();
  else if (strcmp(kind, "Scroller") == 0) {
    node->control = gtk_scrolled_window_new();
    node->content = gtk_fixed_new();
    gtk_scrolled_window_set_child(GTK_SCROLLED_WINDOW(node->control),
                                  node->content);
    node->snap_touches = g_hash_table_new(g_direct_hash, g_direct_equal);
    for (guint axis = 0; axis < 2; axis++) {
      g_signal_connect_object(scroll_adjustment(node, axis), "value-changed",
                               G_CALLBACK(scroll_changed), node, 0);
      g_signal_connect_object(scroll_adjustment(node, axis), "changed",
                               G_CALLBACK(snap_extents_changed), node, 0);
    }
    GtkEventController *scroll_observer = gtk_event_controller_scroll_new(GTK_EVENT_CONTROLLER_SCROLL_BOTH_AXES);
    gtk_event_controller_set_static_name(scroll_observer, "pax-snap-scroll-observer");
    gtk_event_controller_set_propagation_phase(scroll_observer, GTK_PHASE_CAPTURE);
    g_signal_connect_object(scroll_observer, "scroll-begin", G_CALLBACK(snap_scroll_begin), node, 0);
    g_signal_connect_object(scroll_observer, "scroll-end", G_CALLBACK(snap_scroll_end), node, 0);
    g_signal_connect_object(scroll_observer, "scroll", G_CALLBACK(snap_scroll), node, 0);
    gtk_widget_add_controller(node->control, scroll_observer);
    GtkEventController *input_observer = gtk_event_controller_legacy_new();
    gtk_event_controller_set_static_name(input_observer, "pax-snap-input-observer");
    gtk_event_controller_set_propagation_phase(input_observer, GTK_PHASE_CAPTURE);
    g_signal_connect_object(input_observer, "event", G_CALLBACK(snap_event), node, 0);
    gtk_widget_add_controller(node->control, input_observer);
    GtkEventController *focus_observer = gtk_event_controller_focus_new();
    g_signal_connect_object(focus_observer, "leave", G_CALLBACK(snap_focus_leave), node, 0);
    gtk_widget_add_controller(node->control, focus_observer);
    g_signal_connect_object(node->control, "map", G_CALLBACK(snap_map), node, 0);
    g_signal_connect_object(node->control, "unmap", G_CALLBACK(snap_unmap), node, 0);
    node->clip = TRUE;
  } else if (strcmp(kind, "Textbox") == 0) {
    node->control = gtk_entry_new();
    g_signal_connect(node->control, "changed", G_CALLBACK(entry_changed), node);
    g_signal_connect(node->control, "activate", G_CALLBACK(entry_activate),
                     node);
  } else if (strcmp(kind, "Text") == 0) {
    node->control = gtk_label_new("");
    node->form_yalign = 0;
    g_signal_connect_object(node->control, "activate-link", G_CALLBACK(text_link_activated), node, 0);
    gtk_label_set_xalign(GTK_LABEL(node->control), 0);
    gtk_label_set_yalign(GTK_LABEL(node->control), 0);
  } else if (strcmp(kind, "Button") == 0) {
    node->control = gtk_button_new();
    g_signal_connect(node->control, "clicked", G_CALLBACK(button_clicked),
                     node);
  } else if (strcmp(kind, "Checkbox") == 0) {
    node->control = gtk_check_button_new();
    g_signal_connect(node->control, "toggled", G_CALLBACK(checkbox_changed),
                     node);
  } else if (strcmp(kind, "Slider") == 0) {
    node->control = gtk_scale_new_with_range(GTK_ORIENTATION_HORIZONTAL, 0, 100, 1);
    gtk_scale_set_draw_value(GTK_SCALE(node->control), FALSE);
    gtk_range_set_round_digits(GTK_RANGE(node->control), -1);
    g_signal_connect_object(node->control, "change-value",
                             G_CALLBACK(slider_change_value), node, 0);
    g_signal_connect_object(node->control, "value-changed",
                             G_CALLBACK(slider_changed), node, 0);
  } else if (strcmp(kind, "Dropdown") == 0) {
    node->options_model = gtk_string_list_new(NULL);
    // The constructor takes a reference; keep our own for incremental splices.
    node->control = gtk_drop_down_new(G_LIST_MODEL(g_object_ref(node->options_model)), NULL);
    GtkListItemFactory *factory = gtk_signal_list_item_factory_new();
    g_signal_connect_object(factory, "setup", G_CALLBACK(dropdown_setup), node, 0);
    g_signal_connect_object(factory, "bind", G_CALLBACK(dropdown_bind), node, 0);
    gtk_drop_down_set_factory(GTK_DROP_DOWN(node->control), factory);
    gtk_drop_down_set_list_factory(GTK_DROP_DOWN(node->control), factory);
    g_object_unref(factory);
    g_signal_connect_object(node->control, "notify::selected",
                             G_CALLBACK(dropdown_changed), node, 0);
  } else if (strcmp(kind, "RadioList") == 0) {
    node->control = g_object_new(GTK_TYPE_BOX, "orientation", GTK_ORIENTATION_VERTICAL,
                                 "accessible-role", GTK_ACCESSIBLE_ROLE_RADIO_GROUP, NULL);
    gtk_box_set_homogeneous(GTK_BOX(node->control), TRUE);
    // Keep a stable, unparented group head: a single visible option must retain
    // GTK's native radio role/indicator when other options are removed.
    node->radio_group = GTK_CHECK_BUTTON(g_object_ref_sink(gtk_check_button_new()));
  } else if (strcmp(kind, "NativeImage") == 0) {
    node->control = gtk_picture_new();
    // Native images have no GTK interaction of their own. Scene ray-casting
    // still delivers Pax events; underlying native scrollers keep wheel input.
    gtk_widget_set_can_target(GTK_WIDGET(node), FALSE);
    gtk_picture_set_can_shrink(GTK_PICTURE(node->control), TRUE);
    gtk_picture_set_content_fit(GTK_PICTURE(node->control), GTK_CONTENT_FIT_CONTAIN);
  } else if (strcmp(kind, "EventBlocker") == 0)
    node->control = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
  else {
    g_set_error(error, G_IO_ERROR, G_IO_ERROR_NOT_SUPPORTED,
                "Linux native control %s is not implemented", kind);
    g_object_unref(node);
    return NULL;
  }
  gtk_widget_set_parent(node->control, GTK_WIDGET(node));
  PaxInput *input = g_object_new(pax_input_get_type(), NULL);
  input->node = node;
  node->input = GTK_WIDGET(input);
  gtk_widget_set_parent(node->input, GTK_WIDGET(node));
  g_hash_table_insert(scene->nodes, GUINT_TO_POINTER(id), node);
  return node;
}
static gboolean set_mask(PaxNode *node, JsonObject *patch, GError **error) {
  GPtrArray *entries = g_ptr_array_new_with_free_func(mask_free);
  JsonArray *array = json_object_get_array_member(patch, "entries");
  for (guint i = 0; i < json_array_get_length(array); i++) {
    JsonObject *object = json_array_get_object_element(array, i);
    MaskEntry *entry = g_new0(MaskEntry, 1);
    entry->path = gsk_path_parse(json_object_get_string_member(object, "path"));
    entry->clips =
        g_ptr_array_new_with_free_func((GDestroyNotify)gsk_path_unref);
    entry->opacity = CLAMP(number(object, "opacity", 1), 0, 1);
    if (!entry->path) {
      g_ptr_array_unref(entry->clips);
      g_free(entry);
      g_ptr_array_unref(entries);
      g_set_error(error, G_IO_ERROR, G_IO_ERROR_INVALID_DATA,
                  "invalid native mask path");
      return FALSE;
    }
    JsonArray *clips = json_object_get_array_member(object, "clips");
    for (guint j = 0; j < json_array_get_length(clips); j++) {
      GskPath *clip = gsk_path_parse(json_array_get_string_element(clips, j));
      if (!clip) {
        mask_free(entry);
        g_ptr_array_unref(entries);
        g_set_error(error, G_IO_ERROR, G_IO_ERROR_INVALID_DATA,
                    "invalid native mask clip");
        return FALSE;
      }
      g_ptr_array_add(entry->clips, clip);
    }
    g_ptr_array_add(entries, entry);
  }
  g_ptr_array_unref(node->mask);
  node->mask = entries;
  gtk_widget_queue_draw(GTK_WIDGET(node));
  return TRUE;
}
static gboolean is_form(PaxNode *node) {
  return strcmp(node->kind, "Slider") == 0 ||
         strcmp(node->kind, "Dropdown") == 0 ||
         strcmp(node->kind, "RadioList") == 0;
}
static void form_label_attributes(GtkWidget *widget, PaxNode *node) {
  if (GTK_IS_LABEL(widget)) {
    gtk_label_set_attributes(GTK_LABEL(widget), node->form_attributes);
    gtk_label_set_xalign(GTK_LABEL(widget), node->form_xalign);
    gtk_label_set_yalign(GTK_LABEL(widget), node->form_yalign);
    gtk_label_set_justify(GTK_LABEL(widget), node->text_justification);
  }
  for (GtkWidget *child = gtk_widget_get_first_child(widget); child;
       child = gtk_widget_get_next_sibling(child))
    form_label_attributes(child, node);
}
static gboolean update_options(PaxNode *node, JsonObject *patch, GError **error) {
  if (has(patch, "selected_id")) {
    gint64 selected = json_object_get_int_member(patch, "selected_id");
    if (selected < 0 || selected > G_MAXUINT) {
      g_set_error(error, G_IO_ERROR, G_IO_ERROR_INVALID_DATA,
                   "native selection index must fit u32");
      return FALSE;
    }
    node->selected_id = selected;
  }
  if (has(patch, "options")) {
    JsonArray *array = json_object_get_array_member(patch, "options");
    guint count = json_array_get_length(array);
    char **options = g_new0(char *, (gsize)count + 1);
    for (guint i = 0; i < count; i++) {
      JsonNode *option = json_array_get_element(array, i);
      if (json_node_get_value_type(option) != G_TYPE_STRING) {
        g_strfreev(options);
        g_set_error(error, G_IO_ERROR, G_IO_ERROR_INVALID_DATA,
                     "native control options must be strings");
        return FALSE;
      }
      options[i] = g_strdup(json_node_get_string(option));
    }
    if (!g_strv_equal((const char *const *)node->options,
                      (const char *const *)options)) {
      if (GTK_IS_DROP_DOWN(node->control))
        gtk_string_list_splice(node->options_model, 0,
                                g_list_model_get_n_items(G_LIST_MODEL(node->options_model)),
                                (const char *const *)options);
      else {
        // Keep existing radio widgets by index, including their focus and
        // accessibility identity. Only additions/removals change the child set.
        GtkWidget *child = gtk_widget_get_first_child(node->control);
        GtkRoot *root = gtk_widget_get_root(node->control);
        GtkWidget *focus = root ? gtk_root_get_focus(root) : NULL;
        gboolean removed_focus = FALSE;
        for (guint i = 0; i < count; i++) {
          GtkCheckButton *button;
          if (child) {
            button = GTK_CHECK_BUTTON(child);
            child = gtk_widget_get_next_sibling(child);
          } else {
            // GTK 4.14 changes the grouped indicator but keeps CHECKBOX as its
            // default accessible role. Set the role before its AT context exists.
            button = g_object_new(GTK_TYPE_CHECK_BUTTON,
                                    "accessible-role", GTK_ACCESSIBLE_ROLE_RADIO, NULL);
            gtk_check_button_set_group(button, node->radio_group);
            gtk_box_append(GTK_BOX(node->control), GTK_WIDGET(button));
            g_object_set_data(G_OBJECT(button), "pax-option", GUINT_TO_POINTER(i));
            g_signal_connect_object(button, "toggled", G_CALLBACK(radio_changed), node, 0);
          }
          gtk_check_button_set_label(button, options[i]);
          // get_child() excludes the built-in label owned by set_label().
          form_label_attributes(GTK_WIDGET(button), node);
        }
        while (child) {
          GtkWidget *next = gtk_widget_get_next_sibling(child);
          // Once its child is removed, focus may be destroyed; never inspect
          // that borrowed pointer again while removing the remaining tail.
          removed_focus = removed_focus ||
              (focus && (focus == child || gtk_widget_is_ancestor(focus, child)));
          gtk_box_remove(GTK_BOX(node->control), child);
          child = next;
        }
        if (removed_focus) {
          GtkWidget *target = gtk_widget_get_first_child(node->control);
          guint index = MIN(node->selected_id, count ? count - 1 : 0);
          while (target && index--)
            target = gtk_widget_get_next_sibling(target);
          if (target)
            gtk_widget_grab_focus(target);
          else
            gtk_widget_grab_focus(GTK_WIDGET(node->scene));
        }
      }
      g_strfreev(node->options);
      node->options = options;
    } else
      g_strfreev(options);
  }
  guint count = g_strv_length(node->options);
  if (GTK_IS_DROP_DOWN(node->control))
    gtk_drop_down_set_selected(GTK_DROP_DOWN(node->control),
                                node->selected_id < count ? node->selected_id
                                                          : GTK_INVALID_LIST_POSITION);
  else {
    guint index = 0;
    for (GtkWidget *child = gtk_widget_get_first_child(node->control); child;
         child = gtk_widget_get_next_sibling(child), index++)
      gtk_check_button_set_active(GTK_CHECK_BUTTON(child), index == node->selected_id);
  }
  return TRUE;
}
static GdkRGBA message_color(JsonObject *object) {
  JsonArray *color = has(object, "Rgba") ? json_object_get_array_member(object, "Rgba")
                                        : json_object_get_array_member(object, "Rgb");
  GdkRGBA rgba = {
    CLAMP(json_array_get_double_element(color, 0), 0, 1),
    CLAMP(json_array_get_double_element(color, 1), 0, 1),
    CLAMP(json_array_get_double_element(color, 2), 0, 1),
    json_array_get_length(color) == 4
        ? CLAMP(json_array_get_double_element(color, 3), 0, 1) : 1
  };
  return rgba;
}
static char *form_color(JsonObject *appearance, const char *key,
                         const char *fallback) {
  if (!has(appearance, key))
    return g_strdup(fallback);
  GdkRGBA rgba = message_color(json_object_get_object_member(appearance, key));
  return gdk_rgba_to_string(&rgba);
}
static void form_appearance(PaxNode *node, JsonObject *patch) {
  const char *keys[] = {"background", "accent", "background_checked",
                        "stroke_color", "stroke_width", "outline_color",
                        "outline_width", "corner_radius", "color", "hover_color",
                        "outline_stroke_color", "outline_stroke_width"};
  gboolean changed = !node->form_appearance;
  if (!node->form_appearance)
    node->form_appearance = json_object_new();
  for (guint i = 0; i < G_N_ELEMENTS(keys); i++)
    if (has(patch, keys[i]) &&
        (!has(node->form_appearance, keys[i]) ||
         !json_node_equal(json_object_get_member(patch, keys[i]),
                           json_object_get_member(node->form_appearance, keys[i])))) {
      json_object_set_member(node->form_appearance, keys[i],
                              json_node_copy(json_object_get_member(patch, keys[i])));
      changed = TRUE;
    }
  if (GTK_IS_DROP_DOWN(node->control) && has(patch, "style")) {
    JsonObject *style = json_object_get_object_member(patch, "style");
    if (has(style, "fill")) {
      if (!has(node->form_appearance, "indicator_fill") ||
          !json_node_equal(json_object_get_member(style, "fill"),
                           json_object_get_member(node->form_appearance, "indicator_fill"))) {
        json_object_set_member(node->form_appearance, "indicator_fill",
                               json_node_copy(json_object_get_member(style, "fill")));
        changed = TRUE;
      }
    } else if (has(node->form_appearance, "indicator_fill")) {
      json_object_remove_member(node->form_appearance, "indicator_fill");
      changed = TRUE;
    }
  }
  if (!changed)
    return;
  if (!node->form_css) {
    node->form_css = gtk_css_provider_new();
    node->form_css_class = g_strdup_printf("pax-form-%p-%u", (void *)node->scene, node->id);
    gtk_widget_add_css_class(node->control, node->form_css_class);
    // A uniquely scoped provider also reaches GTK's lazily created subnodes.
    // Removing it in dispose prevents stale styles when runtime IDs are reused.
    gtk_style_context_add_provider_for_display(
        gtk_widget_get_display(node->control), GTK_STYLE_PROVIDER(node->form_css),
        GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);
  }
  char *background = form_color(node->form_appearance, "background", "#f3f4f6");
  char *accent = form_color(node->form_appearance, "accent", "#1b64f2");
  char *checked = form_color(node->form_appearance, "background_checked", "#1b64f2");
  char *outline = form_color(node->form_appearance, "outline_color", "#d1d5db");
  char *stroke = form_color(node->form_appearance, "stroke_color", "#d1d5db");
  GString *css = g_string_new(NULL);
  const char *name = node->form_css_class;
  char radius[G_ASCII_DTOSTR_BUF_SIZE], stroke_width[G_ASCII_DTOSTR_BUF_SIZE],
       outline_width[G_ASCII_DTOSTR_BUF_SIZE];
  g_ascii_dtostr(radius, sizeof(radius),
                  MAX(0, number(node->form_appearance, "corner_radius", 5)));
  g_ascii_dtostr(stroke_width, sizeof(stroke_width),
                  MAX(0, number(node->form_appearance, "stroke_width", 1)));
  g_ascii_dtostr(outline_width, sizeof(outline_width),
                  MAX(0, number(node->form_appearance, "outline_width", 1)));
  if (GTK_IS_BUTTON(node->control)) {
    char *color = form_color(node->form_appearance, "color", "#f3f4f6");
    char *hover = form_color(node->form_appearance, "hover_color", color);
    char *border = form_color(node->form_appearance, "outline_stroke_color", "#d1d5db");
    char border_width[G_ASCII_DTOSTR_BUF_SIZE];
    g_ascii_dtostr(border_width, sizeof(border_width),
                    MAX(0, number(node->form_appearance, "outline_stroke_width", 0)));
    // Keep GTK's focus/accessibility/input machinery; replace only the authored
    // surface. Removing theme images/shadows prevents them tinting these colors.
    g_string_append_printf(css,
        ".%s { background-image: none; background-color: %s; box-shadow: none; "
        "border: %spx solid %s; border-radius: %spx; min-width: 0; min-height: 0; }"
        ".%s:hover, .%s:active, .%s.keyboard-activating { background-color: %s; }",
        name, color, border_width, border, radius, name, name, name, hover);
    g_free(color); g_free(hover); g_free(border);
  } else if (GTK_IS_ENTRY(node->control) || GTK_IS_TEXT_VIEW(node->control)) {
    g_string_append_printf(css,
        ".%s { background-image: none; background-color: %s; box-shadow: none; "
        "border: %spx solid %s; border-radius: %spx; min-width: 0; min-height: 0; "
        "outline-width: %spx; outline-color: %s; }"
        ".%s:focus-within { outline-style: solid; }"
        ".%s text { background-color: transparent; }",
        name, background, stroke_width, stroke, radius, outline_width, outline, name, name);
  } else if (GTK_IS_SCALE(node->control)) {
    g_string_append_printf(css,
        ".%s { padding: 0; margin: 0; min-width: 0; min-height: 0; }"
        ".%s trough { background: %s; border-radius: %spx; }"
        ".%s highlight, .%s slider { background: %s; }",
        name, name, background, radius, name, name, accent);
  } else if (GTK_IS_DROP_DOWN(node->control)) {
    g_string_append_printf(css,
        ".%s > button { background: %s; border: %spx solid %s; border-radius: %spx; "
        "min-width: 0; min-height: 0; padding: 2px 6px; }",
        name, background, stroke_width, stroke, radius);
    // Pango styles the option labels, but GTK's arrow is a symbolic icon.
    // Apply its foreground separately so translucent labels are not faded twice.
    if (has(node->form_appearance, "indicator_fill")) {
      char *foreground = form_color(node->form_appearance, "indicator_fill", "inherit");
      g_string_append_printf(css, ".%s > button arrow { color: %s; }", name, foreground);
      g_free(foreground);
    }
    // The real native popover must retain contrast with the same authored labels.
    g_string_append_printf(css,
        ".%s > popover > contents, .%s > popover listview { background-color: %s; }",
        name, name, background);
  } else {
    g_string_append_printf(css,
        ".%s checkbutton { padding: 0; margin: 0; min-height: 0; }"
        ".%s radio { background: %s; border: %spx solid %s; }"
        ".%s radio:checked { background: %s; }",
        name, name, background, outline_width, outline, name, checked);
  }
  gtk_css_provider_load_from_string(node->form_css, css->str);
  g_string_free(css, TRUE);
  g_free(background); g_free(accent); g_free(checked); g_free(outline); g_free(stroke);
}
static void text_view_attributes(PaxNode *node) {
  GtkTextBuffer *buffer = gtk_text_view_get_buffer(GTK_TEXT_VIEW(node->control));
  GtkTextTagTable *table = gtk_text_buffer_get_tag_table(buffer);
  GtkTextTag *previous = gtk_text_tag_table_lookup(table, "pax-style");
  if (previous)
    gtk_text_tag_table_remove(table, previous);
  GtkTextTag *tag = gtk_text_buffer_create_tag(buffer, "pax-style", NULL);
  PangoAttrIterator *iterator = pango_attr_list_get_iterator(node->form_attributes);
  PangoFontDescription *font = pango_font_description_new();
  pango_attr_iterator_get_font(iterator, font, NULL, NULL);
  g_object_set(tag, "font-desc", font, NULL);
  pango_font_description_free(font);
  PangoAttrColor *foreground = (PangoAttrColor *)pango_attr_iterator_get(iterator, PANGO_ATTR_FOREGROUND);
  PangoAttrInt *alpha = (PangoAttrInt *)pango_attr_iterator_get(iterator, PANGO_ATTR_FOREGROUND_ALPHA);
  PangoAttrInt *underline = (PangoAttrInt *)pango_attr_iterator_get(iterator, PANGO_ATTR_UNDERLINE);
  if (foreground) {
    GdkRGBA color = {foreground->color.red / 65535.0f,
                     foreground->color.green / 65535.0f,
                     foreground->color.blue / 65535.0f,
                     alpha ? alpha->value / 65535.0f : 1};
    g_object_set(tag, "foreground-rgba", &color, NULL);
  }
  g_object_set(tag, "underline", underline ? underline->value : PANGO_UNDERLINE_NONE,
                "justification", node->text_justification, NULL);
  pango_attr_iterator_destroy(iterator);
  gtk_text_view_set_justification(GTK_TEXT_VIEW(node->control), node->text_justification);
  text_tag_range(buffer);
}
static void transparent_text(PaxNode *node, gboolean transparent) {
  if (transparent == (node->transparent_text_css != NULL))
    return;
  if (transparent) {
    if (!node->transparent_text_class)
      node->transparent_text_class = g_strdup_printf("pax-transparent-text-%p-%u",
                                                       (void *)node->scene, node->id);
    node->transparent_text_css = gtk_css_provider_new();
    char *css = g_strdup_printf(".%s, .%s label, .%s text { color: transparent; }",
                                node->transparent_text_class,
                                node->transparent_text_class,
                                node->transparent_text_class);
    gtk_css_provider_load_from_string(node->transparent_text_css, css);
    g_free(css);
    gtk_widget_add_css_class(node->control, node->transparent_text_class);
    gtk_style_context_add_provider_for_display(
        gtk_widget_get_display(node->control), GTK_STYLE_PROVIDER(node->transparent_text_css),
        GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);
  } else {
    gtk_widget_remove_css_class(node->control, node->transparent_text_class);
    gtk_style_context_remove_provider_for_display(
        gtk_widget_get_display(node->control), GTK_STYLE_PROVIDER(node->transparent_text_css));
    g_clear_object(&node->transparent_text_css);
  }
}
static PangoAttrList *text_attributes(PaxNode *node, JsonObject *style,
                                      gboolean *zero_alpha, GError **error) {
  PangoAttrList *attributes = pango_attr_list_new();
  *zero_alpha = FALSE;
  if (has(style, "font_size"))
    pango_attr_list_insert(attributes,
                           // Pax pixels equal 3/4 of a point at GTK's 96-DPI
                           // baseline. Let Pango apply GTK's effective font
                           // resolution; display scaling remains GTK's job.
                           pango_attr_size_new(
                               number(style, "font_size", 14) *
                               (72.0 / 96.0) * PANGO_SCALE));
  if (has(style, "fill")) {
    JsonObject *fill = json_object_get_object_member(style, "fill");
    JsonArray *color = has(fill, "Rgba")
                           ? json_object_get_array_member(fill, "Rgba")
                           : json_object_get_array_member(fill, "Rgb");
    if (color) {
      pango_attr_list_insert(
          attributes,
          pango_attr_foreground_new(
              CLAMP(json_array_get_double_element(color, 0), 0, 1) * 65535,
              CLAMP(json_array_get_double_element(color, 1), 0, 1) * 65535,
              CLAMP(json_array_get_double_element(color, 2), 0, 1) * 65535));
      if (json_array_get_length(color) == 4) {
        double alpha = CLAMP(json_array_get_double_element(color, 3), 0, 1);
        *zero_alpha = alpha == 0;
        // Pango's zero alpha means "inherit". Preserve the authored attribute,
        // but supply a transparent CSS foreground for GTK's fallback. Changing
        // foreground, rather than widget opacity, retains native selection ink.
        pango_attr_list_insert(attributes, pango_attr_foreground_alpha_new(
            alpha == 0 ? 0 : MAX(1, alpha * 65535)));
      }
    }
  }
  if (has(style, "font")) {
    JsonObject *font = json_object_get_object_member(style, "font");
    GList *keys = json_object_get_members(font);
    if (keys) {
      JsonObject *details = json_object_get_object_member(font, keys->data);
      const char *source = string(details, "path", string(details, "url", ""));
      // New controls are styled before reconcile() parents them. Register in
      // the owning scene's eventual map, not their temporary unparented map.
      PangoFontMap *font_map = pango_context_get_font_map(
          gtk_widget_get_pango_context(GTK_WIDGET(node->scene)));
      if (*source && !pax_linux_font_load(node->scene->assets, source,
                                         font_map, error)) {
        g_list_free(keys);
        pango_attr_list_unref(attributes);
        return NULL;
      }
      if (has(details, "family"))
        pango_attr_list_insert(
            attributes, pango_attr_family_new(
                            json_object_get_string_member(details, "family")));
      const char *weights[] = {"Thin", "ExtraLight", "Light", "Normal", "Medium",
                               "SemiBold", "Bold", "ExtraBold", "Black"};
      for (guint i = 0; i < G_N_ELEMENTS(weights); i++)
        if (strcmp(string(details, "weight", "Normal"), weights[i]) == 0)
          pango_attr_list_insert(attributes, pango_attr_weight_new((i + 1) * 100));
      const char *slant = string(details, "style", "Normal");
      pango_attr_list_insert(attributes, pango_attr_style_new(
          strcmp(slant, "Italic") == 0 ? PANGO_STYLE_ITALIC
          : strcmp(slant, "Oblique") == 0 ? PANGO_STYLE_OBLIQUE : PANGO_STYLE_NORMAL));
    }
    g_list_free(keys);
  }
  if (boolean(style, "underline", FALSE))
    pango_attr_list_insert(attributes,
                           pango_attr_underline_new(PANGO_UNDERLINE_SINGLE));
  return attributes;
}
static gboolean apply_style(PaxNode *node, JsonObject *patch, GError **error) {
  if (!has(patch, "style")) {
    if (node->form_attributes && GTK_IS_TEXT_VIEW(node->control)) {
      GtkTextBuffer *buffer = gtk_text_view_get_buffer(GTK_TEXT_VIEW(node->control));
      if (!gtk_text_tag_table_lookup(gtk_text_buffer_get_tag_table(buffer), "pax-style"))
        text_view_attributes(node);
    }
    return TRUE;
  }
  JsonObject *style = json_object_get_object_member(patch, "style");
  gboolean zero_alpha = FALSE;
  PangoAttrList *attributes = text_attributes(node, style, &zero_alpha, error);
  if (!attributes) return FALSE;
  transparent_text(node, zero_alpha);
  g_clear_pointer(&node->form_attributes, pango_attr_list_unref);
  node->form_attributes = pango_attr_list_ref(attributes);
  const char *horizontal = string(style, "align_horizontal",
                                   GTK_IS_BUTTON(node->control) ? "Center" : "Left");
  const char *vertical = string(style, "align_vertical",
                                 strcmp(node->kind, "Text") == 0 ? "Top" : "Center");
  const char *multiline = string(style, "align_multiline", horizontal);
  node->form_xalign = strcmp(horizontal, "Right") == 0 ? 1
                       : strcmp(horizontal, "Center") == 0 ? .5 : 0;
  node->form_yalign = strcmp(vertical, "Bottom") == 0 ? 1
                       : strcmp(vertical, "Center") == 0 ? .5 : 0;
  node->text_justification = strcmp(multiline, "Right") == 0 ? GTK_JUSTIFY_RIGHT
                             : strcmp(multiline, "Center") == 0 ? GTK_JUSTIFY_CENTER
                             : GTK_JUSTIFY_LEFT;
  if (GTK_IS_ENTRY(node->control)) {
    gtk_entry_set_attributes(GTK_ENTRY(node->control), attributes);
    gtk_entry_set_alignment(GTK_ENTRY(node->control), node->form_xalign);
  } else if (GTK_IS_TEXT_VIEW(node->control)) {
    text_view_attributes(node);
  } else {
    form_label_attributes(node->control, node);
  }
  pango_attr_list_unref(attributes);
  return TRUE;
}
static void ranged_attribute(PangoAttrList *attributes, PangoAttribute *attribute,
                              guint start, guint end) {
  attribute->start_index = start;
  attribute->end_index = end;
  pango_attr_list_change(attributes, attribute);
}
static gboolean validate_rich_text(JsonObject *document, GError **error) {
  const char *text = string(document, "text", "");
  gsize length = strlen(text);
  if (length > G_MAXUINT || !g_utf8_validate(text, -1, NULL) || !has(document, "runs"))
    goto invalid;
  JsonArray *runs = json_object_get_array_member(document, "runs");
  gint64 cursor = 0;
  for (guint i = 0; i < json_array_get_length(runs); i++) {
    JsonObject *run = json_array_get_object_element(runs, i);
    gint64 start = json_object_get_int_member(run, "start"),
           end = json_object_get_int_member(run, "end");
    if (start != cursor || end <= start || (guint64)end > length ||
        !g_utf8_validate(text + start, end - start, NULL)) goto invalid;
    if (has(run, "size_scale") && (!isfinite(number(run, "size_scale", 1)) ||
                                    number(run, "size_scale", 1) <= 0)) goto invalid;
    if (has(run, "color")) {
      JsonArray *color = json_object_get_array_member(run, "color");
      if (json_array_get_length(color) != 4) goto invalid;
      for (guint c = 0; c < 4; c++) {
        double value = json_array_get_double_element(color, c);
        if (!isfinite(value) || value < 0 || value > 1) goto invalid;
      }
      // Inline CSS currently supports opaque colors. Base TextStyle alpha is
      // independent; an unsupported range must not silently inherit its color.
      if (json_array_get_double_element(color, 3) != 1) goto invalid;
    }
    cursor = end;
  }
  if ((guint64)cursor != length ||
      (has(document, "line_height") &&
       (!isfinite(number(document, "line_height", 0)) ||
         number(document, "line_height", 0) <= 0 ||
         number(document, "line_height", 0) > G_MAXINT / (double)PANGO_SCALE))) goto invalid;
  return TRUE;
invalid:
  g_set_error_literal(error, G_IO_ERROR, G_IO_ERROR_INVALID_DATA,
                       "invalid normalized desktop rich text");
  return FALSE;
}
static void rich_text_content(PaxNode *node) {
  const char *text = string(node->rich_text, "text", "");
  JsonArray *runs = json_object_get_array_member(node->rich_text, "runs");
  GString *markup = g_string_new("");
  const char *active_uri = NULL;
  gint64 active_link_id = 0;
  for (guint i = 0; i < json_array_get_length(runs); i++) {
    JsonObject *run = json_array_get_object_element(runs, i);
    gsize start = json_object_get_int_member(run, "start"),
          end = json_object_get_int_member(run, "end");
    const char *next_uri = has(run, "link") ? string(run, "link", "") : NULL;
    gint64 next_link_id = has(run, "link_id") ? json_object_get_int_member(run, "link_id") : 0;
    if (g_strcmp0(active_uri, next_uri) || active_link_id != next_link_id) {
      if (active_uri) g_string_append(markup, "</a>");
      if (next_uri) {
        char *uri = g_markup_escape_text(next_uri, -1);
        g_string_append_printf(markup, "<a href=\"%s\">", uri);
        g_free(uri);
      }
      active_uri = next_uri;
      active_link_id = next_link_id;
    }
    char *escaped = g_markup_escape_text(text + start, end - start);
    g_string_append(markup, escaped);
    g_free(escaped);
  }
  if (active_uri) g_string_append(markup, "</a>");
  // GtkLabel retains selection when the content did not change. Style-only
  // updates use Pango attributes below rather than replacing native text.
  if (!gtk_label_get_use_markup(GTK_LABEL(node->control)) ||
      strcmp(gtk_label_get_label(GTK_LABEL(node->control)), markup->str))
    gtk_label_set_markup(GTK_LABEL(node->control), markup->str);
  g_string_free(markup, TRUE);
}
static gboolean rich_text_attributes(PaxNode *node, GError **error) {
  PangoAttrList *attributes = node->form_attributes
      ? pango_attr_list_copy(node->form_attributes) : pango_attr_list_new();
  PangoAttrList *links = NULL;
  if (node->link_style) {
    gboolean transparent;
    links = text_attributes(node, node->link_style, &transparent, error);
    if (!links) { pango_attr_list_unref(attributes); return FALSE; }
  }
  PangoAttrIterator *base = pango_attr_list_get_iterator(attributes);
  PangoAttrInt *base_alpha = (PangoAttrInt *)pango_attr_iterator_get(base, PANGO_ATTR_FOREGROUND_ALPHA);
  gboolean base_has_alpha = base_alpha != NULL;
  gboolean base_zero = base_alpha && !base_alpha->value;
  gboolean base_has_foreground = pango_attr_iterator_get(base, PANGO_ATTR_FOREGROUND) != NULL;
  pango_attr_iterator_destroy(base);
  PangoAttrIterator *link_iterator = links ? pango_attr_list_get_iterator(links) : NULL;
  PangoAttrInt *link_alpha = link_iterator
      ? (PangoAttrInt *)pango_attr_iterator_get(link_iterator, PANGO_ATTR_FOREGROUND_ALPHA) : NULL;
  gboolean link_zero = link_alpha && !link_alpha->value;
  if (link_iterator) pango_attr_iterator_destroy(link_iterator);
  if (base_zero || link_zero) {
    if (!base_has_alpha) pango_attr_list_change(attributes, pango_attr_foreground_alpha_new(65535));
    if (!base_has_foreground) {
      // Resolve a theme fallback before applying the transparent renderer color.
      // Explicit range alpha then preserves the ordinary visible text.
      transparent_text(node, FALSE);
      GdkRGBA color;
      gtk_widget_get_color(node->control, &color);
      pango_attr_list_change(attributes, pango_attr_foreground_new(
          color.red * 65535, color.green * 65535, color.blue * 65535));
    }
  }
  transparent_text(node, base_zero || link_zero);
  JsonArray *runs = json_object_get_array_member(node->rich_text, "runs");
  if (has(node->rich_text, "line_height")) {
    double dpi = pango_cairo_context_get_resolution(gtk_widget_get_pango_context(node->control));
    if (dpi <= 0) dpi = 96;
    double height = number(node->rich_text, "line_height", 0) * dpi / 96 * PANGO_SCALE;
    pango_attr_list_change(attributes, pango_attr_line_height_new_absolute(MIN(height, G_MAXINT)));
  }
  for (guint i = 0; i < json_array_get_length(runs); i++) {
    JsonObject *run = json_array_get_object_element(runs, i);
    guint start = json_object_get_int_member(run, "start"),
          end = json_object_get_int_member(run, "end");
    if (has(run, "bold")) ranged_attribute(attributes,
        pango_attr_weight_new(boolean(run, "bold", FALSE) ? PANGO_WEIGHT_BOLD : PANGO_WEIGHT_NORMAL), start, end);
    if (has(run, "italic")) ranged_attribute(attributes,
        pango_attr_style_new(boolean(run, "italic", FALSE) ? PANGO_STYLE_ITALIC : PANGO_STYLE_NORMAL), start, end);
    if (has(run, "underline")) ranged_attribute(attributes,
        pango_attr_underline_new(boolean(run, "underline", FALSE) ? PANGO_UNDERLINE_SINGLE : PANGO_UNDERLINE_NONE), start, end);
    if (has(run, "strikethrough")) ranged_attribute(attributes,
        pango_attr_strikethrough_new(boolean(run, "strikethrough", FALSE)), start, end);
    if (has(run, "font_family")) ranged_attribute(attributes,
        pango_attr_family_new(string(run, "font_family", "")), start, end);
    if (has(run, "size_scale")) ranged_attribute(attributes,
        pango_attr_scale_new(number(run, "size_scale", 1)), start, end);
    if (has(run, "color")) {
      JsonArray *color = json_object_get_array_member(run, "color");
      ranged_attribute(attributes, pango_attr_foreground_new(
          json_array_get_double_element(color, 0) * 65535,
          json_array_get_double_element(color, 1) * 65535,
          json_array_get_double_element(color, 2) * 65535), start, end);
      ranged_attribute(attributes, pango_attr_foreground_alpha_new(65535), start, end);
    }
    // Authored link style overrides inline formatting, matching Apple Text.
    if (has(run, "link") && links) {
      GSList *list = pango_attr_list_get_attributes(links);
      for (GSList *p = list; p; p = p->next)
        ranged_attribute(attributes, p->data, start, end);
      g_slist_free(list);
    }
  }
  gtk_label_set_attributes(GTK_LABEL(node->control), attributes);
  pango_attr_list_unref(attributes);
  if (links) pango_attr_list_unref(links);
  return TRUE;
}
static gboolean update_node(PaxNode *node, JsonObject *patch, GError **error) {
  gboolean text_control_replaced = FALSE;
  if (!strcmp(node->kind, "NativeImage") && !native_image_update(node, patch, error))
    return FALSE;
  if (strcmp(node->kind, "Text") == 0) {
    if (has(patch, "rich_text") && !validate_rich_text(json_object_get_object_member(patch, "rich_text"), error))
      return FALSE;
    gboolean rich = json_object_has_member(patch, "rich_text") ? has(patch, "rich_text") : node->rich_text != NULL;
    if ((boolean(patch, "markdown", FALSE) && !rich) ||
        (rich && boolean(patch, "editable", node->text_editable))) {
      g_set_error_literal(error, G_IO_ERROR, G_IO_ERROR_NOT_SUPPORTED,
          "Markdown requires normalized desktop rich text; editable Markdown is unsupported");
      return FALSE;
    }
  }
  node->updating = TRUE;
  if (json_object_has_member(patch, "parent_frame"))
    node->parent_id = has(patch, "parent_frame")
                          ? json_object_get_int_member(patch, "parent_frame")
                          : -1;
  node->z = number(patch, "z_index", node->z);
  gboolean is_text = strcmp(node->kind, "Text") == 0;
  if (is_text) {
    // Patches omit unchanged automatic axes. Keep their authored constraints
    // separate from resolved bounds so later measurements can grow or shrink.
    node->text_width_constraint =
        number(patch, "size_x", node->text_width_constraint);
    node->text_height_constraint =
        number(patch, "size_y", node->text_height_constraint);
    if (node->text_width_constraint >= 0)
      node->width = node->text_width_constraint;
    if (node->text_height_constraint >= 0)
      node->height = node->text_height_constraint;
  } else {
    node->width = number(patch, "size_x", node->width);
    node->height = number(patch, "size_y", node->height);
  }
  node->opacity = CLAMP(number(patch, "opacity", node->opacity), 0, 1);
  node->radius = MAX(0, number(patch, "corner_radius", node->radius));
  node->clip = boolean(patch, "clip_content", node->clip);
  if (json_object_has_member(patch, "transform_scroller"))
    node->transform_scroller = has(patch, "transform_scroller")
        ? json_object_get_int_member(patch, "transform_scroller") : -1;
  if (has(patch, "transform")) {
    JsonArray *array = json_object_get_array_member(patch, "transform");
    if (json_array_get_length(array) != 6) {
      g_set_error(error, G_IO_ERROR, G_IO_ERROR_INVALID_DATA,
                  "native transform needs six values");
      node->updating = FALSE;
      return FALSE;
    }
    for (guint i = 0; i < 6; i++)
      node->authored_transform[i] = json_array_get_double_element(array, i);
  }
  if (has(patch, "clip_path")) {
    if (node->clip_path)
      gsk_path_unref(node->clip_path);
    const char *path = string(patch, "clip_path", "");
    node->clip_path = *path ? gsk_path_parse(path) : NULL;
    if (*path && !node->clip_path) {
      g_set_error(error, G_IO_ERROR, G_IO_ERROR_INVALID_DATA,
                  "invalid frame clip path");
      node->updating = FALSE;
      return FALSE;
    }
  }
  if (strcmp(node->kind, "Text") == 0) {
    node->text_editable = boolean(patch, "editable", node->text_editable);
    node->text_selectable = boolean(patch, "selectable", node->text_selectable);
    node->text_wrap = boolean(patch, "wrap", node->text_wrap);
    if (json_object_has_member(patch, "rich_text")) {
      g_clear_pointer(&node->rich_text, json_object_unref);
      if (has(patch, "rich_text"))
        node->rich_text = json_object_ref(json_object_get_object_member(patch, "rich_text"));
    }
    if (has(patch, "style_link")) {
      g_clear_pointer(&node->link_style, json_object_unref);
      if (has(patch, "style_link"))
        node->link_style = json_object_ref(json_object_get_object_member(patch, "style_link"));
    }
    if (node->rich_text && !GTK_IS_LABEL(node->control)) {
      GtkWidget *label = gtk_label_new("");
      g_signal_connect_object(label, "activate-link", G_CALLBACK(text_link_activated), node, 0);
      node_control(node, label);
      text_control_replaced = TRUE;
    }
    if (boolean(patch, "editable", FALSE) && !GTK_IS_TEXT_VIEW(node->control)) {
      GtkWidget *view = gtk_text_view_new();
      g_signal_connect_object(gtk_text_view_get_buffer(GTK_TEXT_VIEW(view)), "changed",
                                G_CALLBACK(text_changed), node, 0);
      node_control(node, view);
      text_control_replaced = TRUE;
    }
    if (GTK_IS_LABEL(node->control)) {
      if (node->rich_text && has(patch, "rich_text"))
        rich_text_content(node);
      else if (!node->rich_text && has(patch, "content"))
        gtk_label_set_text(GTK_LABEL(node->control),
                           string(patch, "content", ""));
      if (has(patch, "selectable") || text_control_replaced)
        gtk_label_set_selectable(GTK_LABEL(node->control),
                                 node->text_selectable);
      if (has(patch, "wrap") || text_control_replaced)
        gtk_label_set_wrap(GTK_LABEL(node->control),
                           node->text_wrap);
    }
    if (GTK_IS_TEXT_VIEW(node->control) && has(patch, "editable")) {
      gboolean editable = boolean(patch, "editable", FALSE);
      gtk_text_view_set_editable(GTK_TEXT_VIEW(node->control), editable);
      gtk_text_view_set_cursor_visible(GTK_TEXT_VIEW(node->control), editable);
    }
    if (GTK_IS_TEXT_VIEW(node->control) && (has(patch, "wrap") || text_control_replaced))
      gtk_text_view_set_wrap_mode(GTK_TEXT_VIEW(node->control),
                                   node->text_wrap ? GTK_WRAP_WORD_CHAR : GTK_WRAP_NONE);
    node->clip = boolean(patch, "clip", node->clip);
  }
  if (strcmp(node->kind, "Textbox") == 0 &&
      boolean(patch, "is_text_area", FALSE) &&
      !GTK_IS_TEXT_VIEW(node->control)) {
    GtkWidget *view = gtk_text_view_new();
    gtk_text_view_set_wrap_mode(GTK_TEXT_VIEW(view), GTK_WRAP_WORD_CHAR);
    g_signal_connect_object(gtk_text_view_get_buffer(GTK_TEXT_VIEW(view)), "changed",
                              G_CALLBACK(text_changed), node, 0);
    node_control(node, view);
  }
  if (GTK_IS_ENTRY(node->control)) {
    if (has(patch, "text") &&
        strcmp(gtk_editable_get_text(GTK_EDITABLE(node->control)),
               string(patch, "text", "")) != 0)
      gtk_editable_set_text(GTK_EDITABLE(node->control),
                            string(patch, "text", ""));
    if (has(patch, "placeholder"))
      gtk_entry_set_placeholder_text(GTK_ENTRY(node->control),
                                     string(patch, "placeholder", ""));
    if (boolean(patch, "focus_on_mount", FALSE))
      gtk_widget_grab_focus(node->control);
  } else if (GTK_IS_TEXT_VIEW(node->control)) {
    const char *key = strcmp(node->kind, "Text") == 0 ? "content" : "text";
    if (has(patch, key)) {
      GtkTextBuffer *buffer =
          gtk_text_view_get_buffer(GTK_TEXT_VIEW(node->control));
      GtkTextIter a, b;
      gtk_text_buffer_get_bounds(buffer, &a, &b);
      char *current = gtk_text_buffer_get_text(buffer, &a, &b, TRUE);
      if (strcmp(current, string(patch, key, "")) != 0)
        gtk_text_buffer_set_text(buffer, string(patch, key, ""), -1);
      g_free(current);
    }
  } else if (GTK_IS_BUTTON(node->control) && has(patch, "content")) {
    gtk_button_set_label(GTK_BUTTON(node->control), string(patch, "content", ""));
    if (node->form_attributes)
      form_label_attributes(node->control, node);
  } else if (GTK_IS_CHECK_BUTTON(node->control) && has(patch, "checked"))
    gtk_check_button_set_active(GTK_CHECK_BUTTON(node->control),
                                boolean(patch, "checked", FALSE));
  if (GTK_IS_SCALE(node->control)) {
    double min = number(patch, "min", node->range_min);
    double max = number(patch, "max", node->range_max);
    double step = number(patch, "step", node->range_step);
    double value = number(patch, "value", node->range_value);
    if (!isfinite(min) || !isfinite(max) || !isfinite(step) || !isfinite(value) ||
        max < min || step <= 0 || !isfinite((max - min) / step)) {
      g_set_error(error, G_IO_ERROR, G_IO_ERROR_INVALID_DATA,
                   "Slider needs finite min <= max, value, and positive finite step");
      node->updating = FALSE;
      return FALSE;
    }
    node->range_min = min; node->range_max = max; node->range_step = step;
    // Authored state remains exact within the range. Step normalization belongs
    // to user interaction and must not silently rewrite a reflected Rust value.
    node->range_value = CLAMP(value, min, max);
    gtk_adjustment_configure(gtk_range_get_adjustment(GTK_RANGE(node->control)),
                              node->range_value, min, max, step,
                              MIN(max - min, step * 10), 0);
  } else if ((GTK_IS_DROP_DOWN(node->control) || strcmp(node->kind, "RadioList") == 0) &&
             !update_options(node, patch, error)) {
    node->updating = FALSE;
    return FALSE;
  }
  if (is_form(node) || strcmp(node->kind, "Button") == 0 ||
      strcmp(node->kind, "Textbox") == 0)
    form_appearance(node, patch);
  if (GTK_IS_SCROLLED_WINDOW(node->control)) {
    double inner_width = number(patch, "size_inner_pane_x", node->inner_width);
    double inner_height = number(patch, "size_inner_pane_y", node->inner_height);
    if (!isfinite(node->width) || !isfinite(node->height) ||
        !isfinite(inner_width) || !isfinite(inner_height) ||
        node->width > G_MAXINT || node->height > G_MAXINT ||
        inner_width > G_MAXINT || inner_height > G_MAXINT) {
      g_set_error(error, G_IO_ERROR, G_IO_ERROR_INVALID_DATA,
                   "Scroller extents must be finite and fit GTK integer dimensions");
      node->updating = FALSE;
      return FALSE;
    }
    // Responsive expressions can be negative while a parent is collapsed.
    // GTK requires a legal empty box; later positive patches restore its size.
    node->inner_width = MAX(0, MAX(node->width, inner_width));
    node->inner_height = MAX(0, MAX(node->height, inner_height));
    gtk_widget_set_size_request(node->content, (int)ceil(node->inner_width),
                                (int)ceil(node->inner_height));
    GtkScrolledWindow *scroll = GTK_SCROLLED_WINDOW(node->control);
    GtkPolicyType horizontal, vertical;
    gtk_scrolled_window_get_policy(scroll, &horizontal, &vertical);
    gtk_scrolled_window_set_policy(
        scroll,
        has(patch, "scroll_enabled_x")
            ? (boolean(patch, "scroll_enabled_x", TRUE) ? GTK_POLICY_AUTOMATIC
                                                        : GTK_POLICY_NEVER)
            : horizontal,
        has(patch, "scroll_enabled_y")
            ? (boolean(patch, "scroll_enabled_y", TRUE) ? GTK_POLICY_AUTOMATIC
                                                        : GTK_POLICY_NEVER)
            : vertical);
    const char *point_keys[] = {"snap_points_x", "snap_points_y"};
    gboolean snap_lists_changed = FALSE;
    for (guint axis = 0; axis < 2; axis++) {
      if (!has(patch, point_keys[axis])) continue;
      JsonArray *points = json_object_get_array_member(patch, point_keys[axis]);
      GArray *authored = g_array_new(FALSE, FALSE, sizeof(double));
      for (guint i = 0; i < json_array_get_length(points); i++) {
        double point = json_array_get_double_element(points, i);
        if (isfinite(point)) g_array_append_val(authored, point);
      }
      g_array_sort(authored, snap_compare);
      GArray *old = node->snap_authored[axis];
      if (old && old->len == authored->len &&
          (!authored->len || !memcmp(old->data, authored->data, authored->len * sizeof(double)))) {
        g_array_unref(authored);
      } else {
        g_clear_pointer(&node->snap_authored[axis], g_array_unref);
        node->snap_authored[axis] = authored;
        snap_lists_changed = TRUE;
      }
    }
    if (snap_lists_changed) snap_refresh(node);
    const char *position_keys[] = {"scroll_x", "scroll_y"};
    const char *presentation_keys[] = {"presentation_scroll_x", "presentation_scroll_y"};
    gboolean requested = FALSE;
    for (guint axis = 0; axis < 2; axis++) {
      if (!has(patch, position_keys[axis]) && !has(patch, presentation_keys[axis])) continue;
      GtkAdjustment *adjustment = scroll_adjustment(node, axis);
      double value = number(patch, presentation_keys[axis], number(patch, position_keys[axis], 0));
      // Engine echoes of native movement must not cancel momentum or snapping.
      if (!isfinite(value)) continue;
      double current = node->scroll_request_pending[axis]
                           ? node->requested_scroll[axis]
                           : gtk_adjustment_get_value(adjustment);
      if (fabs(value - current) <= 1e-4) continue;
      if (!requested) snap_cancel(node);
      requested = TRUE;
      node->snap_direction[axis] = 0;
      node->requested_scroll[axis] = value;
      node->scroll_request_pending[axis] = TRUE;
    }
    if (requested) {
      // Keep only the latest request per axis until this commit's geometry is
      // allocated. The eventual acknowledgment includes GTK's legal clamping.
      gtk_widget_queue_allocate(GTK_WIDGET(node));
    }

  }
  if (strcmp(node->kind, "EventBlocker") == 0 && has(patch, "background"))
    node->blocker_background = message_color(json_object_get_object_member(patch, "background"));
  if (!apply_style(node, patch, error)) {
    node->updating = FALSE;
    return FALSE;
  }
  if (text_control_replaced && GTK_IS_LABEL(node->control))
    form_label_attributes(node->control, node);
  if (is_text && GTK_IS_LABEL(node->control) &&
      (json_object_has_member(patch, "rich_text") || has(patch, "style") ||
       has(patch, "style_link") || has(patch, "measure_generation"))) {
    if (node->rich_text) {
      if (!rich_text_attributes(node, error)) { node->updating = FALSE; return FALSE; }
    } else {
      gboolean zero = FALSE;
      if (node->form_attributes) {
        PangoAttrIterator *iterator = pango_attr_list_get_iterator(node->form_attributes);
        PangoAttrInt *alpha = (PangoAttrInt *)pango_attr_iterator_get(iterator, PANGO_ATTR_FOREGROUND_ALPHA);
        zero = alpha && !alpha->value;
        pango_attr_iterator_destroy(iterator);
      }
      transparent_text(node, zero);
      gtk_label_set_attributes(GTK_LABEL(node->control), node->form_attributes);
    }
  }
  if (node->width < 0 || node->height < 0 || has(patch, "measure_generation")) {
    int min, natural_w, natural_h;
    gtk_widget_measure(node->control, GTK_ORIENTATION_HORIZONTAL, -1, &min,
                       &natural_w, NULL, NULL);
    double width_constraint =
        is_text ? node->text_width_constraint : node->width;
    int for_width = is_text
                        ? (width_constraint >= 0 ? (int)ceil(width_constraint)
                                                 : natural_w)
                        : (node->width > 0 ? (int)node->width : natural_w);
    gtk_widget_measure(node->control, GTK_ORIENTATION_VERTICAL,
                       for_width, &min, &natural_h, NULL, NULL);
    if (is_text ? node->text_width_constraint < 0 : node->width < 0)
      node->width = natural_w;
    if (is_text ? node->text_height_constraint < 0 : node->height < 0)
      node->height = natural_h;
    if (has(patch, "measure_generation")) {
      if (is_text)
        node->latest_text_measure_generation =
            json_object_get_int_member(patch, "measure_generation");
      JsonObject *body = json_object_new();
      json_object_set_int_member(body, "id", node->id);
      json_object_set_int_member(
          body, "generation",
          json_object_get_int_member(patch, "measure_generation"));
      json_object_set_double_member(body, "width",
                                    is_text ? node->width : natural_w);
      json_object_set_double_member(body, "height",
                                    is_text ? node->height : natural_h);
      send_object(node->scene, "TextMeasurementResponse", body);
    }
  }
  // Snapshot clipping alone does not constrain descendant event picking.
  gtk_widget_set_overflow(GTK_WIDGET(node), node->clip ? GTK_OVERFLOW_HIDDEN
                                                       : GTK_OVERFLOW_VISIBLE);
  gtk_widget_set_opacity(GTK_WIDGET(node), node->opacity);
  gtk_widget_queue_resize(GTK_WIDGET(node));
  gtk_widget_queue_draw(GTK_WIDGET(node));
  node->updating = FALSE;
  return TRUE;
}
static int compare_nodes(gconstpointer a, gconstpointer b) {
  const PaxNode *left = *(PaxNode *const *)a, *right = *(PaxNode *const *)b;
  // Native creation payloads use layer zero; current native placement is
  // parent_frame + z_index. Masks reveal foreground vectors through native
  // widgets, so every GPU tile belongs below native peers in this container.
  // Ordering by the stale creation layer hides scroller controls behind their
  // owned layer's GPU background.
  if (!!left->tile_key != !!right->tile_key)
    return left->tile_key ? -1 : 1;
  if (left->tile_key && left->layer != right->layer)
    return left->layer < right->layer ? -1 : 1;
  if (left->z != right->z)
    return left->z < right->z ? -1 : 1;
  return left->id < right->id ? -1 : left->id > right->id;
}
static void reconcile(PaxNativeScene *scene) {
  scene->placement_reconciliations++;
  GPtrArray *all = g_ptr_array_new();
  GHashTableIter iter;
  gpointer value;
  g_hash_table_iter_init(&iter, scene->tiles);
  while (g_hash_table_iter_next(&iter, NULL, &value))
    g_ptr_array_add(all, value);
  g_hash_table_iter_init(&iter, scene->nodes);
  while (g_hash_table_iter_next(&iter, NULL, &value))
    g_ptr_array_add(all, value);
  // Scroller patches stay world-space. Resolve retained Text/Frame poses
  // before placing children, independently of creation and z-order.
  for (guint i = 0; i < all->len; i++) {
    PaxNode *node = g_ptr_array_index(all, i);
    if (node->tile_key) continue;
    double *m = node->authored_transform;
    memcpy(node->matrix, m, sizeof(node->matrix));
    if (node->transform_scroller >= 0) {
      PaxNode *owner = g_hash_table_lookup(scene->nodes,
          GUINT_TO_POINTER((guint)node->transform_scroller));
      if (!owner || strcmp(owner->kind, "Scroller") || owner->transform_scroller >= 0) {
        // An incomplete owner cannot safely place or expose its native child.
        memset(node->matrix, 0, sizeof(node->matrix));
        continue;
      }
      double *p = owner->authored_transform, *out = node->matrix;
      out[0]=p[0]*m[0]+p[2]*m[1]; out[1]=p[1]*m[0]+p[3]*m[1];
      out[2]=p[0]*m[2]+p[2]*m[3]; out[3]=p[1]*m[2]+p[3]*m[3];
      out[4]=p[0]*m[4]+p[2]*m[5]+p[4]; out[5]=p[1]*m[4]+p[3]*m[5]+p[5];
    }
  }
  g_ptr_array_sort(all, compare_nodes);
  scene->placement_visits += all->len;
  for (guint i = 0; i < all->len; i++) {
    PaxNode *node = g_ptr_array_index(all, i);
    if (node->tile_key)
      node->parent_id =
          (gint64)GPOINTER_TO_SIZE(g_hash_table_lookup(
              scene->layer_owners, GUINT_TO_POINTER(node->layer))) -
          1;
    PaxNode *parent =
        node->parent_id >= 0
            ? g_hash_table_lookup(scene->nodes,
                                  GUINT_TO_POINTER((guint)node->parent_id))
            : NULL;
    GtkWidget *container =
        parent && parent->content ? parent->content : scene_root(scene);
    if (gtk_widget_get_parent(GTK_WIDGET(node)) != container) {
      g_object_ref(node);
      if (gtk_widget_get_parent(GTK_WIDGET(node)))
        gtk_widget_unparent(GTK_WIDGET(node));
      gtk_fixed_put(GTK_FIXED(container), GTK_WIDGET(node), 0, 0);
      g_object_unref(node);
    }
    graphene_matrix_t world, relative;
    node_matrix(node, &world);
    relative = world;
    gboolean visible = node->tile_key || node_matrix_invertible(node);
    if (parent && !node->tile_key) {
      graphene_matrix_t parent_world, inverse;
      node_matrix(parent, &parent_world);
      if (node_matrix_invertible(parent) &&
          graphene_matrix_inverse(&parent_world, &inverse))
        graphene_matrix_multiply(&world, &inverse, &relative);
      else
        visible = FALSE;
    }
    // Pango cannot reload fonts at a zero projected scale. Retain the native
    // subtree, but keep a singular plane out of GTK's paint/input hierarchy
    // with a safe transform until its placement is representable again.
    if (!visible) graphene_matrix_init_identity(&relative);
    gtk_widget_set_child_visible(GTK_WIDGET(node), visible);
    GskTransform *transform = gsk_transform_matrix(NULL, &relative);
    gtk_fixed_set_child_transform(GTK_FIXED(container), GTK_WIDGET(node),
                                  transform);
    gsk_transform_unref(transform);
    // The native identity is unchanged: reordering preserves editing and
    // selection.
    gtk_widget_insert_before(GTK_WIDGET(node), container, NULL);
  }
  g_ptr_array_unref(all);
  gtk_widget_queue_allocate(GTK_WIDGET(scene));
}
static void request_reconcile(PaxNativeScene *scene) {
  if (scene->commit_open)
    scene->placement_pending = TRUE;
  else
    reconcile(scene);
}
void pax_native_scene_begin_commit(PaxNativeScene *scene) {
  g_return_if_fail(!scene->commit_open);
  scene->commit_open = TRUE;
  scene->placement_pending = FALSE;
}
void pax_native_scene_end_commit(PaxNativeScene *scene, gboolean publish) {
  g_return_if_fail(scene->commit_open);
  gboolean pending = scene->placement_pending;
  scene->placement_pending = FALSE;
  scene->commit_open = FALSE;
  if (publish && pending)
    reconcile(scene);
}
gboolean pax_native_scene_apply(PaxNativeScene *scene, JsonArray *messages,
                                GError **error) {
  for (guint i = 0; i < json_array_get_length(messages); i++) {
    JsonObject *message = json_array_get_object_element(messages, i);
    GList *keys = json_object_get_members(message);
    if (!keys)
      continue;
    const char *kind = keys->data;
    JsonNode *payload = json_object_get_member(message, kind);
    if (g_str_has_suffix(kind, "Create")) {
      char *name = g_strndup(kind, strlen(kind) - 6);
      PaxNode *node =
          create_node(scene, name, json_node_get_object(payload), error);
      g_free(name);
      g_list_free(keys);
      if (!node)
        return FALSE;
      continue;
    }
    if (strcmp(kind, "ShrinkLayersTo") == 0)
      scene->layer_count = MAX(1, json_node_get_int(payload));
    else if (strcmp(kind, "NativeCullUpdate") ==
             0) { /* Advisory culling never discards a focused editor. GTK clips
                     cold widgets. */
    } else if (g_str_has_suffix(kind, "Delete")) {
      guint id = json_node_get_int(payload);
      PaxNode *node = g_hash_table_lookup(scene->nodes, GUINT_TO_POINTER(id));
      if (node) native_image_release(node);
      if (node && gtk_widget_get_parent(GTK_WIDGET(node)))
        gtk_widget_unparent(GTK_WIDGET(node));
      g_hash_table_remove(scene->nodes, GUINT_TO_POINTER(id));
    } else if (g_str_has_suffix(kind, "Update") &&
               JSON_NODE_HOLDS_OBJECT(payload)) {
      JsonObject *patch = json_node_get_object(payload);
      guint id = json_object_get_int_member(patch, "id");
      PaxNode *node = g_hash_table_lookup(scene->nodes, GUINT_TO_POINTER(id));
      if (node) {
        gboolean ok = strcmp(kind, "NativeMaskUpdate") == 0
                          ? set_mask(node, patch, error)
                          : update_node(node, patch, error);
        if (!ok) {
          g_list_free(keys);
          return FALSE;
        }
      }
    } else if (strcmp(kind, "SetCursor") == 0)
      gtk_widget_set_cursor_from_name(
          GTK_WIDGET(scene),
          string(json_node_get_object(payload), "cursor", "default"));
    else {
      g_set_error(error, G_IO_ERROR, G_IO_ERROR_NOT_SUPPORTED,
                  "Linux native message %s needs a platform implementation",
                  kind);
      g_list_free(keys);
      return FALSE;
    }
    g_list_free(keys);
  }
  // Empty accepted frames still deliver pending text-metric invalidations,
  // but need no placement unless an owner, tile, or native patch changed.
  // Standalone apply retains its immediate reconciliation behavior.
  if (!scene->commit_open || json_array_get_length(messages) != 0)
    request_reconcile(scene);
  guint font_serial = pango_font_map_get_serial(pango_context_get_font_map(
      gtk_widget_get_pango_context(GTK_WIDGET(scene))));
  if (scene->font_map_serial != font_serial) {
    scene->font_map_serial = font_serial;
    scene->text_metrics_dirty = TRUE;
  }
  if (scene->text_metrics_dirty) {
    scene->text_metrics_dirty = FALSE;
    GHashTableIter iterator;
    gpointer value;
    g_hash_table_iter_init(&iterator, scene->nodes);
    while (g_hash_table_iter_next(&iterator, NULL, &value)) {
      PaxNode *node = value;
      // Absolute preformatted line spacing was derived from the previous
      // font resolution. Fixed-size labels need this refresh too, even though
      // only autosized nodes request a new engine measurement generation.
      if (node->rich_text && has(node->rich_text, "line_height") &&
          GTK_IS_LABEL(node->control) && !rich_text_attributes(node, error))
        return FALSE;
      if (!node->latest_text_measure_generation ||
          (node->text_width_constraint >= 0 &&
           node->text_height_constraint >= 0))
        continue;
      JsonObject *body = json_object_new();
      json_object_set_int_member(body, "id", node->id);
      json_object_set_int_member(body, "generation",
                                 node->latest_text_measure_generation);
      scene->text_metrics_invalidations++;
      send_object(scene, "TextMeasurementInvalidated", body);
    }
  }
  return TRUE;
}
guint pax_native_scene_layer_count(PaxNativeScene *scene) {
  return scene->layer_count;
}
void pax_native_scene_layer_owner(PaxNativeScene *scene, guint layer,
                                  gint64 owner) {
  gint64 previous = (gint64)GPOINTER_TO_SIZE(g_hash_table_lookup(
                         scene->layer_owners, GUINT_TO_POINTER(layer))) - 1;
  g_hash_table_insert(scene->layer_owners, GUINT_TO_POINTER(layer),
                      GSIZE_TO_POINTER(owner + 1));
  if (scene->commit_open && previous != owner)
    scene->placement_pending = TRUE;
}
void pax_native_scene_texture(PaxNativeScene *scene, JsonObject *surface,
                              GdkTexture *texture) {
  guint layer = json_object_get_int_member(surface, "layer");
  const char *key = json_object_get_string_member(surface, "key");
  char *identity = g_strdup_printf("%u:%s", layer, key);
  PaxNode *node = g_hash_table_lookup(scene->tiles, identity);
  if (!node) {
    node = g_object_new(pax_node_get_type(), NULL);
    g_object_ref_sink(node);
    node->scene = scene;
    node->kind = g_strdup("tile");
    node->tile_key = g_strdup(key);
    node->layer = layer;
    node->z = G_MININT;
    gtk_widget_set_can_target(GTK_WIDGET(node), FALSE);
    g_hash_table_insert(scene->tiles, g_strdup(identity), node);
  }
  node->parent_id = (gint64)GPOINTER_TO_SIZE(g_hash_table_lookup(
                        scene->layer_owners, GUINT_TO_POINTER(layer))) -
                    1;
  node->width = number(surface, "logicalWidth", 1);
  node->height = number(surface, "logicalHeight", 1);
  node->matrix[4] = number(surface, "originX", 0);
  node->matrix[5] = number(surface, "originY", 0);
  // Retain layer-local coordinates so moving an owning scroller also moves
  // clean tiles that do not publish another texture in this frame.
  g_set_object(&node->texture, texture);
  g_free(identity);
  request_reconcile(scene);
  gtk_widget_queue_draw(GTK_WIDGET(node));
}
void pax_native_scene_prune_tiles(PaxNativeScene *scene, JsonArray *surfaces) {
  GHashTable *keep =
      g_hash_table_new_full(g_str_hash, g_str_equal, g_free, NULL);
  for (guint i = 0; i < json_array_get_length(surfaces); i++) {
    JsonObject *s = json_array_get_object_element(surfaces, i);
    g_hash_table_add(
        keep,
        g_strdup_printf("%u:%s", (guint)json_object_get_int_member(s, "layer"),
                        json_object_get_string_member(s, "key")));
  }
  GHashTableIter iter;
  gpointer key, value;
  g_hash_table_iter_init(&iter, scene->tiles);
  while (g_hash_table_iter_next(&iter, &key, &value))
    if (!g_hash_table_contains(keep, key)) {
      if (gtk_widget_get_parent(GTK_WIDGET(value)))
        gtk_widget_unparent(GTK_WIDGET(value));
      g_hash_table_iter_remove(&iter);
      if (scene->commit_open)
        scene->placement_pending = TRUE;
    }
  g_hash_table_unref(keep);
}
void pax_native_scene_stop(PaxNativeScene *scene) {
  restore_pointer_focus_reveal(scene);
  pax_native_scene_release_keys(scene);
  scene->stopped = TRUE;
  native_images_stop(scene);
  memset(scene->pointer_buttons, 0, sizeof scene->pointer_buttons);
  g_hash_table_remove_all(scene->wheel_pointers);
  g_hash_table_remove_all(scene->tiles);
  g_hash_table_remove_all(scene->nodes);
  GtkWidget *root = scene_root(scene);
  if (root)
    gtk_widget_unparent(root);
}

void pax_native_scene_assets(PaxNativeScene *scene, char *const *roots) {
  g_strfreev(scene->assets);
  scene->assets = g_strdupv((char **)roots);
}
static double test_mask_alpha(PaxNode *node, double x, double y) {
  double alpha = 1;
  for (guint i = 0; i < node->mask->len; i++) {
    MaskEntry *entry = g_ptr_array_index(node->mask, i);
    gboolean inside = path_contains(entry->path, x, y);
    for (guint j = 0; inside && j < entry->clips->len; j++)
      inside = path_contains(g_ptr_array_index(entry->clips, j), x, y);
    if (inside)
      alpha *= 1 - entry->opacity;
  }
  return alpha;
}
static void report_mask_point(JsonObject *item, PaxNode *node,
                              GtkWidget *coordinates, double sx, double sy,
                              gboolean fractional) {
  for (guint row = 1; row < 16; row++)
    for (guint column = 1; column < 64; column++) {
      double x = node->width * column / 64, y = node->height * row / 16;
      double alpha = test_mask_alpha(node, x, y);
      gboolean matches = fractional ? alpha > .01 && alpha < .99 : alpha < .01;
      // Keep rounded integer X11 coordinates away from raster boundaries.
      for (int dx = -2; matches && dx <= 2; dx += 4)
        for (int dy = -2; matches && dy <= 2; dy += 4)
          matches = fabs(test_mask_alpha(node, x + dx, y + dy) - alpha) < .01;
      if (!matches)
        continue;
      graphene_point_t from = GRAPHENE_POINT_INIT(x, y), to;
      if (!gtk_widget_compute_point(GTK_WIDGET(node), coordinates, &from, &to))
        return;
      JsonArray *point = json_array_new();
      json_array_add_double_element(point, to.x - sx);
      json_array_add_double_element(point, to.y - sy);
      json_array_add_double_element(point, alpha);
      json_object_set_array_member(
          item, fractional ? "fractionalPoint" : "opaquePoint", point);
      return;
    }
}
static JsonObject *report_text_style(GtkWidget *control) {
  if (GTK_IS_BUTTON(control))
    control = gtk_button_get_child(GTK_BUTTON(control));
  PangoAttrList *attributes = GTK_IS_LABEL(control)
      ? gtk_label_get_attributes(GTK_LABEL(control))
      : GTK_IS_ENTRY(control) ? gtk_entry_get_attributes(GTK_ENTRY(control)) : NULL;
  int weight = PANGO_WEIGHT_NORMAL, slant = PANGO_STYLE_NORMAL,
      underline = PANGO_UNDERLINE_NONE;
  double points = 0;
  GdkRGBA color = {0, 0, 0, 1};
  if (attributes) {
    PangoAttrIterator *iterator = pango_attr_list_get_iterator(attributes);
    PangoAttrInt *value = (PangoAttrInt *)pango_attr_iterator_get(iterator, PANGO_ATTR_WEIGHT);
    if (value) weight = value->value;
    value = (PangoAttrInt *)pango_attr_iterator_get(iterator, PANGO_ATTR_STYLE);
    if (value) slant = value->value;
    value = (PangoAttrInt *)pango_attr_iterator_get(iterator, PANGO_ATTR_UNDERLINE);
    if (value) underline = value->value;
    PangoAttrSize *size = (PangoAttrSize *)pango_attr_iterator_get(iterator, PANGO_ATTR_SIZE);
    if (size) points = size->size / (double)PANGO_SCALE;
    PangoAttrColor *foreground = (PangoAttrColor *)pango_attr_iterator_get(iterator, PANGO_ATTR_FOREGROUND);
    if (foreground) {
      color.red = foreground->color.red / 65535.0f;
      color.green = foreground->color.green / 65535.0f;
      color.blue = foreground->color.blue / 65535.0f;
    }
    value = (PangoAttrInt *)pango_attr_iterator_get(iterator, PANGO_ATTR_FOREGROUND_ALPHA);
    if (value) color.alpha = value->value / 65535.0f;
    pango_attr_iterator_destroy(iterator);
  } else if (GTK_IS_TEXT_VIEW(control)) {
    GtkTextBuffer *buffer = gtk_text_view_get_buffer(GTK_TEXT_VIEW(control));
    GtkTextTag *tag = gtk_text_tag_table_lookup(gtk_text_buffer_get_tag_table(buffer), "pax-style");
    if (!tag) return NULL;
    GdkRGBA *foreground = NULL;
    g_object_get(tag, "weight", &weight, "style", &slant, "underline", &underline,
                  "size-points", &points, "foreground-rgba", &foreground, NULL);
    if (foreground) { color = *foreground; gdk_rgba_free(foreground); }
  } else return NULL;
  JsonObject *style = json_object_new();
  json_object_set_int_member(style, "weight", weight);
  json_object_set_int_member(style, "slant", slant);
  json_object_set_int_member(style, "underline", underline);
  json_object_set_double_member(style, "sizePoints", points);
  JsonArray *fill = json_array_new();
  json_array_add_double_element(fill, color.red);
  json_array_add_double_element(fill, color.green);
  json_array_add_double_element(fill, color.blue);
  json_array_add_double_element(fill, color.alpha);
  json_object_set_array_member(style, "fill", fill);
  return style;
}
JsonObject *pax_native_scene_report(PaxNativeScene *scene) {
  JsonObject *report = json_object_new();
  json_object_set_boolean_member(report, "canvasHasFocus", gtk_widget_has_focus(GTK_WIDGET(scene)));
  GtkRoot *root = gtk_widget_get_root(GTK_WIDGET(scene));
  if (GTK_IS_WINDOW(root))
    json_object_set_boolean_member(report, "windowActive", gtk_window_is_active(GTK_WINDOW(root)));
  json_object_set_int_member(report, "scenePointerPresses",
                             scene->scene_pointer_presses);
  json_object_set_int_member(report, "scenePointerReleases", scene->scene_pointer_releases);
  json_object_set_int_member(report, "scenePointerClicks", scene->scene_pointer_clicks);
  int font_dpi;
  g_object_get(gtk_widget_get_settings(GTK_WIDGET(scene)), "gtk-xft-dpi",
               &font_dpi, NULL);
  json_object_set_int_member(report, "gtkFontDpi", font_dpi);
  json_object_set_int_member(report, "widgetScaleFactor",
                             gtk_widget_get_scale_factor(GTK_WIDGET(scene)));
  json_object_set_int_member(report, "textMetricsInvalidations",
                             scene->text_metrics_invalidations);
  json_object_set_int_member(report, "sceneReconciliations",
                             scene->placement_reconciliations);
  json_object_set_int_member(report, "scenePlacementVisits",
                             scene->placement_visits);
  json_object_set_int_member(report, "nativeImageSources", g_hash_table_size(scene->native_images));
  json_object_set_int_member(report, "nativeImageBytes", scene->image_bytes);
  json_object_set_int_member(report, "nativeImagePending", scene->image_pending);
  json_object_set_int_member(report, "nativeImageQueued", g_queue_get_length(&scene->image_queue));
  JsonArray *nodes = json_array_new();
  GtkNative *native = gtk_widget_get_native(GTK_WIDGET(scene));
  GtkWidget *coordinates = native ? GTK_WIDGET(native) : GTK_WIDGET(scene);
  double sx = 0, sy = 0;
  if (native)
    gtk_native_get_surface_transform(native, &sx, &sy);
  graphene_rect_t scene_bounds;
  if (gtk_widget_compute_bounds(GTK_WIDGET(scene), coordinates, &scene_bounds)) {
    JsonArray *rect = json_array_new();
    json_array_add_double_element(rect, scene_bounds.origin.x - sx);
    json_array_add_double_element(rect, scene_bounds.origin.y - sy);
    json_array_add_double_element(rect, scene_bounds.size.width);
    json_array_add_double_element(rect, scene_bounds.size.height);
    json_object_set_array_member(report, "sceneBoundsInSurface", rect);
  }
  GHashTableIter iter;
  gpointer value;
  g_hash_table_iter_init(&iter, scene->nodes);
  while (g_hash_table_iter_next(&iter, NULL, &value)) {
    PaxNode *node = value;
    JsonObject *item = json_object_new();
    json_object_set_int_member(item, "id", node->id);
    json_object_set_string_member(item, "kind", node->kind);
    json_object_set_string_member(item, "controlType", G_OBJECT_TYPE_NAME(node->control));
    JsonObject *text_style = report_text_style(node->control);
    if (text_style)
      json_object_set_object_member(item, "textStyle", text_style);
    json_object_set_int_member(item, "accessibilityRole",
                               gtk_accessible_get_accessible_role(GTK_ACCESSIBLE(node->control)));
    GtkWidget *focus = root ? gtk_root_get_focus(root) : NULL;
    json_object_set_boolean_member(item, "focused",
        focus && (focus == node->control || gtk_widget_is_ancestor(focus, node->control)));
    if (GTK_IS_PICTURE(node->control)) {
      NativeImage *image = node->native_image;
      json_object_set_string_member(item, "url", node->image_url ? node->image_url : "");
      json_object_set_boolean_member(item, "imageReady", image && image->texture);
      json_object_set_boolean_member(item, "imageFailed", image && image->failed);
      json_object_set_int_member(item, "imageWidth", image && image->texture ? gdk_texture_get_width(image->texture) : 0);
      json_object_set_int_member(item, "imageHeight", image && image->texture ? gdk_texture_get_height(image->texture) : 0);
      json_object_set_int_member(item, "imageUsers", image ? image->users : 0);
      json_object_set_int_member(item, "contentFit", gtk_picture_get_content_fit(GTK_PICTURE(node->control)));
    }
    if (is_form(node)) {
      json_object_set_int_member(item, "controlEvents", node->control_events);
      if (GTK_IS_SCALE(node->control)) {
        json_object_set_double_member(item, "value", gtk_range_get_value(GTK_RANGE(node->control)));
        json_object_set_double_member(item, "min", node->range_min);
        json_object_set_double_member(item, "max", node->range_max);
        json_object_set_double_member(item, "step", node->range_step);
      } else {
        JsonArray *options = json_array_new();
        for (guint i = 0; node->options[i]; i++)
          json_array_add_string_element(options, node->options[i]);
        json_object_set_array_member(item, "options", options);
        json_object_set_int_member(item, "requestedSelectedId", node->selected_id);
        guint selected = GTK_INVALID_LIST_POSITION;
        if (GTK_IS_DROP_DOWN(node->control))
          selected = gtk_drop_down_get_selected(GTK_DROP_DOWN(node->control));
        else {
          guint index = 0;
          for (GtkWidget *child = gtk_widget_get_first_child(node->control); child;
               child = gtk_widget_get_next_sibling(child), index++)
            if (gtk_check_button_get_active(GTK_CHECK_BUTTON(child)))
              selected = index;
        }
        if (selected == GTK_INVALID_LIST_POSITION)
          json_object_set_null_member(item, "selectedId");
        else
          json_object_set_int_member(item, "selectedId", selected);
      }
    }
    graphene_rect_t bounds;
    if (gtk_widget_compute_bounds(GTK_WIDGET(node), coordinates, &bounds)) {
      JsonArray *rect = json_array_new();
      // Native surface coordinates can differ from widget coordinates because
      // of decorations. These are the actual coordinates used by X11 input.
      json_array_add_double_element(rect, bounds.origin.x - sx);
      json_array_add_double_element(rect, bounds.origin.y - sy);
      json_array_add_double_element(rect, bounds.size.width);
      json_array_add_double_element(rect, bounds.size.height);
      json_object_set_array_member(item, "bounds", rect);
    }
    json_object_set_int_member(item, "maskEntries", node->mask->len);
    json_object_set_boolean_member(item, "placementVisible",
                                  gtk_widget_get_child_visible(GTK_WIDGET(node)));
    graphene_point_t local = GRAPHENE_POINT_INIT(node->width * .8,
                                                 node->height * .75),
                     scene_point;
    if (gtk_widget_compute_point(GTK_WIDGET(node), GTK_WIDGET(scene), &local,
                                 &scene_point)) {
      GtkWidget *picked = gtk_widget_pick(GTK_WIDGET(scene), scene_point.x,
                                          scene_point.y, GTK_PICK_DEFAULT);
      json_object_set_string_member(
          item, "samplePickType", picked ? G_OBJECT_TYPE_NAME(picked) : "none");
      for (GtkWidget *parent = picked; parent;
           parent = gtk_widget_get_parent(parent))
        if (G_TYPE_CHECK_INSTANCE_TYPE(parent, pax_node_get_type())) {
          json_object_set_int_member(item, "samplePickNode",
                                     ((PaxNode *)parent)->id);
          break;
        }
    }
    if (GTK_IS_ENTRY(node->control)) {
      if (node->mask->len) {
        report_mask_point(item, node, coordinates, sx, sy, FALSE);
        report_mask_point(item, node, coordinates, sx, sy, TRUE);
      }
      json_object_set_string_member(
          item, "text", gtk_editable_get_text(GTK_EDITABLE(node->control)));
      int start = 0, end = 0;
      gtk_editable_get_selection_bounds(GTK_EDITABLE(node->control), &start,
                                        &end);
      json_object_set_int_member(item, "selectionStart", start);
      json_object_set_int_member(item, "selectionEnd", end);
      GtkRoot *root = gtk_widget_get_root(node->control);
      GtkWidget *focus = root ? gtk_root_get_focus(root) : NULL;
      json_object_set_boolean_member(
          item, "focused",
          focus && (focus == node->control ||
                    gtk_widget_is_ancestor(focus, node->control)));
    } else if (GTK_IS_LABEL(node->control)) {
      json_object_set_string_member(
          item, "text", gtk_label_get_text(GTK_LABEL(node->control)));
      json_object_set_int_member(item, "richTextRunCount", node->rich_text
          ? json_array_get_length(json_object_get_array_member(node->rich_text, "runs")) : 0);
      json_object_set_boolean_member(item, "selectable", gtk_label_get_selectable(GTK_LABEL(node->control)));
      int selection_start = 0, selection_end = 0;
      gtk_label_get_selection_bounds(GTK_LABEL(node->control),
                                      &selection_start, &selection_end);
      json_object_set_int_member(item, "selectionStart", selection_start);
      json_object_set_int_member(item, "selectionEnd", selection_end);
      PangoLayout *layout = gtk_label_get_layout(GTK_LABEL(node->control));
      // A wrapper allocation alone cannot establish glyph containment: GTK may
      // enlarge its child to the label's minimum size. Report both in local
      // coordinates, including the actual drawing offset, for opt-in checks.
      JsonObject *text_layout = json_object_new();
      json_object_set_int_member(text_layout, "controlWidth",
                                 gtk_widget_get_width(node->control));
      json_object_set_int_member(text_layout, "controlHeight",
                                 gtk_widget_get_height(node->control));
      int layout_x, layout_y;
      gtk_label_get_layout_offsets(GTK_LABEL(node->control), &layout_x, &layout_y);
      PangoRectangle extents[2];
      pango_layout_get_pixel_extents(layout, &extents[0], &extents[1]);
      const char *extent_names[] = {"inkBounds", "logicalBounds"};
      for (guint i = 0; i < G_N_ELEMENTS(extents); i++) {
        JsonArray *rectangle = json_array_new();
        json_array_add_int_element(rectangle, layout_x + extents[i].x);
        json_array_add_int_element(rectangle, layout_y + extents[i].y);
        json_array_add_int_element(rectangle, extents[i].width);
        json_array_add_int_element(rectangle, extents[i].height);
        json_object_set_array_member(text_layout, extent_names[i], rectangle);
      }
      json_object_set_int_member(text_layout, "lineCount",
                                 pango_layout_get_line_count(layout));
      json_object_set_int_member(text_layout, "unknownGlyphs",
                                 pango_layout_get_unknown_glyphs_count(layout));
      json_object_set_boolean_member(text_layout, "ellipsized",
                                     pango_layout_is_ellipsized(layout));
      JsonArray *lines = json_array_new();
      PangoLayoutIter *line_iterator = pango_layout_get_iter(layout);
      do {
        PangoRectangle line;
        pango_layout_iter_get_line_extents(line_iterator, NULL, &line);
        pango_extents_to_pixels(NULL, &line);
        JsonArray *rectangle = json_array_new();
        json_array_add_int_element(rectangle, layout_x + line.x);
        json_array_add_int_element(rectangle, layout_y + line.y);
        json_array_add_int_element(rectangle, line.width);
        json_array_add_int_element(rectangle, line.height);
        json_array_add_array_element(lines, rectangle);
      } while (pango_layout_iter_next_line(line_iterator));
      pango_layout_iter_free(line_iterator);
      json_object_set_array_member(text_layout, "lineLogicalBounds", lines);
      json_object_set_object_member(item, "textLayout", text_layout);
      PangoLayoutIter *iterator = pango_layout_get_iter(layout);
      PangoLayoutRun *run = pango_layout_iter_get_run_readonly(iterator);
      if (run && run->item->analysis.font) {
        PangoFont *font = run->item->analysis.font;
        // PangoFc exposes its borrowed matching pattern as a public pointer
        // property. Record the actual file so synthetic slant cannot masquerade
        // as loading an authored italic face in opt-in qualification reports.
        if (g_object_class_find_property(G_OBJECT_GET_CLASS(font), "pattern")) {
          FcPattern *pattern = NULL;
          g_object_get(font, "pattern", &pattern, NULL);
          FcChar8 *file = NULL;
          if (pattern && FcPatternGetString(pattern, FC_FILE, 0, &file) == FcResultMatch)
            json_object_set_string_member(item, "resolvedFontFile", (const char *)file);
        }
        PangoFontDescription *description =
            pango_font_describe(run->item->analysis.font);
        json_object_set_string_member(
            item, "resolvedFont",
            pango_font_description_get_family(description));
        json_object_set_int_member(item, "resolvedFontWeight", pango_font_description_get_weight(description));
        json_object_set_int_member(item, "resolvedFontSlant", pango_font_description_get_style(description));
        char *resolved_description = pango_font_description_to_string(description);
        json_object_set_string_member(item, "resolvedFontDescription", resolved_description);
        g_free(resolved_description);
        pango_font_description_free(description);
      }
      pango_layout_iter_free(iterator);
    } else if (GTK_IS_TEXT_VIEW(node->control)) {
      GtkTextBuffer *buffer = gtk_text_view_get_buffer(GTK_TEXT_VIEW(node->control));
      GtkTextIter start, end;
      gtk_text_buffer_get_bounds(buffer, &start, &end);
      char *text = gtk_text_buffer_get_text(buffer, &start, &end, TRUE);
      json_object_set_string_member(item, "text", text);
      g_free(text);
    } else if (GTK_IS_BUTTON(node->control))
      json_object_set_string_member(
          item, "text", gtk_button_get_label(GTK_BUTTON(node->control)));
    else if (GTK_IS_CHECK_BUTTON(node->control))
      json_object_set_boolean_member(
          item, "checked",
          gtk_check_button_get_active(GTK_CHECK_BUTTON(node->control)));
    else if (GTK_IS_SCROLLED_WINDOW(node->control)) {
      json_object_set_boolean_member(item, "snapPending", node->snap_pending);
      json_object_set_boolean_member(item, "snapAnimating", node->snap_animating);
      json_object_set_boolean_member(item, "snapInputHeld", snap_held(node));
      const char *snap_names[] = {"snapPointsX", "snapPointsY"};
      for (guint axis = 0; axis < 2; axis++) {
        JsonArray *points = json_array_new();
        if (node->snap_points[axis])
          for (guint i = 0; i < node->snap_points[axis]->len; i++)
            json_array_add_double_element(points, g_array_index(node->snap_points[axis], double, i));
        json_object_set_array_member(item, snap_names[axis], points);
      }
      GtkWidget *bars[] = {
        gtk_scrolled_window_get_hscrollbar(GTK_SCROLLED_WINDOW(node->control)),
        gtk_scrolled_window_get_vscrollbar(GTK_SCROLLED_WINDOW(node->control))
      };
      const char *bar_names[] = {"scrollbarBoundsX", "scrollbarBoundsY"};
      for (guint axis = 0; axis < 2; axis++) {
        graphene_rect_t bar_bounds;
        if (gtk_widget_get_mapped(bars[axis]) &&
            gtk_widget_compute_bounds(bars[axis], coordinates, &bar_bounds)) {
          JsonArray *rect = json_array_new();
          json_array_add_double_element(rect, bar_bounds.origin.x - sx);
          json_array_add_double_element(rect, bar_bounds.origin.y - sy);
          json_array_add_double_element(rect, bar_bounds.size.width);
          json_array_add_double_element(rect, bar_bounds.size.height);
          json_object_set_array_member(item, bar_names[axis], rect);
        }
      }
      GtkAdjustment *adjustment = gtk_scrolled_window_get_vadjustment(
          GTK_SCROLLED_WINDOW(node->control));
      json_object_set_double_member(item, "scrollUpper",
                                    gtk_adjustment_get_upper(adjustment));
      json_object_set_double_member(item, "scrollPage",
                                    gtk_adjustment_get_page_size(adjustment));
      json_object_set_double_member(
          item, "scrollX",
          gtk_adjustment_get_value(gtk_scrolled_window_get_hadjustment(
              GTK_SCROLLED_WINDOW(node->control))));
      json_object_set_double_member(
          item, "scrollY",
          gtk_adjustment_get_value(gtk_scrolled_window_get_vadjustment(
              GTK_SCROLLED_WINDOW(node->control))));
    }
    json_array_add_object_element(nodes, item);
  }
  json_object_set_array_member(report, "nodes", nodes);
  return report;
}
