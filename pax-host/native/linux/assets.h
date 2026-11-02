#ifndef PAX_LINUX_ASSETS_H
#define PAX_LINUX_ASSETS_H
#include <glib.h>
#include <pango/pango.h>
char *pax_linux_asset_path(char *const *roots, const char *path,
                           GError **error);
gboolean pax_linux_font_load(char *const *roots, const char *path,
                             PangoFontMap *map, GError **error);
#endif
