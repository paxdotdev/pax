use std::cell::RefCell;
use std::rc::Rc;
use std::{any::Any, ops::Range};

use crate::{
    handle_vtable_update_optional, with_properties_unwrapped, BaseInstance, ExpandedNode,
    InstanceFlags, InstanceNode, InstantiationArgs, PropertiesComputable, PropertiesTreeContext,
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
                },
            ),
        })
    }

    fn expand(self: Rc<Self>, ptc: &mut PropertiesTreeContext) -> Rc<RefCell<ExpandedNode>> {
        let this_expanded_node = self
            .base()
            .expand_from_instance(Rc::clone(&self) as Rc<dyn InstanceNode>, ptc);

        let properties_wrapped = this_expanded_node.borrow().get_properties();

        //Mark all of Repeat's existing children (from previous tick) for
        //unmount.  Then, when we iterate and append_children below, ensure
        //that the mark-for-unmount is reverted This enables changes in repeat
        //source to be mapped to new elements (unchanged elements are marked for
        //unmount / remount before unmount handlers are fired, resulting in no
        //effective changes for persistent nodes.)

        let (range_evaled, vec_evaled) = with_properties_unwrapped!(
            &properties_wrapped,
            RepeatProperties,
            |properties: &mut RepeatProperties| {
                handle_vtable_update_optional!(
                    ptc,
                    this_expanded_node,
                    properties.source_expression_range,
                    std::ops::Range<isize>
                );
                handle_vtable_update_optional!(
                    ptc,
                    this_expanded_node,
                    properties.source_expression_vec,
                    std::vec::Vec<std::rc::Rc<core::cell::RefCell<dyn Any>>>
                );

                if let Some(ref source) = properties.source_expression_range {
                    (Some(source.get().clone()), None)
                } else if let Some(ref source) = properties.source_expression_vec {
                    let vec_evaled = source.get();
                    (None, Some(vec_evaled.clone()))
                } else {
                    unreachable!(); //A valid Repeat must have a repeat source; presumably this has been gated by the parser / compiler
                }
            }
        );

        let mut node = this_expanded_node.borrow_mut();

        //THIS IS A HACK!!! Will be removed once dirty checking is a thing.
        //Is here to let Stacker re-render children on resize.
        let source_len = range_evaled
            .as_ref()
            .map(Range::len)
            .or(vec_evaled.as_ref().map(Vec::len))
            .unwrap();
        let update_repeat_children = node.tab_changed || source_len != node.last_repeat_source_len;
        node.last_repeat_source_len = source_len;

        drop(node);

        if !update_repeat_children {
            return this_expanded_node;
        }

        for cen in this_expanded_node.borrow().get_children_expanded_nodes() {
            ptc.engine
                .node_registry
                .borrow_mut()
                .mark_for_unmount(cen.borrow().id_chain.clone());
        }

        let vec_range_source = vec_evaled
            .or(range_evaled.map(|v| {
                v.map(|i| Rc::new(RefCell::new(i)) as Rc<RefCell<dyn Any>>)
                    .collect::<Vec<_>>()
            }))
            .unwrap();

        {
            this_expanded_node.borrow_mut().clear_child_expanded_nodes();
        }

        for (i, elem) in vec_range_source.iter().enumerate() {
            let new_repeat_item = Rc::new(RefCell::new(RepeatItem {
                i,
                elem: Rc::clone(elem),
            }));
            ptc.push_stack_frame(new_repeat_item);

            for child in self.base().get_children().iter() {
                let mut new_ptc = ptc.clone();
                let id_chain = ptc.get_id_chain(child.base().get_instance_id());

                //Part of hack (see above)
                new_ptc
                    .engine
                    .node_registry
                    .borrow_mut()
                    .remove_expanded_node(&id_chain);

                let expanded_child = Rc::clone(&child).expand(&mut new_ptc);
                expanded_child.borrow_mut().parent_expanded_node =
                    Rc::downgrade(&this_expanded_node);

                new_ptc
                    .engine
                    .node_registry
                    .borrow_mut()
                    .expanded_node_map
                    .insert(id_chain, Rc::clone(&expanded_child));

                new_ptc
                    .engine
                    .node_registry
                    .borrow_mut()
                    .revert_mark_for_unmount(&expanded_child.borrow().id_chain);

                this_expanded_node
                    .borrow_mut()
                    .append_child_expanded_node(expanded_child);
            }

            ptc.pop_stack_frame()
        }

        this_expanded_node
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
}
