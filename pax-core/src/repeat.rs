use std::cell::RefCell;
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
        _ptc: &mut RuntimeContext,
    ) {
        let (range_evaled, vec_evaled) =
            expanded_node.with_properties_unwrapped(|properties: &mut RepeatProperties| {
                if let Some(ref source) = properties.source_expression_range {
                    (Some(source.get().clone()), None)
                } else if let Some(ref source) = properties.source_expression_vec {
                    let vec_evaled = source.get();
                    (None, Some(vec_evaled.clone()))
                } else {
                    unreachable!(); //A valid Repeat must have a repeat source; presumably this has been gated by the parser / compiler
                }
            });

        //THIS IS A HACK!!! Will be removed once dirty checking is a thing.
        //Is here to let Stacker re-render children on resize.
        let _source_len = range_evaled
            .as_ref()
            .map(Range::len)
            .or(vec_evaled.as_ref().map(Vec::len))
            .unwrap();

        //Mark all of Repeat's existing children (from previous tick) for
        //unmount.  Then, when we iterate and append_children below, ensure
        //that the mark-for-unmount is reverted This enables changes in repeat
        //source to be mapped to new elements (unchanged elements are marked for
        //unmount / remount before unmount handlers are fired, resulting in no
        //effective changes for persistent nodes.)

        let _vec_range_source = vec_evaled
            .or(range_evaled.map(|v| {
                v.map(|i| Rc::new(RefCell::new(i)) as Rc<RefCell<dyn Any>>)
                    .collect::<Vec<_>>()
            }))
            .unwrap();

        // for (i, elem) in vec_range_source.iter().enumerate() {
        //     let new_repeat_item = Rc::new(RefCell::new(RepeatItem {
        //         i,
        //         elem: Rc::clone(elem),
        //     }));

        //     for child in self.base().get_template_children() {
        //         expanded_node.expand_child(child)
        //     }
        // }

        // this_expanded_node
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
        });
    }
}
