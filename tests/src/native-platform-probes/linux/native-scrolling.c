/* Native adjustment/snap contract. Build using text-constraints.c's command,
 * substituting this filename. Run with timeout45s in a real GTK session.
 * Synthetic controller signals below test lifecycle guards, not physical touch.
 */
#include "native-scene.h"
#include <math.h>
#include <string.h>

typedef struct { PaxNativeScene *scene; guint checks, events; gboolean failed; } Test;
static void check(Test *t, gboolean value, const char *name) {
  t->checks++; t->failed |= !value; g_print("%s %s\n", value ? "PASS" : "FAIL", name);
}
static void error(gpointer data, const char *message) {
  ((Test *)data)->failed = TRUE; g_printerr("scene error: %s\n", message);
}
static gboolean interrupt(gpointer data, const char *message, gsize length) {
  (void)length; Test *t = data;
  if (strstr(message, "ScrollerPosition")) t->events++;
  return TRUE;
}
static void apply(Test *t, const char *text) {
  JsonParser *parser = json_parser_new(); GError *failure = NULL;
  gboolean ok = json_parser_load_from_data(parser, text, -1, &failure);
  if (ok) ok = pax_native_scene_apply(t->scene, json_node_get_array(json_parser_get_root(parser)), &failure);
  if (!ok) { error(t, failure ? failure->message : "patch rejected"); g_clear_error(&failure); }
  g_object_unref(parser);
}
static void pump(guint milliseconds) {
  gint64 end = g_get_monotonic_time() + milliseconds * 1000;
  while (g_get_monotonic_time() < end) {
    for (guint i = 0; i < 32 && g_main_context_pending(NULL); i++) g_main_context_iteration(NULL, FALSE);
    g_usleep(1000);
  }
}
static GtkWidget *find(GtkWidget *root, GType type) {
  if (g_type_is_a(G_OBJECT_TYPE(root), type)) return root;
  for (GtkWidget *child = gtk_widget_get_first_child(root); child; child = gtk_widget_get_next_sibling(child)) {
    GtkWidget *found = find(child, type); if (found) return found;
  }
  return NULL;
}
static JsonObject *node_report(Test *t) {
  JsonObject *report = pax_native_scene_report(t->scene);
  JsonObject *node = json_object_ref(json_array_get_object_element(json_object_get_array_member(report, "nodes"), 0));
  json_object_unref(report); return node;
}
static void points(Test *t, const char *name, const double *values, guint count) {
  JsonObject *node = node_report(t); JsonArray *array = json_object_get_array_member(node, name);
  gboolean match = json_array_get_length(array) == count;
  for (guint i = 0; match && i < count; i++) match &= fabs(json_array_get_double_element(array, i) - values[i]) < .01;
  check(t, match, name); json_object_unref(node);
}
static void settle(Test *t) {
  gint64 end = g_get_monotonic_time() + 3000000;
  do {
    pump(50);
    JsonObject *node = node_report(t);
    gboolean pending = json_object_get_boolean_member(node,"snapPending") ||
                       json_object_get_boolean_member(node,"snapAnimating");
    json_object_unref(node);
    if (!pending) return;
  } while (g_get_monotonic_time() < end);
  check(t,FALSE,"native adjustment settles within bounded three seconds");
}
static gboolean near(GtkAdjustment *adjustment, double value) {
  g_print("actual %.3f expected %.3f\n", gtk_adjustment_get_value(adjustment), value);
  return fabs(gtk_adjustment_get_value(adjustment)-value) < .1;
}
static void animation_started(Test *t) {
  gint64 until = g_get_monotonic_time() + 1500000;
  while (g_get_monotonic_time() < until) {
    pump(5); JsonObject *node = node_report(t);
    gboolean active = json_object_get_boolean_member(node, "snapAnimating");
    json_object_unref(node);
    if (active) return;
  }
  check(t, FALSE, "snap enters animation before interruption probe");
}
int main(void) {
  gtk_init(); Test t = {0};
  GtkWidget *window = gtk_window_new(); g_object_ref_sink(window);
  GtkWidget *scene = pax_native_scene_new(interrupt, error, &t); t.scene = (PaxNativeScene *)scene;
  gtk_window_set_default_size(GTK_WINDOW(window), 600, 500);
  gtk_window_set_child(GTK_WINDOW(window), scene); gtk_window_present(GTK_WINDOW(window));
  apply(&t, "[{\"ScrollerCreate\":{\"id\":17,\"render_layer_id\":0}},{\"ScrollerUpdate\":{\"id\":17,\"size_x\":300,\"size_y\":200,\"size_inner_pane_x\":900,\"size_inner_pane_y\":800,\"transform\":[1,0,0,1,20,20],\"snap_points_x\":[600,0,300,300,-50,10000],\"snap_points_y\":[600,0,200,400]}}]");
  pump(500);
  GtkWidget *control = find(scene, GTK_TYPE_SCROLLED_WINDOW);
  check(&t, control != NULL, "snap container remains GtkScrolledWindow"); if (!control) goto done;
  GtkAdjustment *x = gtk_scrolled_window_get_hadjustment(GTK_SCROLLED_WINDOW(control));
  GtkAdjustment *y = gtk_scrolled_window_get_vadjustment(GTK_SCROLLED_WINDOW(control));
  double horizontal[] = {0,300,600}, vertical[] = {0,200,400,600};
  points(&t, "snapPointsX", horizontal, 3); points(&t, "snapPointsY", vertical, 4);
  gtk_adjustment_set_value(x, 173); gtk_adjustment_set_value(y, 173); settle(&t);
  check(&t, near(x,300) && near(y,200), "native adjustment changes settle to nearest anchors on both axes");
  guint events = t.events;
  apply(&t, "[{\"ScrollerUpdate\":{\"id\":17,\"snap_points_x\":[0,300,600,300,-50,10000],\"scroll_x\":300,\"scroll_y\":200}}]"); pump(500);
  check(&t, t.events == events, "identical/reordered effective policy and exact position echoes do not retarget");
  apply(&t, "[{\"ScrollerUpdate\":{\"id\":17,\"scroll_x\":173,\"scroll_y\":173}}]"); pump(500);
  check(&t, near(x,300) && near(y,200) && t.events > events, "authored off-anchor request acknowledges its snapped result");
  events = t.events;
  apply(&t, "[{\"ScrollerUpdate\":{\"id\":17,\"scroll_x\":173,\"scroll_y\":173}}]"); pump(500);
  check(&t, near(x,300) && near(y,200) && t.events > events, "repeated authored off-anchor request remains observable");
  apply(&t, "[{\"ScrollerUpdate\":{\"id\":17,\"snap_points_x\":[],\"snap_points_y\":[]}}]");
  gtk_adjustment_set_value(x,173); gtk_adjustment_set_value(y,173); pump(500);
  check(&t, near(x,173) && near(y,173), "clearing points preserves free native scrolling");
  points(&t, "snapPointsX", NULL, 0); points(&t, "snapPointsY", NULL, 0);
  apply(&t, "[{\"ScrollerUpdate\":{\"id\":17,\"snap_points_x\":[0,300,600],\"snap_points_y\":[0,200,400,600]}}]"); pump(500);
  check(&t, near(x,300) && near(y,200), "restored anchors resolve current native offsets");
  apply(&t, "[{\"ScrollerUpdate\":{\"id\":17,\"size_inner_pane_x\":450,\"size_inner_pane_y\":350}}]"); pump(500);
  double shrunk[] = {0,150}; points(&t,"snapPointsX",shrunk,2); points(&t,"snapPointsY",shrunk,2);
  apply(&t, "[{\"ScrollerUpdate\":{\"id\":17,\"size_inner_pane_x\":900,\"size_inner_pane_y\":800}}]"); pump(500);
  points(&t,"snapPointsX",horizontal,3); points(&t,"snapPointsY",vertical,4);
  // Controller inspection is test-only; production never depends on GTK's
  // private controllers or installs the bookkeeping model used by this probe.
  GListModel *controllers = gtk_widget_observe_controllers(control);
  GtkEventController *observer = NULL;
  for (guint i=0;i<g_list_model_get_n_items(controllers);i++) {
    GtkEventController *candidate = g_list_model_get_item(controllers,i);
    if (!g_strcmp0(gtk_event_controller_get_name(candidate),"pax-snap-scroll-observer")) observer = candidate;
    else g_object_unref(candidate);
  }
  g_object_unref(controllers);
  check(&t, observer != NULL,"passive native scrolling observer exists");
  if (observer) {
    g_signal_emit_by_name(observer,"scroll-begin");
    gtk_adjustment_set_value(y,260); pump(500);
    check(&t,near(y,260),"held continuous gesture blocks settling beyond quiescence interval");
    g_signal_emit_by_name(observer,"scroll-end"); pump(500);
    check(&t,near(y,200),"released continuous gesture settles after quiet movement");
    gtk_adjustment_set_value(y,360); pump(180);
    g_signal_emit_by_name(observer,"scroll-begin"); gtk_adjustment_set_value(y,270); pump(500);
    check(&t,near(y,270),"new input cancels active snap animation");
    g_signal_emit_by_name(observer,"scroll-end"); pump(500);
    check(&t,near(y,200),"cancelled animation does not restore stale endpoint");
    gtk_adjustment_set_value(y,360); animation_started(&t);
    g_signal_emit_by_name(observer,"scroll-begin");
    double interrupted = gtk_adjustment_get_value(y); pump(500);
    check(&t,near(y,interrupted),"held no-motion interruption keeps the current offset");
    g_signal_emit_by_name(observer,"scroll-end"); settle(&t);
    check(&t,near(y,400),"release after no-motion interruption resumes pending resolution");
    gtk_adjustment_set_value(y,360); animation_started(&t);
    gtk_widget_set_visible(control,FALSE);
    interrupted = gtk_adjustment_get_value(y); pump(500);
    check(&t,near(y,interrupted),"unmapped scroller pauses pending native snap");
    gtk_widget_set_visible(control,TRUE); settle(&t);
    check(&t,near(y,400),"remapping resumes pending snap from retained offset");
    g_object_unref(observer);
  }
  g_object_set(gtk_widget_get_settings(control),"gtk-enable-animations",FALSE,NULL);
  gtk_adjustment_set_value(y,350); pump(220);
  check(&t,near(y,400),"reduced-motion setting resolves endpoint without snap animation");
  GtkWidget *content = gtk_scrolled_window_get_child(GTK_SCROLLED_WINDOW(control));
  if (GTK_IS_VIEWPORT(content)) content = gtk_viewport_get_child(GTK_VIEWPORT(content));
  apply(&t, "[{\"ScrollerUpdate\":{\"id\":17,\"size_x\":-12,\"size_y\":0,\"size_inner_pane_x\":-28,\"size_inner_pane_y\":-8,\"snap_points_x\":[],\"snap_points_y\":[]}}]"); pump(100);
  int requested_width, requested_height;
  gtk_widget_get_size_request(content, &requested_width, &requested_height);
  check(&t, requested_width == 0 && requested_height == 0,
        "collapsed responsive content maps negative finite extents to an empty native box");
  apply(&t, "[{\"ScrollerUpdate\":{\"id\":17,\"size_x\":300,\"size_y\":200,\"size_inner_pane_x\":900,\"size_inner_pane_y\":800}}]"); pump(100);
  gtk_widget_get_size_request(content, &requested_width, &requested_height);
  check(&t, requested_width == 900 && requested_height == 800 &&
            find(scene, GTK_TYPE_SCROLLED_WINDOW) == control,
        "expanded content restores scroll extents without replacing the native control");
  g_object_ref(y); events=t.events;
  gtk_adjustment_set_value(y,330); events=t.events;
  pax_native_scene_stop(t.scene); pump(300); gtk_adjustment_set_value(y,100); pump(200);
  check(&t,t.events==events,"stop cancels pending snap and retained adjustment callbacks"); g_object_unref(y);
done:
  gtk_window_destroy(GTK_WINDOW(window)); g_object_unref(window);
  g_print("%s %u native scrolling checks\n",t.failed?"FAIL":"PASS",t.checks);
  return t.failed?1:0;
}
