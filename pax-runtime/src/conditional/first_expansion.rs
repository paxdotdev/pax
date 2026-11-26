use super::{ConditionalInstance, ExpandedNode, InstanceNode, LocalProperty, RuntimeContext};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

// The child-list property owns this cursor. The admission queue borrows it weakly:
// a synchronous read, rebind or teardown retires the work without a second commit.
pub(crate) struct FirstExpansion {
    owner: Weak<ExpandedNode>,
    epoch: u64,
    instance: Rc<ConditionalInstance>,
    conditions: Vec<LocalProperty<bool>>,
    branch: Cell<Option<usize>>,
    children: RefCell<Vec<Rc<ExpandedNode>>>,
}

impl FirstExpansion {
    pub(super) fn new(
        node: &Rc<ExpandedNode>,
        instance: &Rc<ConditionalInstance>,
        conditions: &[LocalProperty<bool>],
    ) -> Rc<Self> {
        Rc::new(Self {
            owner: Rc::downgrade(node),
            epoch: node.mount_epoch.get(),
            instance: Rc::clone(instance),
            conditions: conditions.to_vec(),
            branch: Cell::new(None),
            children: RefCell::new(Vec::new()),
        })
    }

    pub(crate) fn owner(&self) -> Option<Rc<ExpandedNode>> {
        self.owner
            .upgrade()
            .filter(|n| n.attached.get() != 0 && n.mount_epoch.get() == self.epoch)
    }

    // Each service unit constructs at most one child. Only the normal children
    // evaluator publishes the complete list and registers its mounts in order.
    pub(crate) fn prepare_one(&self, ctx: &Rc<RuntimeContext>) -> bool {
        let Some(owner) = self.owner() else {
            return true;
        };
        let branch = self.conditions.iter().position(|c| c.get());
        if self.branch.replace(branch) != branch {
            self.children.borrow_mut().clear();
        }
        let index = self.children.borrow().len();
        let next = {
            let templates = self.instance.base().get_instance_children().borrow();
            let range = self.instance.selected_range(branch, templates.len());
            // Let the normal evaluator construct the last child and attach the
            // whole list. This also handles empty and single-child selections.
            (index + 1 < range.len()).then(|| templates[range.start + index].clone())
        };
        if let Some(template) = next {
            let child = owner.create_conditional_child(&template, ctx);
            if !template.is_overlay() {
                child.ensure_parent_bindings(
                    &owner,
                    ctx,
                    &owner.parent_frame,
                    &owner.liquid_glass_scope,
                );
            }
            self.children.borrow_mut().push(child);
            false
        } else {
            owner.children.read(|_| ());
            // An earlier settlement may have seen only the mounted prefix.
            owner.children_listener.invalidate();
            true
        }
    }

    pub(super) fn take_children(&self, branch: Option<usize>) -> Vec<Rc<ExpandedNode>> {
        if self.branch.get() == branch && self.owner().is_some() {
            self.children.take()
        } else {
            Vec::new()
        }
    }
}
