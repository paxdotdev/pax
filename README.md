# Pax 

Pax is a toolkit for creating high-performance, expressive, and portable user interfaces.  

Pax is designed to support any platform via extensible rendering engines, either 2D or 3D, across Web, iOS, Android, Desktop (macOS, Linux, Windows), Embedded devices, and more.


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
  


### Running the project
`./serve.sh`

### Dev Env Setup
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


