/*
 * Native GTK styling regression. Build like text-constraints.c, replacing
 * its filename with this one; run under a real GTK display with timeout 30s.
 * Direct GTK value changes exercise native signals, not physical pointer input.
 */
#include "native-scene.h"
#include <math.h>
#include <string.h>

typedef struct {
  PaxNativeScene *scene;
  guint checks, slider_events, dropdown_events, radio_events, text_events;
  double slider_value;
  guint dropdown_value, radio_value;
  gboolean failed;
} Test;

static void check(Test *test, gboolean passed, const char *name) {
  test->checks++;
  test->failed |= !passed;
  g_print("%s %s\n", passed ? "PASS" : "FAIL", name);
}

static void error(gpointer data, const char *message) {
  Test *test = data;
  test->failed = TRUE;
  g_printerr("scene error: %s\n", message);
}

static gboolean interrupt(gpointer data, const char *text, gsize length) {
  Test *test = data;
  JsonParser *parser = json_parser_new();
  GError *failure = NULL;
  if (!json_parser_load_from_data(parser, text, (gssize)length, &failure)) {
    error(test, failure->message);
    g_clear_error(&failure);
    g_object_unref(parser);
    return FALSE;
  }
  JsonObject *root = json_node_get_object(json_parser_get_root(parser));
  const char *names[] = {"FormSliderChange", "FormDropdownChange", "FormRadioListChange"};
  for (guint i = 0; i < G_N_ELEMENTS(names); i++) {
    if (!json_object_has_member(root, names[i]))
      continue;
    JsonObject *event = json_object_get_object_member(root, names[i]);
    check(test, json_object_get_int_member(event, "id") == 11 + i,
          "native form event preserves node identity");
    if (i == 0) {
      test->slider_events++;
      test->slider_value = json_object_get_double_member(event, "value");
    } else if (i == 1) {
      test->dropdown_events++;
      test->dropdown_value = json_object_get_int_member(event, "selected_id");
    } else {
      test->radio_events++;
      test->radio_value = json_object_get_int_member(event, "selected_id");
    }
    g_print("event %.*s\n", (int)length, text);
  }
  if (json_object_has_member(root, "FormTextboxInput"))
    test->text_events++;
  g_object_unref(parser);
  return !test->failed;
}

static gboolean apply(Test *test, const char *json) {
  JsonParser *parser = json_parser_new();
  GError *failure = NULL;
  gboolean ok = json_parser_load_from_data(parser, json, -1, &failure);
  if (ok)
    ok = pax_native_scene_apply(test->scene,
                               json_node_get_array(json_parser_get_root(parser)), &failure);
  if (!ok) {
    error(test, failure ? failure->message : "patch rejected");
    g_clear_error(&failure);
  }
  g_object_unref(parser);
  return ok;
}

static void pump(void) {
  gint64 deadline = g_get_monotonic_time() + 100000;
  while (g_get_monotonic_time() < deadline) {
    for (guint i = 0; i < 32 && g_main_context_pending(NULL); i++)
      g_main_context_iteration(NULL, FALSE);
    g_usleep(2000);
  }
}

static GtkWidget *find(GtkWidget *widget, GType type, const char *label) {
  if (g_type_is_a(G_OBJECT_TYPE(widget), type) &&
      (!label || (GTK_IS_CHECK_BUTTON(widget) &&
                  g_strcmp0(gtk_check_button_get_label(GTK_CHECK_BUTTON(widget)), label) == 0)))
    return widget;
  for (GtkWidget *child = gtk_widget_get_first_child(widget); child;
       child = gtk_widget_get_next_sibling(child)) {
    GtkWidget *found = find(child, type, label);
    if (found)
      return found;
  }
  return NULL;
}


static GtkWidget *text_label(GtkWidget *widget, const char *text) {
  if (GTK_IS_LABEL(widget) &&
      strcmp(gtk_label_get_text(GTK_LABEL(widget)), text) == 0)
    return widget;
  for (GtkWidget *child = gtk_widget_get_first_child(widget); child;
       child = gtk_widget_get_next_sibling(child)) {
    GtkWidget *found = text_label(child, text);
    if (found)
      return found;
  }
  return NULL;
}

static gboolean pixel(GtkWidget *widget, int x, int y, int r, int g, int b) {
  GtkSnapshot *snapshot = gtk_snapshot_new();
  GdkPaintable *paintable = gtk_widget_paintable_new(widget);
  gdk_paintable_snapshot(paintable, GDK_SNAPSHOT(snapshot),
                          gtk_widget_get_width(widget), gtk_widget_get_height(widget));
  g_object_unref(paintable);
  GskRenderNode *node = gtk_snapshot_free_to_node(snapshot);
  if (!node) {
    g_print("empty snapshot %s mapped=%d width=%d height=%d\n",
              G_OBJECT_TYPE_NAME(widget), gtk_widget_get_mapped(widget),
              gtk_widget_get_width(widget), gtk_widget_get_height(widget));
    return FALSE;
  }
  GskRenderer *renderer = gtk_native_get_renderer(gtk_widget_get_native(widget));
  graphene_rect_t rect = GRAPHENE_RECT_INIT(0, 0, gtk_widget_get_width(widget),
                                            gtk_widget_get_height(widget));
  GdkTexture *texture = gsk_renderer_render_texture(renderer, node, &rect);
  gsk_render_node_unref(node);
  gsize stride = (gsize)gdk_texture_get_width(texture) * 4;
  guchar *pixels = g_malloc(stride * gdk_texture_get_height(texture));
  gdk_texture_download(texture, pixels, stride);
  guchar *p = pixels + y * stride + x * 4;
#if G_BYTE_ORDER == G_LITTLE_ENDIAN
  int actual_r = p[2], actual_g = p[1], actual_b = p[0], actual_a = p[3];
#else
  int actual_r = p[1], actual_g = p[2], actual_b = p[3], actual_a = p[0];
#endif
  g_print("pixel (%d,%d): %d %d %d %d expected %d %d %d\n", x, y,
            actual_r, actual_g, actual_b, actual_a, r, g, b);
  gboolean ok = abs(actual_r-r) <= 3 && abs(actual_g-g) <= 3 &&
                 abs(actual_b-b) <= 3 && actual_a == 255;
  g_free(pixels);
  g_object_unref(texture);
  return ok;
}
static GtkWidget *css_widget(GtkWidget *widget, const char *name) {
  if (!strcmp(gtk_widget_get_css_name(widget), name)) return widget;
  for (GtkWidget *child = gtk_widget_get_first_child(widget); child;
       child = gtk_widget_get_next_sibling(child)) {
    GtkWidget *found = css_widget(child, name);
    if (found) return found;
  }
  return NULL;
}
static gboolean symbol_pixel(GtkWidget *widget, double red, double green,
                              double blue, double alpha) {
  GtkSnapshot *snapshot = gtk_snapshot_new();
  GdkPaintable *paintable = gtk_widget_paintable_new(widget);
  gdk_paintable_snapshot(paintable, GDK_SNAPSHOT(snapshot),
                          gtk_widget_get_width(widget), gtk_widget_get_height(widget));
  g_object_unref(paintable);
  GskRenderNode *node = gtk_snapshot_free_to_node(snapshot);
  if (!node) return alpha == 0;
  graphene_rect_t rect = GRAPHENE_RECT_INIT(0, 0, gtk_widget_get_width(widget), gtk_widget_get_height(widget));
  GdkTexture *texture = gsk_renderer_render_texture(
      gtk_native_get_renderer(gtk_widget_get_native(widget)), node, &rect);
  gsk_render_node_unref(node);
  gsize length = (gsize)gdk_texture_get_width(texture) * gdk_texture_get_height(texture);
  guchar *pixels = g_malloc(length * 4);
  gdk_texture_download(texture, pixels, gdk_texture_get_width(texture) * 4);
  guchar *best = pixels;
  for (gsize i=0;i<length;i++) if (pixels[i*4+3] > best[3]) best=pixels+i*4;
  gboolean ok = fabs(best[2] - red * alpha * 255) <= 4 &&
                fabs(best[1] - green * alpha * 255) <= 4 &&
                fabs(best[0] - blue * alpha * 255) <= 4 &&
                fabs(best[3] - alpha * 255) <= 4;
  g_print("symbol pixel BGRA %u %u %u %u\n",best[0],best[1],best[2],best[3]);
  g_free(pixels); g_object_unref(texture); return ok;
}
static void tag_check(Test *test, GtkTextView *view, gboolean reset) {
  GtkTextBuffer *buffer = gtk_text_view_get_buffer(view);
  GtkTextTag *tag = gtk_text_tag_table_lookup(gtk_text_buffer_get_tag_table(buffer), "pax-style");
  check(test, tag != NULL, "multiline text uses a native GtkTextTag");
  if (!tag) return;
  int weight, style, underline, justify;
  double points;
  GdkRGBA *color = NULL;
  g_object_get(tag, "weight", &weight, "style", &style, "underline", &underline,
                "size-points", &points, "foreground-rgba", &color,
                "justification", &justify, NULL);
  check(test, weight == (reset ? PANGO_WEIGHT_NORMAL : PANGO_WEIGHT_BOLD) &&
              style == (reset ? PANGO_STYLE_NORMAL : PANGO_STYLE_ITALIC),
              reset ? "multiline weight/slant reset" : "multiline weight/slant applied");
  check(test, fabs(points - 13.5) < .01, "18px multiline text remains 13.5pt, with no duplicate scale");
  check(test, underline == (reset ? PANGO_UNDERLINE_NONE : PANGO_UNDERLINE_SINGLE), "multiline underline state reflects latest patch");
  check(test, color && fabs(color->alpha - (reset ? 1 : .5)) < .001, "multiline foreground retains authored alpha");
  check(test, justify == GTK_JUSTIFY_RIGHT, "multiline native paragraph justification applied");
  g_clear_pointer(&color, gdk_rgba_free);
  GtkTextIter a, b;
  gtk_text_buffer_get_bounds(buffer, &a, &b);
  check(test, gtk_text_iter_has_tag(&a, tag), "first glyph carries native style tag");
  gtk_text_iter_backward_char(&b);
  check(test, gtk_text_iter_has_tag(&b, tag), "last glyph carries native style tag");
}
int main(void) {
  gtk_init();
  Test test = {0};
  GtkWidget *window = gtk_window_new();
  g_object_ref_sink(window);
  GtkWidget *scene = pax_native_scene_new(interrupt, error, &test);
  test.scene = (PaxNativeScene *)scene;
  gtk_window_set_default_size(GTK_WINDOW(window), 700, 600);
  gtk_window_set_child(GTK_WINDOW(window), scene);
  gtk_window_present(GTK_WINDOW(window));
  apply(&test, "[{\"ButtonCreate\":{\"id\":21,\"render_layer_id\":0}},"
               "{\"TextboxCreate\":{\"id\":22,\"render_layer_id\":0}},"
               "{\"TextCreate\":{\"id\":23,\"render_layer_id\":0}},"
               "{\"TextboxCreate\":{\"id\":24,\"render_layer_id\":0}},"
               "{\"ButtonUpdate\":{\"id\":21,\"content\":\"Native\",\"size_x\":240,\"size_y\":60,\"transform\":[1,0,0,1,20,20],\"color\":{\"Rgb\":[0.1,0.2,0.3]},\"hover_color\":{\"Rgb\":[0.4,0.5,0.6]},\"outline_stroke_color\":{\"Rgb\":[1,0,0]},\"outline_stroke_width\":3,\"corner_radius\":12}},"
               "{\"TextboxUpdate\":{\"id\":22,\"text\":\"Entry\",\"size_x\":240,\"size_y\":60,\"transform\":[1,0,0,1,20,100],\"background\":{\"Rgb\":[0.1,0.2,0.3]},\"stroke_color\":{\"Rgb\":[1,0,0]},\"stroke_width\":3,\"corner_radius\":12,\"outline_color\":{\"Rgb\":[0,1,0]},\"outline_width\":2}},"
               "{\"TextUpdate\":{\"id\":23,\"content\":\"Label\\nlines\",\"size_x\":240,\"size_y\":100,\"transform\":[1,0,0,1,20,190]}},"
               "{\"TextboxUpdate\":{\"id\":24,\"text\":\"Multiline\\nGlyphs\",\"is_text_area\":true,\"size_x\":240,\"size_y\":100,\"transform\":[1,0,0,1,20,320],\"background\":{\"Rgb\":[0.1,0.2,0.3]},\"stroke_width\":0}}]");
  pump();
  GtkWidget *button = find(scene, GTK_TYPE_BUTTON, NULL);
  GtkWidget *entry = find(scene, GTK_TYPE_ENTRY, NULL);
  GtkWidget *view = find(scene, GTK_TYPE_TEXT_VIEW, NULL);
  check(&test, button && entry && view, "surface styling preserves genuine GTK controls");
  if (!button || !entry || !view) goto done;
  check(&test, pixel(button, 20, 30, 26, 51, 77), "button normal surface matches authored color");
  check(&test, pixel(button, 1, 30, 255, 0, 0), "button authored native border rendered");
  gtk_widget_set_state_flags(button, GTK_STATE_FLAG_PRELIGHT, FALSE); pump();
  check(&test, pixel(button, 20, 30, 102, 128, 153), "native hover state matches authored hover color");
  gtk_widget_unset_state_flags(button, GTK_STATE_FLAG_PRELIGHT);
  gtk_widget_set_state_flags(button, GTK_STATE_FLAG_ACTIVE, FALSE); pump();
  check(&test, pixel(button, 20, 30, 102, 128, 153), "native active state matches authored hover color");
  gtk_widget_unset_state_flags(button, GTK_STATE_FLAG_ACTIVE); pump();
  check(&test, pixel(entry, 210, 30, 26, 51, 77), "entry surface matches authored background");
  check(&test, pixel(entry, 1, 30, 255, 0, 0), "entry authored native border rendered");
  apply(&test, "[{\"ButtonUpdate\":{\"id\":21,\"color\":{\"Rgb\":[0.2,0.4,0.6]}}},{\"TextboxUpdate\":{\"id\":22,\"background\":{\"Rgb\":[0.2,0.4,0.6]}}}]"); pump();
  check(&test, pixel(button, 20, 30, 51, 102, 153), "reactive button color updates without recreating control");
  check(&test, pixel(entry, 210, 30, 51, 102, 153), "reactive entry background updates without recreating control");
  check(&test, find(scene, GTK_TYPE_BUTTON, NULL) == button && find(scene, GTK_TYPE_ENTRY, NULL) == entry,
        "reactive surface patches preserve control identity");
  const char *kinds[] = {"Button", "Textbox", "Text", "Textbox"};
  for (guint reset = 0; reset < 2; reset++) {
    for (guint i = 0; i < 4; i++) {
      char *patch = g_strdup_printf("[{\"%sUpdate\":{\"id\":%u,\"style\":{\"font\":{\"System\":{\"family\":\"Sans\",\"weight\":\"%s\",\"style\":\"%s\"}},\"font_size\":18,\"fill\":{\"Rgba\":[0.2,0.3,0.4,%s]},\"underline\":%s,\"align_horizontal\":\"Right\",\"align_vertical\":\"Bottom\",\"align_multiline\":\"Right\"}}}]",
          kinds[i], 21+i, reset ? "Normal" : "Bold", reset ? "Normal" : "Italic", reset ? "1" : "0.5", reset ? "false" : "true");
      apply(&test, patch); g_free(patch);
    }
    pump();
    GtkWidget *label = gtk_button_get_child(GTK_BUTTON(button));
    check(&test, GTK_IS_LABEL(label) && gtk_label_get_xalign(GTK_LABEL(label)) == 1 && gtk_label_get_yalign(GTK_LABEL(label)) == 1,
          "button native label applies horizontal and vertical alignment");
    check(&test, gtk_entry_get_alignment(GTK_ENTRY(entry)) == 1, "entry applies native horizontal alignment");
    GtkWidget *ordinary = text_label(scene, "Label\nlines");
    check(&test, ordinary && gtk_label_get_xalign(GTK_LABEL(ordinary)) == 1 &&
                 gtk_label_get_yalign(GTK_LABEL(ordinary)) == 1 &&
                 gtk_label_get_justify(GTK_LABEL(ordinary)) == GTK_JUSTIFY_RIGHT,
          "ordinary Text applies horizontal, vertical, and multiline alignment");
    tag_check(&test, GTK_TEXT_VIEW(view), reset);
  }
  apply(&test, "[{\"DropdownCreate\":{\"id\":35,\"render_layer_id\":0}},{\"DropdownUpdate\":{\"id\":35,\"size_x\":280,\"size_y\":50,\"transform\":[1,0,0,1,300,20],\"options\":[\"First\",\"Second\",\"Third\"],\"selected_id\":0,\"background\":{\"Rgb\":[0.1,0.2,0.3]},\"style\":{\"fill\":{\"Rgba\":[0.8,0.9,1,1]}}}}]"); pump();
  GtkWidget *dropdown = find(scene, GTK_TYPE_DROP_DOWN, NULL);
  GtkWidget *arrow = dropdown ? css_widget(dropdown, "arrow") : NULL;
  check(&test, arrow != NULL, "dropdown retains GTK's native arrow");
  if (arrow) {
    check(&test, symbol_pixel(arrow,.8,.9,1,1), "native arrow pixels use authored text foreground");
    apply(&test,"[{\"DropdownUpdate\":{\"id\":35,\"style\":{\"fill\":{\"Rgba\":[1,0.5,0.25,0.5]}}}}]"); pump();
    check(&test,symbol_pixel(arrow,1,.5,.25,.5),"reactive arrow fill applies alpha exactly once");
    GtkWidget *option = text_label(dropdown,"First");
    PangoAttrList *attrs = option ? gtk_label_get_attributes(GTK_LABEL(option)) : NULL;
    PangoAttrIterator *it = attrs ? pango_attr_list_get_iterator(attrs) : NULL;
    PangoAttrInt *foreground_alpha = it ? (PangoAttrInt *)pango_attr_iterator_get(it,PANGO_ATTR_FOREGROUND_ALPHA) : NULL;
    check(&test,foreground_alpha && abs(foreground_alpha->value-32767)<=1,"label retains independent authored Pango alpha");
    if(it) pango_attr_iterator_destroy(it);
    apply(&test,"[{\"DropdownUpdate\":{\"id\":35,\"style\":{\"fill\":{\"Rgba\":[0.8,0.9,1,1]}}}}]"); pump();
    GtkWidget *toggle = find(dropdown, GTK_TYPE_TOGGLE_BUTTON, NULL);
    if (toggle) gtk_toggle_button_set_active(GTK_TOGGLE_BUTTON(toggle), TRUE);
    GtkWidget *popover = find(dropdown, GTK_TYPE_POPOVER, NULL);
    for (guint attempt=0; popover && !gtk_widget_get_mapped(popover) && attempt<20; attempt++) pump();
    check(&test,popover && gtk_widget_get_mapped(popover),"dropdown opens a genuine native popover");
    if(popover && gtk_widget_get_mapped(popover)) {
      // GtkPopover is a separate native surface; inspect its real content
      // widget once allocated rather than snapshotting the surface owner.
      GtkWidget *contents = css_widget(popover, "contents");
      for (guint attempt=0; contents && gtk_widget_get_width(contents)<=0 && attempt<20; attempt++) pump();
      pump();
      check(&test,contents && pixel(contents,gtk_widget_get_width(contents)-20,gtk_widget_get_height(contents)-20,26,51,77),"popup uses authored surface behind styled options");
      gtk_popover_popdown(GTK_POPOVER(popover)); pump();
    }
    check(&test,find(scene,GTK_TYPE_DROP_DOWN,NULL)==dropdown,"reactive contrast updates preserve native control identity");
  }
  GtkWidget *label = find(scene, GTK_TYPE_LABEL, NULL);
  check(&test, label != NULL, "native label remains present after style updates");
  GtkSettings *settings = gtk_widget_get_settings(view);
  int original_dpi;
  g_object_get(settings, "gtk-xft-dpi", &original_dpi, NULL);
  GtkTextBuffer *buffer = gtk_text_view_get_buffer(GTK_TEXT_VIEW(view));
  GtkTextIter start;
  GdkRectangle normal, large, restored;
  gtk_text_buffer_get_start_iter(buffer, &start);
  g_object_set(settings, "gtk-xft-dpi", 96 * 1024, NULL); pump();
  gtk_text_view_get_iter_location(GTK_TEXT_VIEW(view), &start, &normal);
  g_object_set(settings, "gtk-xft-dpi", 144 * 1024, NULL); pump();
  gtk_text_view_get_iter_location(GTK_TEXT_VIEW(view), &start, &large);
  g_object_set(settings, "gtk-xft-dpi", 96 * 1024, NULL); pump();
  gtk_text_view_get_iter_location(GTK_TEXT_VIEW(view), &start, &restored);
  g_print("multiline glyph heights96/144/96: %d %d %d, widgetScale=%d\n",
            normal.height, large.height, restored.height, gtk_widget_get_scale_factor(view));
  check(&test, normal.height > 0 && fabs((double)large.height / normal.height - 1.5) < .15,
        "native multiline glyph height follows effective font DPI once");
  check(&test, normal.height == restored.height, "restoring font DPI restores multiline glyph height");
  g_object_set(settings, "gtk-xft-dpi", original_dpi, NULL); pump();
  gtk_text_buffer_set_text(buffer, "", -1);
  gtk_text_buffer_insert_at_cursor(buffer, "Inserted after empty", -1);
  pump(); tag_check(&test, GTK_TEXT_VIEW(view), TRUE);
  apply(&test, "[{\"TextboxUpdate\":{\"id\":24,\"text\":\"Replaced without style patch\"}}]");
  pump(); tag_check(&test, GTK_TEXT_VIEW(view), TRUE);
  apply(&test, "[{\"ButtonUpdate\":{\"id\":21,\"content\":\"Changed label\"}}]");
  check(&test, gtk_label_get_xalign(GTK_LABEL(gtk_button_get_child(GTK_BUTTON(button)))) == 1,
        "button content-only patch retains native label alignment");
  g_object_ref(buffer);
  guint before_delete_events = test.text_events;
  apply(&test, "[{\"TextboxDelete\":24}]");
  gtk_text_buffer_set_text(buffer, "Retained buffer after native node deletion", -1);
  check(&test, test.text_events == before_delete_events,
        "retained native text buffer disconnects callbacks when its node is deleted");
  g_object_unref(buffer);
  apply(&test, "[{\"TextCreate\":{\"id\":25,\"render_layer_id\":0}},{\"TextUpdate\":{\"id\":25,\"editable\":true,\"content\":\"Editable plain text\",\"size_x\":240,\"size_y\":80,\"wrap\":true}}]");
  GtkWidget *editable_text = find(scene, GTK_TYPE_TEXT_VIEW, NULL);
  check(&test, editable_text && gtk_text_view_get_editable(GTK_TEXT_VIEW(editable_text)) &&
               gtk_text_view_get_wrap_mode(GTK_TEXT_VIEW(editable_text)) == GTK_WRAP_WORD_CHAR,
        "editable Text uses native editing and authored wrapping");
  apply(&test, "[{\"TextUpdate\":{\"id\":25,\"editable\":false,\"wrap\":false}}]");
  check(&test, !gtk_text_view_get_editable(GTK_TEXT_VIEW(editable_text)) &&
               !gtk_text_view_get_cursor_visible(GTK_TEXT_VIEW(editable_text)) &&
               gtk_text_view_get_wrap_mode(GTK_TEXT_VIEW(editable_text)) == GTK_WRAP_NONE,
        "editable=false disables native editing and updates wrapping without replacing styled text");
  apply(&test, "[{\"TextCreate\":{\"id\":26,\"render_layer_id\":0}},{\"TextUpdate\":{\"id\":26,\"content\":\"Transparent selectable ink\",\"size_x\":400,\"size_y\":70,\"transform\":[1,0,0,1,280,450],\"selectable\":true,\"style\":{\"font_size\":28,\"fill\":{\"Rgba\":[1,0,0,0]}}}}]");
  pump();
  GtkWidget *transparent = text_label(scene, "Transparent selectable ink");
  check(&test, transparent && gtk_label_get_selectable(GTK_LABEL(transparent)),
        "transparent text keeps its genuine selectable GtkLabel");
  if (transparent) {
    int width_before, height_before;
    pango_layout_get_pixel_size(gtk_label_get_layout(GTK_LABEL(transparent)), &width_before, &height_before);
    check(&test, width_before > 0 && height_before > 0 && symbol_pixel(transparent, 1, 0, 0, 0),
          "zero fill alpha hides glyph pixels without removing text layout");
    gtk_label_select_region(GTK_LABEL(transparent), 0, -1); pump();
    int selection_start, selection_end;
    check(&test, gtk_label_get_selection_bounds(GTK_LABEL(transparent), &selection_start, &selection_end) &&
                 selection_end - selection_start == (int)strlen("Transparent selectable ink") &&
                 !symbol_pixel(transparent, 1, 0, 0, 0),
          "transparent glyphs retain native text selection and visible highlighting");
    gtk_label_select_region(GTK_LABEL(transparent), 0, 0);
    apply(&test, "[{\"TextUpdate\":{\"id\":26,\"style\":{\"font_size\":28,\"fill\":{\"Rgba\":[1,0,0,0.5]}}}}]"); pump();
    check(&test, symbol_pixel(transparent, 1, 0, 0, .5), "zero to translucent fill applies alpha once");
    apply(&test, "[{\"TextUpdate\":{\"id\":26,\"style\":{\"font_size\":28,\"fill\":{\"Rgb\":[1,0,0]}}}}]"); pump();
    check(&test, symbol_pixel(transparent, 1, 0, 0, 1), "translucent to opaque fill restores glyphs");
    int width_after, height_after;
    pango_layout_get_pixel_size(gtk_label_get_layout(GTK_LABEL(transparent)), &width_after, &height_after);
    check(&test, width_after == width_before && height_after == height_before,
          "fill alpha changes preserve measured glyph geometry and native identity");
    apply(&test, "[{\"TextUpdate\":{\"id\":26,\"style\":{\"font_size\":28,\"fill\":{\"Rgba\":[1,0,0,0]}}}}]"); pump();
    check(&test, symbol_pixel(transparent, 1, 0, 0, 0), "opaque to zero fill hides glyphs again");
    apply(&test, "[{\"TextUpdate\":{\"id\":26,\"style\":{\"font_size\":28}}}]"); pump();
    check(&test, !symbol_pixel(transparent, 1, 0, 0, 0), "removing authored fill restores theme foreground");
    apply(&test, "[{\"TextUpdate\":{\"id\":26,\"style\":{\"font_size\":28,\"fill\":{\"Rgba\":[1,0,0,0]}}}},{\"TextDelete\":26},{\"TextCreate\":{\"id\":26,\"render_layer_id\":0}},{\"TextUpdate\":{\"id\":26,\"content\":\"Reused native text identity\",\"size_x\":400,\"size_y\":70,\"transform\":[1,0,0,1,280,450],\"style\":{\"font_size\":28,\"fill\":{\"Rgb\":[0,0,1]}}}}]"); pump();
    GtkWidget *replacement = text_label(scene, "Reused native text identity");
    check(&test, replacement && symbol_pixel(replacement, 0, 0, 1, 1),
          "deleting transparent text removes its provider before reusing the native ID");
  }
  JsonParser *rejection = json_parser_new();
  json_parser_load_from_data(rejection, "[{\"TextUpdate\":{\"id\":25,\"markdown\":true}}]", -1, NULL);
  GError *expected = NULL;
  gboolean accepted = pax_native_scene_apply(test.scene,
      json_node_get_array(json_parser_get_root(rejection)), &expected);
  check(&test, !accepted && g_error_matches(expected, G_IO_ERROR, G_IO_ERROR_NOT_SUPPORTED),
        "unsupported Markdown fails explicitly instead of displaying literal markup");
  g_clear_error(&expected); g_object_unref(rejection);
 done:
  pax_native_scene_stop(test.scene);
  gtk_window_destroy(GTK_WINDOW(window)); g_object_unref(window);
  g_print("native styling: %u checks, %s\n", test.checks, test.failed ? "FAILED" : "PASSED");
  return test.failed ? 1 : 0;
}
