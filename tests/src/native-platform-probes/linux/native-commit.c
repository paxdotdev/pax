/*
 * Native placement batching regression; no Pax/GPU presentation required.
 * Build like text-constraints.c, substituting native-commit.c, then run under
 * a real GTK display (or xvfb-run) with timeout 30s.
 */
#include "native-scene.h"
#include <math.h>
#include <string.h>

typedef struct { PaxNativeScene *scene; guint checks; gboolean failed; } Test;
typedef struct { gboolean selected; int start, end; } Selection;
static void check(Test *test, gboolean ok, const char *name) {
  test->checks++;
  test->failed |= !ok;
  g_print("%s %s\n", ok ? "PASS" : "FAIL", name);
}
static void error(gpointer data, const char *message) {
  Test *test = data;
  test->failed = TRUE;
  g_printerr("scene error: %s\n", message);
}
static JsonParser *parse(const char *text) {
  JsonParser *parser = json_parser_new();
  GError *error = NULL;
  if (!json_parser_load_from_data(parser, text, -1, &error))
    g_error("invalid fixture: %s", error->message);
  return parser;
}
static gboolean apply(Test *test, const char *text, gboolean expected) {
  JsonParser *parser = parse(text);
  GError *error = NULL;
  gboolean ok = pax_native_scene_apply(test->scene,
      json_node_get_array(json_parser_get_root(parser)), &error);
  check(test, ok == expected && (ok || error != NULL),
        expected ? "native patch accepted" : "invalid native patch reports an error");
  g_clear_error(&error);
  g_object_unref(parser);
  return ok;
}
static guint64 reconciliations(Test *test) {
  JsonObject *report = pax_native_scene_report(test->scene);
  guint64 count = json_object_get_int_member(report, "sceneReconciliations");
  json_object_unref(report);
  return count;
}
static GtkWidget *label(GtkWidget *widget, const char *text) {
  if (GTK_IS_LABEL(widget) && !strcmp(gtk_label_get_text(GTK_LABEL(widget)), text))
    return widget;
  for (GtkWidget *child = gtk_widget_get_first_child(widget); child;
       child = gtk_widget_get_next_sibling(child)) {
    GtkWidget *found = label(child, text);
    if (found) return found;
  }
  return NULL;
}
static Selection selection(GtkWidget *widget, const char *phase) {
  Selection result = {0};
  result.selected = gtk_label_get_selection_bounds(GTK_LABEL(widget),
                                                   &result.start, &result.end);
  g_print("selection phase=%s selected=%d start=%d end=%d\n", phase,
          result.selected, result.start, result.end);
  return result;
}
static gboolean selection_equal(Selection left, Selection right) {
  return left.selected == right.selected && left.start == right.start &&
         left.end == right.end;
}
static void texture(Test *test, const char *key, double x, GdkTexture *texture) {
  JsonObject *surface = json_object_new();
  json_object_set_int_member(surface, "layer", 1);
  json_object_set_string_member(surface, "key", key);
  json_object_set_double_member(surface, "logicalWidth", 20);
  json_object_set_double_member(surface, "logicalHeight", 20);
  json_object_set_double_member(surface, "originX", x);
  json_object_set_double_member(surface, "originY", 5);
  pax_native_scene_texture(test->scene, surface, texture);
  json_object_unref(surface);
}
static void prune(Test *test, const char *surfaces) {
  JsonParser *parser = parse(surfaces);
  pax_native_scene_prune_tiles(test->scene,
      json_node_get_array(json_parser_get_root(parser)));
  g_object_unref(parser);
}
static guint children(GtkWidget *parent) {
  guint count = 0;
  for (GtkWidget *child = gtk_widget_get_first_child(parent); child;
       child = gtk_widget_get_next_sibling(child)) count++;
  return count;
}
static void pump(void) {
  gint64 end = g_get_monotonic_time() + 100000;
  while (g_get_monotonic_time() < end) {
    for (guint i = 0; i < 32 && g_main_context_pending(NULL); i++)
      g_main_context_iteration(NULL, FALSE);
    g_usleep(2000);
  }
}
int main(void) {
  gtk_init();
  Test test = {0};
  GtkWidget *window = gtk_window_new();
  g_object_ref_sink(window);
  GtkWidget *scene = pax_native_scene_new(NULL, error, &test);
  test.scene = (PaxNativeScene *)scene;
  gtk_window_set_default_size(GTK_WINDOW(window), 400, 200);
  gtk_window_set_child(GTK_WINDOW(window), scene);
  gtk_window_present(GTK_WINDOW(window));
  apply(&test,
    "[{\"FrameCreate\":{\"id\":1}},{\"FrameCreate\":{\"id\":2}},"
    "{\"FrameUpdate\":{\"id\":1,\"size_x\":150,\"size_y\":150,\"transform\":[1,0,0,1,20,20]}},"
    "{\"FrameUpdate\":{\"id\":2,\"size_x\":150,\"size_y\":150,\"transform\":[1,0,0,1,200,20]}},"
    "{\"TextCreate\":{\"id\":3,\"parent_frame\":1}},"
    "{\"TextCreate\":{\"id\":4,\"parent_frame\":1}},"
    "{\"TextCreate\":{\"id\":5,\"parent_frame\":2}},"
    "{\"TextUpdate\":{\"id\":3,\"content\":\"Three\",\"selectable\":true,\"size_x\":60,\"size_y\":24,\"z_index\":3,\"transform\":[1,0,0,1,30,35]}},"
    "{\"TextUpdate\":{\"id\":4,\"content\":\"Four\",\"size_x\":60,\"size_y\":24,\"z_index\":4,\"transform\":[1,0,0,1,30,65]}},"
    "{\"TextUpdate\":{\"id\":5,\"content\":\"Five\",\"size_x\":60,\"size_y\":24,\"transform\":[1,0,0,1,210,65]}}]", TRUE);
  pump();
  GtkWidget *three = label(scene, "Three"), *four = label(scene, "Four");
  GtkWidget *five = label(scene, "Five");
  check(&test, three && four && five, "fixture creates genuine native labels");
  if (!three || !four || !five) goto done;
  GtkWidget *node3 = gtk_widget_get_parent(three);
  GtkWidget *node4 = gtk_widget_get_parent(four);
  GtkWidget *node5 = gtk_widget_get_parent(five);
  GtkWidget *a = gtk_widget_get_parent(node3), *b = gtk_widget_get_parent(node5);
  gtk_label_select_region(GTK_LABEL(three), 1, 3);
  Selection selected = selection(three, "initial");
  check(&test, selected.selected && selected.start == 1 && selected.end == 3,
        "selection precondition is established before placement changes");
  check(&test, reconciliations(&test) == 1,
        "standalone apply places immediately exactly once");
  const guint8 pixels[] = {255, 0, 0, 255};
  GBytes *bytes = g_bytes_new_static(pixels, sizeof pixels);
  GdkTexture *image = gdk_memory_texture_new(1, 1, GDK_MEMORY_R8G8B8A8, bytes, 4);
  g_bytes_unref(bytes);
  pax_native_scene_layer_owner(test.scene, 1, 1);
  guint64 before = reconciliations(&test);
  pax_native_scene_begin_commit(test.scene);
  apply(&test, "[{\"TextUpdate\":{\"id\":3,\"z_index\":6}}]", TRUE);
  texture(&test, "left", 5, image);
  texture(&test, "right", 35, image);
  texture(&test, "left", 8, image);
  check(&test, reconciliations(&test) == before && children(a) == 2 &&
        gtk_widget_get_first_child(a) == node3,
        "batch defers tile attachment and sibling placement until commit");
  pax_native_scene_end_commit(test.scene, TRUE);
  check(&test, reconciliations(&test) == before + 1,
        "one patch and three texture updates reconcile exactly once");
  GtkWidget *tile1 = gtk_widget_get_first_child(a);
  GtkWidget *tile2 = gtk_widget_get_next_sibling(tile1);
  check(&test, children(a) == 4 &&
        !gtk_widget_get_first_child(tile1) && !gtk_widget_get_first_child(tile2) &&
        gtk_widget_get_next_sibling(tile2) == node4 &&
        gtk_widget_get_next_sibling(node4) == node3,
        "tiles stay behind native peers and latest native z-order wins");
  pump();
  check(&test, label(scene, "Three") == three &&
        selection_equal(selection(three, "same-parent-batch"), selected),
        "same-parent batched placement preserves label identity and selection");

  before = reconciliations(&test);
  pax_native_scene_begin_commit(test.scene);
  pax_native_scene_layer_owner(test.scene, 1, 1);
  apply(&test, "[]", TRUE);
  prune(&test, "[{\"layer\":1,\"key\":\"left\"},{\"layer\":1,\"key\":\"right\"}]");
  pax_native_scene_end_commit(test.scene, TRUE);
  check(&test, reconciliations(&test) == before && children(a) == 4 &&
        gtk_widget_get_parent(tile1) == a && gtk_widget_get_parent(tile2) == a &&
        selection_equal(selection(three, "unchanged-commit"), selected),
        "unchanged owner, empty native patch and unchanged tiles skip placement");

  before = reconciliations(&test);
  pax_native_scene_begin_commit(test.scene);
  pax_native_scene_layer_owner(test.scene, 1, 2);
  apply(&test, "[]", TRUE);
  check(&test, reconciliations(&test) == before && gtk_widget_get_parent(tile1) == a,
        "owner-only commit defers placement until publication");
  pax_native_scene_end_commit(test.scene, TRUE);
  check(&test, reconciliations(&test) == before + 1 && children(a) == 2 &&
        children(b) == 3 && gtk_widget_get_parent(tile1) == b &&
        gtk_widget_get_parent(tile2) == b && gtk_widget_get_parent(node3) == a,
        "changed owner with an empty native patch moves retained tiles exactly once");
  before = reconciliations(&test);
  pax_native_scene_begin_commit(test.scene);
  pax_native_scene_layer_owner(test.scene, 1, 1);
  apply(&test, "[]", TRUE);
  pax_native_scene_end_commit(test.scene, TRUE);
  check(&test, reconciliations(&test) == before + 1 && children(a) == 4 &&
        children(b) == 1 && gtk_widget_get_parent(tile1) == a &&
        gtk_widget_get_parent(tile2) == a,
        "owner-only restoration publishes without texture replacement");

  // Exercise the unchanged immediate path first. A real parent change unroots
  // GTK widgets, unlike a sibling reorder, so compare its native selection
  // behavior directly rather than assuming batching promises preservation.
  apply(&test, "[{\"TextUpdate\":{\"id\":3,\"parent_frame\":2,\"transform\":[1,0,0,1,210,35]}}]", TRUE);
  pump();
  Selection immediate_reparent = selection(three, "immediate-reparent");
  check(&test, label(scene, "Three") == three && gtk_widget_get_parent(node3) == b,
        "immediate control reparent preserves native label identity");
  apply(&test, "[{\"TextUpdate\":{\"id\":3,\"parent_frame\":1,\"transform\":[1,0,0,1,30,35]}}]", TRUE);
  pump();
  gtk_label_select_region(GTK_LABEL(three), 1, 3);
  check(&test, gtk_widget_get_parent(node3) == a &&
        selection_equal(selection(three, "batch-reparent-precondition"), selected),
        "batched reparent starts from the same parent and selection as immediate control");
  before = reconciliations(&test);
  pax_native_scene_begin_commit(test.scene);
  pax_native_scene_layer_owner(test.scene, 1, 2);
  apply(&test, "[{\"TextUpdate\":{\"id\":3,\"parent_frame\":2,\"transform\":[1,0,0,1,210,35]}}]", TRUE);
  texture(&test, "right", 40, image);
  prune(&test, "[{\"layer\":1,\"key\":\"right\"}]");
  check(&test, gtk_widget_get_parent(node3) == a && reconciliations(&test) == before,
        "owner and parent updates defer placement without an intermediate dispatch");
  pax_native_scene_end_commit(test.scene, TRUE);
  pump();
  check(&test, reconciliations(&test) == before + 1 && children(a) == 1 && children(b) == 3 &&
        gtk_widget_get_parent(node3) == b && gtk_widget_get_last_child(b) == node3 &&
        !gtk_widget_get_first_child(gtk_widget_get_first_child(b)),
        "final owner moves surviving tile and native parent while pruning obsolete tile");
  graphene_matrix_t matrix;
  gsk_transform_to_matrix(gtk_fixed_get_child_transform(GTK_FIXED(b), node3), &matrix);
  check(&test, fabs(graphene_matrix_get_x_translation(&matrix) - 10) < .01 &&
        fabs(graphene_matrix_get_y_translation(&matrix) - 15) < .01,
        "reparented native transform stays relative to its final parent");
  check(&test, label(scene, "Three") == three,
        "batched reparent preserves native label identity");
  check(&test, selection_equal(selection(three, "batched-reparent"), immediate_reparent),
        "batched reparent selection matches the immediate reparent control");
  before = reconciliations(&test);
  pax_native_scene_begin_commit(test.scene);
  pax_native_scene_end_commit(test.scene, TRUE);
  check(&test, reconciliations(&test) == before, "empty commit does no placement work");
  pax_native_scene_begin_commit(test.scene);
  apply(&test, "[{\"TextUpdate\":{\"id\":3,\"parent_frame\":1}}]", TRUE);
  apply(&test, "[{\"UnsupportedMessage\":{}}]", FALSE);
  pax_native_scene_end_commit(test.scene, FALSE);
  check(&test, reconciliations(&test) == before && gtk_widget_get_parent(node3) == b,
        "failed commit discards deferred placement instead of publishing partial ownership");
  apply(&test, "[]", TRUE);
  check(&test, reconciliations(&test) == before + 1 && gtk_widget_get_parent(node3) == a,
        "ending a failed batch restores standalone placement without claiming rollback");
  before = reconciliations(&test);
  texture(&test, "right", 45, image);
  check(&test, reconciliations(&test) == before + 1,
        "standalone texture update continues to place immediately");
  before = reconciliations(&test);
  pax_native_scene_begin_commit(test.scene);
  apply(&test, "[]", TRUE);
  prune(&test, "[]");
  pax_native_scene_end_commit(test.scene, TRUE);
  check(&test, reconciliations(&test) == before + 1 && children(b) == 1 &&
        gtk_widget_get_first_child(b) == node5,
        "tile removal with no native or texture updates requests one placement pass");
  g_object_unref(image);
done:
  pax_native_scene_stop(test.scene);
  gtk_window_destroy(GTK_WINDOW(window));
  g_object_unref(window);
  g_print("native commit checks=%u failed=%d\n", test.checks, test.failed);
  return test.failed ? 1 : 0;
}
