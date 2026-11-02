#pragma once
#include <gio/gio.h>
#include <json-glib/json-glib.h>
#include <string.h>

// Match the established native virtual-route contract: the decoded path is
// split into nonempty segments, '+' remains literal in queries, and the next
// relative base is reconstructed from those segments (without a trailing '/').
static inline char *pax_route_decode(const char *encoded, GError **error) {
  char *decoded = g_uri_unescape_string(encoded, NULL);
  if (!decoded || !g_utf8_validate(decoded, -1, NULL)) {
    g_free(decoded);
    g_set_error_literal(error, G_URI_ERROR, G_URI_ERROR_FAILED,
                        "Route contains invalid UTF-8 or percent encoding");
    return NULL;
  }
  return decoded;
}
static inline gboolean pax_route_is_relative(const char *destination) {
  return !g_uri_peek_scheme(destination) && !g_str_has_prefix(destination, "//");
}
static inline JsonObject *pax_route_resolve(const char *current_uri,
                                            const char *destination,
                                            char **next_uri, GError **error) {
  *next_uri = NULL;
  if (!*destination || !pax_route_is_relative(destination)) return NULL;
  GUri *base = g_uri_parse(current_uri ? current_uri : "pax://app/", G_URI_FLAGS_ENCODED, error);
  if (!base) return NULL;
  GUri *resolved = g_uri_parse_relative(base, destination, G_URI_FLAGS_ENCODED, error);
  g_uri_unref(base);
  if (!resolved) return NULL;
  if (g_strcmp0(g_uri_get_scheme(resolved), "pax") ||
      g_strcmp0(g_uri_get_host(resolved), "app")) {
    g_uri_unref(resolved);
    return NULL;
  }
  JsonObject *route = json_object_new(), *query = json_object_new();
  JsonArray *segments = json_array_new();
  json_object_set_array_member(route, "path_segments", segments);
  json_object_set_object_member(route, "query", query);
  char *path = pax_route_decode(g_uri_get_path(resolved), error);
  GString *normalized = g_string_new("");
  if (!path) goto failed;
  char **parts = g_strsplit(path, "/", -1);
  for (guint i=0; parts[i]; i++) if (*parts[i]) {
    json_array_add_string_element(segments, parts[i]);
    g_string_append_c(normalized, '/'); g_string_append(normalized, parts[i]);
  }
  g_strfreev(parts); g_free(path);
  if (!normalized->len) g_string_append_c(normalized, '/');
  const char *encoded_query = g_uri_get_query(resolved);
  if (encoded_query && *encoded_query) {
    char **items = g_strsplit(encoded_query, "&", -1);
    for (guint i=0; items[i]; i++) {
      char *separator = strchr(items[i], '=');
      if (separator) *separator++ = '\0';
      char *key = pax_route_decode(items[i], error);
      char *value = key ? pax_route_decode(separator ? separator : "", error) : NULL;
      if (!key || !value) {
        g_free(key); g_free(value); g_strfreev(items); goto failed;
      }
      JsonArray *values;
      if (json_object_has_member(query, key)) values = json_object_get_array_member(query,key);
      else { values = json_array_new(); json_object_set_array_member(query,key,values); }
      json_array_add_string_element(values,value); g_free(key); g_free(value);
    }
    g_strfreev(items);
  }
  const char *encoded_fragment = g_uri_get_fragment(resolved);
  if (encoded_fragment) {
    char *fragment = pax_route_decode(encoded_fragment,error);
    if (!fragment) goto failed;
    json_object_set_string_member(route,"fragment",fragment); g_free(fragment);
  } else json_object_set_null_member(route,"fragment");
  *next_uri = g_uri_join(G_URI_FLAGS_ENCODED_QUERY | G_URI_FLAGS_ENCODED_FRAGMENT,
                         "pax",NULL,"app",-1,normalized->str,encoded_query,encoded_fragment);
  g_string_free(normalized,TRUE); g_uri_unref(resolved);
  return route;
failed:
  g_string_free(normalized,TRUE); g_uri_unref(resolved); json_object_unref(route);
  return NULL;
}
