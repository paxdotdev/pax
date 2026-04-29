# interpreter
<!-- summary: API docs for pax-language::interpreter. -->
<!-- tags: api, pax-language -->

## Submodules
- [interpreter::property_resolution](interpreter/property_resolution.md)

## Structs
### `PaxIdentifier`
Symbol reference in a PAXEL expression.

#### Properties
##### `name`
Type: `String`

#### Implementations
##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(name: &amp;str) -&gt; Self</code></pre>

Construct an identifier from its source spelling.

---

### `PaxInfix`
Binary infix operation with left and right expression operands.

#### Implementations
##### `lhs`
<pre><code class="api-signature language-rust ignore">pub fn lhs(&amp;self) -&gt; &amp;<a href="/api/internal/pax-language/interpreter.md#paxexpression">PaxExpression</a></code></pre>

Left-hand expression.

##### `operator_name`
<pre><code class="api-signature language-rust ignore">pub fn operator_name(&amp;self) -&gt; &amp;str</code></pre>

Name of the infix operator.

##### `rhs`
<pre><code class="api-signature language-rust ignore">pub fn rhs(&amp;self) -&gt; &amp;<a href="/api/internal/pax-language/interpreter.md#paxexpression">PaxExpression</a></code></pre>

Right-hand expression.

---

### `PaxNullCoalesce`
Short-circuiting fallback expression. `Some(value) ?? fallback` evaluates to
`value`, `None ?? fallback` evaluates the fallback, and non-option left
operands pass through unchanged.

#### Implementations
##### `lhs`
<pre><code class="api-signature language-rust ignore">pub fn lhs(&amp;self) -&gt; &amp;<a href="/api/internal/pax-language/interpreter.md#paxexpression">PaxExpression</a></code></pre>

Left-hand expression.

##### `rhs`
<pre><code class="api-signature language-rust ignore">pub fn rhs(&amp;self) -&gt; &amp;<a href="/api/internal/pax-language/interpreter.md#paxexpression">PaxExpression</a></code></pre>

Right-hand fallback expression.

---

### `PaxOperator`
Parsed operator token, stored by display name.

#### Implementations
##### `name`
<pre><code class="api-signature language-rust ignore">pub fn name(&amp;self) -&gt; &amp;str</code></pre>

Source spelling for this operator.

##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(name: impl Into&lt;String&gt;) -&gt; Self</code></pre>

Construct an operator from its source spelling.

---

### `PaxPostfix`
Postfix operation node.

#### Implementations
##### `lhs`
<pre><code class="api-signature language-rust ignore">pub fn lhs(&amp;self) -&gt; &amp;<a href="/api/internal/pax-language/interpreter.md#paxexpression">PaxExpression</a></code></pre>

Left-hand expression.

##### `operator_name`
<pre><code class="api-signature language-rust ignore">pub fn operator_name(&amp;self) -&gt; &amp;str</code></pre>

Name of the postfix operator.

---

### `PaxPrefix`
Prefix operation such as numeric negation or boolean not.

#### Implementations
##### `operator_name`
<pre><code class="api-signature language-rust ignore">pub fn operator_name(&amp;self) -&gt; &amp;str</code></pre>

Name of the prefix operator.

##### `rhs`
<pre><code class="api-signature language-rust ignore">pub fn rhs(&amp;self) -&gt; &amp;<a href="/api/internal/pax-language/interpreter.md#paxexpression">PaxExpression</a></code></pre>

Right-hand expression.

---

### `PaxTernary`
Conditional expression with a boolean condition and selected true/false branch.

#### Implementations
##### `condition`
<pre><code class="api-signature language-rust ignore">pub fn condition(&amp;self) -&gt; &amp;<a href="/api/internal/pax-language/interpreter.md#paxexpression">PaxExpression</a></code></pre>

Condition expression.

##### `else_branch`
<pre><code class="api-signature language-rust ignore">pub fn else_branch(&amp;self) -&gt; &amp;<a href="/api/internal/pax-language/interpreter.md#paxexpression">PaxExpression</a></code></pre>

Expression evaluated when the condition is false.

##### `then_branch`
<pre><code class="api-signature language-rust ignore">pub fn then_branch(&amp;self) -&gt; &amp;<a href="/api/internal/pax-language/interpreter.md#paxexpression">PaxExpression</a></code></pre>

Expression evaluated when the condition is true.

## Enums
### `PaxAccessor`
Access path applied after an identifier, such as `.field`, `.0`, or `[index]`.

#### Variants
##### `Tuple`(`usize`)
##### `List`([`PaxExpression`](/api/internal/pax-language/interpreter.md#paxexpression))
##### `Struct`(`String`)
---

### `PaxExpression`
PAXEL expression AST node.

#### Variants
##### `Primary`(`Box`<[`PaxPrimary`](/api/internal/pax-language/interpreter.md#paxprimary)>)
##### `Prefix`(`Box`<[`PaxPrefix`](/api/internal/pax-language/interpreter.md#paxprefix)>)
##### `Infix`(`Box`<[`PaxInfix`](/api/internal/pax-language/interpreter.md#paxinfix)>)
##### `Postfix`(`Box`<[`PaxPostfix`](/api/internal/pax-language/interpreter.md#paxpostfix)>)
##### `Ternary`(`Box`<[`PaxTernary`](/api/internal/pax-language/interpreter.md#paxternary)>)
##### `NullCoalesce`(`Box`<[`PaxNullCoalesce`](/api/internal/pax-language/interpreter.md#paxnullcoalesce)>)
#### Implementations
##### `infix`
<pre><code class="api-signature language-rust ignore">pub fn infix(lhs: <a href="/api/internal/pax-language/interpreter.md#paxexpression">PaxExpression</a>, operator: impl Into&lt;String&gt;, rhs: <a href="/api/internal/pax-language/interpreter.md#paxexpression">PaxExpression</a>) -&gt; Self</code></pre>

Construct an infix expression.

##### `null_coalesce`
<pre><code class="api-signature language-rust ignore">pub fn null_coalesce(lhs: <a href="/api/internal/pax-language/interpreter.md#paxexpression">PaxExpression</a>, rhs: <a href="/api/internal/pax-language/interpreter.md#paxexpression">PaxExpression</a>) -&gt; Self</code></pre>

Construct a null-coalescing expression.

##### `postfix`
<pre><code class="api-signature language-rust ignore">pub fn postfix(lhs: <a href="/api/internal/pax-language/interpreter.md#paxexpression">PaxExpression</a>, operator: impl Into&lt;String&gt;) -&gt; Self</code></pre>

Construct a postfix expression.

##### `prefix`
<pre><code class="api-signature language-rust ignore">pub fn prefix(operator: impl Into&lt;String&gt;, rhs: <a href="/api/internal/pax-language/interpreter.md#paxexpression">PaxExpression</a>) -&gt; Self</code></pre>

Construct a prefix expression.

##### `ternary`
<pre><code class="api-signature language-rust ignore">pub fn ternary(condition: <a href="/api/internal/pax-language/interpreter.md#paxexpression">PaxExpression</a>, then_branch: <a href="/api/internal/pax-language/interpreter.md#paxexpression">PaxExpression</a>, else_branch: <a href="/api/internal/pax-language/interpreter.md#paxexpression">PaxExpression</a>) -&gt; Self</code></pre>

Construct a ternary expression.

---

### `PaxPrimary`
Primary expression forms: literals, symbols, object/list/tuple literals, calls, and ranges.

#### Variants
##### `Literal`([`PaxValue`](/api/pax-runtime-api/pax_value.md#paxvalue))
##### `Grouped`(`Box`<[`PaxExpression`](/api/internal/pax-language/interpreter.md#paxexpression)>, `Option`<[`PaxUnit`](/api/internal/pax-language/interpreter.md#paxunit)>)
##### `Identifier`([`PaxIdentifier`](/api/internal/pax-language/interpreter.md#paxidentifier), `Vec`<[`PaxAccessor`](/api/internal/pax-language/interpreter.md#paxaccessor)>)
##### `Object`(`Vec`<(`String`, [`PaxExpression`](/api/internal/pax-language/interpreter.md#paxexpression))>)
##### `FunctionOrEnum`(`String`, `String`, `Vec`<[`PaxExpression`](/api/internal/pax-language/interpreter.md#paxexpression)>)
##### `Range`([`PaxExpression`](/api/internal/pax-language/interpreter.md#paxexpression), [`PaxExpression`](/api/internal/pax-language/interpreter.md#paxexpression))
##### `Tuple`(`Vec`<[`PaxExpression`](/api/internal/pax-language/interpreter.md#paxexpression)>)
##### `List`(`Vec`<[`PaxExpression`](/api/internal/pax-language/interpreter.md#paxexpression)>)
---

### `PaxUnit`
Unit suffix attached to a grouped numeric expression.

#### Variants
##### `Percent`
##### `Pixels`
##### `Radians`
##### `Degrees`
## Functions
### `compute_paxel`
<pre><code class="api-signature language-rust ignore">pub fn compute_paxel(expr: &amp;str, idr: Rc&lt;dyn <a href="/api/internal/pax-language/interpreter/property_resolution.md#identifierresolver">IdentifierResolver</a>&gt;) -&gt; Result&lt;<a href="/api/pax-runtime-api/pax_value.md#paxvalue">PaxValue</a>, String&gt;</code></pre>

Compute a pax expression to a PaxValue

---

### `parse_pax_expression`
<pre><code class="api-signature language-rust ignore">pub fn parse_pax_expression(expr: &amp;str) -&gt; Result&lt;<a href="/api/internal/pax-language/interpreter.md#paxexpression">PaxExpression</a>, String&gt;</code></pre>

Parse a pax expression into a computable AST

---

### `parse_pax_expression_from_pair`
<pre><code class="api-signature language-rust ignore">pub fn parse_pax_expression_from_pair(expr: Pair&lt;&#39;_, <a href="/api/internal/pax-language/index.md#rule">Rule</a>&gt;) -&gt; Result&lt;<a href="/api/internal/pax-language/interpreter.md#paxexpression">PaxExpression</a>, String&gt;</code></pre>

Parse an already-produced pest pair into a PAXEL expression AST.
