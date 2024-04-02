use core::option::Option;

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use pax_runtime_api::properties::Erasable;
use pax_runtime_api::{Numeric, Property};

use crate::api::Layer;
use crate::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};

/// A special "control-flow" primitive (a la `yield` or perhaps `goto`) — represents a slot into which
/// an slot_child can be rendered.  Slot relies on `slot_children` being present
/// on the [`Runtime`] stack and will not render any content if there are no `slot_children` found.
///
/// Consider a Stacker:  the owner of a Stacker passes the Stacker some nodes to render
/// inside the cells of the Stacker.  To the owner of the Stacker, those nodes might seem like
/// "children," but to the Stacker they are "slot_children" — children provided from
/// the outside.  Inside Stacker's template, there are a number of Slots — this primitive —
/// that become the final rendered home of those slot_children.  This same technique
/// is portable and applicable elsewhere via Slot.
pub struct SlotInstance {
    base: BaseInstance,
}

///Contains the index value for slot, either a literal or an expression.
#[derive(Default)]
pub struct SlotProperties {
    // HACK: these two properties are being used in update:
    pub index: Property<Numeric>,
    pub last_node_id: Property<usize>,
    // to compute this:
    pub showing_node: Property<Weak<ExpandedNode>>,
}

impl InstanceNode for SlotInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self>
    where
        Self: Sized,
    {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    layer: Layer::DontCare,
                    is_component: false,
                },
            ),
        })
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RefCell<RuntimeContext>>,
    ) {
        let cloned_expanded_node = Rc::clone(expanded_node);
        let cloned_context = Rc::clone(context);

        // index should be renamed slot_node_id
        let showing_node =
            expanded_node.with_properties_unwrapped(|properties: &mut SlotProperties| {
                properties.showing_node.clone()
            });
        let deps = vec![showing_node.erase()];

        expanded_node
            .children
            .replace_with(Property::computed_with_name(
                move || {
                    // TODO DAG: expanded_and_flattened_slot_children also need to be a property dependency

                    let ret = if let Some(node) = showing_node.get().upgrade() {
                        let res = cloned_expanded_node
                            .attach_children(vec![Rc::clone(&node)], &cloned_context);
                        res
                    } else {
                        cloned_expanded_node.generate_children(vec![], &cloned_context)
                    };
                    ret
                },
                &deps.iter().collect(),
                &format!("slot_children (node id: {})", expanded_node.id_chain[0]),
            ));
    }

    fn update(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        _context: &Rc<RefCell<RuntimeContext>>,
    ) {
        let containing = expanded_node.containing_component.upgrade();
        let nodes = containing
            .as_ref()
            .expect("slot to have a containing component")
            .expanded_and_flattened_slot_children
            .borrow();
        expanded_node.with_properties_unwrapped(|properties: &mut SlotProperties| {
            let node = nodes
                .as_ref()
                .and_then(|l| l.get(properties.index.get().to_int() as usize).cloned());
            let node = match node {
                Some(rc) => Rc::downgrade(&rc),
                None => Weak::new(),
            };
            if properties
                .showing_node
                .get()
                .upgrade()
                .map(|v| v.id_chain[0])
                != node.upgrade().map(|v| v.id_chain[0])
            {
                properties.showing_node.set(node);
            }
        });
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Slot").finish()
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}
