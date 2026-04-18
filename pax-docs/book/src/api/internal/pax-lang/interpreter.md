# interpreter
<!-- summary: API docs for pax-lang::interpreter. -->
<!-- tags: api, pax-lang -->

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

---

### `PaxOperator`
Parsed operator token, stored by display name.

---

### `PaxPostfix`
Postfix operation node.

---

### `PaxPrefix`
Prefix operation such as numeric negation or boolean not.

## Enums
### `PaxAccessor`
Access path applied after an identifier, such as `.field`, `.0`, or `[index]`.

#### Variants
##### `Tuple`(`usize`)
##### `List`([`PaxExpression`](/api/internal/pax-lang/interpreter.md#paxexpression))
##### `Struct`(`String`)
---

### `PaxExpression`
PAXEL expression AST node.

#### Variants
##### `Primary`(`Box`<[`PaxPrimary`](/api/internal/pax-lang/interpreter.md#paxprimary)>)
##### `Prefix`(`Box`<[`PaxPrefix`](/api/internal/pax-lang/interpreter.md#paxprefix)>)
##### `Infix`(`Box`<[`PaxInfix`](/api/internal/pax-lang/interpreter.md#paxinfix)>)
##### `Postfix`(`Box`<[`PaxPostfix`](/api/internal/pax-lang/interpreter.md#paxpostfix)>)
---

### `PaxPrimary`
Primary expression forms: literals, symbols, object/list/tuple literals, calls, and ranges.

#### Variants
##### `Literal`([`PaxValue`](/api/pax-runtime-api/pax_value.md#paxvalue))
##### `Grouped`(`Box`<[`PaxExpression`](/api/internal/pax-lang/interpreter.md#paxexpression)>, `Option`<[`PaxUnit`](/api/internal/pax-lang/interpreter.md#paxunit)>)
##### `Identifier`([`PaxIdentifier`](/api/internal/pax-lang/interpreter.md#paxidentifier), `Vec`<[`PaxAccessor`](/api/internal/pax-lang/interpreter.md#paxaccessor)>)
##### `Object`(`Vec`<(`String`, [`PaxExpression`](/api/internal/pax-lang/interpreter.md#paxexpression))>)
##### `FunctionOrEnum`(`String`, `String`, `Vec`<[`PaxExpression`](/api/internal/pax-lang/interpreter.md#paxexpression)>)
##### `Range`([`PaxExpression`](/api/internal/pax-lang/interpreter.md#paxexpression), [`PaxExpression`](/api/internal/pax-lang/interpreter.md#paxexpression))
##### `Tuple`(`Vec`<[`PaxExpression`](/api/internal/pax-lang/interpreter.md#paxexpression)>)
##### `List`(`Vec`<[`PaxExpression`](/api/internal/pax-lang/interpreter.md#paxexpression)>)
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
<pre><code class="api-signature language-rust ignore">pub fn compute_paxel(expr: &amp;str, idr: Rc&lt;dyn <a href="/api/internal/pax-lang/interpreter/property_resolution.md#identifierresolver">IdentifierResolver</a>&gt;) -&gt; Result&lt;<a href="/api/pax-runtime-api/pax_value.md#paxvalue">PaxValue</a>, String&gt;</code></pre>

Compute a pax expression to a PaxValue

---

### `parse_pax_expression`
<pre><code class="api-signature language-rust ignore">pub fn parse_pax_expression(expr: &amp;str) -&gt; Result&lt;<a href="/api/internal/pax-lang/interpreter.md#paxexpression">PaxExpression</a>, String&gt;</code></pre>

Parse a pax expression into a computable AST

---

### `parse_pax_expression_from_pair`
<pre><code class="api-signature language-rust ignore">pub fn parse_pax_expression_from_pair(expr: Pair&lt;&#39;_, <a href="/api/internal/pax-lang/index.md#rule">Rule</a>&gt;) -&gt; Result&lt;<a href="/api/internal/pax-lang/interpreter.md#paxexpression">PaxExpression</a>, String&gt;</code></pre>

Parse an already-produced pest pair into a PAXEL expression AST.
