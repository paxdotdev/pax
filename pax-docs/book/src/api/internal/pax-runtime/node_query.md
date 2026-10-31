# node_query
<!-- summary: Scoped queries over mounted application instances. -->
<!-- tags: api, pax-runtime -->

Scoped queries over mounted application instances.

## Structs
### `AnimatedNodeMove`
Easing configuration for one explicit move. The same destination methods are
available as on `NodeRef`; an abandoned node remounts without a flight origin.

#### Implementations
##### `move_to_append_child`
<pre><code class="api-signature language-rust ignore">pub fn move_to_append_child(self, parent: &amp;<a href="../../../api/internal/pax-runtime/node_query.md#noderef">NodeRef</a>) -&gt; Result&lt;(), <a href="../../../api/internal/pax-runtime/node_query.md#nodeerror">NodeError</a>&gt;</code></pre>

Animate a move to the end of this parent's content.

##### `move_to_home`
<pre><code class="api-signature language-rust ignore">pub fn move_to_home(self) -&gt; Result&lt;(), <a href="../../../api/internal/pax-runtime/node_query.md#nodeerror">NodeError</a>&gt;</code></pre>

Animate a return to the declaration's current placement.

##### `move_to_insert_after`
<pre><code class="api-signature language-rust ignore">pub fn move_to_insert_after(self, sibling: &amp;<a href="../../../api/internal/pax-runtime/node_query.md#noderef">NodeRef</a>) -&gt; Result&lt;(), <a href="../../../api/internal/pax-runtime/node_query.md#nodeerror">NodeError</a>&gt;</code></pre>

Animate a move immediately after this sibling.

##### `move_to_insert_before`
<pre><code class="api-signature language-rust ignore">pub fn move_to_insert_before(self, sibling: &amp;<a href="../../../api/internal/pax-runtime/node_query.md#noderef">NodeRef</a>) -&gt; Result&lt;(), <a href="../../../api/internal/pax-runtime/node_query.md#nodeerror">NodeError</a>&gt;</code></pre>

Animate a move immediately before this sibling.

##### `move_to_insert_child`
<pre><code class="api-signature language-rust ignore">pub fn move_to_insert_child(self, parent: &amp;<a href="../../../api/internal/pax-runtime/node_query.md#noderef">NodeRef</a>, index: usize) -&gt; Result&lt;(), <a href="../../../api/internal/pax-runtime/node_query.md#nodeerror">NodeError</a>&gt;</code></pre>

Animate a move to a final zero-based content index.

---

### `NodeQuery`
An ordered snapshot of node references. Queries never silently select the
first match or change the identity of a previously returned reference.

#### Implementations
##### `all`
<pre><code class="api-signature language-rust ignore">pub fn all(self) -&gt; Result&lt;Vec&lt;<a href="../../../api/internal/pax-runtime/node_query.md#noderef">NodeRef</a>&gt;, <a href="../../../api/internal/pax-runtime/node_query.md#nodeerror">NodeError</a>&gt;</code></pre>

Resolve all matches in their traversal order.

##### `index_of`
<pre><code class="api-signature language-rust ignore">pub fn index_of(&amp;self, node: &amp;<a href="../../../api/internal/pax-runtime/node_query.md#noderef">NodeRef</a>) -&gt; Result&lt;usize, <a href="../../../api/internal/pax-runtime/node_query.md#nodeerror">NodeError</a>&gt;</code></pre>

Find the position of an exact instance within this snapshot.

##### `is_empty`
<pre><code class="api-signature language-rust ignore">pub fn is_empty(&amp;self) -&gt; Result&lt;bool, <a href="../../../api/internal/pax-runtime/node_query.md#nodeerror">NodeError</a>&gt;</code></pre>

Whether this snapshot contains no nodes.

##### `key`
<pre><code class="api-signature language-rust ignore">pub fn key(self, key: impl Into&lt;<a href="../../../api/internal/pax-runtime/node_query.md#nodekey">NodeKey</a>&gt;) -&gt; Self</code></pre>

Filter by the nearest owning keyed iteration. Narrow the collection first
when unrelated or nested repeats can contain the same key.

##### `len`
<pre><code class="api-signature language-rust ignore">pub fn len(&amp;self) -&gt; Result&lt;usize, <a href="../../../api/internal/pax-runtime/node_query.md#nodeerror">NodeError</a>&gt;</code></pre>

Number of nodes in this snapshot. Ended declaration lifetimes are errors.

##### `nth`
<pre><code class="api-signature language-rust ignore">pub fn nth(&amp;self, index: usize) -&gt; Result&lt;<a href="../../../api/internal/pax-runtime/node_query.md#noderef">NodeRef</a>, <a href="../../../api/internal/pax-runtime/node_query.md#nodeerror">NodeError</a>&gt;</code></pre>

Select a zero-based position in this snapshot, independently of repeat keys.

##### `one`
<pre><code class="api-signature language-rust ignore">pub fn one(self) -&gt; Result&lt;<a href="../../../api/internal/pax-runtime/node_query.md#noderef">NodeRef</a>, <a href="../../../api/internal/pax-runtime/node_query.md#nodeerror">NodeError</a>&gt;</code></pre>

Resolve exactly one match, rejecting absent or ambiguous selections.

##### `query`
<pre><code class="api-signature language-rust ignore">pub fn query(self, selector: &amp;str) -&gt; Self</code></pre>

Narrow to matching descendants of the selected nodes, deduplicated in
traversal order. Invalid selectors are reported by the terminal operation.

---

### `NodeRef`
A reference to one declaration-owned instance. Explicit moves preserve its
mount. An abandoned transfer can be recovered into a fresh mount; ordinary
declaration removal permanently invalidates this reference.

#### Implementations
##### `animated`
<pre><code class="api-signature language-rust ignore">pub fn animated(&amp;self, duration: impl Into&lt;<a href="../../../api/pax-runtime-api/animation.md#duration">Duration</a>&gt;, easing: <a href="../../../api/pax-runtime-api/animation.md#easingcurve">EasingCurve</a>) -&gt; <a href="../../../api/internal/pax-runtime/node_query.md#animatednodemove">AnimatedNodeMove</a></code></pre>

Configure easing for one move. Motion starts from the current presented
footprint (including an interrupted move) and follows destination layout.
Crossing Overlay contributions temporarily presents above the higher
endpoint, below unrelated higher Overlays, until the animation completes.

##### `content_children`
<pre><code class="api-signature language-rust ignore">pub fn content_children(&amp;self) -&gt; <a href="../../../api/internal/pax-runtime/node_query.md#nodequery">NodeQuery</a></code></pre>

Active content children, with implementation-only control-flow and slot
wrappers flattened. Exit-retained nodes are excluded; culled nodes remain.

##### `engine_id`
<pre><code class="api-signature language-rust ignore">pub fn engine_id(&amp;self) -&gt; <a href="../../../api/internal/pax-runtime/properties.md#expandednodeidentifier">ExpandedNodeIdentifier</a></code></pre>

Runtime identity of this instance; valid only for its application lifetime.

##### `is_detached`
<pre><code class="api-signature language-rust ignore">pub fn is_detached(&amp;self) -&gt; Result&lt;bool, <a href="../../../api/internal/pax-runtime/node_query.md#nodeerror">NodeError</a>&gt;</code></pre>

Whether this handle is awaiting explicit recovery after its borrower unmounted.

##### `move_to_append_child`
<pre><code class="api-signature language-rust ignore">pub fn move_to_append_child(&amp;self, parent: &amp;Self) -&gt; Result&lt;(), <a href="../../../api/internal/pax-runtime/node_query.md#nodeerror">NodeError</a>&gt;</code></pre>

Move this instance to the end of a destination's active content children.
Moves affect runtime presentation only, never the serialized program.

##### `move_to_home`
<pre><code class="api-signature language-rust ignore">pub fn move_to_home(&amp;self) -&gt; Result&lt;(), <a href="../../../api/internal/pax-runtime/node_query.md#nodeerror">NodeError</a>&gt;</code></pre>

Return to the declaration's current placement. A live visitor keeps its
mount and state; an abandoned node starts a fresh mount. Repeat reordering
is respected. If already home, this is a no-op.

##### `move_to_insert_after`
<pre><code class="api-signature language-rust ignore">pub fn move_to_insert_after(&amp;self, sibling: &amp;Self) -&gt; Result&lt;(), <a href="../../../api/internal/pax-runtime/node_query.md#nodeerror">NodeError</a>&gt;</code></pre>

Move this instance immediately after an active sibling. An abandoned
instance starts a fresh mount.

##### `move_to_insert_before`
<pre><code class="api-signature language-rust ignore">pub fn move_to_insert_before(&amp;self, sibling: &amp;Self) -&gt; Result&lt;(), <a href="../../../api/internal/pax-runtime/node_query.md#nodeerror">NodeError</a>&gt;</code></pre>

Move this instance immediately before an active sibling. A live instance
retains its mount; an abandoned instance starts a fresh mount.

##### `move_to_insert_child`
<pre><code class="api-signature language-rust ignore">pub fn move_to_insert_child(&amp;self, parent: &amp;Self, index: usize) -&gt; Result&lt;(), <a href="../../../api/internal/pax-runtime/node_query.md#nodeerror">NodeError</a>&gt;</code></pre>

Move to a final zero-based child index, after removing this instance from
the sequence if it is already a child. The remaining sequence's `len`
appends; larger indices fail.
Failed moves leave placement unchanged. Destination loss ends the mount
but retains a recovery entry; original-owner loss invalidates the handle.

##### `next`
<pre><code class="api-signature language-rust ignore">pub fn next(&amp;self) -&gt; Result&lt;Option&lt;Self&gt;, <a href="../../../api/internal/pax-runtime/node_query.md#nodeerror">NodeError</a>&gt;</code></pre>

The next active sibling, or `None` at the end of the content sequence.

##### `previous`
<pre><code class="api-signature language-rust ignore">pub fn previous(&amp;self) -&gt; Result&lt;Option&lt;Self&gt;, <a href="../../../api/internal/pax-runtime/node_query.md#nodeerror">NodeError</a>&gt;</code></pre>

The previous active sibling, or `None` at the start of the content sequence.

##### `query`
<pre><code class="api-signature language-rust ignore">pub fn query(&amp;self, selector: &amp;str) -&gt; <a href="../../../api/internal/pax-runtime/node_query.md#nodequery">NodeQuery</a></code></pre>

Find descendants matching an ID, class, or type selector. Traversal crosses
component boundaries and follows current destination order, not depth
or the temporary presentation of an animated transfer.

## Enums
### `NodeError`
A failed node lookup or tree operation. Failed moves leave the tree unchanged.

#### Variants
##### `Unmounted`
The declaration's lifetime has ended, or the application is unavailable.

##### `InvalidSelector`(`String`)
A selector could not be parsed.

##### `ExpectedOne`(`usize`)
Exactly one node was requested, but a different number matched.

##### `IndexOutOfBounds` { `index`: `usize`, `len`: `usize` }
A position is outside the selected collection.

##### `NotAChild`
The supplied node is not a member of this collection.

##### `DifferentApplication`
Nodes from different application runtimes cannot be transferred.

##### `Cycle`
The operation would create a cycle in the presentation tree.

##### `InvalidDestination`
This node cannot receive externally transferred content.

##### `HomeUnavailable`
The authored placement is no longer mounted or available.

##### `InvalidMoveGeometry`
Animated moves require a finite, nondegenerate starting footprint.

---

### `NodeKey`
The string or integer identity used by an existing keyed repeat.

#### Variants
##### `String`(`String`)
##### `Integer`(`i64`)
