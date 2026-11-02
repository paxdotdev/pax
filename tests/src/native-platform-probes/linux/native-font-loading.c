/* Sequential font-face registration regression. Build with native-scene.c, assets.c and
 * pkg-config gtk4 json-glib-1.0 fontconfig pangoft2 plus -lm. Pass the directory
 * containing bundled Roboto.ttf and Roboto-Italic.ttf; run in a GTK session.
 */
#include "assets.h"
#include "native-scene.h"
#include <gtk/gtk.h>
#include <pango/pangocairo.h>
#include <pango/pangofc-fontmap.h>
#include <fontconfig/fontconfig.h>
#include <string.h>
static guint checks, failures;
static void check(gboolean value, const char *name) {
  checks++; failures += !value; g_print("%s %s\n",value?"PASS":"FAIL",name);
}
static void pump(void) {
  gint64 until=g_get_monotonic_time()+150000;
  while(g_get_monotonic_time()<until) {
    for(guint i=0;i<32 && g_main_context_pending(NULL);i++)g_main_context_iteration(NULL,FALSE);
    g_usleep(1000);
  }
}
static void inspect(GtkLabel *label, const char *expected_file, PangoStyle expected_style) {
  pump(); PangoLayout *layout=gtk_label_get_layout(label);
  PangoLayoutIter *iterator=pango_layout_get_iter(layout);
  PangoLayoutRun *run=pango_layout_iter_get_run_readonly(iterator);
  PangoFont *font=run?run->item->analysis.font:NULL;
  FcPattern *pattern=NULL; FcChar8 *file=NULL;
  if(font && g_object_class_find_property(G_OBJECT_GET_CLASS(font),"pattern"))g_object_get(font,"pattern",&pattern,NULL);
  if(pattern)FcPatternGetString(pattern,FC_FILE,0,&file);
  PangoFontDescription *description=font?pango_font_describe(font):NULL;
  char *text=description?pango_font_description_to_string(description):g_strdup("none");
  char *basename=file?g_path_get_basename((const char *)file):NULL;
  g_print("actual file=%s description=%s\n",file?(const char *)file:"none",text);
  check(!g_strcmp0(basename,expected_file),"actual glyphs use the requested registered face file");
  check(description && pango_font_description_get_style(description)==expected_style,
        "actual selected face has the authored slant without synthesis");
  g_free(text);g_free(basename);
  if(description)pango_font_description_free(description);
  pango_layout_iter_free(iterator);
}
static void style(GtkLabel *label,PangoStyle slant) {
  PangoAttrList *attributes=pango_attr_list_new();
  pango_attr_list_insert(attributes,pango_attr_family_new("Roboto"));
  pango_attr_list_insert(attributes,pango_attr_style_new(slant));
  pango_attr_list_insert(attributes,pango_attr_size_new(20*PANGO_SCALE));
  gtk_label_set_attributes(label,attributes);pango_attr_list_unref(attributes);
}
static guint invalidations;
static guint64 invalidated_generation;
static gboolean interrupt(gpointer data, const char *text, gsize length) {
  (void)data; JsonParser *parser=json_parser_new();
  if(json_parser_load_from_data(parser,text,length,NULL)) {
    JsonObject *object=json_node_get_object(json_parser_get_root(parser));
    if(json_object_has_member(object,"TextMeasurementInvalidated")) {
      JsonObject *body=json_object_get_object_member(object,"TextMeasurementInvalidated");
      if(json_object_get_int_member(body,"id")==51) {
        invalidations++; invalidated_generation=json_object_get_int_member(body,"generation");
      }
    }
  }
  g_object_unref(parser);return TRUE;
}
static void scene_error(gpointer data,const char *message) {
  (void)data;failures++;g_printerr("scene error: %s\n",message);
}
static void apply(PaxNativeScene *scene,const char *text) {
  JsonParser *parser=json_parser_new();GError *error=NULL;
  gboolean ok=json_parser_load_from_data(parser,text,-1,&error);
  if(ok)ok=pax_native_scene_apply(scene,json_node_get_array(json_parser_get_root(parser)),&error);
  if(!ok)scene_error(NULL,error?error->message:"patch failed");
  g_clear_error(&error);g_object_unref(parser);pump();
}
static void existing_auto_text(char **roots) {
  PangoFontMap *map=pango_cairo_font_map_new();
  FcConfig *config=FcInitLoadConfigAndFonts();
  pango_fc_font_map_set_config(PANGO_FC_FONT_MAP(map),config);FcConfigDestroy(config);
  GtkWidget *window=gtk_window_new();g_object_ref_sink(window);
  GtkWidget *scene=pax_native_scene_new(interrupt,scene_error,NULL);
  gtk_widget_set_font_map(scene,map);g_object_unref(map);
  pax_native_scene_assets((PaxNativeScene *)scene,roots);
  gtk_window_set_child(GTK_WINDOW(window),scene);gtk_window_present(GTK_WINDOW(window));
  apply((PaxNativeScene *)scene,
    "[{\"TextCreate\":{\"id\":51,\"render_layer_id\":0}},"
    "{\"TextUpdate\":{\"id\":51,\"content\":\"Auto italic @@@@@@@@\",\"size_x\":-1,\"size_y\":-1,\"measure_generation\":41,\"transform\":[1,0,0,1,0,0],"
    "\"style\":{\"font\":{\"Web\":{\"family\":\"Roboto\",\"url\":\"Roboto.ttf\",\"weight\":\"Normal\",\"style\":\"Italic\"}},\"font_size\":26}}}]");
  guint before=invalidations;
  apply((PaxNativeScene *)scene,
    "[{\"ButtonCreate\":{\"id\":52,\"render_layer_id\":0}},"
    "{\"ButtonUpdate\":{\"id\":52,\"content\":\"Register face\",\"size_x\":160,\"size_y\":40,\"transform\":[1,0,0,1,0,80],"
    "\"style\":{\"font\":{\"Web\":{\"family\":\"Roboto\",\"url\":\"Roboto-Italic.ttf\",\"weight\":\"Normal\",\"style\":\"Italic\"}},\"font_size\":20}}}]");
  g_print("font invalidations before=%u after=%u generation=%" G_GUINT64_FORMAT "\n",before,invalidations,invalidated_generation);
  check(invalidations==before+1 && invalidated_generation==41,
        "new family face invalidates existing automatic Text with its current generation");
  JsonObject *report=pax_native_scene_report((PaxNativeScene *)scene);
  JsonArray *nodes=json_object_get_array_member(report,"nodes");gboolean correct=FALSE;
  for(guint i=0;i<json_array_get_length(nodes);i++) {
    JsonObject *node=json_array_get_object_element(nodes,i);
    if(json_object_get_int_member(node,"id")==51) {
      const char *file=json_object_get_string_member(node,"resolvedFontFile");
      correct=file && g_str_has_suffix(file,"/Roboto-Italic.ttf");
    }
  }
  check(correct,"existing GTK label resolves newly registered italic face without content/style patch");
  json_object_unref(report);
  before=invalidations;apply((PaxNativeScene *)scene,"[]");
  check(invalidations==before,"unchanged accepted batch does not repeat font invalidation");
  pax_native_scene_stop((PaxNativeScene *)scene);gtk_window_destroy(GTK_WINDOW(window));g_object_unref(window);pump();
}
int main(int argc,char **argv) {
  if(argc!=2)return 2;
  gtk_init();char *roots[]={argv[1],NULL};GError *error=NULL;
  GtkWidget *window=gtk_window_new();g_object_ref_sink(window);
  GtkWidget *label=gtk_label_new("Roboto sequential face @@@@@@@@");
  gtk_window_set_child(GTK_WINDOW(window),label);gtk_window_present(GTK_WINDOW(window));
  PangoFontMap *map=pango_context_get_font_map(gtk_widget_get_pango_context(label));
  g_print("GTK font map type=%s same_as_default=%d\n",G_OBJECT_TYPE_NAME(map),map==pango_cairo_font_map_get_default());
  check(pax_linux_font_load(roots,"Roboto.ttf",map,&error),"register normal bundled face");
  if(error){g_printerr("%s\n",error->message);g_clear_error(&error);}
  style(GTK_LABEL(label),PANGO_STYLE_NORMAL);inspect(GTK_LABEL(label),"Roboto.ttf",PANGO_STYLE_NORMAL);
  check(pax_linux_font_load(roots,"Roboto-Italic.ttf",map,&error),"register italic after normal family was already resolved");
  if(error){g_printerr("%s\n",error->message);g_clear_error(&error);}
  style(GTK_LABEL(label),PANGO_STYLE_ITALIC);inspect(GTK_LABEL(label),"Roboto-Italic.ttf",PANGO_STYLE_ITALIC);
  style(GTK_LABEL(label),PANGO_STYLE_NORMAL);inspect(GTK_LABEL(label),"Roboto.ttf",PANGO_STYLE_NORMAL);
  guint serial=pango_font_map_get_serial(map);
  check(pax_linux_font_load(roots,"Roboto.ttf",map,&error) && pango_font_map_get_serial(map)==serial,
        "repeated registered font preserves font-map serial");
  PangoFontMap *separate=pango_cairo_font_map_new();
  FcConfig *config=FcInitLoadConfigAndFonts();
  pango_fc_font_map_set_config(PANGO_FC_FONT_MAP(separate),config);FcConfigDestroy(config);
  gtk_widget_set_font_map(label,separate);map=separate;
  check(pax_linux_font_load(roots,"Roboto.ttf",map,&error),"same file registers in a distinct Fontconfig configuration");
  style(GTK_LABEL(label),PANGO_STYLE_NORMAL);inspect(GTK_LABEL(label),"Roboto.ttf",PANGO_STYLE_NORMAL);
  check(pax_linux_font_load(roots,"Roboto-Italic.ttf",map,&error),"second configuration registers italic independently");
  style(GTK_LABEL(label),PANGO_STYLE_ITALIC);inspect(GTK_LABEL(label),"Roboto-Italic.ttf",PANGO_STYLE_ITALIC);
  g_object_unref(separate);
  gtk_window_destroy(GTK_WINDOW(window));g_object_unref(window);pump();
  existing_auto_text(roots);
  g_print("%s %u native font loading checks\n",failures?"FAIL":"PASS",checks);
  return failures?1:0;
}
