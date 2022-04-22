# Pax

Pax is a language for high performance, cross-platform computer graphics and user interfaces.

Pax _snaps on_ to a Rust codebase to create expressive GUIs or graphical scenes, connected to Rust application logic.

Though Pax has zero dependencies on Web technologies (no WebViews, no JS runtime, no DOM), it aims to achieve the same universality of Web technologies: to run consistently on _every_ screen, and ultimately to deliver better-than-Web user experiences.


#### Use-cases:

- Expressive GUIs for: native desktop apps, native mobile apps, high-performance web apps
- Interactive cartoons and animations
- Games
- 2D documents, 2D/3D mixed media
- Generative and procedural art, digital experimental media
- Data visualizations
- Visual design tooling



<img src="multi-device-placeholder.png" alt="Two separate rendition wherein a phone, a tablet, and a laptop each display a nebula">




### Low-level, fast, and universal

Every program made with Pax compiles via Rust to machine code: Web Assembly in browsers and LLVM for native platforms. It's very fast and very light-weight. (up to 120FPS rendering, target <100KB baseline disk footprint)

Pax is "write once, deploy everywhere."  Native techniques are applied maximally, including for text rendering, form controls, and scrolling.

### Ergonomic and fun to use

Pax was birthed within Rust.  Authoring Pax in the early days will require writing Rust for application logic.  

That said, Pax is its own language, separate from Rust, and it aims to achieve ergonomics familiar to GUI designers and developers.  [On the roadmap](TODO.md) is the addition of an optional JavaScript runtime.  This will enable hacking on Pax without writing any Rust.

### Sky's the limit

Pax is designed to extend and support _anything you can imagine_ on a screen — from 2D to 3D to VR/AR, embedded multimedia, and more.

Ultimately, Pax is aimed at enabling visual creative tooling — Pax's _raison d'être_ is to enable art and artists as well as developers.

> Note: Today Pax is in alpha, supports GPU-primitive 2D vector graphics, and has working development harnesses for Web (WASM) and native macOS (Swift).  See [the roadmap](TODO.md).

<img src="fast-ergonomic-sky-placeholder.png" alt="A surrealistic painting of a computer chip; A pastel sunrise over a city made of checkboxes, dropdown lists, buttons, and mouse pointers" />


## Basic example

This Pax project describes a 2D rectangle at the center of the viewport that can be clicked.  Upon click, the rectangle transitions its rotation to a new value via an animation.

First let's look at the Pax by itself:

```jsx
// Pax: a centered, clickable rectangle
<Rectangle on_click=self.handle_click transform={
    anchor(50%, 50%)   * 
    align(50%, 50%)    * 
    rotate(self.theta) 
}/>
```

You'll notice it looks a lot like HTML, XAML, or JSX.  You'll also notice a couple of symbols that seem to be defined elsewhere -- 
a click handler called `self.handle_click` and some rotation value `self.theta`.  Those values are defined in the Rust struct that Pax attaches to.

Here's the full example including Rust:

```rust
// Rust
// src/lib.rs
use pax::*;
use pax::std::drawing2D::Rectangle;

#[pax(
    <Rectangle on_click=self.handle_click transform={
        anchor(50%, 50%)   * 
        align(50%, 50%)    * 
        rotate(self.theta) 
    }/>
)]
pub struct HelloWorld {
    theta: f64,
}

impl HelloWorld {
    pub fn handle_click(&mut self, args: ArgsClick) {
        let old_theta = self.theta.get();
        
        //instead of an `ease_to` animation, could set value immediately with `self.theta.set(...)`
        self.theta.ease_to(
            old_theta + f64::PI() * 3.0, //new value
            240,                         //duration of transition, frames
            EasingCurve::OutBack,        //curve to use for interpolation 
        );
    }
}
```

## TypeScript example (future)

With Pax TypeScript, this full example might look like:

```typescript
// TypeScript, speculative API
// This is not yet available
import {pax, EasingCurve} from '@pax-lang/pax';

@pax(`
    <Rectangle onClick=this.handleClick transform={
        anchor(50%, 50%) *
        align(50%, 50%) *
        rotate(this.theta)
    } />
`)
class HelloWorld {
    @property
    theta: number;
    
    handleClick(args: ArgsClick) {
        const oldTheta = this.theta.get();
        
        //instead of an `easeTo` animation, could set value immediately with `self.theta.set(...)`
        this.theta.easeTo(
            oldTheta + Math.PI * 3.0,
            240,
            EasingCurve.OutBack
        );
    }
}

```


## Current status & support

|                                         | Web browsers  | Native iOS          | Native Android    | Native macOS        | Native Windows              | Native Linux |
|-----------------------------------------|---------------|---------------------|-------------------|---------------------|-----------------------------|--------------|
| *Ready to use* [1]                      | ✅             | ⏲                   | ⏲                 | ✅                   | ⏲                           | ⏲            |
| Development harness                     | ✅             | ⏲                   | ⏲                 | ✅                   | ⏲                           | ⏲            |
| 2D rendering and UIs [2]                | ✅ <br/>Canvas | ✅ <br/>CoreGraphics | ✅ <br/>Cairo      | ✅ <br/>CoreGraphics | ✅ <br/>Direct2D             | ✅ <br/>Cairo |
| 3D rendering and UIs                    | ⏲             | ⏲                   | ⏲                 | ⏲                   | ⏲                           | ⏲            |
| Vector graphics APIs                    | ✅             | ✅                   | ✅                 | ✅                   | ✅                           | ✅            |
| 2D layouts                              | ✅             | ✅                   | ✅                 | ✅                   | ✅                           | ✅            |
| Animation APIs                          | ✅             | ✅                   | ✅                 | ✅                   | ✅                           | ✅            |
| Native text rendering                   | ✅ <br/>DOM    | ⏲ <br/>UIKit        | ⏲ <br/>android:\* | ⏲ <br/>UIKit        | ⏲ <br/>System.Windows.Forms | ⏲ <br/>GTK   |
| Native form elements                    | ⏲ <br/>DOM    | ⏲ <br/>UIKit        | ⏲ <br/>android:\* | ⏲ <br/>UIKit        | ⏲ <br/>System.Windows.Forms | ⏲ <br/>GTK   |
| Native event handling (e.g. Click, Tap) | ⏲             | ⏲                   | ⏲                 | ⏲                   | ⏲                           | ⏲            |
| Rust as host language                   | ✅ <br/>WASM   | ✅ <br/>LLVM         | ✅ <br/>LLVM       | ✅ <br/>LLVM         | ✅ <br/>LLVM                 | ✅ <br/>LLVM  |
| JS/TypeScript as host language          | ⏲             | ⏲                   | ⏲                 | ⏲                   | ⏲                           | ⏲            |

| Legend:             |
|---------------------|
| ✅ Supported         |
| ⏲ Not yet supported |



## Get started

[Get started here](https://www.pax-lang.org/get-started) with an example project.



## FAQ


### Is there a specification for the Pax language?

Pax is currently specified by this implementation.

Pax is really an assorted bag of special-purpose languages and a runtime, which as a whole act as an application platform.
In this way, Pax is arguably similar to the assorted bag of {HTML, JS, CSS, modern web browsers}.

Pax breaks down into 3 sub-languages:

**1. Template language**
Data representing the _content_ of a scene graph or UI tree. 
Includes a provision for referencing/linking (`id=some_identifier`). 
Also includes condition/loop logic (`if`, `for`)

```
<Group>
  <Rectangle id=my_rect />
  <Ellipse id=my_ellipse />
</Group>
```

**2. Settings language**
Data representing the _behavior_ of a scene graph or UI tree.  Similarly to HTML/CSS, settings may be _joined_ to a template by use of IDs and selectors.

```
@settings {
  #my_rect { // attaches to element with id `my_rect`
    fill: Color::rgb(100%,0,0)
    stroke: {
      width: 5px
      color: Color::rgb(0,0,0)
    }
    width: 100px
    height: 200px
  }
  #my_ellipse {
    
  }
}
```

Settings may be freely inlined inside template element declarations, too:

```
<Rectangle fill=Color::rgb(100%,0,0) stroke=Stroke {color: Color::rgb(100%,0,0)} />
```

**3. Expression language (PAXEL)**

Pax Expression Language, or PAXEL, is where Pax starts to look more like a programming language.

You can create an Expression with PAXEL anywhere you can set a settings value, in `template` definitions or in `@settings` blocks.

For example in a template:

```
<Rectangle fill={ self.activeColor.adjustBrightness(50%) } />
```
or in a settings block:
```
@settings {
  #my_rectangle {
    fill: { self.activeColor.adjustBrightness(50%) }
  }
}
```

In both cases above, the snippet of PAXEL is `self.activeColor.adjustBrightness(50%)`.  The Pax compiler transpiles all expressions
in a program to machine code, collecting them in a central vtable that gets called / evaluated at runtime.

Because Pax Expressions are pure, side-effect free functions, the Pax runtime can make aggressive optimizations: caching values
and only recomputing when one of the stated inputs changes.  Expressions are also readily parallelizable.

PAXEL is very similar to at least one existing language: Google's CEL. PAXEL shares the following characteristics with CEL[4]:

```
    memory-safe: programs cannot access unrelated memory, such as out-of-bounds array indexes or use-after-free pointer dereferences;
    side-effect-free: a PAXEL program only computes an output from its inputs;
    terminating: PAXEL programs cannot loop forever;
    strongly-typed: values have a well-defined type, and operators and functions check that their arguments have the expected types;
    gradually-typed: a type-checking phase occurs before runtime via `rustc`, which detects and rejects some programs that would violate type constraints.
```

PAXEL has a tighter, more specialized scope than CEL and carries a much smaller runtime footprint.


### How does Pax work cross-platform?

<img src="how-it-works-placeholder.png" alt="placeholder infographic for how Pax compiles to a cartridge that can be loaded into any number of native runtimes" />
<!-- TODO: refine; caption -->

### What is Pax's footprint?

As of the current authoring, the WASM bundle for a very basic Pax app is about 150kb including several known unnecessary libraries and debug symbols.
100kb should be easily achievable and is a reasonable long-term goal.  <50kb is a stretch-goal.

Baseline memory (RAM) footprint is on the order of 50MB; this has not yet been optimized.

CPU has not been well profiled (TODO:) but stands to be improved significantly, especially through rendering optimizations.

### Who is behind Pax?

The first versions of Pax were designed and built by [an individual](https://www.github.com/zackbrown), but that individual's desire is for Pax to be community-owned.

Thus, even from its earliest days, Pax is stewarded through a non-profit: the [Pax Language Foundation](https://foundation.pax-lang.org/).  

Participation in the non-profit is available to any contributor or sponsor.  [Reach out on Discord](https://discord.gg/4E6tcrtCRb) to learn more.


## Inspiration

Pax draws design inspiration from, among others:
- Verilog, VHDL
- Macromedia Flash, Dreamweaver
- The World Wide Web, HTML, CSS
- React, Vue, Angular
- Visual Basic, ASP.NET
- VisiCalc, Lotus 1-2-3, Excel
- The Nintendo Entertainment System


## Art credit

[DALL-E 2](https://openai.com/dall-e-2/) by OpenAI

<img src="support-matrix-cartridge-placeholder.png" alt="A surrealistic painting of a support matrix of green checkmarks and eggs; A series of video game cartridges emitting magic sparks" />

## Footnotes

[1] Note that Pax is currently in alpha and should only be used in settings where that's not a concern.

[2] Native 2D drawing that _just works_ on every device — with a very light footprint — is available thanks to the admirable work behind [Piet](https://github.com/linebender/piet).

[3] PAXEL is similar to Google's Common Expression Language (CEL), but CEL was not a suitable fit for Pax due to its footprint — being written in Go, CEL adds
a prohibitive overhead to compiled binaries (1-2MB) vs. Pax's total target footprint of <100KB.  Pax also has subtly distinct goals
vs CEL and is able to fine-tune its syntax to make it as ergonomic as possible for this particular domain.

[4] Content modified from https://github.com/google/cel-spec/blob/master/doc/langdef.md, substituting PAXEL for CEL + adjustments for accuracy

---


## Get started

[Get started here](https://www.pax-lang.org/get-started) with an example project.


---

## Contributing

See [TODO.md](TODO.md).  There are also generous TODOs sprinkled throughout the codebase.  There may be undocumented nuance or intention behind certain aspects of the project — feel free to strike up a conversation on [Discord](https://discord.gg/4E6tcrtCRb).

## Development

### Running the project, with debugger support

`./serve.sh`

Then attach to the `pax-dev-harness-macos` process using your IDE or debugging client.
(TODO: make these instructions Linux and Windows friendly)

### Environment setup, web chassis

- Install `wasm-opt` via `binaryen`:
   ```shell
   brew install binaryen
   ```

- Install 'wasm-pack' via:
   ```shell
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh 
   ```

- Install `node`: https://nodejs.org/en/download/

- Install `yarn`:
   ```shell
  # if necessary: sudo chown -R yourusername /usr/local/lib/node_modules 
  npm i --global yarn
   ```

### Environment setup, macOS chassis

(TODO: make more thorough)
- Install xcode, command line utils



## Appendix A: Anatomy of a Pax component

Every Pax program defines a _component_.  That component may be mounted full-window into an app, imported and used by other Pax programs, or (future) be embedded as a UI component in existing (non-Pax) codebases, e.g. as a React component or a SwiftUI View. 

For visual reference, consider again the following example, which declares a component called `HelloWorld`:

```rust
//Rust
use pax::*;
use pax::drawing2D::Rectangle;

#[pax(
    <Rectangle on_click=self.handle_click transform={
        anchor(50%, 50%) *
        align(50%, 50%) *
        rotate(self.theta)
    }/>
)]
pub struct HelloWorld {
    theta: f64,
}

impl HelloWorld {
    pub fn handle_click(&mut self, args: ArgsClick) {
        let old_theta = self.theta.get();
        
        //instead of an `ease_to` animation, could set value immediately with `self.theta.set(...)`
        self.theta.ease_to(
            old_theta + f64::PI() * 3.0, //new value
            240,                         //duration of transition, frames
            EasingCurve::OutBack,        //curve to use for interpolation 
        );
    }
}
```

#### Template and settings

`<Rectangle fill=/*some value*/> ...`

Each component declares a template in an XML-like syntax, which describes how its UI should be displayed.  Any element in that template can have its settings assigned as XML key-value pairs.

Settings can also be declared separately from the template, in the style of HTML + CSS:

```
@template {
    <Rectangle id=my_rect />
}

@settings {
    #my_rect {
        fill: Color::rgb(100%, 100%, 0)
        height: 200px
        width: 200px
    }
}
```

#### Expressions

Properties can have literal values, like `transform=translate(200,200)` or `fill=Color::rgba(100%, 100%, 0%, 100%)`

Or values can be dynamic *expressions*, like:
`transform={translate(200,y_counter) * rotate(self.rotation_counter)}` or `fill={Color::rgba(self.red_amount, self.green_amount, 0%, 100%)}`

The mechanism behind this is in fact an entire language, a sub-grammar of Pax called 'Pax Expression Language' or PAXEL for short.[3]

PAXEL expressions have _read-only_ access to the scope of their containing component.
For example: `self.some_prop` describes "a copy of the data from the attached Rust struct member `self.some_prop`"

PAXEL expressions are noteworthy in a few ways:
- Any PAXEL expression must be a pure function of its inputs and must be side-effect free.  
- As a result of the above, PAXEL expressions may be aggressively cached and recalculated only when inputs change.
- In spirit, PAXEL expressions act a lot like spreadsheet formulas, bindable to any property in Pax.

#### Event handlers

`on_click=@self.handle_click` binds a the `handle_click` method defined in the host codebase to the built-in `click` event which Pax fires when a user clicks the mouse on this element.  Events fire as "interrupts" and are allowed to execute arbitrary, side-effectful, imperative logic.

It is in event handlers that you will normally change property values (e.g. `self.red_amount.set(/*new value*/)`, where `self.red_amount` is referenced in the Expression example above.)

Pax includes a number of built-in lifecycle events like `pre_render` and user interaction events like `on_click` and `on_tap`.


## Appendix B: Description of native rendering approach for text, certain other elements

Rather than introduce virtual controls at the canvas layer, Pax orchestrates a layer of native
controls as part of its rendering process.  This native overlay is used both for form controls like checkboxes
and drop-downs, as well as for rendering native text.

In the browser, for example, a pool of DOM nodes is created for form control elements and text.
Those elements are positioned as an overlay on top of any canvas rendering, allowing for a cohesive
experience that blends dynamic graphics (e.g. vectors, animations) with native familiar UI elements (e.g. text boxes.)

[Visual of DOM "marionette" overlay layer on top of parallaxed graphics layer]

TODO: describe benefits of this approach toward a11y, because e.g. full DOM + content is present in the browser


## Appendix C: Declarative and designable

At first glance, Pax templates look quite a bit like familiar templating languages like React/JSX.

On closer inspection, you may notice an important distinction: _Pax's templates are not evaluated within a closure_ — they are declared statically and evaluated entirely at compile time.  
Symbols in expressions that refer to a component's properties, like `color=@self.active_bg_color`, are handled via special runtime lookups
in the expression vtable — again, specifically _not_ a direct reference to some `self` in the scope of some closure.

Because the template is evaluated entirely at compile-time, the template is exactly what it is described to
be in the code — or in other words, it is both _code_ and _data_, in the same sense as Lisp.  Expressions themselves, given their functional constraints,
are roughly equivalent to formulas in spreadsheets: declarative, easy to isolate, easy to hack.

The reason _all of that_ matters is because Pax was **designed to be designed** — in the sense of "design tools" that can read and write Pax code as a comprehensive
description of any visual content, design, prototype, document, production GUI, or scene.



## Appendix D: Tic-tac-toe example

```
//Tic-tac-toe example
<Spread direction=Horizontal cell_count=3 >
  for i in 0..3 {
    <Spread direction=Vertical cell_count=3 >
      for j in 0..3 {
        <Group on_jab=handle_jab with (i, j)>
          if self.cells[i][j] == Cell::Empty {
            <image src="blank.png">
          }else if self.cells[i][j] == Cell:X {
            <Image src="x.png" />
          }else if self.cells[i][j] == Cell::O {
            <Image src="o.png" />
          }
        </Group>
      }
    </Spread>
  }
</Spread>
```


## Get started

[Get started here](https://www.pax-lang.org/get-started) with an example project.