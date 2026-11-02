/* Real X11 pointer selection/focus regression for large native labels in nested
 * Pax scrollers. Build like text-constraints.c; run with GDK_BACKEND=x11 and
 * xdotool/timeout on PATH. This qualifies mouse input, not touch gestures.
 */
#include "native-scene.h"
#include <gdk/x11/gdkx.h>
#include <math.h>
#include <string.h>

typedef struct { PaxNativeScene *scene; guint checks; gboolean failed; } Test;
typedef struct { gboolean done; gint status; } Input;
static void check(Test *test, gboolean ok, const char *name) {
  test->checks++; test->failed |= !ok; g_print("%s %s\n", ok ? "PASS" : "FAIL", name);
}
static void pump(guint milliseconds) {
  gint64 end=g_get_monotonic_time()+milliseconds*1000;
  do {
    for(guint i=0;i<32&&g_main_context_pending(NULL);i++)g_main_context_iteration(NULL,FALSE);
    g_usleep(1000);
  } while(g_get_monotonic_time()<end);
}
static gboolean apply(Test *test, const char *source) {
  JsonParser *parser=json_parser_new();GError *error=NULL;
  gboolean ok=json_parser_load_from_data(parser,source,-1,&error);
  if(ok)ok=pax_native_scene_apply(test->scene,json_node_get_array(json_parser_get_root(parser)),&error);
  check(test,ok,"native fixture patch accepted");
  if(error){g_printerr("%s\n",error->message);g_error_free(error);}
  g_object_unref(parser);return ok;
}
static GtkWidget *find(GtkWidget *root,GType type) {
  if(g_type_is_a(G_OBJECT_TYPE(root),type))return root;
  for(GtkWidget *child=gtk_widget_get_first_child(root);child;child=gtk_widget_get_next_sibling(child)){
    GtkWidget *found=find(child,type);if(found)return found;
  }
  return NULL;
}
static void input_done(GPid pid,gint status,gpointer data) {
  Input *input=data;input->status=status;input->done=TRUE;g_spawn_close_pid(pid);
}
static gboolean input(Test *test,const char *command) {
  GError *error=NULL;gchar **argv=NULL;GPid pid;Input result={0};
  gboolean ok=g_shell_parse_argv(command,NULL,&argv,&error);
  if(ok)ok=g_spawn_async(NULL,argv,NULL,G_SPAWN_SEARCH_PATH|G_SPAWN_DO_NOT_REAP_CHILD,NULL,NULL,&pid,&error);
  if(ok){g_child_watch_add(pid,input_done,&result);while(!result.done)pump(5);ok=g_spawn_check_wait_status(result.status,&error);}
  check(test,ok,"real X11 input sequence completed");
  if(error){g_printerr("%s\n",error->message);g_error_free(error);}
  g_strfreev(argv);pump(400);return ok;
}
static gboolean at_origin(GtkScrolledWindow *outer,GtkScrolledWindow *inner) {
  double x=gtk_adjustment_get_value(gtk_scrolled_window_get_hadjustment(inner));
  double y=gtk_adjustment_get_value(gtk_scrolled_window_get_vadjustment(inner));
  double oy=gtk_adjustment_get_value(gtk_scrolled_window_get_vadjustment(outer));
  g_print("scroll inner_x=%.3f inner_y=%.3f outer_y=%.3f\n",x,y,oy);
  return fabs(x)<.1&&fabs(y)<.1&&fabs(oy)<.1;
}
static void drag(Test *test,GtkWidget *window,GtkWidget *label,int start,int end) {
  graphene_point_t local=GRAPHENE_POINT_INIT(start,10),point;
  if(!gtk_widget_compute_point(label,window,&local,&point)){check(test,FALSE,"label has native window coordinates");return;}
  int scale=gtk_widget_get_scale_factor(window);
  Window xid=gdk_x11_surface_get_xid(gtk_native_get_surface(GTK_NATIVE(window)));
  char *command=g_strdup_printf("timeout 5s xdotool windowfocus --sync %lu mousemove --sync --window %lu %d %d mousedown 1 sleep 0.08 mousemove --sync --window %lu %d %d sleep 0.08 mouseup 1",
      (unsigned long)xid,(unsigned long)xid,(int)round(point.x*scale),(int)round(point.y*scale),
      (unsigned long)xid,(int)round((point.x+end-start)*scale),(int)round(point.y*scale));
  input(test,command);g_free(command);
}
int main(void) {
  gtk_init();Test test={0};GtkWidget *window=gtk_window_new();g_object_ref_sink(window);
  GtkWidget *scene=pax_native_scene_new(NULL,NULL,&test);test.scene=(PaxNativeScene *)scene;
  gtk_window_set_title(GTK_WINDOW(window),"Pax native label pointer focus");
  gtk_window_set_default_size(GTK_WINDOW(window),600,400);
  gtk_window_set_child(GTK_WINDOW(window),scene);gtk_window_present(GTK_WINDOW(window));
  if(!GDK_IS_X11_DISPLAY(gtk_widget_get_display(window))){check(&test,FALSE,"probe requires real X11 input backend");goto done;}
  if(!apply(&test,"[{\"ScrollerCreate\":{\"id\":10}},{\"ScrollerUpdate\":{\"id\":10,\"size_x\":360,\"size_y\":250,\"size_inner_pane_x\":1100,\"size_inner_pane_y\":1900,\"transform\":[1,0,0,1,10,10]}},{\"ScrollerCreate\":{\"id\":20,\"parent_frame\":10}},{\"ScrollerUpdate\":{\"id\":20,\"size_x\":260,\"size_y\":160,\"size_inner_pane_x\":920,\"size_inner_pane_y\":1700,\"transform\":[1,0,0,1,30,30]}},{\"TextCreate\":{\"id\":30,\"parent_frame\":20}},{\"TextUpdate\":{\"id\":30,\"content\":\"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789\\nsecond line\\nthird line\",\"selectable\":true,\"wrap\":false,\"size_x\":900,\"size_y\":1650,\"transform\":[1,0,0,1,40,40],\"style\":{\"font_size\":16,\"align_horizontal\":\"Left\",\"align_vertical\":\"Top\"}}},{\"ButtonCreate\":{\"id\":40,\"parent_frame\":10}},{\"ButtonUpdate\":{\"id\":40,\"content\":\"Keyboard destination\",\"size_x\":180,\"size_y\":40,\"transform\":[1,0,0,1,30,900]}}]"))goto done;
  pump(600);
  GtkWidget *outer_widget=find(scene,GTK_TYPE_SCROLLED_WINDOW);
  GtkWidget *inner_widget=outer_widget?find(gtk_scrolled_window_get_child(GTK_SCROLLED_WINDOW(outer_widget)),GTK_TYPE_SCROLLED_WINDOW):NULL;
  GtkWidget *text=inner_widget?find(inner_widget,GTK_TYPE_LABEL):NULL;
  GtkWidget *button=find(scene,GTK_TYPE_BUTTON);
  check(&test,outer_widget&&inner_widget&&text&&button,"nested scrollers label and keyboard target exist");
  if(!outer_widget||!inner_widget||!text||!button)goto done;
  GtkScrolledWindow *outer=GTK_SCROLLED_WINDOW(outer_widget),*inner=GTK_SCROLLED_WINDOW(inner_widget);
  GtkWidget *outer_view=gtk_scrolled_window_get_child(outer),*inner_view=gtk_scrolled_window_get_child(inner);
  check(&test,GTK_IS_VIEWPORT(outer_view)&&GTK_IS_VIEWPORT(inner_view),"Pax fixed content has native viewport wrappers");
  gtk_widget_grab_focus(scene);pump(100);
  check(&test,at_origin(outer,inner),"fixture begins at nested viewport origins");
  drag(&test,window,text,3,160);
  int a,b;gboolean selected=gtk_label_get_selection_bounds(GTK_LABEL(text),&a,&b);
  g_print("selection first start=%d end=%d\n",a,b);
  check(&test,selected&&a>=0&&b>a&&b<36,"first native pointer drag selects only visible first-line text");
  check(&test,gtk_widget_has_focus(text)&&at_origin(outer,inner),"pointer focus preserves both scroller offsets");
  check(&test,gtk_viewport_get_scroll_to_focus(GTK_VIEWPORT(outer_view))&&gtk_viewport_get_scroll_to_focus(GTK_VIEWPORT(inner_view)),"focus reveal restores without revealing the oversized focused label");
  gtk_label_select_region(GTK_LABEL(text),0,0);
  drag(&test,window,text,8,130);
  selected=gtk_label_get_selection_bounds(GTK_LABEL(text),&a,&b);
  check(&test,selected&&b>a&&b<36&&at_origin(outer,inner),"second drag on already-focused label remains stable");
  gtk_viewport_set_scroll_to_focus(GTK_VIEWPORT(outer_view),FALSE);
  gtk_widget_grab_focus(scene);gtk_label_select_region(GTK_LABEL(text),0,0);pump(100);
  drag(&test,window,text,3,120);
  check(&test,!gtk_viewport_get_scroll_to_focus(GTK_VIEWPORT(outer_view))&&gtk_viewport_get_scroll_to_focus(GTK_VIEWPORT(inner_view))&&at_origin(outer,inner),"pointer focus preserves an ancestor's preexisting disabled reveal setting");
  gtk_viewport_set_scroll_to_focus(GTK_VIEWPORT(outer_view),TRUE);
  gtk_label_select_region(GTK_LABEL(text),0,0);
  input(&test,"timeout 5s xdotool key Tab");
  GtkWidget *focused=gtk_root_get_focus(GTK_ROOT(window));
  check(&test,focused&&(focused==button||gtk_widget_is_ancestor(focused,button)),"real Tab moves focus to the next native button");
  check(&test,gtk_adjustment_get_value(gtk_scrolled_window_get_vadjustment(outer))>500,"keyboard focus still reveals the offscreen native button");
done:
  pax_native_scene_stop(test.scene);gtk_window_destroy(GTK_WINDOW(window));g_object_unref(window);
  g_print("%u native pointer-focus checks, failed=%d\n",test.checks,test.failed);return test.failed?1:0;
}
