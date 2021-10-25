



/******
The metaruntime tracks the following:
    - Component definitions, esp. a main component
        - a unique string `ID` (name) for the component, which becomes its fs location
          and the way it's imported/referenced.  Forward slashes may be supportable for
          nesting into directories, but we need to figure out how to resolve ambiguity
          (e.g. `Main` and `some-directory/Main`)

          Perhaps `foo::Bar`, namespace syntax?

          this needs to play nicely with Rust's import system, which will be the backbone
          for the actual import functionality. we should spot-check compatibility with ES/TS
          imports for a future web runtime.

        - a `Template`  for that component, describing a tree of its contained elements
        - `Properties` for that component's contained elements
            - a semantized object (properties coproduct) describing all properties,
              with pointers to the Expression LUT as needed
        - `Actions` for that component (or just a string representing the contents of the code-behind file)
        - An `Expression` table, really just a hashmap of `unique-id` => `function` (or just a Vec<fn>)
            - Double-check:  does this table belong in the runtime or metaruntime?  Is it needed
              in stand-alone runtime?

How do we compile our actions and code-behind files?
    - Generate into a temp directory?
    - pipe the content into rustc?
    - perhaps these don't get fired by the metaruntime (akin to Flash/HA's edit vs. preview modes) —
        that is, don't deal with actions at all in the metaruntime until compiling to RIL
    - perhaps the whole project is recompiled, and the metaruntime re-attaches itself

How does the metaruntime bolt onto an engine instance (+ chassis) to enable it to
"take over" the render tree definition?

Walk through the

 */

use carbon::{Component, PropertiesCoproduct, Size2D, Transform, Stroke, PropertyValue, Color};
use std::str::FromStr;
use std::rc::Rc;
use std::cell::RefCell;


//TODO:
// - see if we can refactor Rc<RefCell<Transform>> -> Transform + &mut refs
// -


pub fn operate_on_rp(mut rp: RectangleProperties) {
    let new_patch = RectanglePropertiesPatch {
        transform: None,
        fill: None,
        size: None,
        stroke: None,
    };
    rp.patch(new_patch);
}

pub struct RectangleProperties {
    pub size: Size2D,
    pub transform: Rc<RefCell<Transform>>,
    pub stroke: Stroke,
    pub fill: Box<dyn PropertyValue<Color>>,
}

impl Patchable<RectanglePropertiesPatch> for RectangleProperties {
    fn patch(&mut self, patch: RectanglePropertiesPatch) {
        if let Some(p) = patch.transform {
            self.transform = Rc::clone(&p);
        }
        if let Some(p) = patch.size {
            self.size = Rc::clone(&p);
        }
        if let Some(p) = patch.stroke {
            self.stroke = p;
        }
        if let Some(p) = patch.fill {
            self.fill = p;
        }
    }
}


pub trait Patchable<P> {
    fn patch(&mut self, patch: P);
}

pub struct RectanglePropertiesPatch {
    pub size: Option<Size2D>,
    pub transform: Option<Rc<RefCell<Transform>>>,
    pub stroke: Option<Stroke>,
    pub fill: Option<Box<dyn PropertyValue<Color>>>,
}

impl FromStr for RectanglePropertiesPatch {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}




/*
We need a patchable properties object

Does `transform` and `size` go into properties or are they special?
What about `id`?

By putting as much as possible into the properties object (e.g. including transform and size),
we make it easier to DRY the `compute_in_place` logic, and to enable fine-grained control of
which elements expose size/transform vs. those for which those properties are irrelevant

What will it look like to set size?

```
//expose a trait method
my_dyn_rendernode.set_size(some_size2d)
//internally, wraps this value into a property coproduct patch, applies it
```


 */




pub enum NodeKind {
    Rectangle,
    Spread,
    Group,
}

pub struct Node<'a> {
    kind:  NodeKind,
    children: Vec<&'a Node<'a>>,
}

// pub struct Component<'a> {
//     template: &'a Node<'a>,
//     properties: PropertiesCoproduct,
// }

pub struct Vtable {
    functions: Vec<fn()>
}



pub struct Metaruntime {
    components: Vec<Component>,
}

impl Metaruntime {

    fn new() -> Self {
        Metaruntime { components: vec![] }
    }

    fn seriealize() {
        unimplemented!()
    }

}