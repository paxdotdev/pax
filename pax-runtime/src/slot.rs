use std::rc::{Rc, Weak};
use_RefCell!();

use pax_runtime_api::{use_RefCell, ImplToFromPaxAny, Numeric, Property};

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

impl ImplToFromPaxAny for Slot {}

///Contains the index value for slot, either a literal or an expression.
#[derive(Default)]
pub struct Slot {
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
                    is_slot: true,
                },
            ),
        })
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        let weak_ref_self = Rc::downgrade(expanded_node);
        let cloned_context = Rc::clone(context);

        let containing = expanded_node.containing_component.upgrade();
        let containing = &containing
            .as_ref()
            .expect("slot to have a containing component");

        let nodes = containing.expanded_and_flattened_slot_children.clone();
        let listener = containing.slot_child_attached_listener.clone();

        let index = expanded_node
            .with_properties_unwrapped(|properties: &mut Slot| properties.index.clone());
        let deps = vec![index.untyped(), nodes.untyped()];

        expanded_node
            .children
            .replace_with(Property::computed_with_name(
                move || {
                    let Some(cloned_expanded_node) = weak_ref_self.upgrade() else {
                        panic!("ran evaluator after expanded node dropped (repeat elem)")
                    };
                    let node_rc =
                        nodes.read(|nodes| nodes.get(index.get().to_int() as usize).cloned());
                    let node = match &node_rc {
                        Some(rc) => Rc::downgrade(rc),
                        None => Weak::new(),
                    };
                    let ret = cloned_expanded_node.attach_children(
                        node.upgrade().as_slice().to_vec(),
                        &cloned_context,
                        &cloned_expanded_node.parent_frame,
                    );
                    listener.set(());
                    ret
                },
                &deps,
                &format!("slot_children (node id: {})", expanded_node.id.0),
            ));
    }

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
