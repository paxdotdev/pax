
/* originally:

#[properties]
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
// derivable! that can then be chained into the #[properties] macro and then wholly automated away,
// as long as all properties are Tweenable in turn.)
//
// If any properties aren't tweenable, the derive would
// cause a compiler failure, which would require not using `properties` if we're not careful
// (perhaps we can pass flags into properties, like `#[properties(no-derive)]` ? )
struct DeeperStruct {
    a: i64,
    b: String,
}


//lib
pub trait Property<T> {
    fn get(&self) -> T;
    fn set(&mut self, newVal: T);
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
#[properties]
pub struct Main {
    pub num_clicks : i64,
    pub deeper_struct: DeeperStruct,
}
 */




pub struct MainProperties {
    pub num_clicks : Box<dyn Property<i64>>, //TODO! support .get and .set
    pub deeper_struct: Box<dyn Property<DeeperStruct>>,
}

todo!("Register Main in the properties manifest, inside #[properties] macro");


todo!("Move this to stand-alone file; generate via macros + dev server");
pub enum PropertiesCoproduct {
    Main(Rc<RefCell<MainProperties>>),
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
impl Patchable<MainPatch> for MainProperties {
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

// originally:
//
// #[methods]
// impl Main {
//
//     pub fn increment_clicker(&mut self, args: ClickArgs) {
//         self.num_clicks.set(self.num_clicks + 1)
//     }
//
// }
//

todo!("connect methods to an event dispatcher; handle instantiating, storing,
and triggering methods based on enum-IDs-as-addresses");



impl RenderNode for MainProperties


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
