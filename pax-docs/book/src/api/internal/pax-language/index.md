# pax-language
<!-- summary: API reference for pax-language. -->
<!-- tags: api, pax-language -->

## Submodules
- [formatting](formatting.md)
- [deserializer](deserializer.md)
- [interpreter](interpreter.md)
- [helpers](helpers.md)

## Structs
### `PaxParser`
Pest parser generated from the Pax grammar.

## Enums
### `Rule`
#### Variants
##### `EOI`
End-of-input

##### `WHITESPACE`
##### `comment`
##### `pax_component_definition`
/// ////// //////
BEGIN TEMPLATE
///

##### `root_tag_pair`
##### `any_tag_pair`
##### `open_tag`
##### `closing_tag`
##### `self_closing_tag`
##### `matched_tag`
##### `inner_nodes`
##### `identifier`
##### `pascal_identifier`
##### `event_id`
##### `attribute_key_value_pair`
##### `class_attribute`
##### `class_value`
##### `class_string_list`
##### `attribute_transition_binding`
##### `attribute_event_binding`
##### `transition_id`
##### `transition_binding_value`
##### `transition_inline_timeline_value`
##### `transition_inline_timeline_body`
##### `double_binding`
##### `any_template_value`
##### `node_inner_content`
##### `string`
##### `inner`
##### `char`
##### `settings_block_declaration`
/// ////// //////
BEGIN SETTINGS
///

##### `settings_block_element`
##### `settings_conditional`
##### `settings_if_branch`
##### `settings_else_if_branch`
##### `settings_else_branch`
##### `settings_conditional_body`
##### `selector_block`
##### `literal_object`
##### `selector`
##### `settings_key_value_pair`
##### `settings_event_binding`
##### `settings_key`
##### `settings_value`
##### `timeline_block_declaration`
/// ////// //////
BEGIN TIMELINES
///

##### `timeline_block_setting`
##### `timeline_block_setting_value`
##### `timeline_selector_block`
##### `timeline_selector_body`
##### `timeline_property_key_value_pair`
##### `timeline_inline_value`
##### `timeline_track`
##### `timeline_keyframe`
##### `timeline_keyframe_value`
##### `timeline_marker`
##### `timeline_percent`
##### `timeline_duration`
##### `timeline_duration_unit`
##### `timeline_easing_curve`
##### `timeline_target`
##### `timeline_local_target`
##### `timeline_symbol`
##### `gradient_inline_value`
/// ////// //////
BEGIN GRADIENTS
///

##### `gradient_body`
##### `gradient_shape_block`
##### `gradient_shape_key`
##### `gradient_shape_settings`
##### `gradient_shape_setting`
##### `gradient_shape_setting_value`
##### `gradient_stop`
##### `gradient_stop_marker`
##### `gradient_stop_value`
##### `literal_function`
##### `silent_comma`
##### `function_list`
##### `literal_value`
##### `literal_boolean`
##### `literal_some`
##### `literal_none`
##### `literal_option`
##### `literal_number_with_unit`
##### `literal_number`
##### `literal_number_integer`
##### `literal_number_float`
##### `literal_number_unit`
##### `literal_tuple`
##### `literal_tuple_access`
##### `literal_list`
##### `literal_list_access`
##### `literal_enum_value`
##### `literal_enum_args_list`
##### `literal_color`
/// ////// //////
BEGIN COLORS
///

##### `literal_color_space_func`
##### `literal_color_channel`
##### `xo_color_space_func`
##### `literal_color_const`
##### `expression_body`
/// ////// //////
BEGIN EXPRESSIONS
This sub-grammar describes PAXEL, the Pax Expression Language
///

##### `expression_ternary`
##### `expression_coalesce`
##### `expression_binary`
##### `expression_wrapped`
##### `expression_grouped`
##### `expression_grouped_unit`
##### `xo_primary`
##### `xo_prefix`
##### `xo_neg`
##### `xo_bool_not`
##### `xo_infix`
##### `xo_add`
##### `xo_bool_and`
##### `xo_bool_or`
##### `xo_div`
##### `xo_exp`
##### `xo_mod`
##### `xo_mul`
##### `xo_rel_eq`
##### `xo_rel_gt`
##### `xo_rel_gte`
##### `xo_rel_lt`
##### `xo_rel_lte`
##### `xo_rel_neq`
##### `xo_sub`
##### `xo_null_coalesce`
##### `xo_tern_then`
##### `xo_tern_else`
##### `xo_range`
##### `xo_range_exclusive`
##### `xo_literal`
##### `xo_object`
##### `xo_object_settings_key_value_pair`
##### `xo_symbol`
##### `xo_tuple`
##### `xo_list`
##### `xo_enum_or_function_call`
##### `xo_enum_or_function_args_list`
##### `statement_control_flow`
/// ////// //////
BEGIN CONTROL FLOW
///

##### `statement_if`
##### `statement_if_branch`
##### `statement_else_if_branch`
##### `statement_else_branch`
##### `statement_for`
##### `statement_slot`
##### `statement_for_predicate_declaration`
##### `statement_for_source`
##### `statement_for_key`
## Functions
### `parse_pax_err`
<pre><code class="api-signature language-rust ignore">pub fn parse_pax_err(expected_rule: <a href="/api/internal/pax-language/index.md#rule">Rule</a>, input: &amp;str) -&gt; Result&lt;Pair&lt;&#39;_, <a href="/api/internal/pax-language/index.md#rule">Rule</a>&gt;, <a href="/api/internal/pax-language/deserializer/error.md#error">Error</a>&lt;<a href="/api/internal/pax-language/index.md#rule">Rule</a>&gt;&gt;</code></pre>

Parse a string against a Pax grammar rule, preserving the structured pest error.

---

### `parse_pax_pairs`
<pre><code class="api-signature language-rust ignore">pub fn parse_pax_pairs(expected_rule: <a href="/api/internal/pax-language/index.md#rule">Rule</a>, input: &amp;str) -&gt; Result&lt;Pairs&lt;&#39;_, <a href="/api/internal/pax-language/index.md#rule">Rule</a>&gt;, <a href="/api/internal/pax-language/deserializer/error.md#error">Error</a>&lt;<a href="/api/internal/pax-language/index.md#rule">Rule</a>&gt;&gt;</code></pre>

Parse a string into pest pairs for a Pax grammar rule.

---

### `parse_pax_str`
<pre><code class="api-signature language-rust ignore">pub fn parse_pax_str(expected_rule: <a href="/api/internal/pax-language/index.md#rule">Rule</a>, input: &amp;str) -&gt; Result&lt;Pair&lt;&#39;_, <a href="/api/internal/pax-language/index.md#rule">Rule</a>&gt;, String&gt;</code></pre>

Parse a string against a single Pax grammar rule, returning a human-readable error string.
