


struct DeeperStruct {
    a: i64,
    b: String,
}

#[properties] //everything that's `pub` in here becomes a component-scoped property
pub struct Main {

    #[property(0)] //default value is provided to macro in expression language syntax
    pub num_clicks : i64,

    #[property(DeeperStruct {a: 6, b: "hello"})] //explicit `DeeperStruct` is optional; could be {a:6,b:"hello"}
    pub deeper_struct: DeeperStruct
}

//Alternatively, we could have a `properties` data structure
//(which could be trait-friendly with get_ and set_)

impl Main {

    #[method]
    pub fn handle_click(&mut self, args: ClickArgs) {
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