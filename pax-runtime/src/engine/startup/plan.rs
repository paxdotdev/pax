//! Component-owned requirements for the experimental initial presentation.
use super::{startup_key, ExpandedNode, ExpandedNodeIdentifier, RuntimeContext};
use crate::api::{NodeContext, Size};
use kurbo::Rect;
use std::{collections::HashMap, rc::Rc};

/// Axes whose exact extent is needed to place dependent content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StartupAxes {
    pub x: bool,
    pub y: bool,
}
impl StartupAxes {
    pub fn any(self) -> bool {
        self.x || self.y
    }
}

/// Requirements accumulate: visible content can also need an exact layout extent.
/// Regions use window coordinates, including inherited scrolling transforms.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StartupDemand {
    pub region: Option<Rect>,
    pub measure: StartupAxes,
    pub complete: bool,
}
impl StartupDemand {
    pub fn visible(region: Rect) -> Self {
        Self {
            region: Some(region),
            ..Self::default()
        }
    }
    pub fn measure(axes: StartupAxes) -> Self {
        Self {
            measure: axes,
            ..Self::default()
        }
    }
    pub fn complete() -> Self {
        Self {
            complete: true,
            ..Self::default()
        }
    }
    pub fn merge(self, other: Self) -> Self {
        Self {
            region: match (self.region, other.region) {
                (Some(a), Some(b)) => Some(a.union(b)),
                (a, b) => a.or(b),
            },
            measure: StartupAxes {
                x: self.measure.x || other.measure.x,
                y: self.measure.y || other.measure.y,
            },
            complete: self.complete || other.complete,
        }
    }
    fn covers(self, other: Self) -> bool {
        self.complete
            || (!other.complete
                && (!other.measure.x || self.measure.x)
                && (!other.measure.y || self.measure.y)
                && other
                    .region
                    .is_none_or(|b| self.region.is_some_and(|a| a.contains_rect(b))))
    }
}

type Key = (ExpandedNodeIdentifier, u64);
pub(super) type StartupResolver = Rc<dyn Fn(&ExpandedNode, &mut StartupPlan<'_>)>;
#[derive(Clone, Default)]
pub(super) struct StartupRequirements {
    default: Option<StartupDemand>,
    children: HashMap<Key, StartupDemand>,
}
impl StartupRequirements {
    pub(super) fn child(
        &self,
        child: &ExpandedNode,
        inherited: StartupDemand,
    ) -> Option<StartupDemand> {
        // Complete dependencies cannot be weakened by a nested visibility policy.
        if inherited.complete {
            return Some(StartupDemand::complete());
        }
        match (
            self.default,
            self.children.get(&startup_key(child)).copied(),
        ) {
            (Some(a), Some(b)) => Some(a.merge(b)),
            (a, b) => a.or(b),
        }
    }
}

/// Describes child work without mounting nodes or retaining borrows across yields.
/// Implement `InstanceNode::plan_startup`, or register a component-local resolver
/// through `NodeContext::set_startup_planner`. The scheduler owns continuation,
/// lifetime checks, native feedback, and the presentation gate.
///
/// A complete source requirement must be attached to the source, not its owner:
/// doing so leaves the owner's visible content recursively selectable.
pub struct StartupPlan<'a> {
    context: &'a RuntimeContext,
    owner: &'a ExpandedNode,
    demand: StartupDemand,
    requirements: StartupRequirements,
}
impl<'a> StartupPlan<'a> {
    pub(super) fn new(
        context: &'a RuntimeContext,
        owner: &'a ExpandedNode,
        demand: StartupDemand,
    ) -> Self {
        Self {
            context,
            owner,
            demand,
            requirements: StartupRequirements::default(),
        }
    }
    pub fn demand(&self) -> StartupDemand {
        self.demand
    }
    /// Default traversal. Exact extents conservatively require complete children;
    /// a container with more precise dependency knowledge can override this.
    pub fn forward_children(&mut self) {
        let axes = self
            .context
            .startup
            .borrow()
            .measurement_axes
            .get(&startup_key(self.owner))
            .copied()
            .unwrap_or_default();
        let common = self.owner.get_common_properties();
        let common = common.borrow();
        let anchored = (axes.x && depends_on_extent(common.anchor_x.get().or(common.x.get())))
            || (axes.y && depends_on_extent(common.anchor_y.get().or(common.y.get())));
        // Unknown width can change line wrapping throughout the visible prefix.
        let complete = self.demand.complete || self.demand.measure.any() || axes.x || anchored;
        drop(common);
        self.require_children(if complete {
            StartupDemand::complete()
        } else {
            self.demand
        });
    }
    /// Require a child or detached rendering dependency. Repeated requirements merge.
    pub fn require(&mut self, child: &ExpandedNode, demand: StartupDemand) {
        self.requirements
            .children
            .entry(startup_key(child))
            .and_modify(|old| *old = old.merge(demand))
            .or_insert(demand);
    }
    pub fn require_children(&mut self, demand: StartupDemand) {
        self.requirements.default = Some(
            self.requirements
                .default
                .map_or(demand, |old| old.merge(demand)),
        );
    }
    pub fn bounds(&self, node: &ExpandedNode) -> Option<Rect> {
        self.context.startup_bounds(node)
    }
    /// Whether a browser measurement checkpoint covered this requirement.
    /// A visible subtree with no deferred descendants also has complete extents.
    pub fn measured(&self, node: &ExpandedNode, demand: StartupDemand) -> bool {
        let state = self.context.startup.borrow();
        let Some(previous) = state.measured.get(&startup_key(node)).copied() else {
            return false;
        };
        if previous.covers(demand) {
            return true;
        }
        if demand.complete || !demand.measure.any() {
            return false;
        }
        !state
            .deferred
            .iter()
            .chain(state.stack.iter())
            .chain(state.incoming.iter())
            .any(|pending| {
                pending.child.upgrade().is_some_and(|child| {
                    let mut ancestor = child.render_parent_node();
                    while let Some(parent) = ancestor {
                        if parent.id == node.id {
                            return true;
                        }
                        ancestor = parent.render_parent_node();
                    }
                    false
                })
            })
    }
    pub(super) fn finish(self) -> StartupRequirements {
        self.requirements
    }
}

fn depends_on_extent(size: Option<Size>) -> bool {
    match size {
        Some(Size::Percent(p)) | Some(Size::Combined(_, p)) => p.to_float() != 0.0,
        _ => false,
    }
}

impl NodeContext {
    /// Register an experimental component-owned startup planner for this mount.
    /// The callback only describes requirements; it must not mount or run handlers.
    /// It is discarded when initial presentation completes and ignored outside
    /// yielded startup. Capture weak references rather than retaining the node.
    pub fn set_startup_planner(
        &self,
        resolver: impl Fn(&ExpandedNode, &mut StartupPlan<'_>) + 'static,
    ) {
        let Some(node) = self.expanded_node.upgrade() else {
            return;
        };
        let mut state = self.runtime_context.startup.borrow_mut();
        if state.is_prioritizing() {
            let key = startup_key(&node);
            state.resolvers.insert(key, Rc::new(resolver));
            state.plans.clear();
        }
    }
}
