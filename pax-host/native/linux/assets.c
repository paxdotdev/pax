#include "assets.h"
#include <fontconfig/fontconfig.h>
#include <gio/gio.h>
#include <pango/pangocairo.h>
#include <string.h>
char *pax_linux_asset_path(char *const *roots, const char *path,
                           GError **error) {
  if (!path || !*path) {
    g_set_error(error, G_IO_ERROR, G_IO_ERROR_INVALID_ARGUMENT,
                "Empty Pax asset path");
    return NULL;
  }
  if (g_path_is_absolute(path) && g_file_test(path, G_FILE_TEST_IS_REGULAR))
    return g_strdup(path);
  if (g_str_has_prefix(path, "file:")) {
    char *filename = g_filename_from_uri(path, NULL, error);
    if (filename && g_file_test(filename, G_FILE_TEST_IS_REGULAR))
      return filename;
    if (filename)
      g_set_error(error, G_IO_ERROR, G_IO_ERROR_NOT_FOUND,
                  "Pax file asset does not exist: %s", filename);
    g_free(filename);
    return NULL;
  }
  if (strstr(path, "://")) {
    g_set_error(error, G_IO_ERROR, G_IO_ERROR_NOT_SUPPORTED,
                "Linux native assets currently require a packaged file, not "
                "remote URL %s",
                path);
    return NULL;
  }
  for (guint i = 0; roots && roots[i]; i++) {
    char *candidate = g_build_filename(roots[i], path, NULL);
    if (g_file_test(candidate, G_FILE_TEST_IS_REGULAR))
      return candidate;
    g_free(candidate);
    if (g_str_has_prefix(path, "assets/")) {
      candidate = g_build_filename(roots[i], path + 7, NULL);
      if (g_file_test(candidate, G_FILE_TEST_IS_REGULAR))
        return candidate;
      g_free(candidate);
    }
  }
  g_set_error(error, G_IO_ERROR, G_IO_ERROR_NOT_FOUND,
              "Pax asset is missing from configured roots: %s", path);
  return NULL;
}
gboolean pax_linux_font_load(char *const *roots, const char *path,
                             GError **error) {
  if (!path || !*path)
    return TRUE;
  char *filename = pax_linux_asset_path(roots, path, error);
  if (!filename)
    return FALSE;
  static GHashTable *registered;
  if (!registered)
    registered = g_hash_table_new_full(g_str_hash, g_str_equal, g_free, NULL);
  if (g_hash_table_contains(registered, filename)) {
    g_free(filename);
    return TRUE;
  }
  if (!FcConfigAppFontAddFile(FcConfigGetCurrent(),
                              (const FcChar8 *)filename)) {
    g_set_error(error, G_IO_ERROR, G_IO_ERROR_FAILED,
                "Fontconfig rejected packaged Pax font: %s", filename);
    g_free(filename);
    return FALSE;
  }
  g_hash_table_add(registered, filename);
  pango_font_map_changed(pango_cairo_font_map_get_default());
  return TRUE;
}
