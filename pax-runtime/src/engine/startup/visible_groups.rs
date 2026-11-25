//! Admission of a component's first mixed native and canvas output.
//! A repeat's items are independent. A projected slot keeps the actual render
//! parent, so a card's authored media and lettering can settle together.
use super::{
    admission::{AdmissionQueue, Priority},
    PendingMount,
};
use crate::{
    api::Layer, viewport_proximity::AdmissionView, ExpandedNode, InstanceNodePtr, RuntimeContext,
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    rc::{Rc, Weak},
};

type Id = u32;
#[derive(Clone, Copy, Default)]
struct Shape {
    native: bool,
    canvas: bool,
    discovery: bool,
}
impl Shape {
    fn include(&mut self, other: Self) {
        self.native |= other.native;
        self.canvas |= other.canvas;
        self.discovery |= other.discovery;
    }
}
#[derive(Default)]
struct Lane {
    root: Weak<ExpandedNode>,
    parent: Weak<ExpandedNode>,
    age: u32,
    publication: u64,
    mount_epoch: u64,
    jobs: BTreeMap<Id, PendingMount>,
    resources: BTreeMap<Id, Weak<ExpandedNode>>,
    mounted: bool,
    settled: bool,
    members: Vec<Id>,
}
#[derive(Default)]
pub(crate) struct VisibleGroups {
    enabled: bool,
    park: bool,
    published: bool,
    next_publication: u64,
    // Holding the immutable instance keeps the address valid for this cache.
    shapes: HashMap<usize, (InstanceNodePtr, Shape)>,
    lanes: BTreeMap<Id, Lane>,
    membership: HashMap<Id, Id>,
    discovery: BTreeMap<Id, PendingMount>,
    changed: BTreeSet<Id>,
    turns: u32,
    pub(crate) opened: u32,
    pub(crate) preferred: u32,
    pub(crate) parked: u32,
}
impl VisibleGroups {
    pub(crate) fn configure(&mut self, park: bool) {
        self.enabled = true;
        self.park = park;
    }
    // A component boundary owns the native content in its own template and
    // call-site children. Nested component templates have their own native
    // obligations. Vector backing may be supplied by a nested component.
    fn shape(&mut self, instance: &InstanceNodePtr) -> Shape {
        let key = Rc::as_ptr(instance) as *const () as usize;
        if let Some((_, shape)) = self.shapes.get(&key) {
            return *shape;
        }
        self.shapes
            .insert(key, (instance.clone(), Shape::default()));
        let base = instance.base();
        let f = base.flags();
        let mut result = Shape {
            native: matches!(f.layer, Layer::Native | Layer::NativeNonOccluding),
            canvas: matches!(f.layer, Layer::Canvas),
            discovery: f.is_slot,
        };
        if let Some(template) = instance.get_template() {
            for child in template.borrow().iter() {
                self.include_child(&mut result, child);
            }
        }
        for child in base.get_instance_children().borrow().iter() {
            self.include_child(&mut result, child);
        }
        if instance.repeats_admission_items() {
            result.discovery |= result.native && result.canvas;
            result.native = false;
            result.canvas = false;
        }
        result.discovery |= f.is_component && result.native && result.canvas;
        self.shapes.insert(key, (instance.clone(), result));
        result
    }
    fn include_child(&mut self, dst: &mut Shape, child: &InstanceNodePtr) {
        let mut shape = self.shape(child);
        if child.base().flags().is_component {
            // Content authored between these component tags still belongs
            // to the caller, including when delivered through nested slots.
            shape.native = false;
            for payload in child.base().get_instance_children().borrow().iter() {
                self.include_child(&mut shape, payload);
            }
        }
        dst.include(shape);
    }
    pub(super) fn queue(
        &mut self,
        parent: &Rc<ExpandedNode>,
        child: &Rc<ExpandedNode>,
        work: &PendingMount,
    ) {
        self.register(parent, child, Some(work));
    }

    pub(super) fn first_viewport_published(&mut self) {
        self.published = self.enabled;
    }

    pub(in crate::engine) fn immediate(
        &mut self,
        parent: &Rc<ExpandedNode>,
        child: &Rc<ExpandedNode>,
    ) {
        if self.published {
            self.register(parent, child, None);
            self.before_mount(child, true);
        }
    }

    // Replacing the selected output of an already mounted conditional keeps the
    // conditional's lifetime, but invalidates readiness for its previous output.
    pub(in crate::engine) fn branch_changed(
        &mut self,
        root: &Rc<ExpandedNode>,
        children: &[Rc<ExpandedNode>],
    ) {
        if !self.published || !root.instance_node.borrow().selects_admission_branch() {
            return;
        }
        let id = root.id.to_u32();
        if self.membership.get(&id).is_some_and(|group| *group != id) {
            return; // The enclosing group owns these newly discovered resources.
        }
        if self.remove_lane(id).is_some() {
            root.admission_parked.set(false);
        }
        // Surviving children and authored exit transitions retain their presentation.
        if children.is_empty()
            || !root.exiting_children.borrow().is_empty()
            || children.iter().any(|child| child.attached.get() != 0)
        {
            return;
        }
        let mut shape = Shape::default();
        for child in children {
            self.include_child(&mut shape, &child.instance_node.borrow());
        }
        if shape.native && shape.canvas && self.park {
            self.add_lane(root, root, u32::MAX);
            self.membership.insert(id, id);
            self.lanes.get_mut(&id).unwrap().members.push(id);
            self.before_mount(root, true);
        }
    }

    fn add_lane(&mut self, root: &Rc<ExpandedNode>, parent: &Rc<ExpandedNode>, age: u32) {
        let id = root.id.to_u32();
        self.lanes.entry(id).or_insert_with(|| {
            self.next_publication = self
                .next_publication
                .checked_add(1)
                .expect("admission publication exhausted");
            Lane {
                root: Rc::downgrade(root),
                parent: Rc::downgrade(parent),
                age,
                publication: self.next_publication,
                ..Default::default()
            }
        });
    }

    fn register(
        &mut self,
        parent: &Rc<ExpandedNode>,
        child: &Rc<ExpandedNode>,
        work: Option<&PendingMount>,
    ) {
        if !self.enabled {
            return;
        }
        let id = child.id.to_u32();
        let instance = child.instance_node.borrow();
        if instance.is_overlay() || child.is_render_source() {
            return;
        }
        let shape = self.shape(&instance);
        // A bare slot in a repeat forwards the caller’s payload (for example a link).
        // A repeated item wrapper starts its own visual lifetime.
        let inherited = (!parent.instance_node.borrow().repeats_admission_items()
            || instance.base().flags().is_slot)
            .then(|| self.membership.get(&parent.id.to_u32()).copied())
            .flatten()
            .filter(|id| self.lanes.contains_key(id));
        let owner = inherited.or_else(|| {
            if !instance.base().flags().is_component || !shape.native || !shape.canvas {
                return None;
            }
            self.add_lane(child, parent, work.map_or(u32::MAX, |w| w.arrival));
            Some(id)
        });
        if let Some(owner) = owner {
            if let Some(lane) = self.lanes.get_mut(&owner) {
                if let Some(work) = work {
                    lane.jobs.insert(id, work.clone());
                }
                lane.settled = false;
                self.changed.insert(owner);
                if self.membership.insert(id, owner) != Some(owner) {
                    lane.members.push(id);
                }
                if instance.awaits_native_first_content() {
                    lane.resources.insert(id, Rc::downgrade(child));
                }
                return;
            }
        }
        if let Some(work) = work.filter(|_| shape.discovery) {
            self.discovery.insert(id, work.clone());
        }
    }
    pub(in crate::engine) fn scheduling_anchor(
        &self,
        parent: &Rc<ExpandedNode>,
        child: &Rc<ExpandedNode>,
        age: u32,
    ) -> (Option<Id>, Rc<ExpandedNode>, u32) {
        let id = child.id.to_u32();
        if let Some((group, anchor)) = self.capture_anchor(id) {
            return (Some(group), anchor, self.lanes[&group].age.min(age));
        }
        // Attachment already installed the child's parent bindings. A repeat's
        // nominal rectangle can sit at the start of a very long scroll extent
        // while its explicitly positioned slot wrapper is in the viewport.
        if self.discovery.contains_key(&id) {
            let props = child.get_common_properties();
            let props = props.borrow();
            if props.width.get().is_some() && props.height.get().is_some() {
                return (None, child.clone(), age);
            }
        }
        (None, parent.clone(), age)
    }
    pub(in crate::engine) fn capture_anchor(&self, child: Id) -> Option<(Id, Rc<ExpandedNode>)> {
        let group = *self.membership.get(&child)?;
        Some((group, Self::candidate(self.lanes.get(&group)?)?))
    }
    fn candidate(lane: &Lane) -> Option<Rc<ExpandedNode>> {
        if lane.mounted {
            lane.root.upgrade()
        } else {
            lane.parent.upgrade()
        }
    }
    pub(super) fn next(
        &mut self,
        ctx: &RuntimeContext,
        view: &mut AdmissionView,
        queue: &mut AdmissionQueue,
    ) -> Option<PendingMount> {
        if !self.enabled {
            return None;
        }
        self.turns = self.turns.wrapping_add(1);
        if self.turns % 8 == 0 {
            return None;
        } // Keep the ordinary queue's oldest-job service.
        for lane in self.lanes.values() {
            if lane.jobs.is_empty() {
                continue;
            }
            let Some(node) = Self::candidate(lane) else {
                continue;
            };
            if node.attached.get() == 0 || ctx.admission_priority(&node, view) != Priority::Visible
            {
                continue;
            }
            for job in lane.jobs.values() {
                if let Some(work) = queue.pop_specific(job) {
                    self.preferred += 1;
                    return Some(work);
                }
            }
        }
        for job in self.discovery.values() {
            let Some(parent) = job.parent.upgrade() else {
                continue;
            };
            if parent.attached.get() != 0
                && ctx.admission_priority(&parent, view) == Priority::Visible
            {
                if let Some(work) = queue.pop_specific(job) {
                    self.preferred += 1;
                    return Some(work);
                }
            }
        }
        None
    }
    pub(super) fn before_mount(&mut self, child: &Rc<ExpandedNode>, spatial: bool) {
        if !self.enabled {
            return;
        }
        let id = child.id.to_u32();
        self.discovery.remove(&id);
        if let Some(lane) = self.lanes.get_mut(&id) {
            lane.mounted = true;
            lane.mount_epoch = child
                .mount_epoch
                .get()
                .wrapping_add(u64::from(child.attached.get() == 0));
            if self.park && spatial {
                child.admission_parked.set(true);
                self.parked += 1;
            }
        }
        if let Some(lane) = self.membership.get(&id).and_then(|g| self.lanes.get_mut(g)) {
            lane.jobs.remove(&id);
            lane.settled = false;
            self.changed.insert(*self.membership.get(&id).unwrap());
        }
    }
    // A conditional or slot installs its lazy children during mount. The ordinary
    // property/geometry settlement discovers that work. Empty mounts alone cannot
    // witness that this component has selected its first actual output.
    pub(crate) fn did_settle(&mut self) {
        for id in std::mem::take(&mut self.changed) {
            if let Some(lane) = self.lanes.get_mut(&id) {
                lane.settled = lane.jobs.is_empty();
            }
        }
    }
    pub(super) fn discard_work(&mut self, work: &PendingMount) {
        let id = work.key.2.to_u32();
        let previous = self
            .membership
            .get(&id)
            .and_then(|g| self.lanes.get(g))
            .and_then(|lane| lane.jobs.get(&id))
            .or_else(|| self.discovery.get(&id));
        if previous.is_some_and(|p| p.key == work.key) {
            self.discard(id);
        }
    }
    pub(crate) fn discard(&mut self, id: Id) {
        if !self.enabled {
            return;
        }
        self.discovery.remove(&id);
        if let Some(group) = self.membership.remove(&id) {
            if let Some(lane) = self.lanes.get_mut(&group) {
                lane.jobs.remove(&id);
                lane.resources.remove(&id);
                self.changed.insert(group);
            }
            if group == id {
                self.remove_lane(id);
            }
        }
    }
    fn remove_lane(&mut self, id: Id) -> Option<Lane> {
        let lane = self.lanes.remove(&id)?;
        self.changed.remove(&id);
        for member in &lane.members {
            if self.membership.get(member) == Some(&id) {
                self.membership.remove(member);
            }
        }
        Some(lane)
    }
    pub(crate) fn candidates(&mut self) -> Vec<(Id, u64, Vec<Id>)> {
        if !self.enabled {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut retired = Vec::new();
        for (id, lane) in &mut self.lanes {
            let Some(root) = lane.root.upgrade() else {
                retired.push(*id);
                continue;
            };
            if !lane.mounted {
                continue;
            }
            if root.attached.get() == 0 || root.mount_epoch.get() != lane.mount_epoch {
                retired.push(*id);
                continue;
            }
            lane.jobs.retain(|_, j| {
                j.parent
                    .upgrade()
                    .is_some_and(|p| p.attached.get() != 0 && p.mount_epoch.get() == j.parent_epoch)
                    && j.child.upgrade().is_some_and(|c| {
                        c.attached.get() == 0 && c.mount_epoch.get() == j.child_epoch
                    })
            });
            if !lane.jobs.is_empty() || !lane.settled {
                if lane.jobs.is_empty() {
                    self.changed.insert(*id);
                }
                continue;
            }
            if !root.admission_parked.get() {
                retired.push(*id);
                continue;
            }
            lane.resources
                .retain(|_, n| n.upgrade().is_some_and(|c| c.attached.get() != 0));
            out.push((
                *id,
                lane.publication,
                lane.resources.keys().copied().collect(),
            ));
        }
        for id in retired {
            self.remove_lane(id);
        }
        out
    }
    fn take_ready(&mut self, id: Id, publication: u64) -> Option<Rc<ExpandedNode>> {
        let lane = self.lanes.get(&id)?;
        let root = lane.root.upgrade()?;
        if !lane.settled
            || !lane.jobs.is_empty()
            || root.attached.get() == 0
            || root.mount_epoch.get() != lane.mount_epoch
            || lane.publication != publication
            || !root.admission_parked.get()
        {
            return None;
        }
        self.remove_lane(id)?;
        root.admission_parked.set(false);
        self.opened += 1;
        Some(root)
    }
}
impl RuntimeContext {
    pub fn configure_visible_groups(&self, park: bool) {
        self.visible_groups.borrow_mut().configure(park);
    }
    pub fn ready_visible_groups(&self) -> Vec<(u32, u64, Vec<u32>)> {
        self.visible_groups.borrow_mut().candidates()
    }
    pub fn open_visible_group(&self, id: u32, epoch: u64) -> bool {
        let Some(node) = self.visible_groups.borrow_mut().take_ready(id, epoch) else {
            return false;
        };
        if let Some(parent) = node.render_parent_node() {
            self.mark_occlusion_dirty_for_node(parent.id.to_u32());
        }
        self.mark_occlusion_dirty_for_node(id);
        self.mark_canvas_node_dirty(node.id);
        true
    }
    pub fn hidden_native_group_members(&self, ids: &[u32]) -> Vec<u32> {
        ids.iter()
            .copied()
            .filter(|id| {
                let mut current = self.get_expanded_node_by_eid(crate::ExpandedNodeIdentifier(*id));
                while let Some(node) = current {
                    if node.admission_parked.get() {
                        return true;
                    }
                    current = node.presentation_parent_node();
                }
                false
            })
            .collect()
    }
    pub fn admission_group_diagnostics(&self) -> Vec<(u32, String, usize, usize, bool, u8)> {
        let state = self.visible_groups.borrow();
        let mut view = self.admission_view();
        state
            .lanes
            .iter()
            .filter_map(|(&id, l)| {
                let root = l.root.upgrade()?;
                let name = root
                    .instance_node
                    .borrow()
                    .base()
                    .template_node_type_id
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default();
                let rank = VisibleGroups::candidate(l)
                    .map_or(2, |n| self.admission_priority(&n, &mut view) as u8);
                Some((
                    id,
                    name,
                    l.jobs.len(),
                    l.resources.len(),
                    root.admission_parked.get(),
                    rank,
                ))
            })
            .collect()
    }
    pub fn visible_group_progress(&self) -> [u32; 4] {
        let s = self.visible_groups.borrow();
        [s.parked, s.opened, s.preferred, s.lanes.len() as u32]
    }
}
