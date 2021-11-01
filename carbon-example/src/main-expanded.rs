
/* originally:

#[expressable]
struct DeeperStruct {
    a: i64,
    b: String,
}

 */


// Do we need to wrap the following in Property<> ?  What if DeeperStruct is already
// wrapped in a Property in the containing component?
// Refer to how Rectangle `fill` already does Box<dyn Property<Color>> — this suggests
// 'no,' we don't need to wrap DeeperStruct's members in Property<>
//
// That said, it will be a userland responsibility to implement `Tweenable` (or better yet, make it
// derivable! that can then be chained into the #[expressable] macro and then wholly automated away,
// as long as all properties are Tweenable in turn.)
//
// If any properties aren't tweenable, the derive would
// cause a compiler failure, which would require not using `expressable` if we're not careful
// (perhaps we can pass flags into expressable, like `#[expressable(no-derive)]` ? )
struct DeeperStruct {
    a: i64,
    b: String,
}

todo!("Register DeeperStruct in the propertiescoproduct & manifest");


#[cfg(feature="metaruntime")]
lazy_static! {
    static ref DEEPER_STRUCT_MANIFEST: Vec<(&'static str, &'static str)> = {
        vec![
            ("a", "Number"),
            ("b", "String"),
        ]
    };
}

#[cfg(feature="metaruntime")]
impl Manifestable for DeeperStruct {
    fn get_type_identifier() -> &'static str {
        &"DeeperStruct"
    }
    fn get_manifest() -> &'static Vec<(&'static str, &'static str)> {
        DEEPER_STRUCT_MANIFEST.as_ref()
    }
}

#[cfg(feature="metaruntime")]
impl Patchable<DeeperStructPatch> for DeeperStruct {
    fn patch(&mut self, patch: DeeperStructPatch) {
        if let Some(p) = patch.a {
            self.a = p;
        }
        if let Some(p) = patch.b {
            self.b = b;
        }
    }
}

#[cfg(feature="metaruntime")]
pub struct DeeperStructPatch {
    pub a: Option<i64>,
    pub b: Option<String>,
}

#[cfg(feature="metaruntime")]
impl Default for DeeperStructPatch {
    fn default() -> Self {
        DeeperStructPatch {
            a: None,
            b: None,
        }
    }
}

#[cfg(feature="metaruntime")]
impl FromStr for DeeperStructPatch {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}




/* originally:
#[component(Main {
    num_clicks: 5,
    deeper_struct: {
        a: 42,
        b: "Profundo!",
    }
})] //everything that's `pub` in here becomes a component-scoped property
pub struct Main {

    pub num_clicks : i64,

    pub deeper_struct: DeeperStruct,

}
 */

pub struct Main {

    pub num_clicks : Box<dyn Property<i64>>,
    pub deeper_struct: Box<dyn Property<DeeperStruct>>,

}

todo!("Register Main in the properties manifest");



//TODO: generate this.
//TODO: how can we ensure this represents ALL properties? There's a macro
//      sequencing puzzle here.
//      One possibility:  compile twice (e.g. a manual rustc call from the pre-compile script.)
//                        After the first compilation, our manifest file should be complete.
//                        Upon the second compilation, that manifest can feed into this definition.
//      Another possibility: static analysis (blech)
//      Another possibility: introduce another process/server; each properties registration macro
//                           communicates to that server
//  *** Another possibility: idempotent, "full service" macros for each properties registration,
//                           which side-effectfully write to the propertiescoproduct as well as the manifest.
//                           In this scenario, we do not need to decorate the central Coproduct definition
//                           with a macro (or if we do, only to make it discoverable for static analysis)
//                           This approach relies on ALL macros being expanded before any deeper compilation
//                           occurs (see the description of the macro queue here: https://rustc-dev-guide.rust-lang.org/macro-expansion.html#expansion-and-ast-integration )



pub enum PropertiesCoproduct {
    DevAppRoot(Rc<RefCell<DevAppRootProperties>>),
    RepeatItem(Rc<RefCell<RepeatItem>>),
    Spread(Rc<RefCell<SpreadProperties>>),
    SpreadCell(Rc<SpreadCellProperties>),
    Rectangle(Rc<Rectangle>),
    Group(Rc<Group>),
    Placeholder(Rc<Placeholder>),
    Repeat(Rc<Repeat>),
    Text(Rc<Text>),
    Empty,
}




#[cfg(feature="metaruntime")]
lazy_static! {
    static ref MAIN_MANIFEST: Vec<(&'static str, &'static str)> = {
        vec![
            ("num_clicks", "Number"),
            ("deeper_struct", "DeeperStruct"),
        ]
    };
}

#[cfg(feature="metaruntime")]
impl Manifestable for Main {
    fn get_type_identifier() -> &'static str {
        &"Main"
    }
    fn get_manifest() -> &'static Vec<(&'static str, &'static str)> {
        MAIN_MANIFEST.as_ref()
    }
}

#[cfg(feature="metaruntime")]
impl Patchable<MainPatch> for Main {
    fn patch(&mut self, patch: MainPatch) {
        if let Some(p) = patch.num_clicks {
            self.num_clicks = p;
        }
        if let Some(p) = patch.deeper_struct {
            self.b = b;
        }
    }
}

#[cfg(feature="metaruntime")]
pub struct MainPatch {
    pub num_clicks: Option<i64>,
    pub deeper_struct: Option<DeeperStruct>,
}

#[cfg(feature="metaruntime")]
impl Default for MainPatch {
    fn default() -> Self {
        MainPatch {
            num_clicks: None,
            deeper_struct: None,
        }
    }
}

#[cfg(feature="metaruntime")]
impl FromStr for MainPatch {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}








#[methods]
impl Main {

    pub fn increment_clicker(&mut self, args: ClickArgs) {
        self.num_clicks.set(self.num_clicks + 1)
    }

}


/* Approaches for dirty-handling:
    - Check dataframes on each tick (brute-force)
    - inject a setter, ideally with primitive ergonomics (`self.x = self.x + 1`)
        probably done with a macro decorating the struct field
        - setter(a): generate a `set_field_name<T>(new: T)` method for each decorated `field_name: T`
       ***setter(b):   `num_clicks: T` becomes `self.num_clicks.get() //-> T` and `self.num_clicks.set(new: T)`
                       in the expression language, `num_clicks` automatically unwraps `get()`
                       `.get()` feels fine for Rust ergonomics, in line with `unwrap()`
                       `.set(new: T)` is also not the worst, even if it could be better.
                       In TS we can have better ergonomics with `properties`
 */
