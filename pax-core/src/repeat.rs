use std::cell::RefCell;
use std::rc::Rc;

use crate::{ComponentInstance, InstantiationArgs, InstanceNode, InstanceNodePtr, InstanceNodePtrList, RenderTreeContext, ExpandedNode, PropertiesTreeContext, with_properties_unsafe, unsafe_wrap, unsafe_unwrap, handle_vtable_update_optional};
use pax_properties_coproduct::{PropertiesCoproduct, RepeatProperties, TypesCoproduct};
use pax_runtime_api::{CommonProperties, Layer, PropertyInstance, Size};
use piet_common::RenderContext;

/// A special "control-flow" primitive associated with the `for` statement.
/// Repeat allows for nodes to be rendered dynamically per data specified in `source_expression`.
/// That is: for a `source_expression` of length `n`, `Repeat` will render its
/// template `n` times, each with an embedded component context (`RepeatItem`)
/// with an index `i` and a pointer to that relevant datum `source_expression[i]`
pub struct RepeatInstance<R: 'static + RenderContext> {
    pub instance_id: u32,
    pub repeated_template: InstanceNodePtrList<R>,

    instance_prototypical_properties: Rc<RefCell<PropertiesCoproduct>>,
    instance_prototypical_common_properties: Rc<RefCell<CommonProperties>>,
}

impl<R: 'static + RenderContext> InstanceNode<R> for RepeatInstance<R> {
    fn get_instance_id(&self) -> u32 {
        self.instance_id
    }

    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>>
    where
        Self: Sized,
    {
        let mut node_registry = (*args.node_registry).borrow_mut();
        let instance_id = node_registry.mint_instance_id();
        let ret = Rc::new(RefCell::new(RepeatInstance {
            instance_id,
            repeated_template: match args.children {
                None => Rc::new(RefCell::new(vec![])),
                Some(children) => children,
            },

            instance_prototypical_common_properties: Rc::new(RefCell::new(args.common_properties)),
            instance_prototypical_properties: Rc::new(RefCell::new(args.properties)),
        }));

        node_registry.register(instance_id, Rc::clone(&ret) as InstanceNodePtr<R>);
        ret
    }

    fn expand_node_and_compute_properties(&mut self, ptc: &mut PropertiesTreeContext<R>) -> Rc<RefCell<ExpandedNode<R>>> {

        let this_expanded_node = ExpandedNode::get_or_create_with_prototypical_properties(ptc, &self.instance_prototypical_properties, &self.instance_prototypical_common_properties);
        let properties_wrapped =  this_expanded_node.borrow().get_properties();

        //Mark all of Repeat's existing children (from previous tick) for unmount.  Then, when we iterate and append_children below, ensure that the mark-for-unmount is reverted
        for cen in this_expanded_node.borrow().get_children_expanded_nodes() {
            ptc.engine.node_registry.borrow_mut().mark_for_unmount(cen.borrow().id_chain.clone());
        }

        with_properties_unsafe!(&properties_wrapped, PropertiesCoproduct, RepeatProperties, |properties: &mut RepeatProperties| {
            handle_vtable_update_optional!(ptc, properties.source_expression_range, std::ops::Range<isize>);
            handle_vtable_update_optional!(ptc, properties.source_expression_vec, std::vec::Vec<std::rc::Rc<core::cell::RefCell<PropertiesCoproduct>>>);

            if let Some(ref source) = properties.source_expression_range {
                let range_evaled = source.get();
                let mut index = 0;
                for i in range_evaled.start..range_evaled.end {
                    let i_as_datum = Rc::new(RefCell::new(PropertiesCoproduct::isize(i)));
                    let new_repeat_item = Rc::new(RefCell::new(PropertiesCoproduct::RepeatItem(i_as_datum, index)));

                    ptc.push_stack_frame(new_repeat_item);

                    for repeated_template_instance_root in self.repeated_template.borrow().iter() {
                        let mut new_ptc = ptc.clone();
                        new_ptc.current_expanded_node = None;
                        new_ptc.current_instance_node = Rc::clone(repeated_template_instance_root);
                        let expanded_child = crate::recurse_expand_nodes(&mut new_ptc);
                        ptc.engine.node_registry.borrow_mut().revert_mark_for_unmount(&expanded_child.borrow().id_chain);
                        this_expanded_node.borrow_mut().append_child_expanded_node(expanded_child);
                    }

                    ptc.pop_stack_frame();
                    index = index + 1;
                }

            } else if let Some(ref source) = properties.source_expression_vec {
                let vec_evaled = source.get();

                for pc in vec_evaled.iter().enumerate() {

                    let new_repeat_item = Rc::new(RefCell::new(PropertiesCoproduct::RepeatItem(Rc::clone(pc.1), pc.0)));
                    ptc.push_stack_frame(new_repeat_item);

                    for repeated_template_instance_root in self.repeated_template.borrow().iter() {
                        let mut new_ptc = ptc.clone();
                        new_ptc.current_expanded_node = None;
                        new_ptc.current_instance_node = Rc::clone(repeated_template_instance_root);
                        let expanded_child = crate::recurse_expand_nodes(&mut new_ptc);
                        ptc.engine.node_registry.borrow_mut().revert_mark_for_unmount(&expanded_child.borrow().id_chain);
                        this_expanded_node.borrow_mut().append_child_expanded_node(expanded_child);
                    }

                    ptc.pop_stack_frame()
                }
            }
        });
        this_expanded_node
    }

    fn is_invisible_to_slot(&self) -> bool {
        true
    }

    fn get_instance_children(&self) -> InstanceNodePtrList<R> {
        Rc::clone(&self.repeated_template)
    }

    fn get_layer_type(&mut self) -> Layer {
        Layer::DontCare
    }

    fn manages_own_subtree_for_expansion(&self) -> bool {
        true
    }
}