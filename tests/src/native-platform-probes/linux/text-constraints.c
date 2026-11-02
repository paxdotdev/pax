/*
 * Standalone native Text geometry regression; no Pax/GPU presentation required.
 * From the repository root, on Linux with GTK >= 4.14:
 *   cc -std=c11 -O1 -g -Wall -Wextra -Werror \
 *     -Ipax-host/native/linux \
 *     tests/src/native-platform-probes/linux/text-constraints.c \
 *     pax-host/native/linux/native-scene.c pax-host/native/linux/assets.c \
 *     $(pkg-config --cflags --libs gtk4 json-glib-1.0 fontconfig pangoft2) -lm \
 *     -o /tmp/pax-text-constraints
 *   timeout 45s /tmp/pax-text-constraints
 *
 * Run in an existing graphical session (or under xvfb-run). The test observes
 * real main-loop allocations, not manually assigned widget allocations.
 * Text.clip defaults to false: zero/fixed boxes may have visible glyph overflow.
 * Reported geometry here is not evidence of GPU clipping or pixel invisibility.
 */
#include "native-scene.h"
#include <math.h>
#include <stdio.h>
#include <string.h>

#define NODE_ID 17
#define EPSILON 0.01

typedef struct {
  PaxNativeScene *scene;
  gboolean failed;
  guint responses, invalidations, checks;
  guint64 expected_generation, generation;
  guint64 expected_invalidation, invalidated_generation;
  double width, height;
  gint64 deadline;
} Test;

static gboolean check(Test *test, gboolean passed, const char *name) {
  test->checks++;
  g_print("%s %s\n", passed ? "PASS" : "FAIL", name);
  if (!passed)
    test->failed = TRUE;
  return passed;
}

static void scene_error(gpointer data, const char *message) {
  Test *test = data;
  test->failed = TRUE;
  g_printerr("native scene error: %s\n", message);
}

static gboolean interrupt(gpointer data, const char *text, gsize length) {
  Test *test = data;
  JsonParser *parser = json_parser_new();
  GError *error = NULL;
  if (!json_parser_load_from_data(parser, text, (gssize)length, &error)) {
    scene_error(test, error->message);
    g_clear_error(&error);
    g_object_unref(parser);
    return FALSE;
  }
  JsonNode *root = json_parser_get_root(parser);
  if (!JSON_NODE_HOLDS_OBJECT(root)) {
    scene_error(test, "interrupt is not a JSON object");
    g_object_unref(parser);
    return FALSE;
  }
  JsonObject *object = json_node_get_object(root);
  if (json_object_has_member(object, "TextMeasurementResponse")) {
    JsonObject *response =
        json_object_get_object_member(object, "TextMeasurementResponse");
    const char *required[] = {"id", "generation", "width", "height"};
    for (guint i = 0; i < G_N_ELEMENTS(required); i++) {
      if (!json_object_has_member(response, required[i])) {
        scene_error(test, "incomplete TextMeasurementResponse");
        g_object_unref(parser);
        return FALSE;
      }
    }
    test->responses++;
    test->generation = json_object_get_int_member(response, "generation");
    test->width = json_object_get_double_member(response, "width");
    test->height = json_object_get_double_member(response, "height");
    if (json_object_get_int_member(response, "id") != NODE_ID ||
        !test->expected_generation ||
        test->generation != test->expected_generation ||
        !isfinite(test->width) || !isfinite(test->height) ||
        test->width < 0 || test->height < 0)
      scene_error(test, "unexpected ID/generation or invalid measurement bounds");
    g_print("response %.*s\n", (int)length, text);
  }
  if (json_object_has_member(object, "TextMeasurementInvalidated")) {
    JsonObject *event =
        json_object_get_object_member(object, "TextMeasurementInvalidated");
    test->invalidations++;
    if (!json_object_has_member(event, "id") ||
        !json_object_has_member(event, "generation")) {
      scene_error(test, "incomplete TextMeasurementInvalidated");
    } else {
      test->invalidated_generation = json_object_get_int_member(event, "generation");
      if (json_object_get_int_member(event, "id") != NODE_ID ||
          !test->expected_invalidation ||
          test->invalidated_generation != test->expected_invalidation)
        scene_error(test, "unexpected invalidation ID or generation");
    }
    g_print("invalidation %.*s\n", (int)length, text);
  }
  g_object_unref(parser);
  return !test->failed;
}

static gboolean pump(Test *test, gint64 until) {
  while (g_get_monotonic_time() < until) {
    if (g_get_monotonic_time() >= test->deadline) {
      scene_error(test, "30-second test deadline exceeded");
      return FALSE;
    }
    // Keep the loop bounded even if GTK continuously queues frame work.
    for (guint i = 0; i < 32 && g_main_context_pending(NULL); i++)
      g_main_context_iteration(NULL, FALSE);
    g_usleep(2000);
  }
  return !test->failed;
}

static gboolean apply(Test *test, const char *messages) {
  JsonParser *parser = json_parser_new();
  GError *error = NULL;
  gboolean ok = json_parser_load_from_data(parser, messages, -1, &error);
  if (ok) {
    JsonNode *root = json_parser_get_root(parser);
    if (!JSON_NODE_HOLDS_ARRAY(root)) {
      scene_error(test, "test patch is not an array");
      ok = FALSE;
    } else
      ok = pax_native_scene_apply(test->scene, json_node_get_array(root),
                                  &error);
  }
  if (!ok) {
    scene_error(test, error ? error->message : "native patch application failed");
    g_clear_error(&error);
  }
  g_object_unref(parser);
  return ok && !test->failed;
}

static JsonObject *find_text(JsonObject *report) {
  JsonArray *nodes = json_object_get_array_member(report, "nodes");
  for (guint i = 0; i < json_array_get_length(nodes); i++) {
    JsonObject *node = json_array_get_object_element(nodes, i);
    if (json_object_get_int_member(node, "id") == NODE_ID)
      return node;
  }
  return NULL;
}

static gboolean allocated(Test *test, double width, double height) {
  JsonObject *report = pax_native_scene_report(test->scene);
  JsonObject *node = find_text(report);
  gboolean ok = node && json_object_has_member(node, "bounds");
  if (ok) {
    JsonArray *bounds = json_object_get_array_member(node, "bounds");
    ok = json_array_get_length(bounds) == 4 &&
         fabs(json_array_get_double_element(bounds, 2) - ceil(width)) < EPSILON &&
         fabs(json_array_get_double_element(bounds, 3) - ceil(height)) < EPSILON;
  }
  json_object_unref(report);
  return ok;
}

static void print_report(Test *test, const char *stage) {
  JsonObject *report = pax_native_scene_report(test->scene);
  json_object_set_string_member(report, "stage", stage);
  JsonNode *root = json_node_new(JSON_NODE_OBJECT);
  json_node_take_object(root, report);
  char *serialized = json_to_string(root, FALSE);
  g_print("report %s\n", serialized);
  g_free(serialized);
  json_node_free(root);
}

static gboolean stage(Test *test, const char *name, const char *fields,
                      guint64 generation, double expected_width,
                      double expected_height) {
  guint before = test->responses;
  test->expected_generation = generation;
  char *messages = g_strdup_printf("[{\"TextUpdate\":{\"id\":%u,%s}}]",
                                    NODE_ID, fields);
  gboolean ok = apply(test, messages);
  g_free(messages);
  if (!ok)
    return FALSE;
  // A full frame is allowed before checking; report polling also waits for
  // zero-sized widgets, which cannot be detected using "positive allocation".
  if (!pump(test, g_get_monotonic_time() + 100000))
    return FALSE;
  double width = generation ? test->width : expected_width;
  double height = generation ? test->height : expected_height;
  gint64 end = MIN(test->deadline, g_get_monotonic_time() + 2000000);
  while (!allocated(test, width, height) && g_get_monotonic_time() < end)
    if (!pump(test, g_get_monotonic_time() + 10000))
      return FALSE;
  print_report(test, name);
  g_print("stage %s generation=%" G_GUINT64_FORMAT
          " response_count=%u width=%.3f height=%.3f\n",
          name, generation, test->responses - before, width, height);
  ok = check(test, test->responses - before == (generation ? 1u : 0u),
             "exactly one response per request; none without a generation");
  if (generation)
    ok &= check(test, test->generation == generation,
                "response preserves requested generation");
  if (expected_width >= 0)
    ok &= check(test, fabs(width - expected_width) < EPSILON,
                "authored width is exact, including zero");
  else
    ok &= check(test, width > 0, "automatic width has positive intrinsic size");
  if (expected_height >= 0)
    ok &= check(test, fabs(height - expected_height) < EPSILON,
                "authored height is exact, including zero");
  else
    ok &= check(test, height > 0, "automatic height has positive intrinsic size");
  ok &= check(test, allocated(test, width, height),
              "real allocated wrapper bounds agree with resolved measurement");
  return ok;
}

static gboolean ink_size(Test *test, double *width, double *height) {
  JsonObject *report = pax_native_scene_report(test->scene);
  JsonObject *node = find_text(report);
  gboolean ok = node && json_object_has_member(node, "textLayout");
  if (ok) {
    JsonObject *layout = json_object_get_object_member(node, "textLayout");
    ok = json_object_has_member(layout, "inkBounds");
    if (ok) {
      JsonArray *ink = json_object_get_array_member(layout, "inkBounds");
      ok = json_array_get_length(ink) == 4;
      if (ok) {
        *width = json_array_get_double_element(ink, 2);
        *height = json_array_get_double_element(ink, 3);
        ok = *width > 0 && *height > 0;
      }
    }
  }
  json_object_unref(report);
  return check(test, ok, "actual label exposes positive Pango glyph ink bounds");
}

static gboolean change_dpi(Test *test, GtkSettings *settings, int dpi,
                           guint64 generation, gboolean burst) {
  guint before = test->invalidations, responses = test->responses;
  test->expected_generation = 0;
  test->expected_invalidation = generation;
  // GtkSettings overrides are process-local; no desktop preference is written.
  g_object_set(settings, "gtk-xft-dpi", dpi * 1024, NULL);
  if (burst) {
    g_object_set(settings, "gtk-xft-dpi", 120 * 1024, NULL);
    g_object_set(settings, "gtk-xft-dpi", dpi * 1024, NULL);
  }
  if (!pump(test, g_get_monotonic_time() + 100000) ||
      !check(test, test->invalidations == before && test->responses == responses,
             "settings callbacks defer engine events until an accepted batch") ||
      !apply(test, "[]") ||
      !pump(test, g_get_monotonic_time() + 100000))
    return FALSE;
  gboolean ok =
      check(test, test->invalidations - before == (generation ? 1u : 0u),
            generation ? "empty batch coalesces settings changes into one invalidation"
                       : "both-fixed Text does not request metric invalidation");
  if (generation)
    ok &= check(test, test->invalidated_generation == generation,
                "invalidation carries only the most recent measured generation");
  ok &= check(test, test->responses == responses,
              "metric invalidation does not reuse a measurement response");
  int effective = 0;
  g_object_get(settings, "gtk-xft-dpi", &effective, NULL);
  ok &= check(test, effective == dpi * 1024, "requested GTK font DPI took effect");
  if (!ok || !apply(test, "[]") ||
      !pump(test, g_get_monotonic_time() + 100000))
    return FALSE;
  ok = check(test, test->invalidations - before == (generation ? 1u : 0u) &&
                      test->responses == responses,
             "further empty batch emits no duplicate invalidation or response");
  test->expected_invalidation = 0;
  return ok;
}

int main(void) {
  Test test = {.deadline = g_get_monotonic_time() + 30 * G_TIME_SPAN_SECOND};
  if (!gtk_init_check()) {
    g_printerr("FAIL GTK could not open a display\n");
    return 2;
  }
  g_print("GTK %u.%u.%u; geometry-only native Text constraints\n",
          gtk_get_major_version(), gtk_get_minor_version(),
          gtk_get_micro_version());
  GtkSettings *settings = g_object_ref(gtk_settings_get_default());
  int original_dpi = 0;
  g_object_get(settings, "gtk-xft-dpi", &original_dpi, NULL);
  g_object_set(settings, "gtk-xft-dpi", 96 * 1024, NULL);
  if (!pump(&test, g_get_monotonic_time() + 100000)) {
    g_object_set(settings, "gtk-xft-dpi", original_dpi, NULL);
    g_object_unref(settings);
    return 1;
  }
  GtkWidget *window = gtk_window_new();
  g_object_ref_sink(window);
  GtkWidget *widget = pax_native_scene_new(interrupt, scene_error, &test);
  g_object_ref_sink(widget);
  test.scene = (PaxNativeScene *)widget;
  gtk_window_set_title(GTK_WINDOW(window), "Pax native Text constraints");
  gtk_window_set_default_size(GTK_WINDOW(window), 640, 400);
  gtk_window_set_child(GTK_WINDOW(window), widget);
  gtk_window_present(GTK_WINDOW(window));
  gint64 ready = g_get_monotonic_time() + 2000000;
  while ((!gtk_widget_get_mapped(widget) ||
          gtk_widget_get_width(widget) <= 0 ||
          gtk_widget_get_height(widget) <= 0) &&
         g_get_monotonic_time() < ready)
    if (!pump(&test, g_get_monotonic_time() + 10000))
      goto cleanup;
  if (!check(&test, gtk_widget_get_mapped(widget) &&
                       gtk_widget_get_width(widget) > 0 &&
                       gtk_widget_get_height(widget) > 0,
             "real window maps and allocates the native scene") ||
      !apply(&test, "[{\"TextCreate\":{\"id\":17,\"render_layer_id\":0}}]"))
    goto cleanup;

#define RUN(name, fields, generation, width, height)                       \
  do {                                                                    \
    if (!stage(&test, name, fields, generation, width, height))             \
      goto cleanup;                                                       \
  } while (0)

  RUN("intrinsic",
      "\"content\":\"Short label\",\"wrap\":true,\"size_x\":-1,\"size_y\":-1,"
      "\"style\":{\"font_size\":18},\"measure_generation\":1", 1, -1, -1);
  double intrinsic_width = test.width, intrinsic_height = test.height;
  RUN("fixed-width-auto-height",
      "\"size_x\":120,\"measure_generation\":2", 2, 120, -1);
  RUN("deduplicated-content-growth",
      "\"content\":\"Many words wrap into several lines when the width is "
      "constrained. Every trailing word must contribute to the height.\","
      "\"measure_generation\":3", 3, 120, -1);
  double grown_height = test.height;
  if (!check(&test, grown_height > intrinsic_height,
             "content-only patch retains automatic height and grows it"))
    goto cleanup;
  RUN("generation-only-remeasurement",
      "\"measure_generation\":4", 4, 120, grown_height);
  RUN("duplicate-content",
      "\"content\":\"Many words wrap into several lines when the width is "
      "constrained. Every trailing word must contribute to the height.\","
      "\"measure_generation\":5", 5, 120, grown_height);
  RUN("no-request-no-response", "\"opacity\":0.95", 0, 120, grown_height);
  RUN("explicit-zero-width", "\"size_x\":0,\"measure_generation\":6",
      6, 0, -1);
  RUN("auto-width-explicit-zero-height",
      "\"size_x\":-1,\"size_y\":0,\"measure_generation\":7", 7, -1, 0);
  RUN("both-explicit-zero", "\"size_x\":0", 0, 0, 0);
  RUN("both-fixed", "\"size_x\":123.25,\"size_y\":29.25", 0, 123.25, 29.25);
  RUN("fixed-width-return-auto-height",
      "\"size_y\":-1,\"measure_generation\":8", 8, 123.25, -1);
  RUN("auto-width-fixed-height",
      "\"size_x\":-1,\"size_y\":29.25,\"measure_generation\":9",
      9, -1, 29.25);
  RUN("return-both-auto-and-shrink",
      "\"content\":\"Short label\",\"size_y\":-1,\"measure_generation\":10",
      10, intrinsic_width, intrinsic_height);
  RUN("duplicate-short-no-request", "\"content\":\"Short label\"",
      0, intrinsic_width, intrinsic_height);
  RUN("final-generation-only", "\"measure_generation\":11",
      11, intrinsic_width, intrinsic_height);

  double original_ink_width = 0, original_ink_height = 0;
  if (!ink_size(&test, &original_ink_width, &original_ink_height) ||
      !change_dpi(&test, settings, 144, 11, TRUE))
    goto cleanup;
  RUN("live-dpi-144-fresh-measurement", "\"measure_generation\":12",
      12, -1, -1);
  double large_ink_width = 0, large_ink_height = 0;
  if (!check(&test, test.width > intrinsic_width && test.height > intrinsic_height,
             "fresh generation grows automatic bounds at 144 font DPI") ||
      !ink_size(&test, &large_ink_width, &large_ink_height) ||
      !check(&test, large_ink_width > original_ink_width &&
                       large_ink_height > original_ink_height,
             "unchanged native label glyph ink grows with effective font DPI") ||
      !change_dpi(&test, settings, 96, 12, FALSE))
    goto cleanup;
  RUN("live-dpi-96-fresh-measurement", "\"measure_generation\":13",
      13, intrinsic_width, intrinsic_height);
  double restored_ink_width = 0, restored_ink_height = 0;
  if (!ink_size(&test, &restored_ink_width, &restored_ink_height) ||
      !check(&test, fabs(restored_ink_width - original_ink_width) <= 1 &&
                       fabs(restored_ink_height - original_ink_height) <= 1,
             "native glyph ink returns to baseline after font DPI restoration"))
    goto cleanup;
  RUN("fixed-before-dpi-change", "\"size_x\":123.25,\"size_y\":29.25",
      0, 123.25, 29.25);
  if (!change_dpi(&test, settings, 144, 0, FALSE) ||
      !check(&test, allocated(&test, 123.25, 29.25),
             "both authored dimensions remain fixed after font DPI growth") ||
      !change_dpi(&test, settings, 96, 0, FALSE))
    goto cleanup;
  RUN("auto-after-fixed-dpi-cycle",
      "\"size_x\":-1,\"size_y\":-1,\"measure_generation\":14",
      14, intrinsic_width, intrinsic_height);
#undef RUN

cleanup:
  pax_native_scene_stop(test.scene);
  JsonObject *empty = pax_native_scene_report(test.scene);
  check(&test, json_array_get_length(json_object_get_array_member(empty, "nodes")) == 0,
        "stop removes all native nodes");
  json_object_unref(empty);
  guint before = test.responses;
  guint invalidations = test.invalidations;
  // Restore before destruction as well, exercising the stopped callback guard.
  g_object_set(settings, "gtk-xft-dpi", original_dpi, NULL);
  int restored_dpi = 0;
  g_object_get(settings, "gtk-xft-dpi", &restored_dpi, NULL);
  check(&test, restored_dpi == original_dpi, "original GTK font DPI restored");
  gpointer scene_alive = widget;
  g_object_add_weak_pointer(G_OBJECT(widget), &scene_alive);
  gtk_window_set_child(GTK_WINDOW(window), NULL);
  g_object_unref(widget);
  gtk_window_destroy(GTK_WINDOW(window));
  g_object_unref(window);
  // Cleanup remains bounded even after a failed assertion.
  gint64 end = g_get_monotonic_time() + 100000;
  while (g_get_monotonic_time() < end) {
    for (guint i = 0; i < 32 && g_main_context_pending(NULL); i++)
      g_main_context_iteration(NULL, FALSE);
    g_usleep(2000);
  }
  check(&test, scene_alive == NULL, "native scene is finalized after clean close");
  if (scene_alive)
    g_object_remove_weak_pointer(G_OBJECT(scene_alive), &scene_alive);
  check(&test, test.responses == before && test.invalidations == invalidations,
        "no measurement or invalidation callback after stop");
  g_object_unref(settings);
  g_print("RESULT passed=%s checks=%u responses=%u invalidations=%u\n",
          test.failed ? "false" : "true", test.checks, test.responses,
          test.invalidations);
  return test.failed ? 1 : 0;
}
