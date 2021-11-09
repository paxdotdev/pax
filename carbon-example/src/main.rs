
//TODO: do stand-alone structs require defaults declarations?
//      it seems like "no," since all properties in the properties tree
//      will be specified via Components, which must specify properties
#[expressable]
struct DeeperStruct {
    a: i64,
    b: String,
}

#[properties]
pub struct Main {
    pub num_clicks : i64,
    pub deeper_struct: DeeperStruct,
}

impl Main {

    #[method]
    pub fn increment_clicker(&mut self, args: ClickArgs) {
        self.num_clicks.set(self.num_clicks + 1)
    }

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