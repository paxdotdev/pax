use pax::*;


//Can support #[pax] in order to offer granular value-setting
pub struct DeeperStruct {
    a: i64,
    b: &'static str,
}

//rewrite to pub `num_clicks : Property<i64>` etc. AND register metadata with dev server
//Note re: dependencies —
//  - The central PropertiesCoproduct _depends on_ this definition, in order to wrap it into the PropertiesCoproduct
//  - This means that this file cannot directly rely on pax-properties-coproduct.  To do so would introduce a cyclic dep.
//    In particular, be mindful of this when designing macro expansion

#[pax] //could make file ref. explicit: #[pax(file="lib.pax")]
       //in absence, .pax file path is inferred by source name (and `is_present(inline_pax)`)
       //e.g. lib.rs -> try to load lib.pax.  don't try to load .pax if inline_pax is present
pub struct Root {
    pub num_clicks : i64,
    pub current_rotation: f64,
    pub deeper_struct: DeeperStruct,
}


//Might want to support #[pax] here in order to track method definitions
impl Root {

    pub fn new() -> Self {
        Self {
            //Default values.  Could shorthand this into a macro via PAXEL
            num_clicks: 0,
            current_rotation: 0.0,
            deeper_struct: DeeperStruct {
                a: 100,
                b: "Profundo!",
            }
        }
    }

    //On click, increment num_clicks and update the rotation

    //Note the userland ergonomics here, using .get() and .set()
    //vs. the constructor and struct definition of bare types (e.g. i64, which doesn't have a .get() or .set() method)
    //Approaches:
    // - rewrite the struct at macro time; also rewrite the constructor
    // - inject something other than self into increment_clicker, including a .gettable and .settable wrapper
    //   around (note that this injected struct, if it's going to have the pattern struct.num_clicks.set, will
    //   still require some codegen; can't be achieved with generics alone
    
    
    // pub fn increment_clicker(&mut self, args: ClickArgs) {
    //     self.num_clicks.set(self.num_clicks + 1);
    //     self.current_rotation.setTween( //also: setTweenLater, to enqueue a tween after the current (if any) is done
    //         self.num_clicks.get() * 3.14159 / 4,
    //         Tween {duration: 1000, curve: Tween::Ease}
    //     );
    // }

}


/* Approaches for dirty-handling of properties:
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




//DONE: is all descendent property access via Actions + selectors? `$('#some-desc').some_property`
//      or do we need a way to support declaring desc. properties?
//      We do NOT need a way to declar desc. properties here — because they are declared in the
//      `properties` blocks of .dash