use crate::common::{native_surface_opacity, patch_if_needed};
#[allow(unused)]
use crate::*;
use kurbo::{Affine, BezPath};
use pax_engine::api::{Event, Wheel};
use pax_engine::api::{Property, Size};
use pax_engine::*;
use pax_message::{AnyCreatePatch, NativeInterrupt, ScrollerPatch};
use pax_runtime::api::{
    borrow, borrow_mut, use_RefCell, Layer, NodeContext, Platform, TouchEnd, TouchMove, TouchStart,
    OS,
};
use pax_runtime::{
    BaseInstance, ExpandedNode, ExpandedNodeIdentifier, InstanceFlags, InstanceNode,
    InstantiationArgs, RuntimeContext,
};
use std::collections::HashMap;
use std::iter;
use std::rc::Rc;

use_RefCell!();

/// A scrolling container for arbitrary content.
#[pax]
#[engine_import_path("pax_engine")]
#[inlined(
    if self._native_scrolling {
        <ScrollerHost
            scroll_pos_x=bind:scroll_pos_x
            scroll_pos_y=bind:scroll_pos_y
            scroll_width={self.scroll_width}
            scroll_height={self.scroll_height}
            _clip_content={!$suspended}
        >
            for i in 0..self._slot_children_count {
                slot(i)
            }
        </ScrollerHost>
    }

    if !self._native_scrolling {
        <Frame _clip_content={!$suspended}>
            // WARNING: changing the ID of this group affects the designer
            <Group id=_scroller_inner_container x={(-self.scroll_pos_x)px} y={(-self.scroll_pos_y)px} width={self.scroll_width - 8px} height={self.scroll_height}>
                for i in 0..self._slot_children_count {
                    slot(i)
                }
            </Group>
            <Scrollbar
                scroll_y=bind:scroll_pos_y
                width=12px
                anchor_x=100%
                x=100%
                size_inner_pane_y={self.scroll_height}
            />
            <Scrollbar
                scroll_x=bind:scroll_pos_x
                height=12px
                anchor_y=100%
                y=100%
                size_inner_pane_x={self.scroll_width}
            />
            <Rectangle fill=TRANSPARENT/>
        </Frame>
    }

    @settings {
        @mount: on_mount
        @wheel: handle_wheel
        @pre_render: update,
        @touch_move: touch_move,
        @touch_start: touch_start,
        @touch_end: touch_end,
    }

)]
#[custom(Default)]
pub struct Scroller {
    pub scroll_pos_x: Property<f64>,
    pub scroll_pos_y: Property<f64>,
    pub scroll_width: Property<Size>,
    pub scroll_height: Property<Size>,
    pub auto_size: Property<bool>,

    // used by pax create (might want to just make public at some point)
    pub _clip_content: Property<bool>,

    // private fields
    pub _native_scrolling: Property<bool>,
    pub _platform_params: Property<PlatformSpecificScrollParams>,
    pub _momentum_x: Property<f64>,
    pub _momentum_y: Property<f64>,
    pub _damping: Property<f64>,
    pub _slot_children_count: Property<usize>,
    pub _ticks_since_mount: Property<usize>,
}

impl Default for Scroller {
    fn default() -> Self {
        Self {
            scroll_pos_x: Default::default(),
            scroll_pos_y: Default::default(),
            scroll_width: Default::default(),
            scroll_height: Default::default(),
            auto_size: Property::new(false),
            _clip_content: Property::new(true),
            _native_scrolling: Property::new(false),
            _platform_params: Default::default(),
            _momentum_x: Default::default(),
            _momentum_y: Default::default(),
            _damping: Default::default(),
            _slot_children_count: Default::default(),
            _ticks_since_mount: Property::new(0),
        }
    }
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct PlatformSpecificScrollParams {
    // constant decrease in momentum over time
    pub deacceleration: f64,
    // friction (decreases momentum proportionally to current value)
    pub damping: f64,
    // if user "flings", set to high momentum to quickly scroll
    pub fling: bool,
}

#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::core::scroller::ScrollerHostInstance")]
#[custom(Default)]
pub struct ScrollerHost {
    pub scroll_pos_x: Property<f64>,
    pub scroll_pos_y: Property<f64>,
    pub scroll_width: Property<Size>,
    pub scroll_height: Property<Size>,
    pub _presentation_scroll_x: Property<f64>,
    pub _presentation_scroll_y: Property<f64>,
    pub _clip_content: Property<bool>,
}

impl Default for ScrollerHost {
    fn default() -> Self {
        Self {
            scroll_pos_x: Default::default(),
            scroll_pos_y: Default::default(),
            scroll_width: Default::default(),
            scroll_height: Default::default(),
            _presentation_scroll_x: Default::default(),
            _presentation_scroll_y: Default::default(),
            _clip_content: Property::new(true),
        }
    }
}

pub struct ScrollerHostInstance {
    base: BaseInstance,
}

pub struct TouchInfo {
    x: f64,
    y: f64,
}

thread_local! {
    static TOUCH_TRACKER: RefCell<HashMap<i64, TouchInfo>> = RefCell::new(HashMap::new());
}
pub fn no_touches() -> bool {
    TOUCH_TRACKER.with_borrow(|touches| touches.len() == 0)
}

fn mark_canvas_descendants_dirty(expanded_node: &ExpandedNode, context: &Rc<RuntimeContext>) {
    for child in expanded_node.children.get().iter() {
        if borrow!(child.instance_node).base().flags().layer == Layer::Canvas {
            context.mark_canvas_node_dirty(child.id);
            context.set_canvas_dirty(child.occlusion.get().occlusion_layer_id);
        }
        mark_canvas_descendants_dirty(child, context);
    }
}

fn resolve_scroller_island_layer(expanded_node: &ExpandedNode) -> Option<usize> {
    let own_layer = expanded_node.occlusion.get().occlusion_layer_id;
    fn find_descendant_layer(node: &ExpandedNode, own_layer: usize) -> Option<usize> {
        let mut resolved: Option<usize> = None;
        for child in node.children.get().iter() {
            let child_layer = child.occlusion.get().occlusion_layer_id;
            if child_layer != own_layer {
                resolved = Some(match resolved {
                    Some(current) => current.min(child_layer),
                    None => child_layer,
                });
            }
            if let Some(descendant_layer) = find_descendant_layer(child, own_layer) {
                resolved = Some(match resolved {
                    Some(current) => current.min(descendant_layer),
                    None => descendant_layer,
                });
            }
        }
        resolved
    }

    find_descendant_layer(expanded_node, own_layer)
}

fn scroller_clip_path(expanded_node: &ExpandedNode, clip_content: bool) -> Option<BezPath> {
    if !clip_content {
        return None;
    }

    let t_and_b = expanded_node.transform_and_bounds.get();
    let transform = t_and_b.transform;
    let (width, height) = t_and_b.bounds;

    let mut bez_path = BezPath::new();
    bez_path.move_to((0.0, 0.0));
    bez_path.line_to((width, 0.0));
    bez_path.line_to((width, height));
    bez_path.line_to((0.0, height));
    bez_path.line_to((0.0, 0.0));
    bez_path.close_path();

    Some(<Affine>::from(transform) * bez_path)
}

impl InstanceNode for ScrollerHostInstance {
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
                    layer: Layer::Native,
                    is_component: false,
                    is_slot: false,
                },
            ),
        })
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        let id = expanded_node.id.to_u32();
        context.enqueue_native_message(pax_message::NativeMessage::ScrollerCreate(
            AnyCreatePatch {
                id,
                parent_frame: expanded_node.parent_frame.get().map(|v| v.to_u32()),
                occlusion_layer_id: 0,
            },
        ));

        let env = Rc::clone(&expanded_node.stack);
        let children = borrow!(self.base().get_instance_children());
        let children_with_envs = children.iter().cloned().zip(iter::repeat(env));
        let child_parent_frame: Property<Option<ExpandedNodeIdentifier>> =
            Property::new(Some(expanded_node.id));
        let new_children =
            expanded_node.generate_children(children_with_envs, context, &child_parent_frame, true);
        expanded_node.children.set(new_children);

        let weak_self_ref = Rc::downgrade(&expanded_node);
        let context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(ScrollerPatch {
            id,
            ..Default::default()
        }));
        let last_presentation_scroll = Rc::new(RefCell::new((0.0, 0.0)));

        let deps: Vec<_> = borrow!(expanded_node.properties_scope)
            .values()
            .cloned()
            .map(|v| v.get_untyped_property().clone())
            .chain([
                expanded_node.transform_and_bounds.untyped(),
                expanded_node.computed_opacity.untyped(),
                expanded_node.occlusion.untyped(),
            ])
            .collect();
        expanded_node
            .changed_listener
            .replace_with(Property::computed(
                move || {
                    let Some(expanded_node) = weak_self_ref.upgrade() else {
                        return;
                    };
                    let mut old_state = borrow_mut!(last_patch);
                    let mut previous_presentation_scroll = borrow_mut!(last_presentation_scroll);
                    let mut patch = ScrollerPatch {
                        id,
                        ..Default::default()
                    };

                    expanded_node.with_properties_unwrapped(|properties: &mut ScrollerHost| {
                        let computed_tab = expanded_node.transform_and_bounds.get();
                        let (width, height) = computed_tab.bounds;
                        let scroll_width = properties.scroll_width.get().get_pixels(width);
                        let scroll_height = properties.scroll_height.get().get_pixels(height);
                        let presentation_scroll = (
                            properties._presentation_scroll_x.get(),
                            properties._presentation_scroll_y.get(),
                        );
                        let presentation_changed =
                            (presentation_scroll.0 - previous_presentation_scroll.0).abs() > 1e-4
                                || (presentation_scroll.1 - previous_presentation_scroll.1).abs()
                                    > 1e-4;
                        *previous_presentation_scroll = presentation_scroll;
                        let updates = [
                            patch_if_needed(&mut old_state.size_x, &mut patch.size_x, width),
                            patch_if_needed(&mut old_state.size_y, &mut patch.size_y, height),
                            patch_if_needed(
                                &mut old_state.parent_frame,
                                &mut patch.parent_frame,
                                expanded_node.parent_frame.get().map(|v| v.to_u32()),
                            ),
                            patch_if_needed(
                                &mut old_state.z_index,
                                &mut patch.z_index,
                                expanded_node.occlusion.get().z_index,
                            ),
                            patch_if_needed(
                                &mut old_state.size_inner_pane_x,
                                &mut patch.size_inner_pane_x,
                                scroll_width,
                            ),
                            patch_if_needed(
                                &mut old_state.size_inner_pane_y,
                                &mut patch.size_inner_pane_y,
                                scroll_height,
                            ),
                            patch_if_needed(
                                &mut old_state.scroll_x,
                                &mut patch.scroll_x,
                                properties.scroll_pos_x.get(),
                            ),
                            patch_if_needed(
                                &mut old_state.scroll_y,
                                &mut patch.scroll_y,
                                properties.scroll_pos_y.get(),
                            ),
                            patch_if_needed(
                                &mut old_state.presentation_scroll_x,
                                &mut patch.presentation_scroll_x,
                                presentation_scroll.0,
                            ),
                            patch_if_needed(
                                &mut old_state.presentation_scroll_y,
                                &mut patch.presentation_scroll_y,
                                presentation_scroll.1,
                            ),
                            patch_if_needed(
                                &mut old_state.transform,
                                &mut patch.transform,
                                computed_tab.transform.coeffs().to_vec(),
                            ),
                            patch_if_needed(
                                &mut old_state.opacity,
                                &mut patch.opacity,
                                native_surface_opacity(&expanded_node, &context),
                            ),
                            patch_if_needed(
                                &mut old_state.clip_content,
                                &mut patch.clip_content,
                                properties._clip_content.get(),
                            ),
                        ];
                        let scroll_updated = patch.scroll_x.is_some()
                            || patch.scroll_y.is_some()
                            || patch.presentation_scroll_x.is_some()
                            || patch.presentation_scroll_y.is_some();
                        let visual_update = patch.size_x.is_some()
                            || patch.size_y.is_some()
                            || patch.size_inner_pane_x.is_some()
                            || patch.size_inner_pane_y.is_some()
                            || patch.transform.is_some()
                            || patch.opacity.is_some()
                            || patch.clip_content.is_some();
                        if updates.into_iter().any(|updated| updated) {
                            context.enqueue_native_message(
                                pax_message::NativeMessage::ScrollerUpdate(patch),
                            );
                            if visual_update {
                                // Browser-owned scroller islands now keep tiled canvases mounted in
                                // content coordinates, so ordinary scroll motion is handled by the
                                // browser moving the host. Reserve descendant canvas invalidation
                                // for real visual/layout changes; tile-window shifts are handled by
                                // the chassis surface-refresh path instead of rerendering every
                                // canvas node on each scroll tick.
                                mark_canvas_descendants_dirty(&expanded_node, &context);
                            }
                        } else if presentation_changed && !scroll_updated {
                            mark_canvas_descendants_dirty(&expanded_node, &context);
                        }
                    });
                },
                &deps,
            ));
    }

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        expanded_node
            .changed_listener
            .replace_with(Property::default());
        context.enqueue_native_message(pax_message::NativeMessage::ScrollerDelete(
            expanded_node.id.to_u32(),
        ));
    }

    fn handle_native_interrupt(
        &self,
        expanded_node: &Rc<ExpandedNode>,
        interrupt: &NativeInterrupt,
    ) {
        if let NativeInterrupt::Scrollbar(args) = interrupt {
            expanded_node.with_properties_unwrapped(|props: &mut ScrollerHost| {
                if (props.scroll_pos_x.get() - args.scroll_x).abs() > 1e-4 {
                    props.scroll_pos_x.set(args.scroll_x);
                }
                if (props.scroll_pos_y.get() - args.scroll_y).abs() > 1e-4 {
                    props.scroll_pos_y.set(args.scroll_y);
                }
                let presentation_scroll_x = args.presentation_scroll_x.unwrap_or(args.scroll_x);
                let presentation_scroll_y = args.presentation_scroll_y.unwrap_or(args.scroll_y);
                if (props._presentation_scroll_x.get() - presentation_scroll_x).abs() > 1e-4 {
                    props._presentation_scroll_x.set(presentation_scroll_x);
                }
                if (props._presentation_scroll_y.get() - presentation_scroll_y).abs() > 1e-4 {
                    props._presentation_scroll_y.set(presentation_scroll_y);
                }
            });
        }
    }

    fn resolve_effect_clip_path(&self, expanded_node: &ExpandedNode) -> Option<BezPath> {
        scroller_clip_path(
            expanded_node,
            expanded_node.with_properties_unwrapped(|scroller: &mut ScrollerHost| {
                scroller._clip_content.get()
            }),
        )
    }

    fn handle_pre_render(
        &self,
        expanded_node: &ExpandedNode,
        rtc: &Rc<RuntimeContext>,
        rcs: &mut dyn pax_runtime::api::RenderContext,
    ) {
        let total_layer_count = rtc.layer_count.get();
        let mut run_pre_render = false;
        for i in 0..total_layer_count {
            run_pre_render |= rtc.is_canvas_dirty(&i);
        }
        if !run_pre_render {
            return;
        }

        if resolve_scroller_island_layer(expanded_node).is_some() {
            return;
        }

        let (clip_content, scroll_x, scroll_y) =
            expanded_node.with_properties_unwrapped(|scroller: &mut ScrollerHost| {
                (
                    scroller._clip_content.get(),
                    scroller._presentation_scroll_x.get(),
                    scroller._presentation_scroll_y.get(),
                )
            });
        let should_translate = scroll_x.abs() > f64::EPSILON || scroll_y.abs() > f64::EPSILON;
        if !clip_content && !should_translate {
            return;
        }
        let transformed_bez_path = clip_content
            .then(|| self.resolve_effect_clip_path(expanded_node))
            .flatten();

        let layers = rcs.layers();
        for layer in 0..layers {
            if !rcs.begin_node(
                layer,
                expanded_node.id.to_u32(),
                expanded_node.occlusion.get().z_index,
            ) {
                continue;
            }
            rcs.save(layer);
            if let Some(transformed_bez_path) = transformed_bez_path.clone() {
                rcs.clip(layer, transformed_bez_path);
            }
            if should_translate {
                rcs.transform(layer, Affine::translate((-scroll_x, -scroll_y)));
            }
            let _ = rcs.end_node(layer, expanded_node.id.to_u32());
        }
    }

    fn handle_post_render(
        &self,
        expanded_node: &ExpandedNode,
        rtc: &Rc<RuntimeContext>,
        rcs: &mut dyn pax_runtime::api::RenderContext,
    ) {
        if resolve_scroller_island_layer(expanded_node).is_some() {
            return;
        }

        let (clip_content, scroll_x, scroll_y) =
            expanded_node.with_properties_unwrapped(|scroller: &mut ScrollerHost| {
                (
                    scroller._clip_content.get(),
                    scroller._presentation_scroll_x.get(),
                    scroller._presentation_scroll_y.get(),
                )
            });
        if !clip_content && scroll_x.abs() <= f64::EPSILON && scroll_y.abs() <= f64::EPSILON {
            return;
        }

        let total_layer_count = rtc.layer_count.get();
        let mut post_render = false;
        for i in 0..total_layer_count {
            post_render |= rtc.is_canvas_dirty(&i);
        }
        if !post_render {
            return;
        }

        for layer in 0..rcs.layers() {
            rcs.restore(layer);
        }
    }

    fn clips_content(&self, expanded_node: &ExpandedNode) -> bool {
        expanded_node
            .with_properties_unwrapped(|scroller: &mut ScrollerHost| scroller._clip_content.get())
    }

    fn scrolls_content(&self, _expanded_node: &ExpandedNode) -> bool {
        true
    }

    fn resolve_scroll_offset(&self, expanded_node: &ExpandedNode) -> Option<(f64, f64)> {
        Some(
            expanded_node.with_properties_unwrapped(|scroller: &mut ScrollerHost| {
                (
                    scroller._presentation_scroll_x.get(),
                    scroller._presentation_scroll_y.get(),
                )
            }),
        )
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        match expanded_node {
            Some(expanded_node) => {
                expanded_node.with_properties_unwrapped(|_scroller: &mut ScrollerHost| {
                    f.debug_struct("ScrollerHost").finish()
                })
            }
            None => f.debug_struct("ScrollerHost").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}

impl Scroller {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let slot_children_count = ctx.slot_children_count.clone();
        let deps = [slot_children_count.untyped()];
        self._slot_children_count
            .replace_with(Property::computed(move || slot_children_count.get(), &deps));
        self._native_scrolling
            .set(matches!(ctx.platform, Platform::Web));
        let scroll_params = match ctx.os {
            OS::Android => PlatformSpecificScrollParams {
                deacceleration: 0.02,
                damping: 0.03,
                fling: true,
            },
            OS::IPhone => PlatformSpecificScrollParams {
                deacceleration: 0.00,
                damping: 0.04,
                fling: false,
            },
            // just choose some hopefully sane default
            OS::Windows | OS::Mac | OS::Linux | OS::Unknown => PlatformSpecificScrollParams {
                deacceleration: 0.01,
                damping: 0.04,
                fling: false,
            },
        };
        self._platform_params.set(scroll_params);
    }

    fn bind_to_slot_children(&self, ctx: &NodeContext) {
        let slot_children = ctx.slot_children.clone();
        let self_transform = ctx.node_transform_and_bounds.clone();
        let slot_children_attached_listener = ctx.slot_children_attached_listener.clone();
        let scroll_position_y = self.scroll_pos_y.clone();
        let native_scrolling = self._native_scrolling.clone();
        let deps = [
            slot_children_attached_listener.untyped(),
            slot_children.untyped(),
            scroll_position_y.untyped(),
            native_scrolling.untyped(),
        ];

        if self.scroll_height.get() == Default::default() {
            self.scroll_height.replace_with(Property::computed(
                move || {
                    let slot_children = slot_children.get();
                    let mut max_height: f64 = 0.0;
                    let parent_y = self_transform.transform.m[5];
                    let scroll_offset_y = if native_scrolling.get() {
                        0.0
                    } else {
                        scroll_position_y.get()
                    };
                    for child in slot_children.iter() {
                        let tab = child.transform_and_bounds.get();
                        let y = (tab.transform.m[5] + scroll_offset_y) - parent_y;
                        max_height = max_height.max(tab.bounds.1 + y);
                    }
                    Size::Pixels(max_height.into())
                },
                &deps,
            ));
        }
    }

    pub fn update(&mut self, ctx: &NodeContext) {
        self._ticks_since_mount
            .set(self._ticks_since_mount.get() + 1);
        if self._ticks_since_mount.get() == 10 && self.auto_size.get() {
            self.bind_to_slot_children(ctx);
        }

        if self._native_scrolling.get() {
            return;
        }

        let mom_x = self._momentum_x.get();
        let mom_y = self._momentum_y.get();
        if no_touches() {
            self.add_position(ctx, mom_x, mom_y);
        }
        let platform_data = self._platform_params.get();
        let damping = self._damping.get();

        let mut new_mom_x;
        let mut new_mom_y;

        // damping
        let falloff_factor = 1.0 - damping;
        new_mom_x = mom_x * falloff_factor;
        new_mom_y = mom_y * falloff_factor;

        // decelleration
        if new_mom_x > 0.0 {
            new_mom_x = (new_mom_x - platform_data.deacceleration).max(0.0);
        }
        if new_mom_x < 0.0 {
            new_mom_x = (new_mom_x + platform_data.deacceleration).min(0.0);
        }
        if new_mom_y > 0.0 {
            new_mom_y = (new_mom_y - platform_data.deacceleration).max(0.0);
        }
        if new_mom_y < 0.0 {
            new_mom_y = (new_mom_y + platform_data.deacceleration).min(0.0);
        }

        // stop if close to 0
        if new_mom_x.abs() < 0.1 {
            new_mom_x = 0.0;
        }
        if new_mom_y.abs() < 0.1 {
            new_mom_y = 0.0;
        }

        self._momentum_x.set(new_mom_x);
        self._momentum_y.set(new_mom_y);
    }

    pub fn add_position(&self, ctx: &NodeContext, dx: f64, dy: f64) {
        let (bounds_x, bounds_y) = ctx.bounds_self.get();
        let (max_bounds_x, max_bounds_y) = (
            self.scroll_width.get().get_pixels(bounds_x),
            self.scroll_height.get().get_pixels(bounds_y),
        );
        let old_x = self.scroll_pos_x.get();
        let old_y = self.scroll_pos_y.get();
        let target_x = old_x + dx;
        let target_y = old_y + dy;

        let clamped_target_x = target_x.clamp(0.0, (max_bounds_x - bounds_x).max(0.0));
        let clamped_target_y = target_y.clamp(0.0, (max_bounds_y - bounds_y).max(0.0));
        if (self.scroll_pos_x.get() - clamped_target_x).abs() > 1e-3 {
            self.scroll_pos_x.set(clamped_target_x);
        }
        if (self.scroll_pos_y.get() - clamped_target_y).abs() > 1e-3 {
            self.scroll_pos_y.set(clamped_target_y);
        }
    }

    pub fn add_momentum(&self, ddx: f64, ddy: f64) {
        let mom_x = self._momentum_x.get();
        let mom_y = self._momentum_y.get();
        self._momentum_x.set(mom_x + ddx);
        self._momentum_y.set(mom_y + ddy);
    }

    pub fn process_new_touch_pos(&self, ctx: &NodeContext, x: f64, y: f64, ident: i64) {
        TOUCH_TRACKER.with_borrow_mut(|touches| {
            let last = touches
                .get_mut(&ident)
                .expect("should have received touch down before touch move");
            let delta_x = last.x - x;
            let delta_y = last.y - y;
            last.x = x;
            last.y = y;
            self.add_position(ctx, delta_x, delta_y);
            self.add_momentum(delta_x, delta_y);
        });
    }

    pub fn handle_wheel(&mut self, ctx: &NodeContext, args: Event<Wheel>) {
        if self._native_scrolling.get() || args.cancelled() {
            return;
        }
        let delta_x = args.delta_x;
        let delta_y = args.delta_y;
        let (done_x, done_y) = self.moving_passed_bounds(ctx, delta_x, delta_y);
        if !done_x || !done_y {
            args.prevent_default();
        }
        self.add_position(ctx, args.delta_x, args.delta_y);
    }

    fn moving_passed_bounds(&self, ctx: &NodeContext, dx: f64, dy: f64) -> (bool, bool) {
        let width = self.scroll_width.get();
        let height = self.scroll_height.get();
        let bounds = ctx.bounds_self.get();
        let max_bounds = (width.get_pixels(bounds.0), height.get_pixels(bounds.1));
        let x = self.scroll_pos_x.get();
        let y = self.scroll_pos_y.get();
        let x = x + dx;
        let y = y + dy;
        let x = x.clamp(0.0, (max_bounds.0 - bounds.0).max(0.0));
        let y = y.clamp(0.0, (max_bounds.1 - bounds.1).max(0.0));
        (x == self.scroll_pos_x.get(), y == self.scroll_pos_y.get())
    }

    pub fn touch_move(&mut self, ctx: &NodeContext, args: Event<TouchMove>) {
        if self._native_scrolling.get() {
            return;
        }
        for touch in &args.touches {
            self.process_new_touch_pos(ctx, touch.x, touch.y, touch.identifier);
        }
    }

    pub fn touch_start(&mut self, _ctx: &NodeContext, args: Event<TouchStart>) {
        if self._native_scrolling.get() {
            return;
        }
        if no_touches() {
            // this is first touch
            let cached_damping = self._platform_params.get().damping;
            let temp_damping = cached_damping.max(0.5);
            self._damping.set(temp_damping);
        }
        self._momentum_x.set(0.0);
        self._momentum_y.set(0.0);
        TOUCH_TRACKER.with_borrow_mut(|touches| {
            touches.extend(
                args.touches
                    .iter()
                    .map(|e| (e.identifier, TouchInfo { x: e.x, y: e.y })),
            );
        });
    }

    pub fn touch_end(&mut self, ctx: &NodeContext, args: Event<TouchEnd>) {
        if self._native_scrolling.get() {
            return;
        }
        for touch in &args.touches {
            self.process_new_touch_pos(ctx, touch.x, touch.y, touch.identifier);
        }
        TOUCH_TRACKER.with_borrow_mut(|touches| {
            let idents: Vec<_> = args.touches.iter().map(|e| e.identifier).collect();
            touches.retain(|k, _| !idents.contains(k));
        });
        if no_touches() {
            let params = self._platform_params.get();
            self._damping.set(params.damping);

            let mut mom_x = self._momentum_x.get();
            let mut mom_y = self._momentum_y.get();
            if params.fling && mom_x.abs() > 50.0 {
                mom_x *= 1.7;
            }
            if params.fling && mom_y.abs() > 50.0 {
                mom_y *= 1.7;
            }
            self._momentum_x.set(mom_x);
            self._momentum_y.set(mom_y);
        }
    }
}
