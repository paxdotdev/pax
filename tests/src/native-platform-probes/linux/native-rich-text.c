/* Native normalized rich-text contract. Build/run like native-styling.c. */
#include "native-scene.h"
#include <string.h>

typedef struct { PaxNativeScene *scene; guint checks, links; gboolean failed; char *uri; } Test;
static void check(Test *test, gboolean ok, const char *name) {
  test->checks++; test->failed |= !ok;
  g_print("%s %s\n", ok ? "PASS" : "FAIL", name);
}
static void navigate(gpointer data, const char *uri) {
  Test *test = data; test->links++; g_free(test->uri); test->uri = g_strdup(uri);
}
static gboolean apply(Test *test, const char *text, gboolean expected) {
  JsonParser *parser = json_parser_new(); GError *error = NULL;
  if (!json_parser_load_from_data(parser, text, -1, &error)) g_error("%s", error->message);
  gboolean ok = pax_native_scene_apply(test->scene,
      json_node_get_array(json_parser_get_root(parser)), &error);
  check(test, ok == expected && (ok || error), expected ? "patch accepted" : "unsupported/invalid patch rejected");
  if (error) g_print("expected error: %s\n", error->message);
  g_clear_error(&error); g_object_unref(parser); return ok;
}
static GtkWidget *label(GtkWidget *widget) {
  if (GTK_IS_LABEL(widget)) return widget;
  for (GtkWidget *child=gtk_widget_get_first_child(widget);child;child=gtk_widget_get_next_sibling(child)) {
    GtkWidget *found=label(child); if (found) return found;
  }
  return NULL;
}
static PangoAttribute *attribute(GtkWidget *widget, PangoAttrType type, int index) {
  PangoAttrIterator *iterator=pango_attr_list_get_iterator(gtk_label_get_attributes(GTK_LABEL(widget)));
  PangoAttribute *found=NULL;
  do { int start,end; pango_attr_iterator_range(iterator,&start,&end);
    if (index>=start && index<end) { PangoAttribute *a=pango_attr_iterator_get(iterator,type); if(a)found=pango_attribute_copy(a); break; }
  } while(pango_attr_iterator_next(iterator));
  pango_attr_iterator_destroy(iterator); return found;
}
static int integer_attribute(GtkWidget *widget, PangoAttrType type, int index) {
  PangoAttribute *a=attribute(widget,type,index);int value=a?((PangoAttrInt *)a)->value:-1;
  if(a)pango_attribute_destroy(a);
  return value;
}
static void pump(void) {
  gint64 end=g_get_monotonic_time()+100000;
  while(g_get_monotonic_time()<end){for(guint i=0;i<32&&g_main_context_pending(NULL);i++)g_main_context_iteration(NULL,FALSE);g_usleep(2000);}
}
int main(void) {
  gtk_init(); Test test={0}; GtkWidget *window=gtk_window_new();g_object_ref_sink(window);
  GtkWidget *scene=pax_native_scene_new(NULL,NULL,&test);test.scene=(PaxNativeScene *)scene;
  pax_native_scene_navigation(test.scene,navigate);gtk_window_set_default_size(GTK_WINDOW(window),600,250);
  gtk_window_set_child(GTK_WINDOW(window),scene);gtk_window_present(GTK_WINDOW(window));
  apply(&test,"[{\"TextCreate\":{\"id\":1}},{\"TextUpdate\":{\"id\":1,\"markdown\":true,\"content\":\"unused source\",\"rich_text\":{\"text\":\"α<&\\nfn 😀()\",\"line_height\":22,\"runs\":[{\"start\":0,\"end\":5,\"color\":[1,0,0,1]},{\"start\":5,\"end\":14,\"bold\":true,\"italic\":true}]},\"selectable\":true,\"wrap\":false,\"size_x\":-1,\"size_y\":-1,\"measure_generation\":1,\"style\":{\"font_size\":18,\"fill\":{\"Rgba\":[0,0,1,1]}}}}]",TRUE);
  pump();GtkWidget *text=label(scene);check(&test,text!=NULL,"rich text remains a genuine GTK label");if(!text)goto done;
  check(&test,!strcmp(gtk_label_get_text(GTK_LABEL(text)),"α<&\nfn 😀()"),"entities and Unicode render as exact decoded text");
  check(&test,pango_layout_get_line_count(gtk_label_get_layout(GTK_LABEL(text)))==2,"preformatted line break is preserved");
  PangoAttribute *color=attribute(text,PANGO_ATTR_FOREGROUND,0);
  check(&test,color&&((PangoAttrColor *)color)->color.red==65535&&((PangoAttrColor *)color)->color.blue==0,"first UTF8 range has authored red color");if(color)pango_attribute_destroy(color);
  color=attribute(text,PANGO_ATTR_FOREGROUND,5);
  check(&test,color&&((PangoAttrColor *)color)->color.blue==65535,"next range restores base blue color");if(color)pango_attribute_destroy(color);
  check(&test,integer_attribute(text,PANGO_ATTR_WEIGHT,5)==PANGO_WEIGHT_BOLD,"native bold range");
  check(&test,integer_attribute(text,PANGO_ATTR_STYLE,5)==PANGO_STYLE_ITALIC,"native italic range");
  gtk_label_select_region(GTK_LABEL(text),1,3);
  apply(&test,"[{\"TextUpdate\":{\"id\":1,\"style\":{\"font_size\":24,\"fill\":{\"Rgba\":[0,1,0,1]}},\"style_link\":null,\"measure_generation\":2}}]",TRUE);
  int a,b;check(&test,gtk_label_get_selection_bounds(GTK_LABEL(text),&a,&b)&&a==1&&b==3,"style-only update retains native selection");
  check(&test,label(scene)==text&&integer_attribute(text,PANGO_ATTR_WEIGHT,5)==PANGO_WEIGHT_BOLD,"style-only update retains widget and inline attributes");
  apply(&test,"[{\"TextUpdate\":{\"id\":1,\"size_x\":240,\"size_y\":80}}]",TRUE);
  GtkSettings *settings=gtk_widget_get_settings(scene);int original_dpi;
  g_object_get(settings,"gtk-xft-dpi",&original_dpi,NULL);
  g_object_set(settings,"gtk-xft-dpi",96*1024,NULL);pump();apply(&test,"[]",TRUE);
  int height96=integer_attribute(text,PANGO_ATTR_ABSOLUTE_LINE_HEIGHT,0);
  g_object_set(settings,"gtk-xft-dpi",144*1024,NULL);pump();apply(&test,"[]",TRUE);
  int height144=integer_attribute(text,PANGO_ATTR_ABSOLUTE_LINE_HEIGHT,0);
  check(&test,height96>0&&height144>height96*1.45&&height144<height96*1.55,"fixed-size pre line height follows live font DPI without a content patch");
  g_object_set(settings,"gtk-xft-dpi",96*1024,NULL);pump();apply(&test,"[]",TRUE);
  check(&test,integer_attribute(text,PANGO_ATTR_ABSOLUTE_LINE_HEIGHT,0)==height96,"pre line height restores after font DPI returns");
  g_object_set(settings,"gtk-xft-dpi",original_dpi,NULL);pump();apply(&test,"[]",TRUE);
  apply(&test,"[{\"TextUpdate\":{\"id\":1,\"markdown\":true,\"rich_text\":{\"text\":\"go é\",\"runs\":[{\"start\":0,\"end\":5,\"link\":\"../docs?q=é\",\"underline\":true}]},\"style_link\":{\"font_size\":20,\"fill\":{\"Rgba\":[1,0,1,1]},\"underline\":true}}}]",TRUE);
  gboolean handled=FALSE;g_signal_emit_by_name(text,"activate-link","../docs?q=é",&handled);
  check(&test,handled&&test.links==1&&!strcmp(test.uri,"../docs?q=é"),"native link activation delegates exact destination to host routing");
  color=attribute(text,PANGO_ATTR_FOREGROUND,0);check(&test,color&&((PangoAttrColor *)color)->color.red==65535&&((PangoAttrColor *)color)->color.blue==65535,"authored link style applies to native range");if(color)pango_attribute_destroy(color);
  apply(&test,"[{\"TextUpdate\":{\"id\":1,\"style_link\":null,\"z_index\":2}}]",TRUE);
  check(&test,integer_attribute(text,PANGO_ATTR_UNDERLINE,0)==PANGO_UNDERLINE_SINGLE,"null unchanged style_link does not remove retained link style");
  apply(&test,"[{\"TextUpdate\":{\"id\":1,\"markdown\":true,\"rich_text\":{\"text\":\"a bold c\",\"runs\":[{\"start\":0,\"end\":2,\"link\":\"/group\"},{\"start\":2,\"end\":6,\"link\":\"/group\",\"bold\":true},{\"start\":6,\"end\":8,\"link\":\"/group\"}]}}}]",TRUE);
  check(&test,!strcmp(gtk_label_get_label(GTK_LABEL(text)),"<a href=\"/group\">a bold c</a>"),"adjacent styled runs form one native hyperlink and focus stop");
  apply(&test,"[{\"TextUpdate\":{\"id\":1,\"markdown\":true,\"rich_text\":{\"text\":\"one boldtwothree\",\"runs\":[{\"start\":0,\"end\":4,\"link\":\"/x\",\"link_id\":1},{\"start\":4,\"end\":8,\"link\":\"/x\",\"link_id\":1,\"bold\":true},{\"start\":8,\"end\":11,\"link\":\"/x\",\"link_id\":2},{\"start\":11,\"end\":16,\"link\":\"/x\",\"link_id\":3}]}}}]",TRUE);
  check(&test,!strcmp(gtk_label_get_label(GTK_LABEL(text)),"<a href=\"/x\">one bold</a><a href=\"/x\">two</a><a href=\"/x\">three</a>"),"authored link identities retain distinct adjacent links with matching destinations");
  apply(&test,"[{\"TextUpdate\":{\"id\":1,\"editable\":true}}]",FALSE);
  apply(&test,"[{\"TextUpdate\":{\"id\":1,\"rich_text\":{\"text\":\"é\",\"runs\":[{\"start\":0,\"end\":1},{\"start\":1,\"end\":2}]}}}]",FALSE);
  apply(&test,"[{\"TextUpdate\":{\"id\":1,\"markdown\":false,\"content\":\"<plain>&\",\"rich_text\":null}}]",TRUE);
  check(&test,label(scene)==text&&!gtk_label_get_use_markup(GTK_LABEL(text))&&!strcmp(gtk_label_get_text(GTK_LABEL(text)),"<plain>&"),"plain transition retains widget and removes markup interpretation");
  check(&test,integer_attribute(text,PANGO_ATTR_UNDERLINE,0)==-1,"plain transition clears inline underline");
  apply(&test,"[{\"TextUpdate\":{\"id\":1,\"content\":\"Edit\",\"markdown\":false,\"rich_text\":null,\"editable\":true,\"selectable\":true,\"wrap\":true,\"style\":{\"font_size\":17,\"align_horizontal\":\"Right\",\"align_vertical\":\"Bottom\",\"align_multiline\":\"Right\"}}}]",TRUE);
  apply(&test,"[{\"TextUpdate\":{\"id\":1,\"content\":\"**Read**\",\"markdown\":true,\"editable\":false,\"rich_text\":{\"text\":\"Read\",\"runs\":[{\"start\":0,\"end\":4,\"bold\":true}]}}}]",TRUE);
  text=label(scene);
  check(&test,text&&gtk_label_get_selectable(GTK_LABEL(text))&&gtk_label_get_wrap(GTK_LABEL(text)),"editable plain to readonly rich restores retained selectable and wrapping properties");
  check(&test,text&&gtk_label_get_xalign(GTK_LABEL(text))==1&&gtk_label_get_yalign(GTK_LABEL(text))==1&&gtk_label_get_justify(GTK_LABEL(text))==GTK_JUSTIFY_RIGHT,"replaced rich label restores retained horizontal vertical and multiline alignment");
  check(&test,text&&integer_attribute(text,PANGO_ATTR_WEIGHT,0)==PANGO_WEIGHT_BOLD&&integer_attribute(text,PANGO_ATTR_SIZE,0)==17*3*PANGO_SCALE/4,"replaced rich label combines retained base font and new inline attributes");
  apply(&test,"[{\"TextDelete\":1},{\"TextCreate\":{\"id\":1}},{\"TextUpdate\":{\"id\":1,\"content\":\"Fresh\"}}]",TRUE);
  check(&test,!strcmp(gtk_label_get_text(GTK_LABEL(label(scene))),"Fresh"),"same-ID recreation has no stale rich content or attributes");
done:
  pax_native_scene_stop(test.scene);gtk_window_destroy(GTK_WINDOW(window));g_object_unref(window);g_free(test.uri);
  g_print("%u native rich-text checks, failed=%d\n",test.checks,test.failed);return test.failed?1:0;
}
