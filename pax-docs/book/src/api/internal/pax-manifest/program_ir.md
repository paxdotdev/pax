# program_ir
<!-- summary: API docs for pax-manifest::program_ir. -->
<!-- tags: api, pax-manifest -->

## Structs
### `ProgramIR`
Runtime-facing semantic program model derived from a rich `PaxManifest`.

`ProgramIR` is a transitional intermediate representation: it preserves the
semantic program structure while stripping obviously source-only and
compiler-only manifest data. Debug/designtime flows may continue to mount
rich manifests directly until later phases cut execution over to `ProgramIR`.

#### Properties
##### `components`
Type: `BTreeMap`<[`TypeId`](../../../api/internal/pax-manifest/index.md#typeid), `ProgramComponent`>

##### `main_component_type_id`
Type: [`TypeId`](../../../api/internal/pax-manifest/index.md#typeid)

##### `type_table`
Type: `BTreeMap`<[`TypeId`](../../../api/internal/pax-manifest/index.md#typeid), [`TypeDefinition`](../../../api/internal/pax-manifest/index.md#typedefinition)>

##### `assets_dirs`
Type: `Vec`<`String`>
