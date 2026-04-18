# pax_value::numeric
<!-- summary: API docs for pax-runtime-api::pax_value::numeric. -->
<!-- tags: api, pax-runtime-api -->

## Enums
### `Numeric`
Numeric type wrapper for PAXEL and runtime polymorphic numeric operations.
This broad polyfill specifically enables terse numeric operations within expressions, without
the overhead of explicit typing or numeric microsyntax.

#### Variants
##### `I8`(`i8`)
##### `I16`(`i16`)
##### `I32`(`i32`)
##### `I64`(`i64`)
##### `U8`(`u8`)
##### `U16`(`u16`)
##### `U32`(`u32`)
##### `U64`(`u64`)
##### `F64`(`f64`)
##### `F32`(`f32`)
##### `ISize`(`isize`)
##### `USize`(`usize`)
#### Implementations
##### `is_float`
<pre><code class="api-signature language-rust ignore">pub fn is_float(&amp;self) -&gt; bool</code></pre>

Returns true when this value is stored as a floating-point number.

##### `max`
<pre><code class="api-signature language-rust ignore">pub fn max(self, other: Self) -&gt; Self</code></pre>

Returns the larger of two numeric values.

##### `min`
<pre><code class="api-signature language-rust ignore">pub fn min(self, other: Self) -&gt; Self</code></pre>

Returns the smaller of two numeric values.

##### `pow`
<pre><code class="api-signature language-rust ignore">pub fn pow(self, exp: Self) -&gt; Self</code></pre>

Raises this number to `exp`, preserving integer arithmetic when both sides are integral.

##### `to_float`
<pre><code class="api-signature language-rust ignore">pub fn to_float(&amp;self) -&gt; f64</code></pre>

Coerces this numeric value to `f64`.

##### `to_int`
<pre><code class="api-signature language-rust ignore">pub fn to_int(&amp;self) -&gt; i64</code></pre>

Coerces this numeric value to `i64`.
