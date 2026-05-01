use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::iter;
use std::rc::Rc;

#[allow(unused)]
use crate::*;
use pax_engine::api::math::{Transform2, Vector2};
use pax_engine::api::{Axis, EasingCurve, Numeric, Property, Size};
use pax_engine::pax_manifest::cartridge_generation::TRANSITION_PHASE_ENTER;
use pax_engine::*;
use pax_runtime::api::{borrow, borrow_mut, Layer, NodeContext};
use pax_runtime::{
    BaseInstance, Container, ContainerFrame, ExpandedNode, InstanceFlags, InstanceNode,
    InstantiationArgs, LayoutHull, ReceivedChildrenSource, RuntimeContext,
};

const STACKER_REFLOW_FRAMES: u64 = 12;
const STACKER_REFLOW_CURVE: ContainerReflowCurve = ContainerReflowCurve::OutQuad;

/// Stacker lays out a series of nodes either
/// vertically or horizontally (i.e. a single row or column) with a specified gutter in between
/// each node.  `Stacker`s can be stacked inside of each other, horizontally
/// and vertically, along with percentage-based positioning and `Transform2D.anchor` to compose any rectilinear 2D layout.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::layout::stacker::StackerInstance")]
#[custom(Default)]
pub struct Stacker {
    /// The direction the stacker should flow its cells
    pub direction: Property<StackerDirection>,

    /// Spacing between cells
    pub gutter: Property<Size>,

    /// When true, the stacker uses content-child bounds to autosize its cells
    /// and, when possible, its own bounds as well.
    ///
    /// `Stacker` interprets plain `autosize=true` as "autosize the extending
    /// axis only" (`y` for vertical stacks, `x` for horizontal stacks). Use
    /// `autosize_x` / `autosize_y` to override those per-axis defaults.
    ///
    /// The underlying shrink-sizing pass only applies when the measured axis
    /// can be resolved without parent-size cycles, so percent-sized children
    /// continue to use the existing top-down layout behavior unless the
    /// corresponding stacker axis is already explicit.
    pub autosize: Property<bool>,
    /// Optional override for whether autosize manages the `x` axis.
    pub autosize_x: Property<Option<bool>>,
    /// Optional override for whether autosize manages the `y` axis.
    pub autosize_y: Property<Option<bool>>,

    /// Size of each cell, by index.  None-values (or array-index out-of-bounds values)
    /// will fall back to computed, equal-sizing
    pub sizes: Property<Vec<Option<Size>>>,

    /// Whether exiting children stay in normal stack flow or hold their previous frame as ghosts.
    pub exit_mode: Property<ContainerExitMode>,
    /// How surviving children should move when the stack's layout changes.
    ///
    /// Defaults to `Snap` so Stackers remain a stable layout primitive unless
    /// reflow motion is explicitly requested.
    pub reflow_transition: Property<ContainerReflowTransition>,
}

impl Default for Stacker {
    fn default() -> Self {
        Self {
            direction: Property::new(StackerDirection::Vertical),
            gutter: Property::new(Size::Pixels(Numeric::I32(0))),
            autosize: Property::new(false),
            autosize_x: Property::new(None),
            autosize_y: Property::new(None),
            sizes: Property::new(vec![]),
            exit_mode: Property::new(ContainerExitMode::Flow),
            reflow_transition: Property::new(ContainerReflowTransition::default()),
        }
    }
}

pub struct StackerInstance {
    base: BaseInstance,
}

impl InstanceNode for StackerInstance {
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
                    is_slot: false,
                },
            ),
        })
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Stacker").finish()
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn received_children_source(&self) -> ReceivedChildrenSource {
        ReceivedChildrenSource::Projected
    }

    fn handle_setup_projected_children(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        if let Some(containing_component) = expanded_node.containing_component.upgrade() {
            let env = Rc::clone(&expanded_node.stack);
            let children = borrow!(self.base().get_instance_children());
            let children_with_env = children.iter().cloned().zip(iter::repeat(env));
            let new_projected_children = containing_component.create_children_detached(
                children_with_env,
                context,
                &Rc::downgrade(expanded_node),
            );
            *borrow_mut!(expanded_node.expanded_projected_children) = Some(new_projected_children);
        }
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        let weak_ref_self = Rc::downgrade(expanded_node);
        let cloned_context = Rc::clone(context);
        let parent_frame = expanded_node.parent_frame.clone();
        let projected_children = expanded_node
            .expanded_and_flattened_projected_children
            .clone();
        let deps = [projected_children.untyped()];
        expanded_node
            .children
            .replace_with(Property::computed_with_name(
                move || {
                    let Some(node) = weak_ref_self.upgrade() else {
                        panic!("ran evaluator after expanded node dropped (stacker children)")
                    };
                    node.attach_children(projected_children.get(), &cloned_context, &parent_frame)
                },
                &deps,
                &format!("stacker_children (node id: {})", expanded_node.id.0),
            ));
        let _ = expanded_node.children.get();

        expanded_node.with_properties_unwrapped(|properties: &mut Stacker| {
            properties.bind_container(&expanded_node.get_node_context(context));
        });
    }

    fn update(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, _context: &Rc<RuntimeContext>) {
        expanded_node.compute_flattened_projected_children();
    }
}

impl Container for Stacker {
    // Computes child rectangles for content children and assigns container-owned frames.
    fn bind_container(&self, ctx: &NodeContext) {
        let sizes = self.sizes.clone();
        let bound = ctx.bounds_self.clone();
        let gutter = self.gutter.clone();
        let direction = self.direction.clone();
        let autosize = self.autosize.clone();
        let autosize_x = self.autosize_x.clone();
        let autosize_y = self.autosize_y.clone();
        let exit_mode = self.exit_mode.clone();
        let reflow_transition = self.reflow_transition.clone();
        let bound_for_update = bound.clone();
        let autosize_for_update = autosize.clone();
        let autosize_x_for_update = autosize_x.clone();
        let autosize_y_for_update = autosize_y.clone();
        let received_children_for_update = ctx.received_children.clone();
        let retained_received_children_for_update = ctx.retained_received_children.clone();
        let direction_for_update = direction.clone();
        let sizes_for_update = sizes.clone();
        let gutter_for_update = gutter.clone();
        let exit_mode_for_update = exit_mode.clone();
        let reflow_transition_for_update = reflow_transition.clone();
        let expanded_node = ctx.expanded_node.clone();
        let prior_frames = Rc::new(RefCell::new(HashMap::new()));
        let last_named_transition_warning = Rc::new(RefCell::new(None::<String>));
        let update_layout = Rc::new(move || {
            let received_children = received_children_for_update.get();
            let retained_received_children = retained_received_children_for_update.get();
            let bounds = bound_for_update.get();
            let direction = direction_for_update.get();
            let gutter = gutter_for_update.get();
            let sizes = sizes_for_update.get();
            let autosize = autosize_for_update.get();
            let autosize_x = autosize_x_for_update.get();
            let autosize_y = autosize_y_for_update.get();
            let exit_mode = exit_mode_for_update.get();
            let reflow_transition = reflow_transition_for_update.get();
            let reflow_kind = reflow_transition.kind.get();
            if matches!(reflow_kind, ContainerReflowTransitionKind::Named) {
                let name = reflow_transition.name.get();
                if last_named_transition_warning.borrow().as_ref() != Some(&name) {
                    log::warn!(
                        "named stacker reflow transition '{}' is not implemented yet; falling back to snap",
                        name
                    );
                    *last_named_transition_warning.borrow_mut() = Some(name);
                }
            }

            for breakout_child in received_children
                .iter()
                .chain(retained_received_children.iter())
                .filter(|child| child.is_layout_breakout())
            {
                if breakout_child.container_frame.get().is_some() {
                    breakout_child.container_frame.set(None);
                }
            }

            let active_children: Vec<_> = received_children
                .into_iter()
                .filter(|child| !child.is_layout_breakout())
                .collect();
            let retained_children: Vec<_> = retained_received_children
                .into_iter()
                .filter(|child| !child.is_layout_breakout())
                .collect();
            let mut children: Vec<_> =
                Vec::with_capacity(active_children.len() + retained_children.len());
            children.extend(active_children.iter().cloned());
            children.extend(retained_children.iter().cloned());

            let Some(node) = expanded_node.upgrade() else {
                return;
            };
            let common_props = node.get_common_properties();
            let common_props = borrow!(common_props);
            let width_explicit = common_props.width.get().is_some();
            let height_explicit = common_props.height.get().is_some();
            drop(common_props);

            let active_measurements = active_children
                .iter()
                .map(|child| measure_child_bounds(&node, child))
                .collect::<Vec<_>>();
            let active_layout = compute_stacker_layout(
                direction.clone(),
                bounds,
                gutter,
                sizes.clone(),
                autosize,
                autosize_x,
                autosize_y,
                &active_measurements,
                active_children.len(),
                width_explicit,
                height_explicit,
            );

            let mut snapshots: Vec<_> = active_children
                .iter()
                .map(|child| StackerChildSnapshot {
                    id: child.id,
                    exiting: false,
                    entering: child_enter_transition_active(child),
                })
                .collect();
            snapshots.extend(retained_children.iter().map(|child| StackerChildSnapshot {
                id: child.id,
                exiting: true,
                entering: false,
            }));
            let current_frames: HashMap<_, _> = children
                .iter()
                .filter_map(|child| child.container_frame.get().map(|frame| (child.id, frame)))
                .collect();
            let prior_frames_map = prior_frames.borrow().clone();
            let flow_children =
                flow_ordered_children(&snapshots, &prior_frames_map, &current_frames, &direction);
            let flow_measurements = flow_children
                .iter()
                .filter_map(|snapshot| {
                    children
                        .iter()
                        .find(|child| child.id == snapshot.id)
                        .map(|child| measure_child_bounds(&node, child))
                })
                .collect::<Vec<_>>();
            let flow_layout = compute_stacker_layout(
                direction.clone(),
                bounds,
                gutter,
                sizes.clone(),
                autosize,
                autosize_x,
                autosize_y,
                &flow_measurements,
                flow_children.len(),
                width_explicit,
                height_explicit,
            );

            let plan = plan_stacker_layout_from_specs(
                &snapshots,
                &prior_frames_map,
                &current_frames,
                exit_mode,
                reflow_kind,
                &active_layout.cell_specs,
                &flow_layout.cell_specs,
                bounds,
                direction,
                gutter,
            );

            let mut next_frames = HashMap::new();
            for child in &children {
                let Some(placement) = plan
                    .placements
                    .iter()
                    .find(|placement| placement.id == child.id)
                else {
                    child.container_frame.set(None);
                    continue;
                };

                if placement.animate {
                    let frames = reflow_transition.frames.get();
                    if child.container_frame.get().is_none() {
                        if let Some(seed_frame) = placement.seed_frame {
                            child.container_frame.set(Some(seed_frame));
                        }
                    }
                    child.container_frame.ease_to(
                        Some(placement.frame),
                        frames,
                        reflow_transition.curve.get().to_easing_curve(),
                    );
                } else {
                    child.container_frame.set(Some(placement.frame));
                }

                next_frames.insert(child.id, placement.frame);
            }
            *prior_frames.borrow_mut() = next_frames;

            match active_layout.measured_size {
                Some(measured_size) => {
                    if node.measured_size.get() != Some(measured_size) {
                        node.set_measured_size(measured_size.0, measured_size.1);
                    }
                }
                None => {
                    if node.measured_size.get().is_some() {
                        node.measured_size.set(None);
                    }
                }
            }
        });

        update_layout();
        let deps = [
            bound.untyped(),
            direction.untyped(),
            sizes.untyped(),
            gutter.untyped(),
            autosize.untyped(),
            autosize_x.untyped(),
            autosize_y.untyped(),
            exit_mode.untyped(),
            reflow_transition.untyped(),
            ctx.received_children.untyped(),
            ctx.retained_received_children.untyped(),
        ];
        let update_layout_callback = Rc::clone(&update_layout);
        ctx.subscribe(&deps, move || update_layout_callback());
    }
}

#[pax]
#[engine_import_path("pax_engine")]
/// Whether exiting children remain in normal layout flow or become ghosts.
pub enum ContainerExitMode {
    /// Keep exiting children in the stack's in-flow layout until their out-transition finishes.
    #[default]
    Flow,
    /// Hold exiting children at their previous frame as overlays while the remaining children
    /// resolve layout without them.
    Ghost,
}

#[pax]
#[engine_import_path("pax_engine")]
/// Which reflow animation source to use when children move to new stack positions.
pub enum ContainerReflowTransitionKind {
    /// Snap immediately to the new layout.
    #[default]
    Snap,
    /// Use a duration and easing curve.
    Ease,
    /// Reserved for a future named motion-curve lookup in the current component scope.
    Named,
}

#[pax]
#[engine_import_path("pax_engine")]
/// Easing curve used by `ContainerReflowTransitionKind::Ease`.
pub enum ContainerReflowCurve {
    #[default]
    Linear,
    Hold,
    InQuad,
    OutQuad,
    InOutQuad,
    InBack,
    OutBack,
    InOutBack,
}

impl ContainerReflowCurve {
    fn to_easing_curve(self) -> EasingCurve {
        match self {
            Self::Linear => EasingCurve::Linear,
            Self::Hold => EasingCurve::Hold,
            Self::InQuad => EasingCurve::InQuad,
            Self::OutQuad => EasingCurve::OutQuad,
            Self::InOutQuad => EasingCurve::InOutQuad,
            Self::InBack => EasingCurve::InBack,
            Self::OutBack => EasingCurve::OutBack,
            Self::InOutBack => EasingCurve::InOutBack,
        }
    }
}

#[pax]
#[engine_import_path("pax_engine")]
#[custom(Default)]
/// Reflow animation applied to surviving children when the stack layout changes.
pub struct ContainerReflowTransition {
    /// Which reflow animation source to use.
    pub kind: Property<ContainerReflowTransitionKind>,
    /// Duration in frames for `Ease`.
    pub frames: Property<u64>,
    /// Curve used for `Ease`.
    pub curve: Property<ContainerReflowCurve>,
    /// Reserved for a future named motion-curve lookup when `kind` is `Named`.
    pub name: Property<String>,
}

impl Default for ContainerReflowTransition {
    fn default() -> Self {
        Self {
            kind: Property::new(ContainerReflowTransitionKind::Snap),
            frames: Property::new(STACKER_REFLOW_FRAMES),
            curve: Property::new(STACKER_REFLOW_CURVE),
            name: Property::new(String::new()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StackerChildSnapshot<Id> {
    id: Id,
    exiting: bool,
    entering: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct StackerChildPlacement<Id> {
    id: Id,
    seed_frame: Option<ContainerFrame>,
    frame: ContainerFrame,
    animate: bool,
}

#[derive(Clone, Debug)]
struct StackerLayoutPlan<Id> {
    placements: Vec<StackerChildPlacement<Id>>,
}

fn plan_stacker_layout<Id: Copy + Eq + Hash>(
    children: &[StackerChildSnapshot<Id>],
    prior_frames: &HashMap<Id, ContainerFrame>,
    current_frames: &HashMap<Id, ContainerFrame>,
    exit_mode: ContainerExitMode,
    reflow_kind: ContainerReflowTransitionKind,
    bounds: (f64, f64),
    direction: StackerDirection,
    gutter: Size,
    sizes: Vec<Option<Size>>,
) -> StackerLayoutPlan<Id> {
    let active_children: Vec<_> = children
        .iter()
        .copied()
        .filter(|child| !child.exiting)
        .collect();
    let active_cell_specs = compute_cell_specs(
        active_children.len(),
        bounds,
        direction.clone(),
        gutter,
        sizes.clone(),
    );
    let flow_children: Vec<_> = match exit_mode {
        ContainerExitMode::Flow => {
            flow_ordered_children(children, prior_frames, current_frames, &direction)
        }
        ContainerExitMode::Ghost => active_children.clone(),
    };
    let flow_cell_specs = compute_cell_specs(
        flow_children.len(),
        bounds,
        direction.clone(),
        gutter,
        sizes,
    );
    plan_stacker_layout_from_specs(
        children,
        prior_frames,
        current_frames,
        exit_mode,
        reflow_kind,
        &active_cell_specs,
        &flow_cell_specs,
        bounds,
        direction,
        gutter,
    )
}

fn plan_stacker_layout_from_specs<Id: Copy + Eq + Hash>(
    children: &[StackerChildSnapshot<Id>],
    prior_frames: &HashMap<Id, ContainerFrame>,
    current_frames: &HashMap<Id, ContainerFrame>,
    exit_mode: ContainerExitMode,
    reflow_kind: ContainerReflowTransitionKind,
    active_cell_specs: &[StackerCell],
    flow_cell_specs: &[StackerCell],
    bounds: (f64, f64),
    direction: StackerDirection,
    gutter: Size,
) -> StackerLayoutPlan<Id> {
    let active_children: Vec<_> = children
        .iter()
        .copied()
        .filter(|child| !child.exiting)
        .collect();
    let active_frames: HashMap<_, _> = active_children
        .iter()
        .zip(active_cell_specs.iter())
        .map(|(child, cell)| (child.id, container_frame_for_cell(cell)))
        .collect();

    let flow_children: Vec<_> = match exit_mode {
        ContainerExitMode::Flow => {
            flow_ordered_children(children, prior_frames, current_frames, &direction)
        }
        ContainerExitMode::Ghost => active_children.clone(),
    };
    let flow_frames: HashMap<_, _> = flow_children
        .iter()
        .zip(flow_cell_specs.iter())
        .map(|(child, cell)| (child.id, container_frame_for_cell(cell)))
        .collect();
    let virtual_entering_frames = match exit_mode {
        ContainerExitMode::Flow => virtualize_flow_entering_children(
            children,
            &active_frames,
            &flow_frames,
            bounds,
            &direction,
            gutter,
        ),
        ContainerExitMode::Ghost => HashMap::new(),
    };

    let placements = children
        .iter()
        .filter_map(|child| {
            let (seed_frame, frame) = match exit_mode {
                ContainerExitMode::Flow
                    if child.exiting
                        && matches!(reflow_kind, ContainerReflowTransitionKind::Ease) =>
                {
                    (
                        None,
                        prior_frames
                            .get(&child.id)
                            .copied()
                            .or_else(|| current_frames.get(&child.id).copied())?,
                    )
                }
                ContainerExitMode::Flow if child.exiting => {
                    (None, flow_frames.get(&child.id).copied()?)
                }
                ContainerExitMode::Flow
                    if matches!(reflow_kind, ContainerReflowTransitionKind::Ease) =>
                {
                    (
                        child
                            .entering
                            .then(|| virtual_entering_frames.get(&child.id).copied())
                            .flatten(),
                        active_frames.get(&child.id).copied()?,
                    )
                }
                ContainerExitMode::Flow if child.entering => (
                    None,
                    virtual_entering_frames
                        .get(&child.id)
                        .copied()
                        .or_else(|| active_frames.get(&child.id).copied())?,
                ),
                ContainerExitMode::Flow => (None, flow_frames.get(&child.id).copied()?),
                ContainerExitMode::Ghost if child.exiting => (
                    None,
                    prior_frames
                        .get(&child.id)
                        .copied()
                        .or_else(|| current_frames.get(&child.id).copied())?,
                ),
                ContainerExitMode::Ghost => (None, active_frames.get(&child.id).copied()?),
            };

            let animate = matches!(reflow_kind, ContainerReflowTransitionKind::Ease)
                && !child.exiting
                && seed_frame
                    .or_else(|| current_frames.get(&child.id).copied())
                    .or_else(|| prior_frames.get(&child.id).copied())
                    .map(|current| current != frame)
                    .unwrap_or(false);

            Some(StackerChildPlacement {
                id: child.id,
                seed_frame,
                frame,
                animate,
            })
        })
        .collect();

    StackerLayoutPlan { placements }
}

fn child_enter_transition_active(child: &Rc<ExpandedNode>) -> bool {
    let transition_config = borrow!(child.instance_node)
        .base()
        .transition_config()
        .clone();
    enter_transition_active(
        child.transition_phase.get(),
        child.transition_playhead.get(),
        child.transition_playhead_millis.get(),
        transition_config.enter_frame_count,
        transition_config.enter_millis_count,
    )
}

fn enter_transition_active(
    phase: u64,
    playhead: f64,
    playhead_millis: f64,
    enter_frame_count: u64,
    enter_millis_count: Option<u64>,
) -> bool {
    if phase != TRANSITION_PHASE_ENTER {
        return false;
    }
    match enter_millis_count {
        Some(millis) => playhead_millis < millis as f64,
        None => playhead < enter_frame_count as f64,
    }
}

fn virtualize_flow_entering_children<Id: Copy + Eq + Hash>(
    children: &[StackerChildSnapshot<Id>],
    active_frames: &HashMap<Id, ContainerFrame>,
    flow_frames: &HashMap<Id, ContainerFrame>,
    bounds: (f64, f64),
    direction: &StackerDirection,
    gutter: Size,
) -> HashMap<Id, ContainerFrame> {
    // In Flow mode, stable and exiting children keep their current in-flow slots.
    // Newly entering children therefore cannot safely inhabit their target slots yet
    // without overlapping the existing occupants. Keep them in virtual tail slots
    // until the enter transition settles and they can rejoin normal flow.
    if flow_frames.is_empty() {
        return HashMap::new();
    }

    let active_bound = match direction {
        StackerDirection::Horizontal => bounds.0,
        StackerDirection::Vertical => bounds.1,
    };
    let gutter_px = resolve_size_px(gutter, active_bound);
    let mut next_main_axis = flow_frames
        .values()
        .copied()
        .map(|frame| frame_main_axis_end(frame, direction))
        .fold(f64::NEG_INFINITY, f64::max);

    let mut virtual_frames = HashMap::new();
    for child in children
        .iter()
        .copied()
        .filter(|child| child.entering && !child.exiting)
    {
        let Some(target_frame) = active_frames.get(&child.id).copied() else {
            continue;
        };
        let frame =
            with_frame_main_axis_position(target_frame, direction, next_main_axis + gutter_px);
        next_main_axis = frame_main_axis_end(frame, direction);
        virtual_frames.insert(child.id, frame);
    }

    virtual_frames
}

fn flow_ordered_children<Id: Copy + Eq + Hash>(
    children: &[StackerChildSnapshot<Id>],
    prior_frames: &HashMap<Id, ContainerFrame>,
    current_frames: &HashMap<Id, ContainerFrame>,
    direction: &StackerDirection,
) -> Vec<StackerChildSnapshot<Id>> {
    let mut active = Vec::new();
    let mut exiting = Vec::new();
    for child in children.iter().copied() {
        if child.exiting {
            exiting.push(child);
        } else if child.entering {
            continue;
        } else {
            active.push(child);
        }
    }

    if exiting.is_empty() {
        return active;
    }

    exiting.sort_by(|a, b| {
        frame_main_axis_position_for_child(a.id, prior_frames, current_frames, direction)
            .partial_cmp(&frame_main_axis_position_for_child(
                b.id,
                prior_frames,
                current_frames,
                direction,
            ))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut merged = active;
    for exiting_child in exiting {
        let exiting_pos = frame_main_axis_position_for_child(
            exiting_child.id,
            prior_frames,
            current_frames,
            direction,
        );
        let insert_at = merged
            .iter()
            .position(|active_child| {
                frame_main_axis_position_for_child(
                    active_child.id,
                    prior_frames,
                    current_frames,
                    direction,
                ) > exiting_pos
            })
            .unwrap_or(merged.len());
        merged.insert(insert_at, exiting_child);
    }

    merged
}

fn frame_main_axis_position_for_child<Id: Copy + Eq + Hash>(
    id: Id,
    prior_frames: &HashMap<Id, ContainerFrame>,
    current_frames: &HashMap<Id, ContainerFrame>,
    direction: &StackerDirection,
) -> f64 {
    prior_frames
        .get(&id)
        .copied()
        .or_else(|| current_frames.get(&id).copied())
        .map(|frame| frame_main_axis_position(frame, direction))
        .unwrap_or(f64::INFINITY)
}

fn frame_main_axis_position(frame: ContainerFrame, direction: &StackerDirection) -> f64 {
    match direction {
        StackerDirection::Horizontal => frame.transform.m[4],
        StackerDirection::Vertical => frame.transform.m[5],
    }
}

fn frame_main_axis_end(frame: ContainerFrame, direction: &StackerDirection) -> f64 {
    frame_main_axis_position(frame, direction)
        + match direction {
            StackerDirection::Horizontal => frame.bounds.0,
            StackerDirection::Vertical => frame.bounds.1,
        }
}

fn with_frame_main_axis_position(
    frame: ContainerFrame,
    direction: &StackerDirection,
    main_axis_position: f64,
) -> ContainerFrame {
    let translated = match direction {
        StackerDirection::Horizontal => {
            Transform2::translate(Vector2::new(main_axis_position, frame.transform.m[5]))
        }
        StackerDirection::Vertical => {
            Transform2::translate(Vector2::new(frame.transform.m[4], main_axis_position))
        }
    };
    ContainerFrame {
        transform: translated,
        bounds: frame.bounds,
    }
}

fn compute_cell_specs(
    cells: usize,
    bounds: (f64, f64),
    direction: StackerDirection,
    gutter: Size,
    mut sizes: Vec<Option<Size>>,
) -> Vec<StackerCell> {
    if cells == 0 {
        return Vec::new();
    }

    let active_bound = match direction {
        StackerDirection::Horizontal => bounds.0,
        StackerDirection::Vertical => bounds.1,
    };
    let gutter_px = resolve_size_px(gutter, active_bound);
    let usable_interior_space = active_bound - (cells.saturating_sub(1) as f64) * gutter_px;
    let per_cell_space = usable_interior_space / cells as f64;

    let mut cell_space = vec![per_cell_space; cells];
    while sizes.len() < cell_space.len() {
        sizes.push(None);
    }

    if !sizes.is_empty() {
        let mut used_space = 0.0;
        let mut remaining_cells = 0_usize;
        for (i, size) in sizes.iter().take(cells).enumerate() {
            if let Some(size) = size {
                let space = resolve_size_px(*size, active_bound);
                used_space += space;
                cell_space[i] = space;
            } else {
                cell_space[i] = -1.0;
                remaining_cells += 1;
            }
        }

        if remaining_cells > 0 {
            let remaining_per_cell_space =
                ((usable_interior_space - used_space) / remaining_cells as f64).max(5.0);
            for space in &mut cell_space {
                if *space == -1.0 {
                    *space = remaining_per_cell_space;
                }
            }
        }
    }

    let mut used_space = 0.0;
    (0..cells)
        .map(|i| {
            let cell = match direction {
                StackerDirection::Horizontal => StackerCell {
                    height_px: bounds.1,
                    width_px: cell_space[i],
                    x_px: i as f64 * gutter_px + used_space,
                    y_px: 0.0,
                },
                StackerDirection::Vertical => StackerCell {
                    height_px: cell_space[i],
                    width_px: bounds.0,
                    x_px: 0.0,
                    y_px: i as f64 * gutter_px + used_space,
                },
            };
            used_space += cell_space[i];
            cell
        })
        .collect()
}

fn resolve_size_px(size: Size, active_bound: f64) -> f64 {
    match size {
        Size::Pixels(pix) => pix.to_float(),
        Size::Percent(per) => (Numeric::F64(active_bound) * (per / Numeric::F64(100.0))).to_float(),
        Size::Combined(pix, per) => {
            (pix + (Numeric::F64(active_bound) * (per / Numeric::F64(100.0)))).to_float()
        }
    }
}

fn container_frame_for_cell(cell: &StackerCell) -> ContainerFrame {
    ContainerFrame {
        transform: Transform2::translate(Vector2::new(cell.x_px, cell.y_px)),
        bounds: (cell.width_px, cell.height_px),
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct ChildMeasurement {
    width: Option<f64>,
    height: Option<f64>,
}

impl ChildMeasurement {
    fn axis(&self, axis: Axis) -> Option<f64> {
        match axis {
            Axis::X => self.width,
            Axis::Y => self.height,
        }
    }
}

struct StackerLayoutResult {
    cell_specs: Vec<StackerCell>,
    measured_size: Option<(f64, f64)>,
}

fn compute_stacker_layout(
    direction: StackerDirection,
    bounds: (f64, f64),
    gutter: Size,
    sizes: Vec<Option<Size>>,
    autosize: bool,
    autosize_x: Option<bool>,
    autosize_y: Option<bool>,
    child_measurements: &[ChildMeasurement],
    slot_children_count: usize,
    width_explicit: bool,
    height_explicit: bool,
) -> StackerLayoutResult {
    let standard_cell_specs =
        compute_standard_cell_specs(&direction, bounds, gutter, &sizes, slot_children_count);

    if !autosize {
        return StackerLayoutResult {
            cell_specs: standard_cell_specs,
            measured_size: None,
        };
    }

    let (manage_x, manage_y) =
        resolve_stacker_autosize_axes(&direction, autosize, autosize_x, autosize_y);
    let (main_axis, cross_axis, autosize_main_axis, autosize_cross_axis) = match &direction {
        StackerDirection::Horizontal => (
            Axis::X,
            Axis::Y,
            manage_x && !width_explicit,
            manage_y && !height_explicit,
        ),
        StackerDirection::Vertical => (
            Axis::Y,
            Axis::X,
            manage_y && !height_explicit,
            manage_x && !width_explicit,
        ),
    };

    let main_bound = axis_value(bounds, main_axis);
    let cross_bound = axis_value(bounds, cross_axis);

    if autosize_main_axis && size_depends_on_parent(gutter) {
        return StackerLayoutResult {
            cell_specs: standard_cell_specs,
            measured_size: None,
        };
    }

    let gutter_px = gutter.get_pixels(main_bound);

    let mut main_sizes = Vec::with_capacity(slot_children_count);
    let mut cross_sizes = Vec::with_capacity(slot_children_count);

    for index in 0..slot_children_count {
        let explicit_main = sizes
            .get(index)
            .copied()
            .flatten()
            .map(|size| {
                if autosize_main_axis && size_depends_on_parent(size) {
                    None
                } else {
                    Some(size.get_pixels(main_bound))
                }
            })
            .flatten();

        let main_size = if let Some(explicit_main) = explicit_main {
            explicit_main
        } else if autosize_main_axis {
            let Some(measured_main) = child_measurements
                .get(index)
                .and_then(|measurement| measurement.axis(main_axis))
            else {
                return StackerLayoutResult {
                    cell_specs: standard_cell_specs,
                    measured_size: None,
                };
            };
            measured_main
        } else {
            match direction {
                StackerDirection::Horizontal => standard_cell_specs[index].width_px,
                StackerDirection::Vertical => standard_cell_specs[index].height_px,
            }
        };

        let cross_size = if autosize_cross_axis {
            let Some(measured_cross) = child_measurements
                .get(index)
                .and_then(|measurement| measurement.axis(cross_axis))
            else {
                return StackerLayoutResult {
                    cell_specs: standard_cell_specs,
                    measured_size: None,
                };
            };
            measured_cross
        } else {
            cross_bound
        };

        if !autosize_cross_axis && !cross_bound.is_finite() {
            return StackerLayoutResult {
                cell_specs: standard_cell_specs,
                measured_size: None,
            };
        }

        main_sizes.push(main_size);
        cross_sizes.push(cross_size);
    }

    let content_main = if slot_children_count == 0 {
        0.0
    } else {
        main_sizes.iter().sum::<f64>() + gutter_px * (slot_children_count.saturating_sub(1) as f64)
    };
    let content_cross = cross_sizes.iter().copied().fold(0.0, f64::max);

    let mut cursor = 0.0;
    let cell_specs = (0..slot_children_count)
        .map(|index| {
            let main_size = main_sizes[index];
            let cross_size = cross_sizes[index];
            let cell = match &direction {
                StackerDirection::Horizontal => StackerCell {
                    width_px: main_size,
                    height_px: cross_size,
                    x_px: cursor,
                    y_px: 0.0,
                },
                StackerDirection::Vertical => StackerCell {
                    width_px: cross_size,
                    height_px: main_size,
                    x_px: 0.0,
                    y_px: cursor,
                },
            };
            cursor += main_size + gutter_px;
            cell
        })
        .collect();

    let measured_size = if autosize_main_axis || autosize_cross_axis {
        Some(match &direction {
            StackerDirection::Horizontal => (
                if autosize_main_axis {
                    content_main
                } else {
                    bounds.0
                },
                if autosize_cross_axis {
                    content_cross
                } else {
                    bounds.1
                },
            ),
            StackerDirection::Vertical => (
                if autosize_cross_axis {
                    content_cross
                } else {
                    bounds.0
                },
                if autosize_main_axis {
                    content_main
                } else {
                    bounds.1
                },
            ),
        })
    } else {
        None
    };

    StackerLayoutResult {
        cell_specs,
        measured_size,
    }
}

fn resolve_axis_autosize(
    autosize: bool,
    axis_override: Option<bool>,
    default_when_enabled: bool,
) -> bool {
    axis_override.unwrap_or(autosize && default_when_enabled)
}

fn resolve_stacker_autosize_axes(
    direction: &StackerDirection,
    autosize: bool,
    autosize_x: Option<bool>,
    autosize_y: Option<bool>,
) -> (bool, bool) {
    let (default_x, default_y) = match direction {
        StackerDirection::Horizontal => (true, false),
        StackerDirection::Vertical => (false, true),
    };
    (
        resolve_axis_autosize(autosize, autosize_x, default_x),
        resolve_axis_autosize(autosize, autosize_y, default_y),
    )
}

fn compute_standard_cell_specs(
    direction: &StackerDirection,
    bounds: (f64, f64),
    gutter: Size,
    sizes: &[Option<Size>],
    slot_children_count: usize,
) -> Vec<StackerCell> {
    if slot_children_count == 0 {
        return vec![];
    }

    let active_bound = axis_value(
        bounds,
        match direction {
            StackerDirection::Horizontal => Axis::X,
            StackerDirection::Vertical => Axis::Y,
        },
    );
    let gutter_px = gutter.get_pixels(active_bound);
    let usable_interior_space =
        active_bound - gutter_px * (slot_children_count.saturating_sub(1) as f64);
    let mut cell_space =
        vec![usable_interior_space / slot_children_count as f64; slot_children_count];

    let mut used_space = 0.0;
    let mut remaining_indices = Vec::new();
    for (index, size) in sizes.iter().take(slot_children_count).enumerate() {
        if let Some(size) = size {
            let space = size.get_pixels(active_bound);
            used_space += space;
            cell_space[index] = space;
        } else {
            remaining_indices.push(index);
        }
    }

    if !remaining_indices.is_empty() {
        let remaining_per_cell_space =
            ((usable_interior_space - used_space) / remaining_indices.len() as f64).max(5.0);
        for index in remaining_indices {
            cell_space[index] = remaining_per_cell_space;
        }
    }

    let mut cursor = 0.0;
    (0..slot_children_count)
        .map(|index| {
            let space = cell_space[index];
            let cell = match direction {
                StackerDirection::Horizontal => StackerCell {
                    width_px: space,
                    height_px: bounds.1,
                    x_px: cursor,
                    y_px: 0.0,
                },
                StackerDirection::Vertical => StackerCell {
                    width_px: bounds.0,
                    height_px: space,
                    x_px: 0.0,
                    y_px: cursor,
                },
            };
            cursor += space + gutter_px;
            cell
        })
        .collect()
}

fn measure_child_bounds(
    _parent: &std::rc::Rc<ExpandedNode>,
    child: &std::rc::Rc<ExpandedNode>,
) -> ChildMeasurement {
    child_measurement_from_hull(child.subtree_layout_hull.get())
}

fn child_measurement_from_hull(hull: LayoutHull) -> ChildMeasurement {
    ChildMeasurement {
        width: hull.forward_extent_x(),
        height: hull.forward_extent_y(),
    }
}

fn axis_value(bounds: (f64, f64), axis: Axis) -> f64 {
    match axis {
        Axis::X => bounds.0,
        Axis::Y => bounds.1,
    }
}

fn size_depends_on_parent(size: Size) -> bool {
    const EPSILON: f64 = f64::EPSILON;
    match size {
        Size::Pixels(_) => false,
        Size::Percent(percent) => percent.to_float().abs() > EPSILON,
        Size::Combined(_, percent) => percent.to_float().abs() > EPSILON,
    }
}

// Internal cell rectangle mirrored into container frames.
#[pax]
#[engine_import_path("pax_engine")]
pub struct StackerCell {
    // Cell x offset in pixels.
    pub x_px: f64,
    // Cell y offset in pixels.
    pub y_px: f64,
    // Cell width in pixels.
    pub width_px: f64,
    // Cell height in pixels.
    pub height_px: f64,
}

/// Flow direction for a `Stacker`.
#[pax]
#[engine_import_path("pax_engine")]
pub enum StackerDirection {
    /// Stack children top-to-bottom.
    #[default]
    Vertical,
    /// Stack children left-to-right.
    Horizontal,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_cell(cell: &StackerCell, x: f64, y: f64, width: f64, height: f64) {
        assert_eq!(cell.x_px, x);
        assert_eq!(cell.y_px, y);
        assert_eq!(cell.width_px, width);
        assert_eq!(cell.height_px, height);
    }

    #[test]
    fn autosize_vertical_uses_child_measurements() {
        let layout = compute_stacker_layout(
            StackerDirection::Vertical,
            (120.0, 200.0),
            Size::Pixels(8.into()),
            vec![],
            true,
            Some(true),
            None,
            &[
                ChildMeasurement {
                    width: Some(60.0),
                    height: Some(24.0),
                },
                ChildMeasurement {
                    width: Some(90.0),
                    height: Some(40.0),
                },
            ],
            2,
            false,
            false,
        );

        assert_eq!(layout.measured_size, Some((90.0, 72.0)));
        assert_eq!(layout.cell_specs.len(), 2);
        assert_cell(&layout.cell_specs[0], 0.0, 0.0, 60.0, 24.0);
        assert_cell(&layout.cell_specs[1], 0.0, 32.0, 90.0, 40.0);
    }

    #[test]
    fn autosize_falls_back_when_main_axis_cannot_be_measured() {
        let layout = compute_stacker_layout(
            StackerDirection::Vertical,
            (120.0, 200.0),
            Size::Pixels(8.into()),
            vec![],
            true,
            None,
            None,
            &[
                ChildMeasurement {
                    width: Some(60.0),
                    height: None,
                },
                ChildMeasurement {
                    width: Some(90.0),
                    height: Some(40.0),
                },
            ],
            2,
            false,
            false,
        );

        assert_eq!(layout.measured_size, None);
        assert_eq!(layout.cell_specs.len(), 2);
        assert_cell(&layout.cell_specs[0], 0.0, 0.0, 120.0, 96.0);
        assert_cell(&layout.cell_specs[1], 0.0, 104.0, 120.0, 96.0);
    }

    #[test]
    fn autosize_uses_explicit_cross_axis_when_child_widths_fill_parent() {
        let layout = compute_stacker_layout(
            StackerDirection::Vertical,
            (160.0, 240.0),
            Size::Pixels(10.into()),
            vec![],
            true,
            None,
            None,
            &[
                ChildMeasurement {
                    width: None,
                    height: Some(30.0),
                },
                ChildMeasurement {
                    width: None,
                    height: Some(50.0),
                },
            ],
            2,
            true,
            false,
        );

        assert_eq!(layout.measured_size, Some((160.0, 90.0)));
        assert_eq!(layout.cell_specs.len(), 2);
        assert_cell(&layout.cell_specs[0], 0.0, 0.0, 160.0, 30.0);
        assert_cell(&layout.cell_specs[1], 0.0, 40.0, 160.0, 50.0);
    }

    #[test]
    fn autosize_defaults_to_extending_axis_only() {
        let vertical = compute_stacker_layout(
            StackerDirection::Vertical,
            (160.0, 240.0),
            Size::Pixels(10.into()),
            vec![],
            true,
            None,
            None,
            &[
                ChildMeasurement {
                    width: None,
                    height: Some(30.0),
                },
                ChildMeasurement {
                    width: None,
                    height: Some(50.0),
                },
            ],
            2,
            false,
            false,
        );
        assert_eq!(vertical.measured_size, Some((160.0, 90.0)));

        let horizontal = compute_stacker_layout(
            StackerDirection::Horizontal,
            (160.0, 240.0),
            Size::Pixels(10.into()),
            vec![],
            true,
            None,
            None,
            &[
                ChildMeasurement {
                    width: Some(30.0),
                    height: None,
                },
                ChildMeasurement {
                    width: Some(50.0),
                    height: None,
                },
            ],
            2,
            false,
            false,
        );
        assert_eq!(horizontal.measured_size, Some((90.0, 240.0)));
    }

    #[test]
    fn axis_overrides_can_disable_default_or_enable_cross_axis() {
        let layout = compute_stacker_layout(
            StackerDirection::Vertical,
            (120.0, 200.0),
            Size::Pixels(8.into()),
            vec![],
            true,
            Some(true),
            Some(false),
            &[
                ChildMeasurement {
                    width: Some(60.0),
                    height: Some(24.0),
                },
                ChildMeasurement {
                    width: Some(90.0),
                    height: Some(40.0),
                },
            ],
            2,
            false,
            false,
        );

        assert_eq!(layout.measured_size, Some((90.0, 200.0)));
        assert_eq!(layout.cell_specs.len(), 2);
        assert_cell(&layout.cell_specs[0], 0.0, 0.0, 60.0, 96.0);
        assert_cell(&layout.cell_specs[1], 0.0, 104.0, 90.0, 96.0);
    }

    #[test]
    fn child_measurement_uses_local_hull_extent() {
        let measurement = child_measurement_from_hull(LayoutHull::from_axis_ranges(
            Some((0.0, 60.0)),
            Some((0.0, 72.0)),
        ));

        assert_eq!(measurement.width, Some(60.0));
        assert_eq!(measurement.height, Some(72.0));
    }
    use pax_engine::api::pax_value::{PaxAny, ToFromPaxAny};
    use pax_engine::api::{Platform, Property, OS};
    use pax_engine::pax_manifest::cartridge_generation::{
        ComponentTransitionConfig, TRANSITION_PHASE_EXIT,
    };
    use pax_language::interpreter::{PaxAccessor, PaxExpression, PaxIdentifier, PaxPrimary};
    use pax_manifest::ExpressionInfo;
    use pax_runtime::api::CommonProperties;
    use pax_runtime::api::{PaxValue, ToPaxValue};
    use pax_runtime::{
        BaseInstance, CommonPropertiesInit, ComponentInstance, ExpandedNode, Globals,
        InstanceFlags, InstanceNode, InstantiationArgs, PropertiesInit, PropertiesScopeInit,
        RepeatInstance, RepeatProperties, RouteLocation, RuntimeContext,
        RuntimePropertiesStackFrame, TransformAndBounds,
    };
    use std::fmt;

    fn frame(x: f64, y: f64, width: f64, height: f64) -> ContainerFrame {
        ContainerFrame {
            transform: Transform2::translate(Vector2::new(x, y)),
            bounds: (width, height),
        }
    }

    fn test_globals() -> Globals {
        Globals {
            frames_elapsed: Property::new(0),
            elapsed_millis: Property::new(0),
            viewport: Property::new(TransformAndBounds {
                transform: Transform2::identity(),
                bounds: (100.0, 100.0),
            }),
            route_location: Property::new(RouteLocation::root()),
            browser_allows_scroller_vector_layers: Property::new(true),
            browser_allows_nested_scroller_vector_layers: Property::new(true),
            platform: Platform::Unknown,
            os: OS::Unknown,
            get_elapsed_millis: Rc::new(|| 0),
        }
    }

    fn default_properties_factory() -> Box<
        dyn Fn(
            Rc<RuntimePropertiesStackFrame>,
            Option<Rc<ExpandedNode>>,
        ) -> Option<Rc<RefCell<PaxAny>>>,
    > {
        Box::new(|_, _| Some(Rc::new(RefCell::new(PaxAny::Builtin(Default::default())))))
    }

    fn default_common_properties_factory() -> Box<
        dyn Fn(
            Rc<RuntimePropertiesStackFrame>,
            Option<Rc<ExpandedNode>>,
        ) -> Option<Rc<RefCell<CommonProperties>>>,
    > {
        Box::new(|_, _| Some(Rc::new(RefCell::new(CommonProperties::default()))))
    }

    fn component_args(template: Vec<Rc<dyn InstanceNode>>) -> InstantiationArgs {
        InstantiationArgs {
            prototypical_common_properties: CommonPropertiesInit::Factory(
                default_common_properties_factory(),
            ),
            prototypical_properties: PropertiesInit::Factory(default_properties_factory()),
            handler_registry: None,
            children: None,
            component_template: Some(RefCell::new(template)),
            component_settings: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: PropertiesScopeInit::None,
        }
    }

    fn primitive_args(children: Vec<Rc<dyn InstanceNode>>) -> InstantiationArgs {
        InstantiationArgs {
            prototypical_common_properties: CommonPropertiesInit::Factory(
                default_common_properties_factory(),
            ),
            prototypical_properties: PropertiesInit::Factory(default_properties_factory()),
            handler_registry: None,
            children: Some(RefCell::new(children)),
            component_template: None,
            component_settings: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: PropertiesScopeInit::None,
        }
    }

    fn stacker_args_with(
        children: Vec<Rc<dyn InstanceNode>>,
        stacker: Stacker,
    ) -> InstantiationArgs {
        InstantiationArgs {
            prototypical_common_properties: CommonPropertiesInit::Factory(
                default_common_properties_factory(),
            ),
            prototypical_properties: PropertiesInit::Factory(Box::new(move |_, _| {
                Some(Rc::new(RefCell::new(stacker.clone().to_pax_any())))
            })),
            handler_registry: None,
            children: Some(RefCell::new(children)),
            component_template: None,
            component_settings: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: PropertiesScopeInit::None,
        }
    }

    fn stacker_args(children: Vec<Rc<dyn InstanceNode>>) -> InstantiationArgs {
        stacker_args_with(children, Stacker::default())
    }

    fn horizontal_stacker_properties() -> Stacker {
        Stacker {
            direction: Property::new(StackerDirection::Horizontal),
            gutter: Property::new(Size::Pixels(10.into())),
            ..Default::default()
        }
    }

    fn leaf_component_args(transition_config: ComponentTransitionConfig) -> InstantiationArgs {
        InstantiationArgs {
            prototypical_common_properties: CommonPropertiesInit::Factory(
                default_common_properties_factory(),
            ),
            prototypical_properties: PropertiesInit::Factory(default_properties_factory()),
            handler_registry: None,
            children: None,
            component_template: Some(RefCell::new(Vec::new())),
            component_settings: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config,
            properties_scope: PropertiesScopeInit::None,
        }
    }

    fn repeat_args(
        source: Property<PaxValue>,
        children: Vec<Rc<dyn InstanceNode>>,
    ) -> InstantiationArgs {
        let key_expression =
            ExpressionInfo::new(PaxExpression::Primary(Box::new(PaxPrimary::Identifier(
                PaxIdentifier::new("item"),
                vec![PaxAccessor::Struct("id".to_string())],
            ))));
        let source_for_factory = source.clone();
        InstantiationArgs {
            prototypical_common_properties: CommonPropertiesInit::Factory(
                default_common_properties_factory(),
            ),
            prototypical_properties: PropertiesInit::Factory(Box::new(move |_, expanded_node| {
                expanded_node.is_none().then(|| {
                    Rc::new(RefCell::new(
                        RepeatProperties {
                            source_expression: source_for_factory.clone(),
                            iterator_i_symbol: Property::new(Some("i".to_string())),
                            iterator_elem_symbol: Property::new(Some("item".to_string())),
                            repeat_key_expression: Some(key_expression.clone()),
                        }
                        .to_pax_any(),
                    ))
                })
            })),
            handler_registry: None,
            children: Some(RefCell::new(children)),
            component_template: None,
            component_settings: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: PropertiesScopeInit::None,
        }
    }

    fn item(id: &str) -> PaxValue {
        PaxValue::Object(
            vec![("id".to_string(), id.to_string().to_pax_value())]
                .into_iter()
                .collect(),
        )
    }

    fn source(ids: &[&str]) -> PaxValue {
        PaxValue::Vec(ids.iter().map(|id| item(id)).collect())
    }

    struct TestLeaf {
        base: BaseInstance,
    }

    impl InstanceNode for TestLeaf {
        fn instantiate(args: InstantiationArgs) -> Rc<Self>
        where
            Self: Sized,
        {
            Rc::new(Self {
                base: BaseInstance::new(
                    args,
                    InstanceFlags {
                        invisible_to_slot: false,
                        invisible_to_raycasting: false,
                        layer: Layer::DontCare,
                        is_component: false,
                        is_slot: false,
                    },
                ),
            })
        }

        fn resolve_debug(
            &self,
            f: &mut fmt::Formatter,
            _expanded_node: Option<&ExpandedNode>,
        ) -> fmt::Result {
            f.debug_struct("TestLeaf").finish()
        }

        fn base(&self) -> &BaseInstance {
            &self.base
        }
    }

    #[test]
    fn compute_cell_specs_returns_empty_for_zero_children() {
        assert!(compute_cell_specs(
            0,
            (200.0, 120.0),
            StackerDirection::Horizontal,
            Size::Pixels(8.into()),
            Vec::new(),
        )
        .is_empty());
    }

    #[test]
    fn enter_transition_activity_ends_when_playhead_reaches_duration() {
        assert!(enter_transition_active(
            TRANSITION_PHASE_ENTER,
            5.0,
            80.0,
            10,
            None
        ));
        assert!(!enter_transition_active(
            TRANSITION_PHASE_ENTER,
            10.0,
            160.0,
            10,
            None
        ));
        assert!(!enter_transition_active(
            TRANSITION_PHASE_ENTER,
            15.0,
            240.0,
            10,
            None
        ));
        assert!(!enter_transition_active(
            TRANSITION_PHASE_EXIT,
            5.0,
            80.0,
            10,
            None
        ));
        assert!(enter_transition_active(
            TRANSITION_PHASE_ENTER,
            10.0,
            99.0,
            1,
            Some(100)
        ));
    }

    #[test]
    fn stacker_reflow_defaults_to_snap() {
        assert!(matches!(
            ContainerReflowTransitionKind::default(),
            ContainerReflowTransitionKind::Snap
        ));
        assert!(matches!(
            ContainerReflowTransition::default().kind.get(),
            ContainerReflowTransitionKind::Snap
        ));
        assert!(matches!(
            Stacker::default().reflow_transition.get().kind.get(),
            ContainerReflowTransitionKind::Snap
        ));
    }

    #[test]
    fn ghost_reflow_keeps_exit_frame_and_animates_survivors() {
        let children = vec![
            StackerChildSnapshot {
                id: 1_u32,
                exiting: false,
                entering: false,
            },
            StackerChildSnapshot {
                id: 2_u32,
                exiting: true,
                entering: false,
            },
            StackerChildSnapshot {
                id: 3_u32,
                exiting: false,
                entering: false,
            },
        ];
        let prior_frames = HashMap::from([
            (1_u32, frame(0.0, 0.0, 100.0, 30.0)),
            (2_u32, frame(0.0, 40.0, 100.0, 30.0)),
            (3_u32, frame(0.0, 80.0, 100.0, 30.0)),
        ]);
        let current_frames = prior_frames.clone();

        let plan = plan_stacker_layout(
            &children,
            &prior_frames,
            &current_frames,
            ContainerExitMode::Ghost,
            ContainerReflowTransitionKind::Ease,
            (100.0, 70.0),
            StackerDirection::Vertical,
            Size::Pixels(10.into()),
            Vec::new(),
        );

        assert_eq!(
            compute_cell_specs(
                2,
                (100.0, 70.0),
                StackerDirection::Vertical,
                Size::Pixels(10.into()),
                Vec::new()
            )
            .len(),
            2
        );
        assert_eq!(plan.placements.len(), 3);
        assert_eq!(plan.placements[0].id, 1_u32);
        assert_eq!(plan.placements[0].frame, frame(0.0, 0.0, 100.0, 30.0));
        assert!(!plan.placements[0].animate);
        assert_eq!(plan.placements[1].id, 2_u32);
        assert_eq!(plan.placements[1].frame, frame(0.0, 40.0, 100.0, 30.0));
        assert!(!plan.placements[1].animate);
        assert_eq!(plan.placements[2].id, 3_u32);
        assert_eq!(plan.placements[2].frame, frame(0.0, 40.0, 100.0, 30.0));
        assert!(plan.placements[2].animate);
    }

    #[test]
    fn flow_mode_keeps_exiting_child_in_prior_slot() {
        let children = vec![
            StackerChildSnapshot {
                id: 1_u32,
                exiting: false,
                entering: false,
            },
            StackerChildSnapshot {
                id: 3_u32,
                exiting: false,
                entering: false,
            },
            StackerChildSnapshot {
                id: 2_u32,
                exiting: true,
                entering: false,
            },
        ];
        let prior_frames = HashMap::from([
            (1_u32, frame(0.0, 0.0, 100.0, 30.0)),
            (2_u32, frame(0.0, 40.0, 100.0, 30.0)),
            (3_u32, frame(0.0, 80.0, 100.0, 30.0)),
        ]);
        let current_frames = prior_frames.clone();

        let plan = plan_stacker_layout(
            &children,
            &prior_frames,
            &current_frames,
            ContainerExitMode::Flow,
            ContainerReflowTransitionKind::Snap,
            (100.0, 110.0),
            StackerDirection::Vertical,
            Size::Pixels(10.into()),
            vec![
                Some(Size::Pixels(30.into())),
                Some(Size::Pixels(30.into())),
                Some(Size::Pixels(30.into())),
            ],
        );

        assert_eq!(plan.placements.len(), 3);
        assert_eq!(plan.placements[0].id, 1_u32);
        assert_eq!(plan.placements[0].frame, frame(0.0, 0.0, 100.0, 30.0));
        assert_eq!(plan.placements[1].id, 3_u32);
        assert_eq!(plan.placements[1].frame, frame(0.0, 80.0, 100.0, 30.0));
        assert_eq!(plan.placements[2].id, 2_u32);
        assert_eq!(plan.placements[2].frame, frame(0.0, 40.0, 100.0, 30.0));
    }

    #[test]
    fn flow_mode_places_entering_child_by_target_layout_not_overfull_flow_count() {
        let children = vec![
            StackerChildSnapshot {
                id: 1_u32,
                exiting: false,
                entering: false,
            },
            StackerChildSnapshot {
                id: 3_u32,
                exiting: false,
                entering: false,
            },
            StackerChildSnapshot {
                id: 4_u32,
                exiting: false,
                entering: true,
            },
            StackerChildSnapshot {
                id: 2_u32,
                exiting: true,
                entering: false,
            },
        ];
        let prior_frames = HashMap::from([
            (1_u32, frame(0.0, 0.0, 100.0, 30.0)),
            (2_u32, frame(0.0, 40.0, 100.0, 30.0)),
            (3_u32, frame(0.0, 80.0, 100.0, 30.0)),
        ]);
        let current_frames = prior_frames.clone();

        let plan = plan_stacker_layout(
            &children,
            &prior_frames,
            &current_frames,
            ContainerExitMode::Flow,
            ContainerReflowTransitionKind::Snap,
            (100.0, 110.0),
            StackerDirection::Vertical,
            Size::Pixels(10.into()),
            vec![
                Some(Size::Pixels(30.into())),
                Some(Size::Pixels(30.into())),
                Some(Size::Pixels(30.into())),
            ],
        );

        let entering = plan
            .placements
            .iter()
            .find(|placement| placement.id == 4_u32)
            .unwrap();
        assert_eq!(entering.frame, frame(0.0, 120.0, 100.0, 30.0));
    }

    #[test]
    fn flow_mode_keeps_initial_entering_children_in_active_slots() {
        let children = vec![
            StackerChildSnapshot {
                id: 1_u32,
                exiting: false,
                entering: true,
            },
            StackerChildSnapshot {
                id: 2_u32,
                exiting: false,
                entering: true,
            },
        ];

        let plan = plan_stacker_layout(
            &children,
            &HashMap::new(),
            &HashMap::new(),
            ContainerExitMode::Flow,
            ContainerReflowTransitionKind::Snap,
            (100.0, 70.0),
            StackerDirection::Vertical,
            Size::Pixels(10.into()),
            vec![Some(Size::Pixels(30.into())), Some(Size::Pixels(30.into()))],
        );

        assert_eq!(plan.placements.len(), 2);
        assert_eq!(plan.placements[0].frame, frame(0.0, 0.0, 100.0, 30.0));
        assert_eq!(plan.placements[1].frame, frame(0.0, 40.0, 100.0, 30.0));
    }

    #[test]
    fn flow_reflow_ease_marches_stable_and_entering_children_toward_active_layout() {
        let children = vec![
            StackerChildSnapshot {
                id: 1_u32,
                exiting: false,
                entering: false,
            },
            StackerChildSnapshot {
                id: 3_u32,
                exiting: false,
                entering: false,
            },
            StackerChildSnapshot {
                id: 4_u32,
                exiting: false,
                entering: true,
            },
            StackerChildSnapshot {
                id: 2_u32,
                exiting: true,
                entering: false,
            },
        ];
        let prior_frames = HashMap::from([
            (1_u32, frame(0.0, 0.0, 100.0, 30.0)),
            (2_u32, frame(0.0, 40.0, 100.0, 30.0)),
            (3_u32, frame(0.0, 80.0, 100.0, 30.0)),
        ]);
        let current_frames = prior_frames.clone();

        let plan = plan_stacker_layout(
            &children,
            &prior_frames,
            &current_frames,
            ContainerExitMode::Flow,
            ContainerReflowTransitionKind::Ease,
            (100.0, 110.0),
            StackerDirection::Vertical,
            Size::Pixels(10.into()),
            vec![
                Some(Size::Pixels(30.into())),
                Some(Size::Pixels(30.into())),
                Some(Size::Pixels(30.into())),
            ],
        );

        let stable = plan
            .placements
            .iter()
            .find(|placement| placement.id == 3_u32)
            .unwrap();
        assert_eq!(stable.seed_frame, None);
        assert_eq!(stable.frame, frame(0.0, 40.0, 100.0, 30.0));
        assert!(stable.animate);

        let entering = plan
            .placements
            .iter()
            .find(|placement| placement.id == 4_u32)
            .unwrap();
        assert_eq!(entering.seed_frame, Some(frame(0.0, 120.0, 100.0, 30.0)));
        assert_eq!(entering.frame, frame(0.0, 80.0, 100.0, 30.0));
        assert!(entering.animate);

        let exiting = plan
            .placements
            .iter()
            .find(|placement| placement.id == 2_u32)
            .unwrap();
        assert_eq!(exiting.seed_frame, None);
        assert_eq!(exiting.frame, frame(0.0, 40.0, 100.0, 30.0));
        assert!(!exiting.animate);
    }

    #[test]
    fn flow_reflow_ease_keeps_exiting_child_on_prior_frame() {
        let children = vec![
            StackerChildSnapshot {
                id: 1_u32,
                exiting: false,
                entering: false,
            },
            StackerChildSnapshot {
                id: 2_u32,
                exiting: true,
                entering: false,
            },
            StackerChildSnapshot {
                id: 3_u32,
                exiting: false,
                entering: false,
            },
        ];
        let prior_frames = HashMap::from([
            (1_u32, frame(0.0, 0.0, 100.0, 30.0)),
            (2_u32, frame(0.0, 40.0, 100.0, 30.0)),
            (3_u32, frame(0.0, 80.0, 100.0, 30.0)),
        ]);
        let current_frames = HashMap::from([
            (1_u32, frame(0.0, 0.0, 100.0, 30.0)),
            (2_u32, frame(0.0, 40.0, 100.0, 30.0)),
            (3_u32, frame(0.0, 80.0, 100.0, 30.0)),
        ]);

        let plan = plan_stacker_layout(
            &children,
            &prior_frames,
            &current_frames,
            ContainerExitMode::Flow,
            ContainerReflowTransitionKind::Ease,
            (100.0, 70.0),
            StackerDirection::Vertical,
            Size::Pixels(10.into()),
            Vec::new(),
        );

        let exiting = plan
            .placements
            .iter()
            .find(|placement| placement.id == 2_u32)
            .unwrap();
        assert_eq!(exiting.frame, frame(0.0, 40.0, 100.0, 30.0));
        assert!(!exiting.animate);
    }

    #[test]
    fn stacker_assigns_distinct_container_frames_to_projected_children() {
        let leaf_a: Rc<dyn InstanceNode> = TestLeaf::instantiate(primitive_args(Vec::new()));
        let leaf_b: Rc<dyn InstanceNode> = TestLeaf::instantiate(primitive_args(Vec::new()));
        let leaf_c: Rc<dyn InstanceNode> = TestLeaf::instantiate(primitive_args(Vec::new()));
        let stacker: Rc<dyn InstanceNode> = StackerInstance::instantiate(stacker_args_with(
            vec![leaf_a, leaf_b, leaf_c],
            horizontal_stacker_properties(),
        ));
        let root_component = ComponentInstance::instantiate(component_args(vec![stacker]));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let stacker_node = root.children.get().first().cloned().unwrap();
        let children = stacker_node.children.get();
        assert_eq!(children.len(), 3);

        let x_positions: Vec<_> = children
            .iter()
            .map(|child| child.transform_and_bounds.get().transform.m[4])
            .collect();
        assert_eq!(x_positions.len(), 3);
        assert!((x_positions[0] - 0.0).abs() < 1e-9);
        assert!((x_positions[1] - 36.666666666666664).abs() < 1e-9);
        assert!((x_positions[2] - 73.33333333333333).abs() < 1e-9);
    }

    #[test]
    fn stacker_assigns_distinct_container_frames_on_initial_mount() {
        let leaf_a: Rc<dyn InstanceNode> = TestLeaf::instantiate(primitive_args(Vec::new()));
        let leaf_b: Rc<dyn InstanceNode> = TestLeaf::instantiate(primitive_args(Vec::new()));
        let leaf_c: Rc<dyn InstanceNode> = TestLeaf::instantiate(primitive_args(Vec::new()));
        let stacker: Rc<dyn InstanceNode> = StackerInstance::instantiate(stacker_args_with(
            vec![leaf_a, leaf_b, leaf_c],
            horizontal_stacker_properties(),
        ));
        let root_component = ComponentInstance::instantiate(component_args(vec![stacker]));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let stacker_node = root.children.get().first().cloned().unwrap();
        let children = stacker_node.children.get();
        assert_eq!(children.len(), 3);

        let x_positions: Vec<_> = children
            .iter()
            .map(|child| child.transform_and_bounds.get().transform.m[4])
            .collect();
        assert_eq!(x_positions.len(), 3);
        assert!((x_positions[0] - 0.0).abs() < 1e-9);
        assert!((x_positions[1] - 36.666666666666664).abs() < 1e-9);
        assert!((x_positions[2] - 73.33333333333333).abs() < 1e-9);
    }

    #[test]
    fn stacker_assigns_distinct_container_frames_to_projected_component_children() {
        let component_a: Rc<dyn InstanceNode> =
            ComponentInstance::instantiate(component_args(Vec::new()));
        let component_b: Rc<dyn InstanceNode> =
            ComponentInstance::instantiate(component_args(Vec::new()));
        let component_c: Rc<dyn InstanceNode> =
            ComponentInstance::instantiate(component_args(Vec::new()));
        let stacker: Rc<dyn InstanceNode> = StackerInstance::instantiate(stacker_args_with(
            vec![component_a, component_b, component_c],
            horizontal_stacker_properties(),
        ));
        let root_component = ComponentInstance::instantiate(component_args(vec![stacker]));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let stacker_node = root.children.get().first().cloned().unwrap();
        let children = stacker_node.children.get();
        assert_eq!(children.len(), 3);

        let x_positions: Vec<_> = children
            .iter()
            .map(|child| child.transform_and_bounds.get().transform.m[4])
            .collect();
        assert_eq!(x_positions.len(), 3);
        assert!((x_positions[0] - 0.0).abs() < 1e-9);
        assert!((x_positions[1] - 36.666666666666664).abs() < 1e-9);
        assert!((x_positions[2] - 73.33333333333333).abs() < 1e-9);
    }

    #[test]
    fn stacker_retains_single_exiting_child_for_projected_keyed_repeat_changes() {
        let source_property = Property::new(source(&["a", "b", "c"]));
        let leaf: Rc<dyn InstanceNode> =
            ComponentInstance::instantiate(leaf_component_args(ComponentTransitionConfig {
                has_enter: true,
                enter_frame_count: 10,
                has_exit: true,
                exit_frame_count: 10,
                timeout_ms: 5_000,
                ..Default::default()
            }));
        let repeat: Rc<dyn InstanceNode> =
            RepeatInstance::instantiate(repeat_args(source_property.clone(), vec![leaf]));
        let stacker: Rc<dyn InstanceNode> =
            StackerInstance::instantiate(stacker_args(vec![repeat]));
        let root_component = ComponentInstance::instantiate(component_args(vec![stacker]));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let stacker_node = root.children.get().first().cloned().unwrap();
        let initial_active = borrow!(stacker_node.active_children).clone();
        assert_eq!(initial_active.len(), 3);
        let initial_ids: Vec<_> = initial_active.iter().map(|child| child.id).collect();

        source_property.set(source(&["a", "c", "d"]));
        root.recurse_update(&context);

        let active = borrow!(stacker_node.active_children).clone();
        let exiting = borrow!(stacker_node.exiting_children).clone();
        let node_ctx = stacker_node.get_node_context(&context);
        let received_children = node_ctx.received_children.get();
        let retained_received_children = node_ctx.retained_received_children.get();
        assert_eq!(active.len(), 3);
        assert_eq!(exiting.len(), 1);
        assert_eq!(received_children.len(), 3);
        assert_eq!(retained_received_children.len(), 1);
        assert_eq!(active[0].id, initial_ids[0]);
        assert_eq!(active[1].id, initial_ids[2]);
        assert_ne!(active[2].id, initial_ids[0]);
        assert_ne!(active[2].id, initial_ids[1]);
        assert_ne!(active[2].id, initial_ids[2]);
        assert_eq!(exiting[0].id, initial_ids[1]);
        assert_eq!(exiting[0].transition_phase.get(), TRANSITION_PHASE_EXIT);
        assert_eq!(received_children[0].id, initial_ids[0]);
        assert_eq!(received_children[1].id, initial_ids[2]);
        assert_eq!(retained_received_children[0].id, initial_ids[1]);
    }
}
