use std::{any::Any, rc::Rc};

use crate::api::PropertyInstance;

use crate::{ExpressionTable, RuntimePropertiesStackFrame};

/// Manages vtable updates (if necessary) for a given `dyn PropertyInstance`.
/// Is a no-op for `PropertyLiteral`s, and mutates (by calling `.set`) `PropertyExpression` instances.
/// # Examples
/// ```text
/// handle_vtable_update!(ptc, self.height, Size);
/// ```
pub fn handle_vtable_update<V: Default + Clone + 'static>(
    table: &ExpressionTable,
    stack: &Rc<RuntimePropertiesStackFrame>,
    property: &mut Box<dyn PropertyInstance<V>>,
) {
    todo!("cast to an intermediate known type for RIL statement in expression vtable.  (using : ExplicitType and .into())\
    Ensure this type is codified before passing into the Box<dyn Any>.  This should fix the downcasting-and-also-need-to-.into() on the other side of dyn Any");
    if let Some(vtable_id) = property._get_vtable_id() {
        let new_value_wrapped: Box<dyn Any> = table.compute_vtable_value(&stack, vtable_id);
        if let Ok(downcast_value) = new_value_wrapped.downcast::<V>() {
            property.set(*downcast_value);
        } else {
            panic!("property has an unexpected type")
        }
    } else {
    }
}

/// Does same as [`handle_vtable_update`], but manages case (as a no-op) where the property is wrapped in an outer Option,
/// e.g. for CommonProperties.
/// # Examples
/// ```text
/// // In this example `scale_x` is `Option`al (`Option<Rc<RefCell<dyn PropertyInstance<Size>>>>`)
/// handle_vtable_update_optional!(ptc, self.scale_x, Size);
/// ```
pub fn handle_vtable_update_optional<V: Default + Clone + 'static>(
    table: &ExpressionTable,
    stack: &Rc<RuntimePropertiesStackFrame>,
    optional_property: Option<&mut Box<dyn PropertyInstance<V>>>,
) {
    if let Some(property) = optional_property {
        handle_vtable_update(table, stack, property);
    }
}
