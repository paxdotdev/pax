# cartridge_generation
<!-- summary: API docs for pax-manifest::cartridge_generation. -->
<!-- tags: api, pax-manifest -->

## Structs
### `CommonProperty`
Common property metadata passed into cartridge codegen templates.

---

### `ComponentInfo`
Template context for generating a component's cartridge code.

#### Properties
##### `type_id`
Type: [`TypeId`](/api/internal/pax-manifest/index.md#typeid)

##### `pascal_identifier`
Type: `String`

##### `primitive_instance_import_path`
Type: `Option`<`String`>

##### `properties`
Type: `Vec`<[`PropertyInfo`](/api/internal/pax-manifest/cartridge_generation.md#propertyinfo)>

##### `handlers`
Type: `Vec`<[`HandlerInfo`](/api/internal/pax-manifest/cartridge_generation.md#handlerinfo)>

---

### `HandlerInfo`
Event handler entry passed into cartridge codegen templates.

#### Properties
##### `name`
Type: `String`

##### `args_type`
Type: `Option`<`String`>

---

### `PropertyInfo`
Property entry passed into cartridge codegen templates.

#### Properties
##### `name`
Type: `String`

##### `property_type`
Type: [`PropertyDefinition`](/api/internal/pax-manifest/index.md#propertydefinition)
