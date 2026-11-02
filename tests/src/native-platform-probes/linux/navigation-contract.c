/* Standalone shared native route vectors, no display or external URI launch.
 * cc -std=c11 -Wall -Wextra -Werror -Ipax-host/native/linux \
 *   tests/src/native-platform-probes/linux/navigation-contract.c \
 *   $(pkg-config --cflags --libs gio-2.0 json-glib-1.0) -o /tmp/pax-navigation-contract
 * /tmp/pax-navigation-contract tests/src/native-platform-probes/navigation-cases.json
 */
#include "navigation.h"
#include <stdio.h>
static guint checks, failures;
static void check(gboolean value, const char *name) {
  checks++; failures += !value; printf("%s %s\n",value?"PASS":"FAIL",name);
}
static void step(char **base, JsonObject *item) {
  GError *error = NULL; char *next = NULL;
  const char *url=json_object_get_string_member(item,"url");
  JsonObject *route=pax_route_resolve(*base,url,&next,&error);
  JsonNode *actual=json_node_new(JSON_NODE_OBJECT);
  if(route) json_node_take_object(actual,route);
  else json_node_init_null(actual);
  gboolean match=route && json_node_equal(actual,json_object_get_member(item,"expected"));
  check(match,url);
  if(!match) {
    char *text=json_to_string(actual,FALSE);
    printf("actual %s error %s\n",text,error?error->message:"");g_free(text);
  }
  if(next){g_free(*base);*base=next;}
  g_clear_error(&error);json_node_free(actual);
}
int main(int argc,char **argv) {
  if(argc!=2)return 2;
  JsonParser *parser=json_parser_new();GError *error=NULL;
  if(!json_parser_load_from_file(parser,argv[1],&error)){fprintf(stderr,"%s\n",error->message);return 2;}
  JsonObject *root=json_node_get_object(json_parser_get_root(parser));
  JsonArray *steps=json_object_get_array_member(root,"fixture_steps");char *base=NULL;
  for(guint i=0;i<json_array_get_length(steps);i++)step(&base,json_array_get_object_element(steps,i));
  g_free(base);
  steps=json_object_get_array_member(root,"native_edge_cases");
  for(guint i=0;i<json_array_get_length(steps);i++){
    JsonObject *item=json_array_get_object_element(steps,i);base=NULL;
    JsonObject *initial=pax_route_resolve(NULL,json_object_get_string_member(item,"initial"),&base,&error);
    check(initial!=NULL,"initial route normalizes before relative navigation");
    if(initial)json_object_unref(initial);
    step(&base,item);g_free(base);
  }
  steps=json_object_get_array_member(root,"non_virtual_destinations");
  for(guint i=0;i<json_array_get_length(steps);i++){
    const char *url=json_array_get_string_element(steps,i);base=NULL;
    JsonObject *route=pax_route_resolve(NULL,url,&base,&error);
    check(!route && !base && !error,"explicit schemes and authorities do not mutate virtual route");
    if(route) json_object_unref(route);
    g_free(base);g_clear_error(&error);
  }
  base=NULL;
  JsonObject *invalid=pax_route_resolve(NULL,"/bad%00value",&base,&error);
  check(!invalid && !base && error,"invalid encoded NUL is rejected without changing route");
  if(invalid) json_object_unref(invalid);
  g_free(base);g_clear_error(&error);
  g_object_unref(parser);printf("%s %u native navigation checks\n",failures?"FAIL":"PASS",checks);
  return failures?1:0;
}
