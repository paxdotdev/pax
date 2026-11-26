//! A flow row can get its height from a component projected through a slot.
//! Its Rust feedback binding is opaque to the layout graph. Describe that edge
//! once, then follow only live projection and ordinary intrinsic measurements.
use super::{MountKey, StartupAxes, StartupDemand};
use crate::{api::NodeContext, ExpandedNode, InstanceNodePtr};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::{Rc, Weak};

const HEIGHT: StartupAxes = StartupAxes { x: false, y: true };
type Nodes = Vec<Rc<ExpandedNode>>;
pub(in crate::engine) type Loans = BTreeMap<MountKey, BTreeMap<u32, Rc<ExpandedNode>>>;

fn key(n: &InstanceNodePtr) -> usize {
    Rc::as_ptr(n) as *const () as usize
}
fn named(n: &InstanceNodePtr, id: &str) -> bool {
    n.base()
        .template_node_selector_info
        .as_ref()
        .and_then(|s| s.id.as_ref())
        .is_some_and(|token| token.token_value == id)
}

// Search only the author's namespace. A component's children are call-site
// expressions. Its private template is entered explicitly at a slot boundary.
fn paths(
    nodes: &[InstanceNodePtr],
    matches: &dyn Fn(&InstanceNodePtr) -> bool,
) -> Vec<Vec<InstanceNodePtr>> {
    fn visit(
        nodes: &[InstanceNodePtr],
        matches: &dyn Fn(&InstanceNodePtr) -> bool,
        path: &mut Vec<InstanceNodePtr>,
        found: &mut Vec<Vec<InstanceNodePtr>>,
    ) {
        for n in nodes {
            path.push(n.clone());
            if matches(n) {
                found.push(path.clone());
            } else {
                visit(
                    &n.base().get_instance_children().borrow(),
                    matches,
                    path,
                    found,
                );
            }
            path.pop();
        }
    }
    let mut found = Vec::new();
    visit(nodes, matches, &mut Vec::new(), &mut found);
    found
}

#[derive(Default)]
struct Route {
    steps: HashMap<usize, InstanceNodePtr>,
    rows: HashSet<usize>,
    sources: HashSet<usize>,
    bridged: HashSet<usize>,
}
impl Route {
    fn add(&mut self, path: &[InstanceNodePtr]) {
        for n in path {
            self.steps.insert(key(n), n.clone());
        }
        for n in path.iter().take(path.len().saturating_sub(1)) {
            self.bridge(n);
        }
    }
    fn bridge(&mut self, n: &InstanceNodePtr) {
        if let Some(template) = n.get_template() {
            if self.bridged.insert(key(n)) {
                for p in paths(&template.borrow(), &|n| n.base().flags().is_slot) {
                    self.add(&p);
                }
            }
        }
    }
    fn prepare(scope: &InstanceNodePtr, row: &str, source: &str) -> Option<Self> {
        let mut route = Self::default();
        for row_path in paths(&scope.get_template()?.borrow(), &|n| named(n, row)) {
            let host = row_path.last().unwrap();
            let projected = host.base().get_instance_children().borrow();
            for payload in paths(&projected, &|n| {
                n.get_template()
                    .is_some_and(|t| !paths(&t.borrow(), &|c| named(c, source)).is_empty())
            }) {
                let provider = payload.last().unwrap();
                for p in paths(&provider.get_template().unwrap().borrow(), &|c| {
                    named(c, source)
                }) {
                    route.sources.insert(key(p.last().unwrap()));
                    route.add(&p);
                }
                route.add(&payload);
                route.bridge(host);
                route.rows.insert(key(host));
                route.add(&row_path);
            }
        }
        (!route.sources.is_empty()).then_some(route)
    }
}
pub(super) struct Source {
    scope: Weak<ExpandedNode>,
    epoch: u64,
    route: Route,
}

impl NodeContext {
    /// Describe a vertical Stacker row whose pixel height is supplied by Rust
    /// from a projected component's intrinsic content. `row` is an authored ID
    /// in this component. `source` is an ID inside the projected component.
    /// Register in the declaring component's mount handler. Only selected slot
    /// content participates. The ordinary measurement binding still owns the value.
    pub fn declare_projected_height(&self, row: &str, source: &str) -> Result<(), &'static str> {
        let Some(scope) = self.expanded_node.upgrade() else {
            return Ok(());
        };
        let mut startup = self.runtime_context.startup.borrow_mut();
        if !startup.accepts_mount_input() {
            return Ok(());
        }
        let route = Route::prepare(&scope.instance_node.borrow(), row, source).ok_or(
            "projected height requires a named row and a source inside its projected component",
        )?;
        startup
            .height_sources
            .retain(|_, s| s.scope.strong_count() > 0);
        startup.height_sources.insert(
            (scope.id.0, row.to_owned()),
            Rc::new(Source {
                scope: Rc::downgrade(&scope),
                epoch: scope.mount_epoch.get(),
                route,
            }),
        );
        startup.height_revision += 1;
        Ok(())
    }
}

#[derive(Default, serde::Serialize)]
pub struct HeightObservation {
    pub row: u32,
    pub consumers: Vec<u32>,
    pub selected_sources: Vec<u32>,
    pub pending_mounts: Vec<u32>,
    pub pending_feedback: Vec<u32>,
    /// Property effects still need to publish their layout results.
    pub pending_settlement: bool,
    /// The source stack's measured height before the application adds its own padding.
    pub exact: Option<f64>,
}

impl Source {
    fn observe(&self, loans: &mut Loans, reports: &mut Option<Vec<HeightObservation>>) {
        let Some(scope) = self
            .scope
            .upgrade()
            .filter(|n| n.attached.get() != 0 && n.mount_epoch.get() == self.epoch)
        else {
            return;
        };
        self.find_row(&scope, loans, reports);
    }
    fn find_row(
        &self,
        parent: &Rc<ExpandedNode>,
        loans: &mut Loans,
        reports: &mut Option<Vec<HeightObservation>>,
    ) {
        for child in parent.active_children_view.get() {
            let k = key(&child.instance_node.borrow());
            if self.route.rows.contains(&k) {
                let Some(flow) = parent
                    .instance_node
                    .borrow()
                    .measurement_flow(parent, HEIGHT)
                else {
                    continue;
                };
                let Some(i) = flow.iter().position(|(n, _)| n.id == child.id) else {
                    continue;
                };
                if flow[i].1.is_some() {
                    continue;
                }
                let consumers: Nodes = flow.into_iter().skip(i + 1).map(|(n, _)| n).collect();
                let mut report = HeightObservation {
                    row: child.id.0,
                    ..Default::default()
                };
                self.visit(
                    &child,
                    None,
                    &consumers,
                    loans,
                    &mut report,
                    reports.is_some(),
                );
                if let Some(reports) = reports {
                    report.consumers = consumers.iter().map(|n| n.id.0).collect();
                    if report.selected_sources.len() != 1
                        || !report.pending_mounts.is_empty()
                        || !report.pending_feedback.is_empty()
                    {
                        report.exact = None;
                    }
                    reports.push(report);
                }
            } else if self.route.steps.contains_key(&k) && child.attached.get() != 0 {
                self.find_row(&child, loans, reports);
            }
        }
    }
    fn visit(
        &self,
        n: &Rc<ExpandedNode>,
        demand: Option<StartupDemand>,
        consumers: &Nodes,
        loans: &mut Loans,
        report: &mut HeightObservation,
        checked: bool,
    ) {
        if n.attached.get() == 0 {
            if let Some(parent) = n.render_parent_node() {
                let key = (
                    parent.id,
                    parent.mount_epoch.get(),
                    n.id,
                    n.mount_epoch.get(),
                );
                let owners = loans.entry(key).or_default();
                for c in consumers {
                    owners.insert(c.id.0, c.clone());
                }
            }
            if checked {
                report.pending_mounts.push(n.id.0);
            }
            return;
        }
        let instance = n.instance_node.borrow();
        let source = self.route.sources.contains(&key(&instance));
        let demand = demand.or_else(|| source.then(|| StartupDemand::measure(HEIGHT)));
        if let Some(demand) = demand {
            if checked {
                if source {
                    report.selected_sources.push(n.id.0);
                    report.exact = n.measured_size.get().map(|(_, h)| h);
                }
                if !instance.has_current_extent(n, demand.measure) {
                    report.pending_feedback.push(n.id.0);
                }
            }
            if instance.is_layout_measurement_boundary() {
                return;
            }
            if let Some(flow) = instance.measurement_flow(n, demand.measure) {
                for (child, allocation) in flow {
                    if allocation.is_none() {
                        self.visit(&child, Some(demand), consumers, loans, report, checked);
                    }
                }
                return;
            }
        }
        let mut children = n.active_children_view.get();
        if demand.is_some() {
            children.extend(n.exiting_children_view.get());
        }
        for child in children {
            if demand.is_some() {
                if child.is_layout_breakout() {
                    continue;
                }
            } else if !self
                .route
                .steps
                .contains_key(&key(&child.instance_node.borrow()))
            {
                continue;
            }
            self.visit(&child, demand, consumers, loans, report, checked);
        }
    }
}

impl crate::RuntimeContext {
    pub(in crate::engine) fn height_loans(&self) -> Loans {
        let mut loans = Loans::new();
        let sources: Vec<_> = self
            .startup
            .borrow()
            .height_sources
            .values()
            .cloned()
            .collect();
        for source in sources {
            source.observe(&mut loans, &mut None);
        }
        loans
    }
    /// Observe projected source measurements at a settled tick without mounting
    /// nodes. A pending native generation leaves the last ordinary layout readable.
    pub fn inspect_projected_heights(&self) -> Vec<HeightObservation> {
        let _graph = self.property_graph.enter();
        let mut reports = Some(Vec::new());
        let sources: Vec<_> = self
            .startup
            .borrow()
            .height_sources
            .values()
            .cloned()
            .collect();
        for source in sources {
            source.observe(&mut Loans::new(), &mut reports);
        }
        let mut reports = reports.unwrap();
        if pax_runtime_api::properties::has_pending_effects() {
            for report in &mut reports {
                report.pending_settlement = true;
                report.exact = None;
            }
        }
        reports
    }
}
