#ifndef PAX_LINUX_NATIVE_SCENE_H
#define PAX_LINUX_NATIVE_SCENE_H
#include <gtk/gtk.h>
#include <json-glib/json-glib.h>
typedef struct _PaxNativeScene PaxNativeScene;
typedef gboolean (*PaxSceneInterrupt)(gpointer, const char *, gsize);
typedef void (*PaxSceneError)(gpointer, const char *);
typedef void (*PaxSceneNavigate)(gpointer, const char *);
GtkWidget *pax_native_scene_new(PaxSceneInterrupt interrupt,
                                PaxSceneError error, gpointer data);
gboolean pax_native_scene_apply(PaxNativeScene *scene, JsonArray *messages,
                                GError **error);
/* One synchronous, non-nested commit. Do not dispatch the GTK main context
 * between begin/end. FALSE abandons deferred placement after a caller error;
 * already-applied property updates are not rolled back. Without a commit,
 * apply/texture continue to place immediately. */
void pax_native_scene_begin_commit(PaxNativeScene *scene);
void pax_native_scene_end_commit(PaxNativeScene *scene, gboolean publish);
void pax_native_scene_assets(PaxNativeScene *scene, char *const *roots);
void pax_native_scene_navigation(PaxNativeScene *scene, PaxSceneNavigate navigate);
guint pax_native_scene_layer_count(PaxNativeScene *scene);
void pax_native_scene_layer_owner(PaxNativeScene *scene, guint layer,
                                  gint64 owner);
void pax_native_scene_texture(PaxNativeScene *scene, JsonObject *surface,
                              GdkTexture *texture);
void pax_native_scene_prune_tiles(PaxNativeScene *scene, JsonArray *surfaces);
void pax_native_scene_stop(PaxNativeScene *scene);
void pax_native_scene_release_keys(PaxNativeScene *scene);
/* Opt-in test/introspection data. Does not synthesize input or read GPU pixels.
 */
JsonObject *pax_native_scene_report(PaxNativeScene *scene);

#endif
