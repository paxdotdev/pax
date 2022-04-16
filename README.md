# Pax 

Pax is a language for high performance, cross-platform computer graphics and user interfaces.

[TODO: GIF of three devices, each showing a progression of: 1. responsive form/CRUD app + layouts, 2. game, e.g. spaceship/asteroid shooter, 3. animated data viz + text, a la d3]


## Goals

**Portable**
- run on any device
- tiny footprint: suitable for web applications and embedded applications
- extremely fast (animations up to 120fps on supporting hardware)
- based on Rust and LLVM, compiles to machine code on most platforms and WASM (Web Assembly) for the Web.

**All-purpose**
- "Any UI you can imagine" -- 2D, 3D, digital documents, VR/AR, web apps, CRUD apps, data visualization, interactive cartoons, experimental art, embedded GUIs
- Native UI controls for every platform (dropdowns and text boxes, scrolling, etc.)
- Native text rendering & styling for every platform
- Native accessibility (a11y) support for every platform
- Expressive & intuitive layouts
- Complex, fine-tuned animations

**eXtensible**
- "Components all the way down" as reusable, extensible UI building blocks
- Extensible rendering back-ends, meaning any platform can be supported
- Can support any type of digital media — audio, video, etc.
- Agnostic "host language" means any language can be supported (Rust, TypeScript/JavaScript, C++, .NET CLR, Python...)
- Free and open source (MIT / Apache 2.0)

Above all: Make the digital medium more expressive, for productivity and for art.

## How it works

Pax attaches to a _host codebase_, which is responsible for any imperative or side-effectful logic (e.g. network requests, operating system interactions.)  This divide allows Pax itself to remain highly declarative.

In practice, this looks like: write template and settings in Pax, write event handlers and data structures in the host language.

[image: infographic illustrating layers of Pax and Host, illustrating the divide described above + "imperative", "declarative", "expressions", "side-effects"] 

Currently Pax supports Rust as a host language, though support for JavaScript/TypeScript is on the [roadmap](TODO.md). For Pax itself: the compiler tooling and the runtimes are all written in Rust.

Following is a simple example.  This Pax UI describes a 2D rectangle at the center of the viewport that can be clicked.  Upon click, the rectangle increments its rotation by 1/20 radians.

```rust
//Rust
use pax::*;
use pax::drawing2D::Rectangle;

#[pax(
    <Rectangle on_click=@self.handle_click transform=@{
        align(50%, 50%) *
        rotate(self.num_clicks / 20.0)
    }/>
)]
pub struct HelloWorld {
    num_clicks: isize,
}

impl HelloWorld {
    pub fn handle_click(&mut self, args: ArgsClick) {
        let old_num_clicks = self.num_clicks.get();
        self.num_clicks.set(old_num_clicks + 1);
    }
}
```

With Pax TypeScript, this example would look like:

```typescript
//TypeScript, speculative API
//This is not yet available
import pax from '@pax-lang/pax';

@pax(`
    <Rectangle onClick=@this.handleClick transform=@{
        align(50%, 50%) *
        rotate(this.numClicks / 20.0)
    } />
`)
class HelloWorld {
    @property
    numClicks: number;
    
    handleClick(args: ArgsClick) {
        const oldNumClicks = this.numClicks.get();
        this.numClicks.set(oldNumClicks + 1);
    }
}

```

You'll notice a few moving pieces here:

#### Template and settings

`<Rectangle fill=/*some value*/> ...`

Each component declares a template in an XML-like syntax, which describes how its UI should be displayed.  Any element in that template can have its settings assigned as XML key-value pairs.

#### Expressions

Properties can have literal values, like `transform=translate(200,200)` or `fill=Color::rgba(100%, 100%, 0%, 100%)`

Or values can be dynamic *expressions*, like:
`transform=@{translate(200,y_counter) * rotate(self.rotation_counter)}` or `fill=@{Color::rgba(self.red_amount, self.green_amount, 0%, 100%)}`

Notice the `@`-signs — these signal to the Pax compiler that the subsequent symbol(s) are dynamic, and should be evaluated in the context of the host codebase.  The contents of the `{}` block are evaluated and the return value is bound to the setting.  

The mechanism behind this is in fact a whole language, a sub-grammar of Pax called 'Pax Expression Language' or PAXEL for short.[3] 

PAXEL expressions are distinctive in a few ways:
     - Any PAXEL expression must be a pure function of its inputs and must be side-effect free.  E.g. there's simply no way in the PAXEL language to _set_ a value.
     - As a result of the above, PAXEL expressions may be aggressively cached and recalculated only when inputs change.
     - In spirit, expressions act a lot like spreadsheet formulas

#### Event handlers

`on_click=@self.handle_click` binds a the `handle_click` method defined in the host codebase to the built-in `click` event which Pax fires when a user clicks the mouse on this element.  Events fire as "interrupts" and are allowed to execute arbitrary, side-effectful, imperative logic.  

It is in event handlers that you will normally change property values (e.g. `self.red_amount.set(/*new value*/)`, where `self.red_amount` is referenced in the Expression example above.)

Pax includes a number of built-in lifecycle events like `pre_render` and user interaction events like `on_click` and `on_tap`.


## Current status & support

Pax is in its early days but has ambitions to mature robustly.

|                                         | Web browsers  | Native iOS          | Native Android    | Native macOS        | Native Windows              |
|-----------------------------------------|---------------|---------------------|-------------------|---------------------|-----------------------------|
| *Ready to use* [1]                      | ✅             | ⏲                   | ⏲                 | ⏲                   | ⏲                           |
| 2D rendering and UIs [2]                | ✅ <br/>Canvas | ✅ <br/>CoreGraphics | ✅ <br/>Cairo      | ✅ <br/>CoreGraphics | ✅ <br/>Direct2D             |
| 3D rendering and UIs                    | ⏲             | ⏲                   | ⏲                 | ⏲                   | ⏲                           |
| Vector graphics APIs                    | ✅             | ✅                   | ✅                 | ✅                   | ✅                           |
| 2D layouts                              | ✅             | ✅                   | ✅                 | ✅                   | ✅                           |
| Animation APIs                          | ✅             | ✅                   | ✅                 | ✅                   | ✅                           |
| Native text rendering                   | ✅ <br/>DOM    | ⏲ <br/>UIKit        | ⏲ <br/>android:\* | ⏲ <br/>UIKit        | ⏲ <br/>System.Windows.Forms |
| Native form elements                    | ⏲ <br/>DOM    | ⏲ <br/>UIKit        | ⏲ <br/>android:\* | ⏲ <br/>UIKit        | ⏲ <br/>System.Windows.Forms |
| Native event handling (e.g. Click, Tap) | ✅             | ⏲                   | ⏲                 | ⏲                   | ⏲                           |
| Rust as host language                   | ✅ <br/>WASM   | ✅ <br/>LLVM         | ✅ <br/>LLVM       | ✅ <br/>LLVM         | ✅ <br/>LLVM                 |
| JS/TypeScript as host language          | ⏲             | ⏲                   | ⏲                 | ⏲                   | ⏲                           |

| Legend:             |
|---------------------|
| ✅ Supported         |
| ⏲ Not yet supported |


[1] Note that Pax is currently in alpha and should only be used in settings where that's not a concern. 

[2] Native 2D drawing that _just works_ on every device — with a very light footprint — is available thanks to the admirable work behind [Piet](https://github.com/linebender/piet). 

[3] PAXEL is similar to Google's Common Expression Language (CEL), but CEL was not a suitable fit for Pax due to its footprint — being written in Go, CEL adds
a prohibitive overhead to compiled binaries (1-2MB) vs. Pax's total target footprint of <100KB.  Pax also has subtly distinct goals
vs CEL and is able to fine-tune its syntax to make it as ergonomic as possible for this particular domain.


## What's in the box

 - Compiler
   - Builds a pax project into platform-specific "cartridges" (a la the Nintendo Entertainment System.) These cartrides can be mounted as self-contained native apps or embedded as UI components in existing codebases.
 - Runtime (core)
   - Written in Rust, logic for rendering and computation
 - Runtime (per-platform native chassis)
   - Written in combination of Rust and native platform code (e.g. TypeScript for Web, Swift for macOS/iOS)
   - Handles platform-native concerns like: user input, native element rendering, text and form controls
 - Examples
 - APIs
   - Tools for authoring Pax apps

   
## Native rendering, native controls

Rather than introduce virtual controls at the canvas layer, Pax orchestrates a layer of native
controls as part of its rendering process.  This native overlay is used both for form controls like checkboxes
and drop-downs, as well as for rendering native text.

In the browser, for example, a pool of DOM nodes is created for form control elements and text.
Those elements are positioned as an overlay on top of any canvas rendering, allowing for a cohesive
experience that blends dynamic graphics (e.g. vectors, animations) with native familiar UI elements (e.g. text boxes.)

[Visual of DOM "marionette" overlay layer on top of parallaxed graphics layer]

TODO: describe benefits of this approach toward a11y


## Declarative and designable

At first glance, Pax templates look quite a bit like familiar templating languages like React/JSX.

On closer inspection, you may notice a key distinction: _Pax's templates are not evaluated within a closure_ — they are declared statically and evaluated entirely at compile time.  
Symbols in expressions that refer to a component's properties, like `color=@self.active_bg_color`, are handled via special runtime lookups
in the expression vtable — again, specifically _not_ a reference to some `self` in the scope of some closure.

Because the template is evaluated entirely at compile-time, the template is exactly what it is described to
be in the code — or in other words, it is both _code_ and _data_, in the same sense as Lisp.  Expressions themselves, given their functional constraints,
are roughly equivalent to formulas in spreadsheets: declarative, easy to isolate, easy to hack.

The reason _all of that_ matters is because Pax was **designed to be designed** — in the sense of "design tools" that can read and write Pax code as a comprehensive
description of any visual content, document, or scene.



## Inspiration

Pax draws design inspiration from, among others:
 - Verilog, VHDL
 - Macromedia Flash, Dreamweaver
 - The World Wide Web, HTML, CSS
 - React, Vue, Angular
 - Visual Basic, ASP.NET
 - VisiCalc, Lotus 1-2-3, Excel
 - The Nintendo Entertainment System


## Development

### Running the project
`./serve.sh`

### Dev Env Setup
(for web chassis)
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




```
Scratch pad:






Pax aims to be an expressive substrate for *anything you can imagine* on a screen — for example:
    - User interfaces, from the mundane to the experimental
    - 2D vector graphics and animations
    - 3D scenes and games, including VR and AR
    - Data visualizations
    - Text and documents
    





- how does this fit into my workflow?  (cartridges)
- how does this differ from other template tools?  why might I choose this instead of, say, React?




Pax aims to be high-performance

Pax is a language for the Web
Pax is the Web for web3
    - Language: Pax is an alternative to HTML/CSS/JavaScript 
    - Backwards-compatible: works with any modern web browser via Web Assembly
    - Universal: also runs natively (LLVM) on iOS, Android, macOS, Windows, Linux



Pax is like the l, but includes 

Animations, generative art, data visualization
Layouts, UI animations, form controls and elements, modularity and reusability
Games, 2D, 3D


Pax is a language for high-performance, cross-platform computer graphics and user interfaces.  
Pax is a language for creating high-performance, cross-platform user interfaces.















And this breaks the mnemonic, but is a fundamental goal:

**Designable**
- Pax is designed from the ground-up to be deterministically _machine_ read&writable alongside ergonomic _human_ authoring.
- In other words: Pax is designed to be _designed_ with visual tools that feel like Illustrator or Figma, as an alternative to hand-writing.

Above all, Pax aims to be a medium for _art_.  Utility, also, yes: that's table-stakes.  If Pax helps push the envelope for how humans express ourselves with interactive digital media, then it was worth building.    








## Who?










Pax is designed to be designed — that is, the language specification, compiler, and runtime are designed around the goal of enabling interoperability with visual designer tooling.

At the same time, it aims to be both 







## Anatomy of Pax

1. Host Codebase
    1. Pax works as a _companion language_ to a host codebase.  As of this writing, Pax supports Rust for host codebases — i.e. you author UIs in Pax driven by the logic written in Rust. JavaScript/TypeScript host codebase support is planned for the future.
2. Pax declarations (either "code-behind" with a `.pax` file, or inline in Rust/TypeScript)
3. Pax compiler builds "cartridge", which fits (NES) into native codebases with SDK (a la React)







The core design goal of Pax is to be _designable._

That is, anything you could imagine _designing_ visually on a screen, e.g. through a vector
graphics design tool, or a 3D scene for a video game, should be elegantly expressible with Pax.

Further, Pax's language syntax was designed for tooling to read and write.  The compiler API accepts
parsed language tokens through the same bridge that is exposed for future design tools to perform operations like
"create vector ellipse" or "change bezier path control point."



Not
only can you express sky's-the-limit UIs with it (not unlike a game engine,)
but the language itself was designed to be "hyper-declarative"




Pax draws inspiration from hardware description languages like Verilog and VHDL, which operate
highly declaratively.

In much the same way as Verilog and VHDL allow you to "declare transitor arrangements," Pax allows
you to "declare UI arrangements," with the express goal of supporting any UI you can imagine or design.

Similar to a Hardware Description Language,[1] Pax aims to be hyper-


[1] "HDLs", like Verilog or VHDL

a tool for authoring cross-platform
native, high-performance, delightful UIs




As of the authoring of this document, Pax only supports vector and text graphics for 2D back-ends[1], but there's a clear path
to 3D support.  This would be an excellent area for community contribution; the hardest part will simply be
the creation of 3D primitives

[1] Thanks to the hard work of the folks behind `piet`




Currently Rust is the only supported host language.  Pax is itself built with Rust.  Support for JavaScript/TypeScript as a host language is on the [roadmap](TODO.md).

That is, you _describe_ the definition and properties of a UI with the Pax language, and
handle 

Currently, Pax supports


- Component definitions
- Drawing and rendering primitives, e.g. shapes, text, and form controls
- Template declarations, as compilations of components and primitives


Pax currently works alongside Rust and is built with Rust.though support for authoring UI logic in JavaScript/TypeScript is also on the 



## A bit more detail

Portable, All-purpose, eXpressive
User Interface Description Language





## How does it work?

Pax UIs are defined as `.pax` files alongside 'host language' files, either Rust or JavaScript/TypeScript.

Pax UIs and basic behaviors are expressed in the declarative Pax format, while complex, host-specific, and turing-complete logic is handled by the host language.

//TODO: explainer image showing pax & host files alongside each other
_Read more about the individual pieces of Pax in [Pax at a glance]_

When compiled, Pax files become low-level assembly code -- WASM or LLVM depending on the target platform.

This "cartridge" of binary code is then loaded into a platform-native chassis, which is loaded by a host application, e.g. by a Web app or an iOS app. 

//TODO: explainer image of NES cartridge & console





## Pax at a glance

Properties & settings (Pax)
Expressions (Pax)
Event handlers (host language)


Pax GUIs are intended to run anywhere, as stand-alone applications or embedded as components inside other applications.


This project is the first implementation of the Pax User Interface Language.  This includes:
  - Language parser, compiler, and debugging tools
  - Rendering engines for macOS, iOS, Android, Windows, and Web

## Pax's goals

Portable
Performant
Professional (license, stability)

All-purpose
Approachable
Accessible (as in accessibility)

eXtensible
eXpressive
eXhaustive (redundant with all-purpose?)

**P**ortable
- run everywhere
- tiny footprint: suitable for web applications and embedded applications
  [react, angular, Qt, etc. footprint vs. pax]
- excellent performance for a wide range of hardware
- Fast & tiny binary "cartridges" describe production applications

**A**ll-purpose
- 2D, 3D, digital documents, web apps, VR/AR, embedded GUIs
- Per-platform native UI controls (dropdowns, scroll, etc.)
- Expressive & intuitive layouts
- Animations (bouncy UI ex.), simulations (particles ex.), complex GUIs (spreadsheet ex.), data-viz (d3-style animated chart ex.)

e**X**tensible
- Reusable UI building blocks
- Open source (MIT / Apache 2.0)






Pax must latch onto another language in order to be turing complete
First, Pax is not Turing complete.  Pax relies on a companion language for Turing completeness, currently:
  - Rust,
  - TypeScript, or
  - JavaScript
Bindings could (and hopefully one day will!) be written for all programming languages.
    
[Visual: bar-chart-esque; three languages, "helmet" or layers, etc. of Pax for the UI]

Second, Pax aims to support the expression of any conceivable UI — across 2D, 3D, forms, documents, VR/AR

Second, Pax is designed to interface with design tools





<br>
*Portable, All-purpose, eXtensible*


What can be built with Pax?



  

## Some nerdier goals

  - Deterministic state system, enabling reproducible UIs (e.g. for screenshot/regression testing) 
  
  - Designable
     
  - Hackable
  


```
