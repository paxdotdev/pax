/* Raw Wheel/native-default contract. Build like native-scroller-offsets.c,
 * adding x11 to pkg-config. Run on a private Xvfb display with xdotool on PATH,
 * GDK_BACKEND=x11 GSK_RENDERER=cairo; repeat with GDK_SCALE=2.
 * Real X11 wheel events qualify mouse input, not a physical touchpad.
 */
#include "native-scene.h"
#include <gdk/x11/gdkx.h>
#include <math.h>
#include <string.h>

typedef struct {
  PaxNativeScene *scene;
  guint checks, failures, wheels, positions, legacy_scrolls;
  gboolean cancel;
  double x, y, dx, dy;
  gboolean shift, control;
} Test;
typedef struct { gboolean done; gint status; } Input;
static void check(Test *t, gboolean ok, const char *name) {
  t->checks++; t->failures += !ok;
  g_print("%s %s\n", ok ? "PASS" : "FAIL", name);
}
static void scene_error(gpointer data, const char *message) {
  ((Test *)data)->failures++;
  g_printerr("scene error: %s\n", message);
}
static gboolean interrupt(gpointer data, const char *text, gsize length) {
  Test *t = data; JsonParser *parser = json_parser_new(); GError *error = NULL;
  if (!json_parser_load_from_data(parser, text, (gssize)length, &error)) {
    scene_error(t, error->message); g_clear_error(&error); g_object_unref(parser); return FALSE;
  }
  JsonObject *root = json_node_get_object(json_parser_get_root(parser));
  gboolean cancel = FALSE;
  if (json_object_has_member(root, "Wheel")) {
    JsonObject *body = json_object_get_object_member(root, "Wheel");
    t->wheels++;
    t->x = json_object_get_double_member(body, "x");
    t->y = json_object_get_double_member(body, "y");
    t->dx = json_object_get_double_member(body, "delta_x");
    t->dy = json_object_get_double_member(body, "delta_y");
    t->shift = t->control = FALSE;
    JsonArray *modifiers = json_object_get_array_member(body, "modifiers");
    for (guint i = 0; i < json_array_get_length(modifiers); i++) {
      const char *name = json_array_get_string_element(modifiers, i);
      t->shift |= !strcmp(name, "Shift"); t->control |= !strcmp(name, "Control");
    }
    g_print("wheel #%u at %.3f,%.3f delta %.3f,%.3f shift=%d control=%d cancel=%d\n",
        t->wheels, t->x, t->y, t->dx, t->dy, t->shift, t->control, t->cancel);
    cancel = t->cancel;
  }
  t->positions += json_object_has_member(root, "ScrollerPosition");
  t->legacy_scrolls += json_object_has_member(root, "Scroll");
  g_object_unref(parser); return cancel;
}
static void pump(guint milliseconds) {
  gint64 end = g_get_monotonic_time() + milliseconds * 1000;
  do {
    for (guint i = 0; i < 32 && g_main_context_pending(NULL); i++)
      g_main_context_iteration(NULL, FALSE);
    g_usleep(1000);
  } while (g_get_monotonic_time() < end);
}
static gboolean apply(Test *t, const char *text) {
  JsonParser *parser = json_parser_new(); GError *error = NULL;
  gboolean ok = json_parser_load_from_data(parser, text, -1, &error);
  if (ok) ok = pax_native_scene_apply(t->scene,
      json_node_get_array(json_parser_get_root(parser)), &error);
  if (!ok) { scene_error(t, error ? error->message : "patch rejected"); g_clear_error(&error); }
  g_object_unref(parser); return ok;
}
static void find_scrollers(GtkWidget *widget, GPtrArray *result) {
  if (GTK_IS_SCROLLED_WINDOW(widget)) g_ptr_array_add(result, widget);
  for (GtkWidget *child = gtk_widget_get_first_child(widget); child;
       child = gtk_widget_get_next_sibling(child)) find_scrollers(child, result);
}
static void input_done(GPid pid, gint status, gpointer data) {
  Input *input = data; input->status = status; input->done = TRUE; g_spawn_close_pid(pid);
}
static void wheel(Test *t, GtkWidget *window, int x, int y, guint button,
                  gboolean modifiers, gboolean backlog) {
  graphene_point_t from = GRAPHENE_POINT_INIT(x, y), point;
  if (!gtk_widget_compute_point(GTK_WIDGET(t->scene), window, &from, &point)) {
    check(t, FALSE, "wheel point resolves into owned native window"); return;
  }
  Window xid = gdk_x11_surface_get_xid(gtk_native_get_surface(GTK_NATIVE(window)));
  int scale = gtk_widget_get_scale_factor(window);
  char *following_motion = backlog ? g_strdup_printf(" mousemove_relative -- %d 0", 25 * scale) : g_strdup("");
  char *command = g_strdup_printf(
      "timeout 5s xdotool windowfocus %lu mousemove --window %lu %d %d sleep 0.05 %sclick %u%s%s",
      (unsigned long)xid, (unsigned long)xid, (int)round(point.x*scale), (int)round(point.y*scale),
      modifiers ? "keydown Shift_L keydown Control_L " : "", button,
      modifiers ? " keyup Control_L keyup Shift_L" : "", following_motion);
  g_free(following_motion);
  gchar **argv = NULL; GError *error = NULL; GPid pid; Input result = {0};
  gboolean ok = g_shell_parse_argv(command, NULL, &argv, &error);
  if (ok) ok = g_spawn_async(NULL, argv, NULL, G_SPAWN_SEARCH_PATH | G_SPAWN_DO_NOT_REAP_CHILD,
      NULL, NULL, &pid, &error);
  if (ok) {
    g_child_watch_add(pid, input_done, &result);
    // Deliberately queue wheel then motion while this UI thread is busy. The
    // wheel must use the preceding pointer event, not the newer server position.
    if (backlog) g_usleep(350000);
    while (!result.done) pump(5);
    ok = g_spawn_check_wait_status(result.status, &error);
  }
  check(t, ok, "real wheel input completed on owned X11 window");
  if (error) { g_printerr("%s\n", error->message); g_error_free(error); }
  g_strfreev(argv); g_free(command); pump(250);
}
static gboolean close_to(double a, double b) { return fabs(a-b) < .1; }
int main(void) {
  gtk_init(); Test t = {0};
  g_print("GTK %u.%u.%u\n", gtk_get_major_version(), gtk_get_minor_version(), gtk_get_micro_version());
  GtkWidget *window = gtk_window_new(); g_object_ref_sink(window);
  GtkWidget *scene = pax_native_scene_new(interrupt, scene_error, &t); t.scene = (PaxNativeScene *)scene;
  // A nonzero embedding offset catches surface/widget coordinate confusion.
  gtk_widget_set_margin_start(scene, 17); gtk_widget_set_margin_top(scene, 23);
  gtk_window_set_default_size(GTK_WINDOW(window), 450, 350);
  gtk_window_set_child(GTK_WINDOW(window), scene);
  GPtrArray *scrollers = g_ptr_array_new();
  if (!apply(&t,
      "[{\"ScrollerCreate\":{\"id\":10}},"
      "{\"ScrollerUpdate\":{\"id\":10,\"size_x\":360,\"size_y\":250,\"size_inner_pane_x\":1000,\"size_inner_pane_y\":1600,\"transform\":[1,0,0,1,10,10]}},"
      "{\"ScrollerCreate\":{\"id\":20,\"parent_frame\":10}},"
      "{\"ScrollerUpdate\":{\"id\":20,\"size_x\":260,\"size_y\":160,\"size_inner_pane_x\":900,\"size_inner_pane_y\":1200,\"transform\":[1,0,0,1,30,30]}},"
      "{\"TextCreate\":{\"id\":30,\"parent_frame\":20}},"
      "{\"TextUpdate\":{\"id\":30,\"content\":\"Native wheel target\",\"selectable\":true,\"wrap\":false,\"size_x\":850,\"size_y\":1100,\"transform\":[1,0,0,1,40,40]}}]")) goto done;
  gtk_window_present(GTK_WINDOW(window)); pump(500);
  check(&t, GDK_IS_X11_DISPLAY(gtk_widget_get_display(window)), "probe uses X11 input backend");
  if (!GDK_IS_X11_DISPLAY(gtk_widget_get_display(window))) goto done;
  find_scrollers(scene, scrollers);
  check(&t, scrollers->len == 2, "real nested GtkScrolledWindows are mounted");
  if (scrollers->len != 2) goto done;
  GtkScrolledWindow *outer = g_ptr_array_index(scrollers, 0), *inner = g_ptr_array_index(scrollers, 1);
  GtkAdjustment *oy = gtk_scrolled_window_get_vadjustment(outer);
  GtkAdjustment *ix = gtk_scrolled_window_get_hadjustment(inner), *iy = gtk_scrolled_window_get_vadjustment(inner);
  t.cancel = TRUE;
  guint wheels = t.wheels, positions = t.positions;
  wheel(&t, window, 70, 70, 5, TRUE, FALSE);
  check(&t, t.wheels == wheels+1 && t.dx == 0 && t.dy > 0,
      "nested native target sends one down-positive Wheel before GTK default");
  check(&t, close_to(t.x, 70) && close_to(t.y, 70), "wheel position is scene logical coordinates at native widget scale");
  check(&t, t.shift && t.control, "wheel carries current native modifier state");
  check(&t, close_to(gtk_adjustment_get_value(oy), 0) && close_to(gtk_adjustment_get_value(iy), 0) &&
      t.positions == positions, "prevent_default suppresses inner and outer native scrolling and position messages");
  wheels = t.wheels;
  wheel(&t, window, 70, 70, 7, FALSE, FALSE);
  check(&t, t.wheels == wheels+1 && t.dx > 0 && t.dy == 0 && !t.shift && !t.control,
      "horizontal wheel preserves direction and clears released modifiers");
  check(&t, close_to(gtk_adjustment_get_value(ix), 0) && t.positions == positions,
      "horizontal prevent_default leaves native offset unchanged");
  t.cancel = FALSE; wheels = t.wheels;
  wheel(&t, window, 70, 70, 5, FALSE, FALSE);
  check(&t, t.wheels == wheels+1 && t.dy > 0, "uncancelled input still dispatches exactly once");
  check(&t, gtk_adjustment_get_value(iy) > 0 && close_to(gtk_adjustment_get_value(oy), 0) &&
      t.positions > positions, "uncancelled input uses native inner scrolling and actual position acknowledgement");
  double y = gtk_adjustment_get_value(iy); positions = t.positions; wheels = t.wheels;
  t.cancel = TRUE;
  wheel(&t, window, 70, 70, 4, FALSE, FALSE);
  check(&t, t.wheels == wheels+1 && t.dy < 0 && t.dx == 0 && close_to(gtk_adjustment_get_value(iy), y) &&
      t.positions == positions, "upward cancellation works after native movement without fabricated offsets");
  wheels = t.wheels;
  wheel(&t, window, 400, 290, 6, FALSE, FALSE);
  check(&t, t.wheels == wheels+1 && t.dx < 0 && t.dy == 0 && close_to(t.x, 400) && close_to(t.y, 290),
      "vector-only scene space receives leftward wheel with correct coordinates");
  check(&t, t.legacy_scrolls == 0, "raw Wheel never synthesizes legacy Scroll interrupts");
  wheels = t.wheels;
  wheel(&t, window, 70, 70, 5, FALSE, TRUE);
  check(&t, t.wheels == wheels+1 && close_to(t.x, 70) && close_to(t.y, 70),
      "queued wheel keeps the preceding pointer position despite later cursor movement");
  gtk_widget_set_visible(window, FALSE); pump(100);
  gtk_window_present(GTK_WINDOW(window)); pump(150); wheels = t.wheels;
  wheel(&t, window, 400, 290, 5, FALSE, FALSE);
  check(&t, t.wheels == wheels+1 && close_to(t.x, 400) && close_to(t.y, 290),
      "remapped scene reestablishes its pointer position without stale coordinates");
  pax_native_scene_stop(t.scene); wheels = t.wheels;
  wheel(&t, window, 400, 290, 5, FALSE, FALSE);
  check(&t, t.wheels == wheels, "stopped scene dispatches no wheel input");
done:
  pax_native_scene_stop(t.scene); gtk_window_destroy(GTK_WINDOW(window)); g_object_unref(window);
  g_ptr_array_unref(scrollers);
  g_print("%s %u raw wheel checks; failures=%u\n", t.failures ? "FAIL" : "PASS", t.checks, t.failures);
  return t.failures ? 1 : 0;
}
