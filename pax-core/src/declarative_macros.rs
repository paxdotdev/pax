use std::{any::Any, rc::Rc};

use pax_runtime_api::PropertyInstance;

use crate::{ExpandedNode, ExpressionTable, RuntimePropertiesStackFrame};

/// Manages vtable updates (if necessary) for a given `dyn PropertyInstance`.
/// Is a no-op for `PropertyLiteral`s, and mutates (by calling `.set`) `PropertyExpression` instances.
/// # Examples
/// ```text
/// handle_vtable_update!(ptc, self.height, Size);
/// ```
pub fn handle_vtable_update<V: Default + Clone + std::fmt::Debug + 'static>(
    table: &ExpressionTable,
    stack: &Rc<RuntimePropertiesStackFrame>,
    property: &mut Box<dyn PropertyInstance<V>>,
) {
    if let Some(vtable_id) = property._get_vtable_id() {
        let new_value_wrapped: Box<dyn Any> = table.compute_vtable_value(&stack, vtable_id);
        if let Ok(downcast_value) = new_value_wrapped.downcast::<V>() {
            // pax_runtime_api::log(&format!(
            //     "setting property with value {:?} to {:?}",
            //     property.get(),
            //     downcast_value,
            // ));
            property.set(*downcast_value);
        } else {
            //downcast failed
            // panic!("property has an unexpected type")
        }
    } else {
        // pax_runtime_api::log(&format!(
        //     "node couldn't find v_table id for property with value {:?} on type {}",
        //     property.get(),
        //     std::any::type_name::<V>()
        // ));
    }
}

/// Does same as [`handle_vtable_update`], but manages case (as a no-op) where the property is wrapped in an outer Option,
/// e.g. for CommonProperties.
/// # Examples
/// ```text
/// // In this example `scale_x` is `Option`al (`Option<Rc<RefCell<dyn PropertyInstance<Size>>>>`)
/// handle_vtable_update_optional!(ptc, self.scale_x, Size);
/// ```
pub fn handle_vtable_update_optional<V: Default + Clone + std::fmt::Debug + 'static>(
    table: &ExpressionTable,
    stack: &Rc<RuntimePropertiesStackFrame>,
    optional_property: Option<&mut Box<dyn PropertyInstance<V>>>,
) {
    if let Some(property) = optional_property {
        handle_vtable_update(table, stack, property);
    }
}
