use std::cell::RefCell;
use std::iter;
use std::rc::Rc;
use std::{any::Any, ops::Range};

use crate::declarative_macros::handle_vtable_update_optional;
use crate::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use pax_runtime_api::Layer;

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
    pub source_expression_vec:
        Option<Box<dyn pax_runtime_api::PropertyInstance<Vec<Rc<RefCell<dyn Any>>>>>>,
    pub source_expression_range:
        Option<Box<dyn pax_runtime_api::PropertyInstance<std::ops::Range<isize>>>>,
    pub last_len: usize,
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

    fn recompute_children(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &mut RuntimeContext,
    ) {
        let vec = expanded_node.with_properties_unwrapped(|properties: &mut RepeatProperties| {
            if let Some(ref source) = properties.source_expression_range {
                source
                    .get()
                    .clone()
                    .into_iter()
                    .map(|v| Rc::new(RefCell::new(v)) as Rc<RefCell<dyn Any>>)
                    .collect::<Vec<_>>()
            } else if let Some(ref source) = properties.source_expression_vec {
                source.get().clone()
            } else {
                //A valid Repeat must have a repeat source; presumably this has been gated by the parser / compiler
                unreachable!();
            }
        });

        let template_children = self.base().get_template_children();
        let children_with_envs = iter::repeat(template_children)
            .zip(vec.into_iter())
            .enumerate()
            .flat_map(|(i, (children, elem))| {
                let new_repeat_item = Rc::new(RefCell::new(RepeatItem {
                    i,
                    elem: Rc::clone(&elem),
                })) as Rc<RefCell<dyn Any>>;
                let new_env = expanded_node.stack.push(&new_repeat_item);
                children.clone().into_iter().zip(iter::repeat(new_env))
            });
        expanded_node.set_children(children_with_envs, context);
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
        let should_update_children =
            expanded_node.with_properties_unwrapped(|properties: &mut RepeatProperties| {
                handle_vtable_update_optional(
                    context.expression_table(),
                    &expanded_node.stack,
                    properties.source_expression_range.as_mut(),
                );
                handle_vtable_update_optional(
                    context.expression_table(),
                    &expanded_node.stack,
                    properties.source_expression_vec.as_mut(),
                );
                let current_len = properties
                    .source_expression_range
                    .as_ref()
                    .map(|v| v.get())
                    .map(Range::len)
                    .or(properties
                        .source_expression_vec
                        .as_ref()
                        .map(|v| v.get())
                        .map(Vec::len))
                    .unwrap();
                //THIS IS A HACK!!! Will be removed once dirty checking is a thing.
                //Is here to let Stacker re-render children on resize.
                let update_children = current_len != properties.last_len;
                update_children
            });
        if should_update_children {
            self.recompute_children(expanded_node, context);
        }
    }
}
