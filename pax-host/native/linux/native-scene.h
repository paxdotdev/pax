#ifndef PAX_LINUX_NATIVE_SCENE_H
#define PAX_LINUX_NATIVE_SCENE_H
#include <gtk/gtk.h>
#include <json-glib/json-glib.h>
typedef struct _PaxNativeScene PaxNativeScene;
typedef gboolean (*PaxSceneInterrupt)(gpointer, const char *, gsize);
typedef void (*PaxSceneError)(gpointer, const char *);
GtkWidget *pax_native_scene_new(PaxSceneInterrupt interrupt,
                                PaxSceneError error, gpointer data);
gboolean pax_native_scene_apply(PaxNativeScene *scene, JsonArray *messages,
                                GError **error);
void pax_native_scene_assets(PaxNativeScene *scene, char *const *roots);
guint pax_native_scene_layer_count(PaxNativeScene *scene);
void pax_native_scene_layer_owner(PaxNativeScene *scene, guint layer,
                                  gint64 owner);
void pax_native_scene_texture(PaxNativeScene *scene, JsonObject *surface,
                              GdkTexture *texture);
void pax_native_scene_prune_tiles(PaxNativeScene *scene, JsonArray *surfaces);
void pax_native_scene_stop(PaxNativeScene *scene);
/* Opt-in test/introspection data. Does not synthesize input or read GPU pixels.
 */
JsonObject *pax_native_scene_report(PaxNativeScene *scene);

#endif
