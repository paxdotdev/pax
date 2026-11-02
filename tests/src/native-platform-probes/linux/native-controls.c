/*
 * Native form-control adapter regression. Build like text-constraints.c, replacing
 * its filename with this one; run under a real GTK display with timeout 30s.
 * Direct GTK value changes exercise native signals, not physical pointer input.
 */
#include "native-scene.h"
#include <math.h>
#include <string.h>

typedef struct {
  PaxNativeScene *scene;
  guint checks, slider_events, dropdown_events, radio_events;
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

static guint events(Test *test) {
  return test->slider_events + test->dropdown_events + test->radio_events;
}

static void check_mask_pick(Test *test, GtkWidget *scene, GtkWidget *scale) {
  graphene_point_t local = GRAPHENE_POINT_INIT(150, 20), position;
  gboolean located = gtk_widget_compute_point(scale, scene, &local, &position);
  check(test, located, "native scale has a scene-space input point");
  if (!located)
    return;
  for (guint fractional = 0; fractional < 2; fractional++) {
    char *patch = g_strdup_printf(
        "[{\"NativeMaskUpdate\":{\"id\":11,\"entries\":["
        "{\"path\":\"M 0 0 L 300 0 L 300 40 L 0 40 Z\",\"clips\":[],\"opacity\":%s}]}}]",
        fractional ? "0.5" : "1");
    apply(test, patch);
    g_free(patch);
    pump();
    GtkWidget *picked = gtk_widget_pick(scene, position.x, position.y, GTK_PICK_DEFAULT);
    gboolean native = picked && (picked == scale || gtk_widget_is_ancestor(picked, scale));
    check(test, fractional ? native : !native,
          fractional ? "fractional vector mask retains native slider picking"
                     : "opaque vector mask prevents native slider picking");
  }
  apply(test, "[{\"NativeMaskUpdate\":{\"id\":11,\"entries\":[]}}]");
  pump();
  GtkWidget *picked = gtk_widget_pick(scene, position.x, position.y, GTK_PICK_DEFAULT);
  check(test, picked && (picked == scale || gtk_widget_is_ancestor(picked, scale)),
        "removing vector mask restores ordinary native slider picking");
}

static gboolean attributes_match(PangoAttrList *attributes, int weight,
                                 int style, int alpha) {
  if (!attributes)
    return FALSE;
  PangoAttrIterator *iterator = pango_attr_list_get_iterator(attributes);
  PangoAttrInt *w = (PangoAttrInt *)pango_attr_iterator_get(iterator, PANGO_ATTR_WEIGHT);
  PangoAttrInt *s = (PangoAttrInt *)pango_attr_iterator_get(iterator, PANGO_ATTR_STYLE);
  PangoAttrInt *a = (PangoAttrInt *)pango_attr_iterator_get(iterator, PANGO_ATTR_FOREGROUND_ALPHA);
  gboolean matches = w && s && a && w->value == weight &&
                     s->value == style && a->value == alpha;
  pango_attr_iterator_destroy(iterator);
  return matches;
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

static void check_typography(Test *test, GtkWidget *scene) {
  apply(test, "[{\"TextCreate\":{\"id\":21,\"render_layer_id\":0}},"
              "{\"ButtonCreate\":{\"id\":22,\"render_layer_id\":0}},"
              "{\"TextboxCreate\":{\"id\":23,\"render_layer_id\":0}},"
              "{\"TextUpdate\":{\"id\":21,\"content\":\"Typography text\",\"size_x\":200,\"size_y\":40}},"
              "{\"ButtonUpdate\":{\"id\":22,\"content\":\"Typography button\",\"size_x\":200,\"size_y\":40}},"
              "{\"TextboxUpdate\":{\"id\":23,\"text\":\"Typography entry\",\"size_x\":200,\"size_y\":40}}]");
  const char *kinds[] = {"Text", "Button", "Textbox", "Dropdown", "RadioList"};
  const guint ids[] = {21, 22, 23, 12, 13};
  for (guint reset = 0; reset < 2; reset++) {
    for (guint i = 0; i < G_N_ELEMENTS(kinds); i++) {
      char *patch = g_strdup_printf(
          "[{\"%sUpdate\":{\"id\":%u,\"style\":{\"font\":{\"Web\":"
          "{\"family\":\"Arial\",\"url\":\"\",\"weight\":\"%s\",\"style\":\"%s\"}},"
          "\"font_size\":18,\"underline\":%s,\"fill\":{\"Rgba\":[0.2,0.3,0.4,%s]}}}}]",
          kinds[i], ids[i], reset ? "Normal" : "Bold",
          reset ? "Normal" : "Italic", reset ? "false" : "true", reset ? "1" : "0.5");
      apply(test, patch);
      g_free(patch);
    }
    pump();
    GtkWidget *text = text_label(scene, "Typography text");
    GtkWidget *button = text_label(scene, "Typography button");
    GtkWidget *entry = find(scene, GTK_TYPE_ENTRY, NULL);
    GtkWidget *radio = text_label(scene, "Solo");
    GtkWidget *choice = text_label(find(scene, GTK_TYPE_DROP_DOWN, NULL), "Restored");
    PangoAttrList *lists[] = {
        text ? gtk_label_get_attributes(GTK_LABEL(text)) : NULL,
        button ? gtk_label_get_attributes(GTK_LABEL(button)) : NULL,
        entry ? gtk_entry_get_attributes(GTK_ENTRY(entry)) : NULL,
        choice ? gtk_label_get_attributes(GTK_LABEL(choice)) : NULL,
        radio ? gtk_label_get_attributes(GTK_LABEL(radio)) : NULL};
    for (guint i = 0; i < G_N_ELEMENTS(kinds); i++) {
      char *name = g_strdup_printf("%s native label %s weight, slant, and fill alpha",
                                  kinds[i], reset ? "resets" : "applies");
      check(test, attributes_match(lists[i], reset ? PANGO_WEIGHT_NORMAL : PANGO_WEIGHT_BOLD,
                                  reset ? PANGO_STYLE_NORMAL : PANGO_STYLE_ITALIC,
                                  reset ? 65535 : (guint16)(.5 * 65535)), name);
      g_free(name);
      PangoAttrIterator *iterator = lists[i] ? pango_attr_list_get_iterator(lists[i]) : NULL;
      PangoAttrInt *underline = iterator
          ? (PangoAttrInt *)pango_attr_iterator_get(iterator, PANGO_ATTR_UNDERLINE) : NULL;
      name = g_strdup_printf("%s native label %s underline", kinds[i], reset ? "clears" : "applies");
      check(test, iterator && (reset ? !underline || underline->value == PANGO_UNDERLINE_NONE
                                    : underline && underline->value == PANGO_UNDERLINE_SINGLE), name);
      g_free(name);
      if (iterator) pango_attr_iterator_destroy(iterator);
    }
  }
}

int main(void) {
  gtk_init();
  Test test = {0};
  GtkWidget *window = gtk_window_new();
  g_object_ref_sink(window);
  GtkWidget *scene = pax_native_scene_new(interrupt, error, &test);
  test.scene = (PaxNativeScene *)scene;
  gtk_window_set_default_size(GTK_WINDOW(window), 600, 480);
  gtk_window_set_child(GTK_WINDOW(window), scene);
  gtk_window_present(GTK_WINDOW(window));
  if (!apply(&test,
      "[{\"SliderCreate\":{\"id\":11,\"render_layer_id\":0}},"
      "{\"DropdownCreate\":{\"id\":12,\"render_layer_id\":0}},"
      "{\"RadioListCreate\":{\"id\":13,\"render_layer_id\":0}},"
      "{\"SliderUpdate\":{\"id\":11,\"size_x\":300,\"size_y\":40,"
      "\"transform\":[1,0,0,1,20,20],\"min\":-2,\"max\":2,\"step\":0.25,\"value\":0.5}},"
      "{\"DropdownUpdate\":{\"id\":12,\"size_x\":300,\"size_y\":40,"
      "\"transform\":[1,0,0,1,20,90],\"options\":[\"Alpha\",\"Beta\",\"Gamma\"],\"selected_id\":1}},"
      "{\"RadioListUpdate\":{\"id\":13,\"size_x\":300,\"size_y\":120,"
      "\"transform\":[1,0,0,1,20,160],\"options\":[\"Alpha\",\"Beta\",\"Gamma\"],\"selected_id\":1}}]"))
    goto done;
  pump();
  GtkWidget *scale = find(scene, GTK_TYPE_SCALE, NULL);
  GtkWidget *dropdown = find(scene, GTK_TYPE_DROP_DOWN, NULL);
  GtkWidget *alpha = find(scene, GTK_TYPE_CHECK_BUTTON, "Alpha");
  GtkWidget *beta = find(scene, GTK_TYPE_CHECK_BUTTON, "Beta");
  GtkWidget *gamma = find(scene, GTK_TYPE_CHECK_BUTTON, "Gamma");
  check(&test, scale && dropdown && alpha && beta && gamma, "all three control families use real GTK widgets");
  if (!scale || !dropdown || !alpha || !beta || !gamma)
    goto done;
  check(&test, events(&test) == 0, "initial engine patches produce no native form feedback");
  check(&test, fabs(gtk_range_get_value(GTK_RANGE(scale)) - .5) < .001, "initial slider value applied");
  GtkAdjustment *adjustment = gtk_range_get_adjustment(GTK_RANGE(scale));
  check(&test, gtk_adjustment_get_lower(adjustment) == -2 &&
              gtk_adjustment_get_upper(adjustment) == 2 &&
              gtk_adjustment_get_step_increment(adjustment) == .25, "slider range and step are native adjustment values");
  check(&test, gtk_drop_down_get_selected(GTK_DROP_DOWN(dropdown)) == 1, "initial dropdown selection applied");
  check(&test, gtk_check_button_get_active(GTK_CHECK_BUTTON(beta)) &&
              !gtk_check_button_get_active(GTK_CHECK_BUTTON(alpha)), "initial radio selection is exclusive");
  check(&test, gtk_accessible_get_accessible_role(GTK_ACCESSIBLE(scale)) == GTK_ACCESSIBLE_ROLE_SLIDER,
        "slider exposes native accessible slider role");
  check(&test, gtk_accessible_get_accessible_role(GTK_ACCESSIBLE(dropdown)) == GTK_ACCESSIBLE_ROLE_COMBO_BOX,
        "dropdown exposes native accessible combo-box role");
  check(&test, gtk_accessible_get_accessible_role(GTK_ACCESSIBLE(beta)) == GTK_ACCESSIBLE_ROLE_RADIO,
        "grouped choices expose the native radio role");
  check_mask_pick(&test, scene, scale);

  gtk_range_set_value(GTK_RANGE(scale), .75);
  gtk_range_set_value(GTK_RANGE(scale), 1.0);
  gtk_range_set_value(GTK_RANGE(scale), 1.25);
  check(&test, test.slider_events == 3 && fabs(test.slider_value - 1.25) < .001,
        "successive native slider changes each publish their value");
  gtk_drop_down_set_selected(GTK_DROP_DOWN(dropdown), 2);
  check(&test, test.dropdown_events == 1 && test.dropdown_value == 2,
        "native dropdown selection publishes its index");
  gtk_check_button_set_active(GTK_CHECK_BUTTON(alpha), TRUE);
  check(&test, test.radio_events == 1 && test.radio_value == 0 &&
              !gtk_check_button_get_active(GTK_CHECK_BUTTON(beta)),
        "radio activation emits once and deactivation emits no false selection");

  guint before = events(&test);
  apply(&test, "[{\"SliderUpdate\":{\"id\":11,\"value\":-1}},"
               "{\"DropdownUpdate\":{\"id\":12,\"selected_id\":0}},"
               "{\"RadioListUpdate\":{\"id\":13,\"selected_id\":2}}]");
  pump();
  check(&test, events(&test) == before, "programmatic value patches suppress form feedback");
  check(&test, scale == find(scene, GTK_TYPE_SCALE, NULL) &&
              dropdown == find(scene, GTK_TYPE_DROP_DOWN, NULL) &&
              gamma == find(scene, GTK_TYPE_CHECK_BUTTON, "Gamma"),
        "ordinary value patches preserve native widget identity");
  check(&test, gtk_range_get_value(GTK_RANGE(scale)) == -1 &&
              gtk_drop_down_get_selected(GTK_DROP_DOWN(dropdown)) == 0 &&
              gtk_check_button_get_active(GTK_CHECK_BUTTON(gamma)),
        "deduplicated patches update each native value");

  apply(&test, "[{\"SliderUpdate\":{\"id\":11,\"min\":0,\"max\":10,\"step\":2,\"value\":6}},"
               "{\"DropdownUpdate\":{\"id\":12,\"options\":[\"One\",\"Two\"],\"selected_id\":1}},"
               "{\"RadioListUpdate\":{\"id\":13,\"options\":[\"One\",\"Two\"],\"selected_id\":1}}]");
  pump();
  check(&test, events(&test) == before, "range and options replacement suppress transient feedback");
  check(&test, gtk_adjustment_get_lower(adjustment) == 0 &&
              gtk_adjustment_get_upper(adjustment) == 10 &&
              gtk_adjustment_get_step_increment(adjustment) == 2 &&
              gtk_range_get_value(GTK_RANGE(scale)) == 6, "range replacement retains the explicitly requested value and step");
  check(&test, g_list_model_get_n_items(gtk_drop_down_get_model(GTK_DROP_DOWN(dropdown))) == 2 &&
              gtk_drop_down_get_selected(GTK_DROP_DOWN(dropdown)) == 1, "dropdown options replacement applies the new selection");
  GtkWidget *two = find(scene, GTK_TYPE_CHECK_BUTTON, "Two");
  check(&test, two && gtk_check_button_get_active(GTK_CHECK_BUTTON(two)) &&
              !find(scene, GTK_TYPE_CHECK_BUTTON, "Gamma"), "radio options replacement removes old options and selects new option");
  GListModel *model = gtk_drop_down_get_model(GTK_DROP_DOWN(dropdown));
  apply(&test, "[{\"DropdownUpdate\":{\"id\":12,\"options\":[\"One\",\"Two\"]}},"
               "{\"RadioListUpdate\":{\"id\":13,\"options\":[\"One\",\"Two\"]}}]");
  pump();
  check(&test, model == gtk_drop_down_get_model(GTK_DROP_DOWN(dropdown)) &&
              two == find(scene, GTK_TYPE_CHECK_BUTTON, "Two"),
        "identical option patches preserve native choice identities");
  check(&test, events(&test) == before, "identical options produce no feedback");
  apply(&test, "[{\"SliderUpdate\":{\"id\":11,\"value\":7.1}}]");
  check(&test, events(&test) == before && fabs(gtk_range_get_value(GTK_RANGE(scale)) - 7.1) < .001,
        "authored off-grid value remains exact without synthetic input");
  guint slider_before = test.slider_events;
  gtk_range_set_value(GTK_RANGE(scale), 7.2);
  check(&test, test.slider_events == slider_before + 1 &&
              gtk_range_get_value(GTK_RANGE(scale)) == 8 && test.slider_value == 8,
        "native accessible values snap to the nearest min-based step exactly once");
  before = events(&test);
  apply(&test, "[{\"SliderUpdate\":{\"id\":11,\"min\":10,\"max\":20,\"step\":2,\"value\":14}}]");
  pump();
  check(&test, events(&test) == before && gtk_range_get_value(GTK_RANGE(scale)) == 14 &&
              gtk_adjustment_get_lower(adjustment) == 10 && gtk_adjustment_get_upper(adjustment) == 20,
        "both range endpoints can move beyond the old range atomically");
  apply(&test, "[{\"SliderUpdate\":{\"id\":11,\"min\":0,\"max\":10,\"step\":2,\"value\":6}}]");
  pump();
  check(&test, events(&test) == before && gtk_range_get_value(GTK_RANGE(scale)) == 6,
        "range can return below the prior minimum without feedback");
  apply(&test, "[{\"RadioListUpdate\":{\"id\":13,\"options\":[\"One\",\"Two\",\"Third\"],\"selected_id\":1}}]");
  pump();
  two = find(scene, GTK_TYPE_CHECK_BUTTON, "Two");
  check(&test, two && gtk_widget_grab_focus(two), "middle radio accepts real GTK focus");
  pump();
  GtkWidget *focused = gtk_root_get_focus(GTK_ROOT(window));
  check(&test, focused && (focused == two || gtk_widget_is_ancestor(focused, two)),
        "middle radio owns root focus before shrinking multiple options");
  apply(&test, "[{\"RadioListUpdate\":{\"id\":13,\"options\":[\"Solo\"],\"selected_id\":0}}]");
  pump();
  check(&test, find(scene, GTK_TYPE_CHECK_BUTTON, "Solo") &&
              !find(scene, GTK_TYPE_CHECK_BUTTON, "Third") && events(&test) == before,
        "removing focused middle and trailing options is safe and silent");
  apply(&test, "[{\"RadioListUpdate\":{\"id\":13,\"options\":[\"One\",\"Two\"],\"selected_id\":1}}]");
  pump();

  apply(&test, "[{\"DropdownUpdate\":{\"id\":12,\"options\":[]}},"
               "{\"RadioListUpdate\":{\"id\":13,\"options\":[]}}]");
  pump();
  check(&test, events(&test) == before, "empty options produce no phantom selection events");
  check(&test, g_list_model_get_n_items(gtk_drop_down_get_model(GTK_DROP_DOWN(dropdown))) == 0 &&
              !find(scene, GTK_TYPE_CHECK_BUTTON, "Two"), "empty option lists remove native choices");
  apply(&test, "[{\"DropdownUpdate\":{\"id\":12,\"options\":[\"First\",\"Restored\"]}},"
               "{\"RadioListUpdate\":{\"id\":13,\"options\":[\"First\",\"Restored\"]}}]");
  pump();
  check(&test, events(&test) == before, "refilling options produces no programmatic feedback");
  GtkWidget *restored = find(scene, GTK_TYPE_CHECK_BUTTON, "Restored");
  check(&test, restored && gtk_check_button_get_active(GTK_CHECK_BUTTON(restored)) &&
              gtk_drop_down_get_selected(GTK_DROP_DOWN(dropdown)) == 1,
        "empty lists retain requested selection when options return");
  apply(&test, "[{\"RadioListUpdate\":{\"id\":13,\"options\":[\"Solo\"],\"selected_id\":0}}]");
  pump();
  restored = find(scene, GTK_TYPE_CHECK_BUTTON, "Solo");
  check(&test, restored && gtk_check_button_get_active(GTK_CHECK_BUTTON(restored)) &&
              gtk_accessible_get_accessible_role(GTK_ACCESSIBLE(restored)) == GTK_ACCESSIBLE_ROLE_RADIO,
        "single-option list remains a selected native radio");
  check(&test, events(&test) == before, "single-option replacement produces no feedback");
  check_typography(&test, scene);
  check(&test, events(&test) == before, "typography changes produce no form feedback");
  g_object_ref(scale);
  g_object_ref(dropdown);
  if (restored)
    g_object_ref(restored);
  pax_native_scene_stop(test.scene);
  gtk_range_set_value(GTK_RANGE(scale), 8);
  gtk_drop_down_set_selected(GTK_DROP_DOWN(dropdown), GTK_INVALID_LIST_POSITION);
  if (restored)
    gtk_check_button_set_active(GTK_CHECK_BUTTON(restored), FALSE);
  pump();
  check(&test, events(&test) == before, "stopped scene emits no further form events");
  g_object_unref(scale);
  g_object_unref(dropdown);
  if (restored)
    g_object_unref(restored);
done:
  pax_native_scene_stop(test.scene);
  gtk_window_destroy(GTK_WINDOW(window));
  g_object_unref(window);
  pump();
  g_print("RESULT passed=%s checks=%u slider=%u dropdown=%u radio=%u\n",
          test.failed ? "false" : "true", test.checks, test.slider_events,
          test.dropdown_events, test.radio_events);
  return test.failed ? 1 : 0;
}
