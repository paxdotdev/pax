# node_inspection
<!-- summary: On-demand node attribution shared by debug tools and release performance probes. -->
<!-- tags: api, pax-runtime -->

On-demand node attribution shared by debug tools and release performance probes.

## Structs
### `NodeInspection`
Identity and lifecycle of a currently cached expanded node. No tree traversal or
geometry preparation is performed; IDs are meaningful only in this engine instance.

#### Properties
##### `engine_id`
Type: `u32`

##### `node_type`
Type: `Option`<`String`>

##### `component`
Type: `Option`<`String`>

##### `template_node_id`
Type: `Option`<`usize`>

##### `source_start`
Type: `Option`<(`usize`, `usize`)>

One-based template line/column, when the parser supplied a source span.
Release programs retain spans but deliberately omit source file paths.

##### `source_end`
Type: `Option`<(`usize`, `usize`)>

##### `template_parent`
Type: `Option`<`u32`>

##### `render_parent`
Type: `Option`<`u32`>

##### `containing_component`
Type: `Option`<`u32`>

##### `render_source_owner`
Type: `Option`<`u32`>

##### `attached`
Type: `u32`

##### `has_occlusion_record`
Type: `bool`
