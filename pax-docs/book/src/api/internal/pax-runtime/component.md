# component
<!-- summary: API docs for pax-runtime::component. -->
<!-- tags: api, pax-runtime -->

## Structs
### `ComponentInstance`
A render node with its own runtime context.  Will push a frame
to the runtime stack including the specified `slot_children` and
a `PaxType` properties object.  `Component` is used at the root of
applications, at the root of reusable components like `Stacker`, and
in special applications like `Repeat` where it houses the `RepeatItem`
properties attached to each of Repeat's virtual nodes.

#### Properties
##### `template`
Type: [`InstanceNodePtrList`](/api/internal/pax-runtime/rendering.md#instancenodeptrlist)

##### `timelines`
Type: `Vec`<`Rc`<`RefCell`<[`Timeline`](/api/pax-runtime-api/animation.md#timeline)>>>

---

### `ScrollPosition`
Built-in `$scroll_position` value synthesized for components inside scrollers.

#### Properties
##### `x`
Type: `f64`

##### `y`
Type: `f64`

#### Implementations
##### `create_builtin_if_exists`
<pre><code class="api-signature language-rust ignore">pub fn create_builtin_if_exists(property_scope: Ref&lt;&#39;_, HashMap&lt;String, <a href="/api/pax-runtime-api/variables.md#variable">Variable</a>&gt;&gt;) -&gt; Option&lt;HashMap&lt;String, <a href="/api/pax-runtime-api/variables.md#variable">Variable</a>&gt;&gt;</code></pre>

Create a stack frame containing `$scroll_position` when scroll properties are present.
