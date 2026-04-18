# pax_value::functions
<!-- summary: API docs for pax-runtime-api::pax_value::functions. -->
<!-- tags: api, pax-runtime-api -->

## Structs
### `Functions`
Registry bootstrap for built-in PAXEL helper functions.

#### Implementations
##### `has_function`
<pre><code class="api-signature language-rust ignore">pub fn has_function(scope: &amp;str, name: &amp;str) -&gt; bool</code></pre>

Returns true if a helper function exists in the named scope.

##### `register_all_functions`
<pre><code class="api-signature language-rust ignore">pub fn register_all_functions()</code></pre>

Registers built-in math, color, and transform helper functions.

## Traits
### `HelperFunctions`
Registers helper functions made available to PAXEL scopes.
