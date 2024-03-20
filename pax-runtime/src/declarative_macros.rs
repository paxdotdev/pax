use pax_runtime_api::{Interpolatable, Property};
use std::rc::Rc;

use crate::{ExpressionTable, Globals, RuntimePropertiesStackFrame};

/// Manages vtable updates (if necessary) for a given `dyn PropertyInstance`.
/// Is a no-op for `PropertyLiteral`s, and mutates (by calling `.set`) `PropertyExpression` instances.
/// # Examples
/// ```text
/// handle_vtable_update!(ptc, self.height, Size);
/// ```
pub fn handle_vtable_update<V: Default + Clone + Interpolatable + 'static>(
    table: &ExpressionTable,
    stack: &Rc<RuntimePropertiesStackFrame>,
    property: &Property<V>,
    globals: &Globals,
) {
    // if let Some(vtable_id) = property._get_vtable_id() {
    //     let new_value_wrapped: Box<dyn Any> = table.compute_vtable_value(&stack, vtable_id);
    //     if let Ok(downcast_value) = new_value_wrapped.downcast::<V>() {
    //         property.set(*downcast_value);
    //     } else {
    //         panic!(
    //             "property has an unexpected type for vtable id {}",
    //             vtable_id
    //         );
    //     }
    // } else if let Some(new_value) =
    //     table.compute_eased_value(property._get_transition_manager(), globals)
    // {
    //     property.set(new_value);
    // } else {
    // }
}

/// Does same as [`handle_vtable_update`], but manages case (as a no-op) where the property is wrapped in an outer Option,
/// e.g. for CommonProperties.
/// # Examples
/// ```text
/// // In this example `scale_x` is `Option`al (`Option<Rc<RefCell<dyn PropertyInstance<Size>>>>`)
/// handle_vtable_update_optional!(ptc, self.scale_x, Size);
/// ```
pub fn handle_vtable_update_optional<V: Default + Clone + Interpolatable + 'static>(
    table: &ExpressionTable,
    stack: &Rc<RuntimePropertiesStackFrame>,
    optional_property: Option<&Property<V>>,
    globals: &Globals,
) {
    // if let Some(property) = optional_property {
    //     handle_vtable_update(table, stack, property, globals);
    // }
}
