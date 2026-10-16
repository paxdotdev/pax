#include "native-scene.h"
#include "assets.h"
#include <math.h>
#include <string.h>

typedef struct _PaxNode PaxNode;
typedef struct {
  GskPath *path;
  GPtrArray *clips;
  double opacity;
} MaskEntry;
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
  gint64 parent_id;
  int z;
  char *kind, *tile_key;
  GtkWidget *control, *content, *input;
  GdkTexture *texture;
  GPtrArray *mask;
  GskPath *clip_path;
  double matrix[6], width, height, radius, opacity, inner_width, inner_height;
  gboolean clip, updating;
};
struct _PaxNativeScene {
  GtkWidget parent;
  GHashTable *nodes, *tiles, *layer_owners, *pressed_keys;
  PaxSceneInterrupt interrupt;
  PaxSceneError error;
  gpointer data;
  guint layer_count;
  guint64 scene_pointer_presses;
  gboolean stopped;
  char **assets;
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
static void text_changed(GtkTextBuffer *buffer, PaxNode *node) {
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
static void scroll_changed(GtkAdjustment *adjustment, PaxNode *node) {
  (void)adjustment;
  if (node->updating)
    return;
  GtkScrolledWindow *scroller = GTK_SCROLLED_WINDOW(node->control);
  JsonObject *body = json_object_new();
  json_object_set_int_member(body, "id", node->id);
  double x =
      gtk_adjustment_get_value(gtk_scrolled_window_get_hadjustment(scroller));
  double y =
      gtk_adjustment_get_value(gtk_scrolled_window_get_vadjustment(scroller));
  json_object_set_double_member(body, "scroll_x", x);
  json_object_set_double_member(body, "scroll_y", y);
  json_object_set_double_member(body, "presentation_scroll_x", x);
  json_object_set_double_member(body, "presentation_scroll_y", y);
  send_object(node->scene, "ScrollerPosition", body);
}
static void node_matrix(PaxNode *node, graphene_matrix_t *matrix) {
  graphene_matrix_init_from_2d(matrix, node->matrix[0], node->matrix[1],
                               node->matrix[2], node->matrix[3],
                               node->matrix[4], node->matrix[5]);
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
  if (state & GDK_SUPER_MASK)
    json_array_add_string_element(array, "Command");
  json_object_set_array_member(body, "modifiers", array);
}
static gboolean pointer_event(GtkEventControllerLegacy *controller,
                              GdkEvent *event, PaxNativeScene *scene) {
  GdkEventType type = gdk_event_get_event_type(event);
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
  GtkWidget *widget =
      gtk_event_controller_get_widget(GTK_EVENT_CONTROLLER(controller));
  GtkNative *native = gtk_widget_get_native(widget);
  double sx, sy;
  gtk_native_get_surface_transform(native, &sx, &sy);
  graphene_point_t from = GRAPHENE_POINT_INIT(x + sx, y + sy), to;
  if (!gtk_widget_compute_point(GTK_WIDGET(native), GTK_WIDGET(scene), &from,
                                &to))
    return FALSE;
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
  if (type == GDK_BUTTON_RELEASE) {
    JsonObject *copy = json_object_ref(body);
    send_object(scene, kind, body);
    send_object(scene, "Click", copy);
  } else
    send_object(scene, kind, body);
  return FALSE;
}
static void add_pointer_controller(GtkWidget *widget, PaxNativeScene *scene) {
  GtkEventController *controller = gtk_event_controller_legacy_new();
  gtk_event_controller_set_propagation_phase(controller, GTK_PHASE_BUBBLE);
  g_signal_connect(controller, "event", G_CALLBACK(pointer_event), scene);
  gtk_widget_add_controller(widget, controller);
}
static const char *key_name(guint key, char utf8[8]) {
  switch (key) {
  case GDK_KEY_Return:
  case GDK_KEY_KP_Enter:
    return "Enter";
  case GDK_KEY_Tab:
  case GDK_KEY_ISO_Left_Tab:
    return "Tab";
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
  JsonObject *body = json_object_new();
  char utf8[8] = {0};
  json_object_set_string_member(body, "key", key_name(key, utf8));
  json_object_set_boolean_member(
      body, "is_repeat",
      g_hash_table_contains(scene->pressed_keys, GUINT_TO_POINTER(code)));
  g_hash_table_add(scene->pressed_keys, GUINT_TO_POINTER(code));
  append_modifiers(body, state);
  return send_object(scene, "KeyDown", body);
}
static void key_released(GtkEventControllerKey *controller, guint key,
                         guint code, GdkModifierType state,
                         PaxNativeScene *scene) {
  (void)controller;
  g_hash_table_remove(scene->pressed_keys, GUINT_TO_POINTER(code));
  JsonObject *body = json_object_new();
  char utf8[8] = {0};
  json_object_set_string_member(body, "key", key_name(key, utf8));
  json_object_set_boolean_member(body, "is_repeat", FALSE);
  append_modifiers(body, state);
  send_object(scene, "KeyUp", body);
}
static void focus_left(GtkEventControllerFocus *controller,
                       PaxNativeScene *scene) {
  (void)controller;
  // Focus can leave before a physical key release reaches this widget.
  g_hash_table_remove_all(scene->pressed_keys);
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
  if (node->control)
    gtk_widget_allocate(node->control, width, height, baseline, NULL);
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
      node_matrix(node, &world);
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
  if (node->texture) {
    graphene_rect_t rect = GRAPHENE_RECT_INIT(0, 0, node->width, node->height);
    gtk_snapshot_append_texture(snapshot, node->texture, &rect);
  }
  if (node->control)
    gtk_widget_snapshot_child(widget, node->control, snapshot);
  if (node->mask->len)
    gtk_snapshot_pop(snapshot);
  while (clips--)
    gtk_snapshot_pop(snapshot);
  if (transformed_clip)
    gtk_snapshot_restore(snapshot);
}
static void node_dispose(GObject *object) {
  PaxNode *node = (PaxNode *)object;
  while (gtk_widget_get_first_child(GTK_WIDGET(node)))
    gtk_widget_unparent(gtk_widget_get_first_child(GTK_WIDGET(node)));
  node->control = node->input = node->content = NULL;
  g_clear_object(&node->texture);
  G_OBJECT_CLASS(pax_node_parent_class)->dispose(object);
}
static void node_finalize(GObject *object) {
  PaxNode *node = (PaxNode *)object;
  g_free(node->kind);
  g_free(node->tile_key);
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
  node->parent_id = -1;
  node->mask = g_ptr_array_new_with_free_func(mask_free);
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
static void scene_dispose(GObject *object) {
  PaxNativeScene *scene = (PaxNativeScene *)object;
  scene->stopped = TRUE;
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
  g_hash_table_unref(scene->tiles);
  g_hash_table_unref(scene->layer_owners);
  g_hash_table_unref(scene->pressed_keys);
  G_OBJECT_CLASS(pax_native_scene_parent_class)->finalize(object);
}
static void pax_native_scene_class_init(PaxNativeSceneClass *klass) {
  klass->measure = scene_measure;
  klass->size_allocate = scene_allocate;
  klass->snapshot = scene_snapshot;
  G_OBJECT_CLASS(klass)->dispose = scene_dispose;
  G_OBJECT_CLASS(klass)->finalize = scene_finalize;
}
static void pax_native_scene_init(PaxNativeScene *scene) {
  scene->nodes = g_hash_table_new_full(g_direct_hash, g_direct_equal, NULL,
                                       g_object_unref);
  scene->tiles =
      g_hash_table_new_full(g_str_hash, g_str_equal, g_free, g_object_unref);
  scene->layer_owners = g_hash_table_new(g_direct_hash, g_direct_equal);
  scene->pressed_keys = g_hash_table_new(g_direct_hash, g_direct_equal);
  scene->layer_count = 1;
  GtkWidget *root = gtk_fixed_new();
  gtk_widget_set_parent(root, GTK_WIDGET(scene));
  gtk_widget_set_overflow(GTK_WIDGET(scene), GTK_OVERFLOW_HIDDEN);
}
GtkWidget *pax_native_scene_new(PaxSceneInterrupt interrupt,
                                PaxSceneError error, gpointer data) {
  PaxNativeScene *scene = g_object_new(pax_native_scene_get_type(), NULL);
  scene->interrupt = interrupt;
  scene->error = error;
  scene->data = data;
  add_pointer_controller(GTK_WIDGET(scene), scene);
  GtkEventController *keys = gtk_event_controller_key_new();
  gtk_event_controller_set_propagation_phase(keys, GTK_PHASE_CAPTURE);
  g_signal_connect(keys, "key-pressed", G_CALLBACK(key_pressed), scene);
  g_signal_connect(keys, "key-released", G_CALLBACK(key_released), scene);
  gtk_widget_add_controller(GTK_WIDGET(scene), keys);
  GtkEventController *focus = gtk_event_controller_focus_new();
  g_signal_connect(focus, "leave", G_CALLBACK(focus_left), scene);
  gtk_widget_add_controller(GTK_WIDGET(scene), focus);
  return GTK_WIDGET(scene);
}
static void node_control(PaxNode *node, GtkWidget *control) {
  if (node->control)
    gtk_widget_unparent(node->control);
  node->control = control;
  gtk_widget_set_parent(control, GTK_WIDGET(node));
  if (node->input)
    gtk_widget_insert_after(node->input, GTK_WIDGET(node), control);
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
    g_signal_connect(
        gtk_scrolled_window_get_hadjustment(GTK_SCROLLED_WINDOW(node->control)),
        "value-changed", G_CALLBACK(scroll_changed), node);
    g_signal_connect(
        gtk_scrolled_window_get_vadjustment(GTK_SCROLLED_WINDOW(node->control)),
        "value-changed", G_CALLBACK(scroll_changed), node);
    node->clip = TRUE;
  } else if (strcmp(kind, "Textbox") == 0) {
    node->control = gtk_entry_new();
    g_signal_connect(node->control, "changed", G_CALLBACK(entry_changed), node);
    g_signal_connect(node->control, "activate", G_CALLBACK(entry_activate),
                     node);
  } else if (strcmp(kind, "Text") == 0) {
    node->control = gtk_label_new("");
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
static gboolean apply_style(PaxNode *node, JsonObject *patch, GError **error) {
  if (!has(patch, "style"))
    return TRUE;
  JsonObject *style = json_object_get_object_member(patch, "style");
  PangoAttrList *attributes = pango_attr_list_new();
  if (has(style, "font_size"))
    pango_attr_list_insert(attributes,
                           pango_attr_size_new_absolute(
                               number(style, "font_size", 14) * PANGO_SCALE));
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
    }
  }
  if (has(style, "font")) {
    JsonObject *font = json_object_get_object_member(style, "font");
    GList *keys = json_object_get_members(font);
    if (keys) {
      JsonObject *details = json_object_get_object_member(font, keys->data);
      const char *source = string(details, "path", string(details, "url", ""));
      if (*source && !pax_linux_font_load(node->scene->assets, source, error)) {
        g_list_free(keys);
        pango_attr_list_unref(attributes);
        return FALSE;
      }
      if (has(details, "family"))
        pango_attr_list_insert(
            attributes, pango_attr_family_new(
                            json_object_get_string_member(details, "family")));
    }
    g_list_free(keys);
  }
  if (boolean(style, "underline", FALSE))
    pango_attr_list_insert(attributes,
                           pango_attr_underline_new(PANGO_UNDERLINE_SINGLE));
  GtkWidget *label = node->control;
  if (GTK_IS_BUTTON(label))
    label = gtk_button_get_child(GTK_BUTTON(label));
  if (GTK_IS_LABEL(label))
    gtk_label_set_attributes(GTK_LABEL(label), attributes);
  else if (GTK_IS_ENTRY(label))
    gtk_entry_set_attributes(GTK_ENTRY(label), attributes);
  pango_attr_list_unref(attributes);
  return TRUE;
}
static gboolean update_node(PaxNode *node, JsonObject *patch, GError **error) {
  node->updating = TRUE;
  if (json_object_has_member(patch, "parent_frame"))
    node->parent_id = has(patch, "parent_frame")
                          ? json_object_get_int_member(patch, "parent_frame")
                          : -1;
  node->z = number(patch, "z_index", node->z);
  node->width = number(patch, "size_x", node->width);
  node->height = number(patch, "size_y", node->height);
  node->opacity = CLAMP(number(patch, "opacity", node->opacity), 0, 1);
  node->radius = MAX(0, number(patch, "corner_radius", node->radius));
  node->clip = boolean(patch, "clip_content", node->clip);
  if (has(patch, "transform")) {
    JsonArray *array = json_object_get_array_member(patch, "transform");
    if (json_array_get_length(array) != 6) {
      g_set_error(error, G_IO_ERROR, G_IO_ERROR_INVALID_DATA,
                  "native transform needs six values");
      node->updating = FALSE;
      return FALSE;
    }
    for (guint i = 0; i < 6; i++)
      node->matrix[i] = json_array_get_double_element(array, i);
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
    if (boolean(patch, "editable", FALSE) && !GTK_IS_TEXT_VIEW(node->control)) {
      GtkWidget *view = gtk_text_view_new();
      g_signal_connect(gtk_text_view_get_buffer(GTK_TEXT_VIEW(view)), "changed",
                       G_CALLBACK(text_changed), node);
      node_control(node, view);
    }
    if (GTK_IS_LABEL(node->control)) {
      if (has(patch, "content"))
        gtk_label_set_text(GTK_LABEL(node->control),
                           string(patch, "content", ""));
      if (has(patch, "selectable"))
        gtk_label_set_selectable(GTK_LABEL(node->control),
                                 boolean(patch, "selectable", FALSE));
      if (has(patch, "wrap"))
        gtk_label_set_wrap(GTK_LABEL(node->control),
                           boolean(patch, "wrap", FALSE));
    }
    node->clip = boolean(patch, "clip", node->clip);
  }
  if (strcmp(node->kind, "Textbox") == 0 &&
      boolean(patch, "is_text_area", FALSE) &&
      !GTK_IS_TEXT_VIEW(node->control)) {
    GtkWidget *view = gtk_text_view_new();
    gtk_text_view_set_wrap_mode(GTK_TEXT_VIEW(view), GTK_WRAP_WORD_CHAR);
    g_signal_connect(gtk_text_view_get_buffer(GTK_TEXT_VIEW(view)), "changed",
                     G_CALLBACK(text_changed), node);
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
  } else if (GTK_IS_BUTTON(node->control) && has(patch, "content"))
    gtk_button_set_label(GTK_BUTTON(node->control),
                         string(patch, "content", ""));
  else if (GTK_IS_CHECK_BUTTON(node->control) && has(patch, "checked"))
    gtk_check_button_set_active(GTK_CHECK_BUTTON(node->control),
                                boolean(patch, "checked", FALSE));
  if (GTK_IS_SCROLLED_WINDOW(node->control)) {
    node->inner_width =
        MAX(node->width, number(patch, "size_inner_pane_x", node->inner_width));
    node->inner_height = MAX(
        node->height, number(patch, "size_inner_pane_y", node->inner_height));
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
    if (has(patch, "scroll_x") || has(patch, "presentation_scroll_x"))
      gtk_adjustment_set_value(
          gtk_scrolled_window_get_hadjustment(scroll),
          number(patch, "presentation_scroll_x", number(patch, "scroll_x", 0)));
    if (has(patch, "scroll_y") || has(patch, "presentation_scroll_y"))
      gtk_adjustment_set_value(
          gtk_scrolled_window_get_vadjustment(scroll),
          number(patch, "presentation_scroll_y", number(patch, "scroll_y", 0)));
  }
  if (!apply_style(node, patch, error)) {
    node->updating = FALSE;
    return FALSE;
  }
  if (node->width < 0 || node->height < 0 || has(patch, "measure_generation")) {
    int min, natural_w, natural_h;
    gtk_widget_measure(node->control, GTK_ORIENTATION_HORIZONTAL, -1, &min,
                       &natural_w, NULL, NULL);
    gtk_widget_measure(node->control, GTK_ORIENTATION_VERTICAL,
                       node->width > 0 ? (int)node->width : natural_w, &min,
                       &natural_h, NULL, NULL);
    if (node->width < 0)
      node->width = natural_w;
    if (node->height < 0)
      node->height = natural_h;
    if (has(patch, "measure_generation")) {
      JsonObject *body = json_object_new();
      json_object_set_int_member(body, "id", node->id);
      json_object_set_int_member(
          body, "generation",
          json_object_get_int_member(patch, "measure_generation"));
      json_object_set_double_member(body, "width", natural_w);
      json_object_set_double_member(body, "height", natural_h);
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
  GPtrArray *all = g_ptr_array_new();
  GHashTableIter iter;
  gpointer value;
  g_hash_table_iter_init(&iter, scene->tiles);
  while (g_hash_table_iter_next(&iter, NULL, &value))
    g_ptr_array_add(all, value);
  g_hash_table_iter_init(&iter, scene->nodes);
  while (g_hash_table_iter_next(&iter, NULL, &value))
    g_ptr_array_add(all, value);
  g_ptr_array_sort(all, compare_nodes);
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
    if (parent && !node->tile_key) {
      graphene_matrix_t parent_world, inverse;
      node_matrix(parent, &parent_world);
      if (graphene_matrix_inverse(&parent_world, &inverse))
        graphene_matrix_multiply(&world, &inverse, &relative);
    }
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
  reconcile(scene);
  return TRUE;
}
guint pax_native_scene_layer_count(PaxNativeScene *scene) {
  return scene->layer_count;
}
void pax_native_scene_layer_owner(PaxNativeScene *scene, guint layer,
                                  gint64 owner) {
  g_hash_table_insert(scene->layer_owners, GUINT_TO_POINTER(layer),
                      GSIZE_TO_POINTER(owner + 1));
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
  reconcile(scene);
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
    }
  g_hash_table_unref(keep);
}
void pax_native_scene_stop(PaxNativeScene *scene) {
  scene->stopped = TRUE;
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
JsonObject *pax_native_scene_report(PaxNativeScene *scene) {
  JsonObject *report = json_object_new();
  json_object_set_int_member(report, "scenePointerPresses",
                             scene->scene_pointer_presses);
  JsonArray *nodes = json_array_new();
  GtkNative *native = gtk_widget_get_native(GTK_WIDGET(scene));
  GtkWidget *coordinates = native ? GTK_WIDGET(native) : GTK_WIDGET(scene);
  double sx = 0, sy = 0;
  if (native)
    gtk_native_get_surface_transform(native, &sx, &sy);
  GHashTableIter iter;
  gpointer value;
  g_hash_table_iter_init(&iter, scene->nodes);
  while (g_hash_table_iter_next(&iter, NULL, &value)) {
    PaxNode *node = value;
    JsonObject *item = json_object_new();
    json_object_set_int_member(item, "id", node->id);
    json_object_set_string_member(item, "kind", node->kind);
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
      PangoLayoutIter *iterator =
          pango_layout_get_iter(gtk_label_get_layout(GTK_LABEL(node->control)));
      PangoLayoutRun *run = pango_layout_iter_get_run_readonly(iterator);
      if (run && run->item->analysis.font) {
        PangoFontDescription *description =
            pango_font_describe(run->item->analysis.font);
        json_object_set_string_member(
            item, "resolvedFont",
            pango_font_description_get_family(description));
        pango_font_description_free(description);
      }
      pango_layout_iter_free(iterator);
    } else if (GTK_IS_BUTTON(node->control))
      json_object_set_string_member(
          item, "text", gtk_button_get_label(GTK_BUTTON(node->control)));
    else if (GTK_IS_CHECK_BUTTON(node->control))
      json_object_set_boolean_member(
          item, "checked",
          gtk_check_button_get_active(GTK_CHECK_BUTTON(node->control)));
    else if (GTK_IS_SCROLLED_WINDOW(node->control)) {
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
