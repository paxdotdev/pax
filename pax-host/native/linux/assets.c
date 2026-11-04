#include "assets.h"
#include <fontconfig/fontconfig.h>
#include <gio/gio.h>
#include <pango/pangocairo.h>
#include <pango/pangofc-fontmap.h>
#include <string.h>
char *pax_linux_asset_path(char *const *roots, const char *path,
                           GError **error) {
  if (!path || !*path) {
    g_set_error(error, G_IO_ERROR, G_IO_ERROR_INVALID_ARGUMENT,
                "Empty Pax asset path");
    return NULL;
  }
  // Web-authored components use /assets/ for the same bundled asset root.
  // Normalize it before absolute-file handling for images and native fonts alike.
  if (g_str_has_prefix(path, "/assets/")) path++;
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
typedef struct {
  FcConfig *config;
  GHashTable *files;
} RegisteredFonts;
static GPtrArray *font_configs, *font_maps;
static void font_map_free(gpointer data) {
  GWeakRef *reference = data;
  g_weak_ref_clear(reference);
  g_free(reference);
}
static FcConfig *font_config(PangoFontMap *map) {
  FcConfig *config = pango_fc_font_map_get_config(PANGO_FC_FONT_MAP(map));
  return config ? config : FcConfigGetCurrent();
}
static void remember_font_map(PangoFontMap *map) {
  if (!font_maps)
    font_maps = g_ptr_array_new_with_free_func(font_map_free);
  for (guint i = 0; i < font_maps->len;) {
    PangoFontMap *known = g_weak_ref_get(g_ptr_array_index(font_maps, i));
    if (!known) {
      g_ptr_array_remove_index(font_maps, i);
      continue;
    }
    gboolean same = known == map;
    g_clear_object(&known);
    if (same)
      return;
    i++;
  }
  GWeakRef *reference = g_new0(GWeakRef, 1);
  g_weak_ref_init(reference, map);
  g_ptr_array_add(font_maps, reference);
  // A newly encountered map may have cached the family before another map
  // registered its faces in their shared Fontconfig configuration.
  pango_fc_font_map_config_changed(PANGO_FC_FONT_MAP(map));
}
gboolean pax_linux_font_load(char *const *roots, const char *path,
                             PangoFontMap *map, GError **error) {
  if (!path || !*path)
    return TRUE;
  if (!PANGO_IS_FC_FONT_MAP(map)) {
    g_set_error(error, G_IO_ERROR, G_IO_ERROR_NOT_SUPPORTED,
                "Packaged Linux fonts require a PangoFc font map");
    return FALSE;
  }
  char *filename = pax_linux_asset_path(roots, path, error);
  if (!filename)
    return FALSE;
  remember_font_map(map);
  PangoFontMap *default_map = pango_cairo_font_map_get_default();
  if (PANGO_IS_FC_FONT_MAP(default_map))
    remember_font_map(default_map);
  FcConfig *config = font_config(map);
  if (!font_configs)
    font_configs = g_ptr_array_new();
  RegisteredFonts *fonts = NULL;
  for (guint i = 0; i < font_configs->len; i++) {
    RegisteredFonts *candidate = g_ptr_array_index(font_configs, i);
    if (candidate->config == config) {
      fonts = candidate;
      break;
    }
  }
  if (!fonts) {
    fonts = g_new0(RegisteredFonts, 1);
    // Registrations have process lifetime. Retaining the configuration prevents
    // pointer reuse from making a later configuration look already populated.
    fonts->config = FcConfigReference(config);
    fonts->files = g_hash_table_new_full(g_str_hash, g_str_equal, g_free, NULL);
    g_ptr_array_add(font_configs, fonts);
  }
  if (g_hash_table_contains(fonts->files, filename)) {
    g_free(filename);
    return TRUE;
  }
  if (!FcConfigAppFontAddFile(config, (const FcChar8 *)filename)) {
    g_set_error(error, G_IO_ERROR, G_IO_ERROR_FAILED,
                "Fontconfig rejected packaged Pax font: %s", filename);
    g_free(filename);
    return FALSE;
  }
  g_hash_table_add(fonts->files, filename);
  // This is a Fontconfig mutation, not a backend font-map notification.
  // Clear every known live map using this configuration, including existing
  // labels that already resolved another face of the same family.
  for (guint i = 0; i < font_maps->len;) {
    PangoFontMap *known = g_weak_ref_get(g_ptr_array_index(font_maps, i));
    if (!known) {
      g_ptr_array_remove_index(font_maps, i);
      continue;
    }
    if (font_config(known) == config)
      pango_fc_font_map_config_changed(PANGO_FC_FONT_MAP(known));
    g_object_unref(known);
    i++;
  }
  return TRUE;
}
