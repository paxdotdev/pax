
// use pax::runtime::{ClickEvent}

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



// pub fn handle_click(evt: ClickEvent) {
//
// }