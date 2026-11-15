/* Authored-offset lifecycle regression. From the repository root on Linux:
 *   cc -std=c11 -O1 -g -Wall -Wextra -Werror -Ipax-host/native/linux \
 *     tests/src/native-platform-probes/linux/native-scroller-offsets.c \
 *     pax-host/native/linux/native-scene.c pax-host/native/linux/assets.c \
 *     $(pkg-config --cflags --libs gtk4 json-glib-1.0 fontconfig pangoft2) -lm \
 *     -o /tmp/pax-native-scroller-offsets
 *   timeout 30s xvfb-run -a env GSK_RENDERER=cairo /tmp/pax-native-scroller-offsets
 * Real GTK main-loop allocation is required; no manual allocation or input.
 */
#include "native-scene.h"
#include <math.h>
#include <string.h>

typedef struct { guint id; double x, y; } Position;
typedef struct {
  PaxNativeScene *scene;
  GArray *positions;
  guint checks;
  gboolean failed;
} Test;
static void check(Test *t, gboolean value, const char *name) {
  t->checks++;
  t->failed |= !value;
  g_print("%s %s\n", value ? "PASS" : "FAIL", name);
}
static void error(gpointer data, const char *message) {
  ((Test *)data)->failed = TRUE;
  g_printerr("scene error: %s\n", message);
}
static gboolean interrupt(gpointer data, const char *text, gsize length) {
  Test *t = data;
  JsonParser *parser = json_parser_new();
  GError *failure = NULL;
  if (!json_parser_load_from_data(parser, text, (gssize)length, &failure)) {
    error(t, failure->message);
    g_clear_error(&failure);
    g_object_unref(parser);
    return FALSE;
  }
  JsonObject *message = json_node_get_object(json_parser_get_root(parser));
  if (json_object_has_member(message, "ScrollerPosition")) {
    JsonObject *body = json_object_get_object_member(message, "ScrollerPosition");
    Position position = {
      (guint)json_object_get_int_member(body, "id"),
      json_object_get_double_member(body, "scroll_x"),
      json_object_get_double_member(body, "scroll_y")
    };
    if (!isfinite(position.x) || !isfinite(position.y) ||
        position.x != json_object_get_double_member(body, "presentation_scroll_x") ||
        position.y != json_object_get_double_member(body, "presentation_scroll_y"))
      error(t, "ScrollerPosition must contain finite, coherent native/presentation offsets");
    g_array_append_val(t->positions, position);
    g_print("position id=%u x=%.3f y=%.3f\n", position.id, position.x, position.y);
  }
  g_object_unref(parser);
  return TRUE;
}
static void apply(Test *t, const char *text) {
  JsonParser *parser = json_parser_new();
  GError *failure = NULL;
  gboolean ok = json_parser_load_from_data(parser, text, -1, &failure);
  if (ok)
    ok = pax_native_scene_apply(t->scene, json_node_get_array(json_parser_get_root(parser)), &failure);
  if (!ok) {
    error(t, failure ? failure->message : "patch rejected");
    g_clear_error(&failure);
  }
  g_object_unref(parser);
}
static void pump(guint milliseconds) {
  gint64 end = g_get_monotonic_time() + milliseconds * 1000;
  while (g_get_monotonic_time() < end) {
    for (guint i = 0; i < 32 && g_main_context_pending(NULL); i++)
      g_main_context_iteration(NULL, FALSE);
    g_usleep(1000);
  }
}
static GtkWidget *find(GtkWidget *root) {
  if (GTK_IS_SCROLLED_WINDOW(root)) return root;
  for (GtkWidget *child = gtk_widget_get_first_child(root); child;
       child = gtk_widget_get_next_sibling(child)) {
    GtkWidget *found = find(child);
    if (found) return found;
  }
  return NULL;
}
static GtkWidget *find_text(GtkWidget *root) {
  if (GTK_IS_LABEL(root) && !strcmp(gtk_label_get_text(GTK_LABEL(root)), "Local")) return root;
  for (GtkWidget *child=gtk_widget_get_first_child(root); child; child=gtk_widget_get_next_sibling(child)) {
    GtkWidget *found=find_text(child); if (found) return found;
  }
  return NULL;
}
static gboolean close_to(double actual, double expected) {
  return fabs(actual - expected) < .1;
}
static gboolean offsets(GtkAdjustment *x, GtkAdjustment *y, double expected_x, double expected_y) {
  g_print("offsets actual=(%.3f,%.3f) expected=(%.3f,%.3f) pages=(%.3f,%.3f) uppers=(%.3f,%.3f)\n",
      gtk_adjustment_get_value(x), gtk_adjustment_get_value(y), expected_x, expected_y,
      gtk_adjustment_get_page_size(x), gtk_adjustment_get_page_size(y),
      gtk_adjustment_get_upper(x), gtk_adjustment_get_upper(y));
  return close_to(gtk_adjustment_get_value(x), expected_x) &&
         close_to(gtk_adjustment_get_value(y), expected_y);
}
static gboolean acknowledged(Test *t, guint since, double x, double y) {
  if (t->positions->len <= since) return FALSE;
  Position last = g_array_index(t->positions, Position, t->positions->len - 1);
  return last.id == 17 && close_to(last.x, x) && close_to(last.y, y);
}
static double maximum(GtkAdjustment *value) {
  return MAX(gtk_adjustment_get_lower(value),
             gtk_adjustment_get_upper(value) - gtk_adjustment_get_page_size(value));
}
int main(void) {
  gtk_init();
  Test t = {.positions = g_array_new(FALSE, FALSE, sizeof(Position))};
  GtkWidget *window = gtk_window_new();
  g_object_ref_sink(window);
  GtkWidget *scene = pax_native_scene_new(interrupt, error, &t);
  t.scene = (PaxNativeScene *)scene;
  gtk_window_set_default_size(GTK_WINDOW(window), 600, 500);
  gtk_window_set_child(GTK_WINDOW(window), scene);
  apply(&t, "[{\"ScrollerCreate\":{\"id\":17,\"render_layer_id\":0}},{\"ScrollerUpdate\":{\"id\":17,\"size_x\":300,\"size_y\":200,\"size_inner_pane_x\":900,\"size_inner_pane_y\":800,\"transform\":[1,0,0,1,20,20],\"scroll_x\":173,\"scroll_y\":127}}]");
  check(&t, t.positions->len == 0, "initial authored offsets are not acknowledged against unallocated zero extents");
  gtk_window_present(GTK_WINDOW(window));
  pump(500);
  GtkWidget *control = find(scene);
  check(&t, control != NULL, "authored offsets use a native GtkScrolledWindow");
  if (!control) goto done;
  GtkAdjustment *x = gtk_scrolled_window_get_hadjustment(GTK_SCROLLED_WINDOW(control));
  GtkAdjustment *y = gtk_scrolled_window_get_vadjustment(GTK_SCROLLED_WINDOW(control));
  check(&t, gtk_adjustment_get_page_size(x) > 0 && gtk_adjustment_get_page_size(y) > 0,
        "GTK establishes actual page sizes through its main loop");
  check(&t, offsets(x, y, 173, 127), "first allocated frame preserves both authored nonzero offsets");
  check(&t, acknowledged(&t, 0, 173, 127), "first allocation acknowledges the actual authored result");

  apply(&t, "[{\"ScrollerUpdate\":{\"id\":17,\"size_x\":350,\"size_y\":240}}]");
  pump(300);
  check(&t, offsets(x, y, 173, 127), "viewport resize retains offsets that remain in range");
  guint before = t.positions->len;
  apply(&t, "[{\"ScrollerUpdate\":{\"id\":17,\"size_inner_pane_x\":1400,\"size_inner_pane_y\":1200,\"scroll_x\":820,\"scroll_y\":730}}]");
  pump(300);
  check(&t, offsets(x, y, 820, 730), "extent growth and authored movement use the new allocation together");
  check(&t, acknowledged(&t, before, 820, 730), "growth request acknowledges its unclamped new-extent result");
  before = t.positions->len;
  apply(&t, "[{\"ScrollerUpdate\":{\"id\":17,\"scroll_x\":820,\"scroll_y\":730}}]");
  pump(200);
  check(&t, offsets(x, y, 820, 730) && t.positions->len == before,
        "exact current-position echoes do not emit redundant interrupts");

  before = t.positions->len;
  apply(&t, "[{\"ScrollerUpdate\":{\"id\":17,\"scroll_x\":0,\"scroll_y\":0}}]");
  pump(200);
  check(&t, offsets(x, y, 0, 0) && acknowledged(&t, before, 0, 0),
        "explicit programmatic zero applies and is acknowledged");
  before = t.positions->len;
  apply(&t, "[{\"ScrollerUpdate\":{\"id\":17,\"scroll_x\":5000,\"scroll_y\":-100}}]");
  pump(200);
  check(&t, offsets(x, y, maximum(x), 0), "out-of-range authored offsets clamp to native legal bounds");
  check(&t, acknowledged(&t, before, maximum(x), 0), "clamped authored request acknowledges its actual result");
  before = t.positions->len;
  apply(&t, "[{\"ScrollerUpdate\":{\"id\":17,\"size_inner_pane_x\":450,\"size_inner_pane_y\":320}}]");
  pump(300);
  check(&t, offsets(x, y, maximum(x), 0) && acknowledged(&t, before, maximum(x), 0),
        "content shrink naturally clamps and publishes the native offset");
  double retained_x = gtk_adjustment_get_value(x);
  apply(&t, "[{\"ScrollerUpdate\":{\"id\":17,\"size_inner_pane_x\":1400,\"size_inner_pane_y\":1200}}]");
  pump(300);
  check(&t, offsets(x, y, retained_x, 0), "later growth does not resurrect a previously clamped request");
  check(&t, find(scene) == control, "resize and programmatic movement preserve native control identity");

  apply(&t, "[{\"ScrollerUpdate\":{\"id\":17,\"scroll_x\":0,\"scroll_y\":0}},{\"FrameCreate\":{\"id\":20,\"parent_frame\":17,\"render_layer_id\":0}},{\"FrameUpdate\":{\"id\":20,\"size_x\":100,\"size_y\":80,\"transform_scroller\":17,\"transform\":[1,0,0,1,12,14]}},{\"TextCreate\":{\"id\":21,\"parent_frame\":20,\"render_layer_id\":0}},{\"TextUpdate\":{\"id\":21,\"size_x\":60,\"size_y\":20,\"content\":\"Local\",\"selectable\":false,\"transform_scroller\":17,\"transform\":[1,0,0,1,17,21]}}]");
  pump(200);
  GtkWidget *label=find_text(scene);
  check(&t, label!=NULL, "retained text is mounted inside the Frame");
  if (label) {
    graphene_point_t zero={0,0}, before_point, after_point;
    GtkWidget *placement=gtk_widget_get_parent(label);
    gboolean measured=gtk_widget_compute_point(placement, control, &zero, &before_point);
    apply(&t, "[{\"ScrollerUpdate\":{\"id\":17,\"transform\":[0,1,-1,0,280,30]}}]");
    pump(200);
    measured &= gtk_widget_compute_point(placement, control, &zero, &after_point);
    check(&t, measured && close_to(before_point.x,17) && close_to(before_point.y,21) &&
          close_to(after_point.x,17) && close_to(after_point.y,21),
          "host-only rotation preserves Scroller-local Text through a Frame");
    apply(&t, "[{\"TextUpdate\":{\"id\":21,\"parent_frame\":null,\"transform_scroller\":null,\"transform\":[1,0,0,1,50,55]}}]");
    pump(200);
    measured=gtk_widget_compute_point(placement, scene, &zero, &after_point);
    check(&t, measured && close_to(after_point.x,50) && close_to(after_point.y,55),
          "explicit null coordinate owner restores world placement");
  }
  apply(&t, "[{\"TextDelete\":21},{\"FrameDelete\":20}]");

  g_object_ref(x);
  g_object_ref(y);
  apply(&t, "[{\"ScrollerDelete\":17}]");
  before = t.positions->len;
  gtk_adjustment_set_value(x, 40);
  gtk_adjustment_set_value(y, 50);
  pump(250);
  check(&t, find(scene) == NULL && t.positions->len == before,
        "deleting the control disconnects retained adjustment callbacks");
  g_object_unref(x);
  g_object_unref(y);

  before = t.positions->len;
  apply(&t, "[{\"ScrollerCreate\":{\"id\":17,\"render_layer_id\":0}},{\"ScrollerUpdate\":{\"id\":17,\"size_x\":300,\"size_y\":200,\"size_inner_pane_x\":900,\"size_inner_pane_y\":800,\"transform\":[1,0,0,1,20,20],\"scroll_x\":80,\"scroll_y\":90}},{\"ScrollerUpdate\":{\"id\":17,\"scroll_x\":0}}]");
  pump(300);
  control = find(scene);
  check(&t, control != NULL, "a new scroller can reuse the retired identity");
  if (control) {
    x = gtk_scrolled_window_get_hadjustment(GTK_SCROLLED_WINDOW(control));
    y = gtk_scrolled_window_get_vadjustment(GTK_SCROLLED_WINDOW(control));
    check(&t, offsets(x, y, 0, 90) && acknowledged(&t, before, 0, 90),
          "latest pending per-axis request wins, including explicit zero");
  }
  apply(&t, "[{\"ScrollerDelete\":17},{\"ScrollerCreate\":{\"id\":19,\"render_layer_id\":0}},{\"ScrollerUpdate\":{\"id\":19,\"size_x\":300,\"size_y\":200,\"size_inner_pane_x\":900,\"size_inner_pane_y\":800,\"scroll_x\":100,\"scroll_y\":200}},{\"ScrollerDelete\":19}]");
  before = t.positions->len;
  pump(300);
  check(&t, find(scene) == NULL && t.positions->len == before,
        "deletion before allocation cancels pending authored movement");
done:
  pax_native_scene_stop(t.scene);
  gtk_window_destroy(GTK_WINDOW(window));
  g_object_unref(window);
  g_array_unref(t.positions);
  g_print("%s %u authored scroller offset checks\n", t.failed ? "FAIL" : "PASS", t.checks);
  return t.failed ? 1 : 0;
}
