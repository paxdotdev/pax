use std::any::Any;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::iter;
use std::rc::Rc;

use pax_runtime_api::Property;

use crate::api::Layer;
use crate::declarative_macros::handle_vtable_update_optional;
use crate::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};

/// A special "control-flow" primitive associated with the `for` statement.
/// Repeat allows for nodes to be rendered dynamically per data specified in `source_expression`.
/// That is: for a `source_expression` of length `n`, `Repeat` will render its
/// template `n` times, each with an embedded component context (`RepeatItem`)
/// with an index `i` and a pointer to that relevant datum `source_expression[i]`
pub struct RepeatInstance {
    pub base: BaseInstance,
}

///Contains modal _vec_ and _range_ variants, describing whether the Repeat source
///is encoded as a Vec<T> (where T is a `dyn Any` properties type) or as a Range<isize>
#[derive(Default)]
pub struct RepeatProperties {
    pub source_expression_vec: Option<Property<Vec<Rc<RefCell<dyn Any>>>>>,
    pub source_expression_range: Option<Property<std::ops::Range<isize>>>,
    last_len: usize,
    last_bounds: (f64, f64),
}

pub struct RepeatItem {
    pub elem: Rc<RefCell<dyn Any>>,
    pub i: usize,
}

impl InstanceNode for RepeatInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self>
    where
        Self: Sized,
    {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: true,
                    invisible_to_raycasting: true,
                    layer: Layer::DontCare,
                    is_component: false,
                },
            ),
        })
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Repeat").finish()
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn update(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, context: &mut RuntimeContext) {
        let new_vec =
            expanded_node.with_properties_unwrapped(|properties: &mut RepeatProperties| {
                handle_vtable_update_optional(
                    context.expression_table().borrow(),
                    &expanded_node.stack,
                    properties.source_expression_range.as_ref(),
                    context.globals(),
                );
                handle_vtable_update_optional(
                    context.expression_table().borrow(),
                    &expanded_node.stack,
                    properties.source_expression_vec.as_ref(),
                    context.globals(),
                );

                let vec = if let Some(ref source) = properties.source_expression_range {
                    Box::new(
                        source
                            .get()
                            .clone()
                            .map(|v| Rc::new(RefCell::new(v)) as Rc<RefCell<dyn Any>>),
                    ) as Box<dyn ExactSizeIterator<Item = Rc<RefCell<dyn Any>>>>
                } else if let Some(ref source) = properties.source_expression_vec {
                    Box::new(source.get().clone().into_iter())
                        as Box<dyn ExactSizeIterator<Item = Rc<RefCell<dyn Any>>>>
                } else {
                    //A valid Repeat must have a repeat source; presumably this has been gated by the parser / compiler
                    unreachable!();
                };

                let current_len = vec.len();

                let exp_props = expanded_node.layout_properties.borrow();
                let current_bounds = exp_props
                    .as_ref()
                    .map(|t| t.computed_tab.bounds)
                    .unwrap_or_default();
                let update_children =
                    current_len != properties.last_len || current_bounds != properties.last_bounds;

                properties.last_len = current_len;
                properties.last_bounds = current_bounds;
                update_children.then_some(vec)
            });

        if let Some(vec) = new_vec {
            let template_children = self.base().get_instance_children();
            let children_with_envs = iter::repeat(template_children)
                .zip(vec.into_iter())
                .enumerate()
                .flat_map(|(i, (children, elem))| {
                    let new_repeat_item = Rc::new(RefCell::new(RepeatItem {
                        i,
                        elem: Rc::clone(&elem),
                    })) as Rc<RefCell<dyn Any>>;
                    let property_names = expanded_node.properties_scope.borrow();
                    let new_env = expanded_node
                        .stack
                        .push(property_names.clone(), &new_repeat_item);
                    children
                        .borrow()
                        .clone()
                        .into_iter()
                        .zip(iter::repeat(new_env))
                });
            expanded_node.set_children(children_with_envs, context);
        }
    }

    fn handle_mount(&self, _expanded_node: &Rc<ExpandedNode>, _context: &mut RuntimeContext) {
        // No-op: wait with creating child-nodes until update tick, since the
        // condition has then been evaluated
    }
}
