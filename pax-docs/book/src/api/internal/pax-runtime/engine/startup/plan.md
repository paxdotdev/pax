# engine::startup::plan
<!-- summary: Component-owned requirements for the experimental initial presentation. -->
<!-- tags: api, pax-runtime -->

Component-owned requirements for the experimental initial presentation.

## Structs
### `StartupAxes`
Axes whose exact extent is needed to place dependent content.

#### Properties
##### `x`
Type: `bool`

##### `y`
Type: `bool`

---

### `StartupDemand`
Requirements accumulate: visible content can also need an exact layout extent.
Regions use window coordinates, including inherited scrolling transforms.

#### Properties
##### `region`
Type: `Option`<`Rect`>

##### `measure`
Type: [`StartupAxes`](../../../../../api/internal/pax-runtime/engine/startup/plan.md#startupaxes)

##### `complete`
Type: `bool`

---

### `StartupPlan`
Describes child work without mounting nodes or retaining borrows across yields.
Implement `InstanceNode::plan_startup`, or register a component-local resolver
through `NodeContext::set_startup_planner`. The scheduler owns continuation,
lifetime checks, native feedback, and the presentation gate.

A complete source requirement must be attached to the source, not its owner:
doing so leaves the owner's visible content recursively selectable.

#### Implementations
##### `forward_children`
<pre><code class="api-signature language-rust ignore">pub fn forward_children(&amp;mut self)</code></pre>

Default traversal. Exact extents conservatively require complete children;
a container with more precise dependency knowledge can override this.

##### `measured`
<pre><code class="api-signature language-rust ignore">pub fn measured(&amp;self, node: &amp;ExpandedNode, demand: <a href="../../../../../api/internal/pax-runtime/engine/startup/plan.md#startupdemand">StartupDemand</a>) -&gt; bool</code></pre>

Whether a browser measurement checkpoint covered this requirement.
A visible subtree with no deferred descendants also has complete extents.

##### `require`
<pre><code class="api-signature language-rust ignore">pub fn require(&amp;mut self, child: &amp;ExpandedNode, demand: <a href="../../../../../api/internal/pax-runtime/engine/startup/plan.md#startupdemand">StartupDemand</a>)</code></pre>

Require a child or detached rendering dependency. Repeated requirements merge.
