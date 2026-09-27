use super::*;
use kurbo::{BezPath, Rect, Shape};
use pax_runtime::api::math::Transform2;
use pax_runtime::api::pax_value::ToFromPaxAny;
use pax_runtime::api::{
    AlphaMaskPaint, CommonProperties, Fill, Material, Platform, Stroke, TargetInfo, OS,
};
use pax_runtime::{
    CommonPropertiesInit, ComponentInstance, Globals, PropertiesInit, PropertiesScopeInit,
    RouteLocation, TransformAndBounds,
};
use std::cell::Cell;

fn context() -> Rc<RuntimeContext> {
    Rc::new(RuntimeContext::new(Globals {
        elapsed_frames: Property::new(0),
        elapsed_millis: Property::new(0),
        viewport: Property::new(TransformAndBounds {
            transform: Transform2::identity(),
            bounds: (100.0, 100.0),
        }),
        gyro: Property::default(),
        accel: Property::default(),
        route_location: Property::new(RouteLocation::root()),
        browser_allows_scroller_vector_layers: Property::new(true),
        browser_allows_nested_scroller_vector_layers: Property::new(true),
        platform: Platform::Unknown,
        os: OS::Unknown,
        target: TargetInfo::new(Platform::Unknown, OS::Unknown),
        get_elapsed_millis: Rc::new(|| 0),
    }))
}

fn args() -> InstantiationArgs {
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Factory(Box::new(|_, _| {
            Some(Rc::new(RefCell::new(CommonProperties::default())))
        })),
        prototypical_properties: PropertiesInit::Factory(Box::new(|_, _| {
            Some(Rc::new(RefCell::new(Mask::default().to_pax_any())))
        })),
        handler_registry: None,
        children: None,
        component_template: None,
        component_settings: None,
        template_node_identifier: None,
        template_node_type_id: None,
        template_node_selector_info: None,
        transition_config: Default::default(),
        properties_scope: PropertiesScopeInit::None,
    }
}

struct CountingSource {
    base: BaseInstance,
    coverage_calls: Cell<usize>,
    mount_calls: Cell<usize>,
    unmount_calls: Cell<usize>,
    paints: Vec<AlphaMaskPaint>,
}

impl InstanceNode for CountingSource {
    fn instantiate(args: InstantiationArgs) -> Rc<Self> {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    layer: Layer::Canvas,
                    is_component: false,
                    is_slot: false,
                },
            ),
            coverage_calls: Cell::new(0),
            mount_calls: Cell::new(0),
            unmount_calls: Cell::new(0),
            paints: Vec::new(),
        })
    }
    fn base(&self) -> &BaseInstance {
        &self.base
    }
    fn handle_mount(self: Rc<Self>, _: &Rc<ExpandedNode>, _: &Rc<RuntimeContext>) {
        self.mount_calls.set(self.mount_calls.get() + 1);
    }
    fn handle_unmount(&self, _: &Rc<ExpandedNode>, _: &Rc<RuntimeContext>) {
        self.unmount_calls.set(self.unmount_calls.get() + 1);
    }
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("CountingSource").finish()
    }
    fn resolve_coverage_path(&self, _: &ExpandedNode) -> Option<BezPath> {
        self.coverage_calls.set(self.coverage_calls.get() + 1);
        Some(Rect::new(0.0, 0.0, 20.0, 20.0).to_path(0.1))
    }
    fn resolve_alpha_mask_paints(&self, _: &ExpandedNode) -> Vec<AlphaMaskPaint> {
        self.paints.clone()
    }
}

#[derive(Default)]
struct RecordingRenderer {
    saves: usize,
    restores: usize,
    clips: usize,
    alpha_clips: Vec<Vec<AlphaMaskPaint>>,
}

#[test]
fn component_mask_source_initializes_handwriter_and_expands_its_template() {
    use crate::drawing::handwriter::Handwriter;
    use pax_runtime::api::pax_value::PaxAny;
    use pax_runtime::api::NodeContext;
    use pax_runtime::{Handler, HandlerRegistry};

    fn mount_handwriter(
        properties: Rc<RefCell<PaxAny>>,
        context: &NodeContext,
        _event: Option<PaxAny>,
    ) {
        Handwriter::mut_from_pax_any(&mut borrow_mut!(properties))
            .unwrap()
            .on_mount(context);
    }

    // The visible control proves that the same component/handler construction
    // is valid before exercising its use as the detached mask source.
    for as_mask_source in [false, true] {
        let context = context();
        let mut source_args = args();
        source_args.prototypical_properties = PropertiesInit::Factory(Box::new(|_, _| {
            Some(Rc::new(RefCell::new(Handwriter::default().to_pax_any())))
        }));
        source_args.component_template =
            Some(RefCell::new(vec![CountingSource::instantiate(args())]));
        let mut handlers = HandlerRegistry::default();
        handlers.handlers.insert(
            "mount".into(),
            vec![Handler::new_component_handler(mount_handwriter)],
        );
        source_args.handler_registry = Some(Rc::new(RefCell::new(handlers)));
        let source = ComponentInstance::instantiate(source_args);
        let mut root_args = args();
        if as_mask_source {
            let mut mask_args = args();
            mask_args.children = Some(RefCell::new(vec![
                CountingSource::instantiate(args()),
                source,
            ]));
            root_args.component_template =
                Some(RefCell::new(vec![MaskInstance::instantiate(mask_args)]));
        } else {
            root_args.component_template = Some(RefCell::new(vec![source]));
        }
        let root =
            ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &context);
        root.recurse_update(&context);
        let first = root.children.get()[0].clone();
        let source_node = if as_mask_source {
            borrow!(first.sidecar_children)[0].clone()
        } else {
            first
        };
        assert!(
            source_node
                .with_properties_unwrapped(|p: &mut Handwriter| !p._elements.get().is_empty()),
            "Handwriter's mount-generated geometry is absent (mask source: {as_mask_source})"
        );
        assert_eq!(source_node.children.get().len(), 1);
        assert_eq!(source_node.is_render_source(), as_mask_source);
        let original =
            source_node.with_properties_unwrapped(|p: &mut Handwriter| p._elements.get());
        source_node.with_properties_unwrapped(|p: &mut Handwriter| p.text.set("Pax".into()));
        context.drain_node_effects();
        assert_ne!(
            source_node.with_properties_unwrapped(|p: &mut Handwriter| p._elements.get()),
            original
        );
        let source_id = source_node.id;
        let child_id = source_node.children.get()[0].id;
        root.recurse_unmount(&context);
        assert!(context.get_expanded_node_by_eid(source_id).is_none());
        assert!(context.get_expanded_node_by_eid(child_id).is_none());
        assert!(borrow!(source_node.mounted_children).is_empty());
    }
}

#[test]
fn component_sources_do_not_present_native_leaves_or_containers() {
    use crate::core::frame::{Frame, FrameInstance};
    use crate::core::group::{Group, GroupInstance};
    use crate::core::text::{Text, TextInstance};

    let context = context();
    let mut text_args = args();
    text_args.prototypical_properties = PropertiesInit::Factory(Box::new(|_, _| {
        Some(Rc::new(RefCell::new(Text::default().to_pax_any())))
    }));
    let mut group_args = args();
    group_args.prototypical_properties = PropertiesInit::Factory(Box::new(|_, _| {
        Some(Rc::new(RefCell::new(Group::default().to_pax_any())))
    }));
    group_args.children = Some(RefCell::new(vec![
        TextInstance::instantiate(text_args),
        CountingSource::instantiate(args()),
    ]));
    let mut frame_args = args();
    frame_args.prototypical_properties = PropertiesInit::Factory(Box::new(|_, _| {
        Some(Rc::new(RefCell::new(Frame::default().to_pax_any())))
    }));
    frame_args.children = Some(RefCell::new(vec![GroupInstance::instantiate(group_args)]));
    let mut source_args = args();
    source_args.component_template =
        Some(RefCell::new(vec![FrameInstance::instantiate(frame_args)]));
    let source_instance = ComponentInstance::instantiate(source_args);
    let mut mask_args = args();
    mask_args.children = Some(RefCell::new(vec![
        CountingSource::instantiate(args()),
        source_instance.clone(),
    ]));
    let mut root_args = args();
    root_args.component_template = Some(RefCell::new(vec![MaskInstance::instantiate(mask_args)]));
    let root = ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &context);
    root.recurse_update(&context);
    let mask = root.children.get()[0].clone();
    let source = borrow!(mask.sidecar_children)[0].clone();
    let frame = source.children.get()[0].clone();
    let group = frame.children.get()[0].clone();
    let text = group.children.get()[0].clone();
    let canvas = group.children.get()[1].clone();
    for node in [&source, &frame, &group, &text, &canvas] {
        assert!(node.is_render_source());
        assert!(context.get_expanded_node_by_eid(node.id).is_some());
        assert!(!node.ray_cast_test(pax_runtime::api::math::Point2::new(10.0, 10.0)));
    }
    let messages = context.take_native_messages();
    assert!(
        messages.iter().all(|message| match message {
            pax_message::NativeMessage::FrameCreate(patch) => patch.id == mask.id.to_u32(),
            pax_message::NativeMessage::FrameUpdate(patch) => patch.id == mask.id.to_u32(),
            _ => false,
        }),
        "source nodes must not create native resources: {messages:?}"
    );

    // Whole-component replacement uses the same source role and balances old
    // native-free descendants, rather than remounting them as visible nodes.
    let old_ids = [frame.id, group.id, text.id, canvas.id];
    source.fully_recreate_with_new_data(source_instance, &context);
    context.drain_node_effects();
    assert!(source.is_render_source());
    assert!(context
        .take_native_messages()
        .iter()
        .all(|message| matches!(
            message,
            pax_message::NativeMessage::FrameUpdate(patch) if patch.id == mask.id.to_u32()
        )));
    for id in old_ids {
        assert!(context.get_expanded_node_by_eid(id).is_none());
    }
    root.recurse_unmount(&context);
    assert!(context.get_expanded_node_by_eid(source.id).is_none());
    let messages = context.take_native_messages();
    assert!(
        messages.iter().all(|message| matches!(
            message,
            pax_message::NativeMessage::FrameDelete(id) if *id == mask.id.to_u32()
        )),
        "source teardown must not delete resources it never created: {messages:?}"
    );
}

#[test]
fn projected_conditional_source_keeps_its_role_and_balances_lifetimes() {
    use pax_runtime::{ConditionalInstance, ConditionalProperties, Slot, SlotInstance};

    let context = context();
    let shown = Property::new(false);
    let leaf = CountingSource::instantiate(args());
    let mut conditional_args = args();
    let condition = shown.clone();
    conditional_args.prototypical_properties = PropertiesInit::Factory(Box::new(move |_, _| {
        Some(Rc::new(RefCell::new(
            ConditionalProperties {
                boolean_expression: condition.clone(),
                conditional_branches: Vec::new(),
            }
            .to_pax_any(),
        )))
    }));
    conditional_args.children = Some(RefCell::new(vec![leaf.clone()]));
    let mut slot_args = args();
    slot_args.prototypical_properties = PropertiesInit::Factory(Box::new(|_, _| {
        Some(Rc::new(RefCell::new(
            Slot {
                is_remainder: Property::new(true),
                ..Default::default()
            }
            .to_pax_any(),
        )))
    }));
    let mut source_args = args();
    source_args.children = Some(RefCell::new(vec![ConditionalInstance::instantiate(
        conditional_args,
    )]));
    source_args.component_template = Some(RefCell::new(vec![SlotInstance::instantiate(slot_args)]));
    let mut mask_args = args();
    mask_args.children = Some(RefCell::new(vec![
        CountingSource::instantiate(args()),
        ComponentInstance::instantiate(source_args),
    ]));
    let mut root_args = args();
    root_args.component_template = Some(RefCell::new(vec![MaskInstance::instantiate(mask_args)]));
    let root = ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &context);
    root.recurse_update(&context);
    let mask = root.children.get()[0].clone();
    let source = borrow!(mask.sidecar_children)[0].clone();
    let slot = source.children.get()[0].clone();
    assert!(slot.children.get().is_empty());
    assert_eq!(leaf.mount_calls.get(), 0);

    shown.set(true);
    root.recurse_update(&context);
    let projected = slot.children.get()[0].clone();
    assert!(projected.is_render_source());
    assert_eq!(leaf.mount_calls.get(), 1);
    shown.set(false);
    root.recurse_update(&context);
    assert!(slot.children.get().is_empty());
    assert_eq!(leaf.unmount_calls.get(), 1);
    assert!(context.get_expanded_node_by_eid(projected.id).is_none());

    shown.set(true);
    root.recurse_update(&context);
    assert!(slot.children.get()[0].is_render_source());
    assert_eq!(leaf.mount_calls.get(), 2);
    root.recurse_unmount(&context);
    assert_eq!(leaf.unmount_calls.get(), 2);
}

impl RenderContext for RecordingRenderer {
    fn save(&mut self, _: usize) {
        self.saves += 1;
    }
    fn restore(&mut self, _: usize) {
        self.restores += 1;
    }
    fn clip(&mut self, _: usize, _: BezPath) {
        self.clips += 1;
    }
    fn clip_alpha(&mut self, _: usize, paints: &[AlphaMaskPaint], _: f64) {
        self.alpha_clips.push(paints.to_vec());
    }
    fn layers(&self) -> usize {
        1
    }
    fn fill_with_opacity(&mut self, _: usize, _: BezPath, _: &Fill, _: f64) {
        unreachable!()
    }
    fn stroke_with_opacity(&mut self, _: usize, _: BezPath, _: &Stroke, _: f64) {
        unreachable!()
    }
    fn stroke_with_draw_range_and_material_and_opacity(
        &mut self,
        _: usize,
        _: BezPath,
        _: &Stroke,
        _: &Material,
        _: f64,
        _: f64,
        _: f64,
    ) {
        unreachable!()
    }
    fn transform(&mut self, _: usize, _: Affine) {
        unreachable!()
    }
    fn load_image(&mut self, _: &str, _: &[u8], _: usize, _: usize) {
        unreachable!()
    }
    fn draw_image_with_opacity(&mut self, _: usize, _: &str, _: Rect, _: f64) {
        unreachable!()
    }
    fn get_image_size(&mut self, _: &str) -> Option<(usize, usize)> {
        unreachable!()
    }
    fn image_loaded(&self, _: &str) -> bool {
        unreachable!()
    }
    fn resize_layers_to(&mut self, _: usize, _: Rc<RefCell<Vec<bool>>>) {
        unreachable!()
    }
    fn clear(&mut self, _: usize) {
        unreachable!()
    }
    fn flush(&mut self, _: usize, _: Rc<RefCell<Vec<bool>>>) {
        unreachable!()
    }
    fn resize(&mut self, _: usize, _: usize) {
        unreachable!()
    }
    fn refresh_layers(&mut self, _: &[usize]) {
        unreachable!()
    }
}

#[test]
fn alpha_render_skips_coverage_and_balances_even_an_empty_mask() {
    for alpha in [true, false] {
        for painted in [true, false] {
            let context = context();
            let mut source = CountingSource::instantiate(args());
            if painted {
                Rc::get_mut(&mut source)
                    .unwrap()
                    .paints
                    .push(AlphaMaskPaint {
                        path: Rect::new(0.0, 0.0, 20.0, 20.0).to_path(0.1),
                        transform: Affine::IDENTITY,
                        fill: Fill::default(),
                        opacity: 0.5,
                    });
            }
            let mut mask_args = args();
            mask_args.children = Some(RefCell::new(vec![
                CountingSource::instantiate(args()),
                source.clone(),
            ]));
            mask_args.prototypical_properties = PropertiesInit::Factory(Box::new(move |_, _| {
                Some(Rc::new(RefCell::new(
                    Mask {
                        alpha: Property::new(alpha),
                        feather: Property::new(6.0),
                    }
                    .to_pax_any(),
                )))
            }));
            let mask = MaskInstance::instantiate(mask_args);
            let mut root_args = args();
            root_args.component_template = Some(RefCell::new(vec![mask.clone()]));
            let root =
                ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &context);
            root.recurse_update(&context);
            let node = root.children.get()[0].clone();
            source.coverage_calls.set(0);
            let mut renderer = RecordingRenderer::default();
            mask.handle_pre_render(&node, &context, &mut renderer);
            mask.handle_post_render(&node, &context, &mut renderer);
            assert_eq!((renderer.saves, renderer.restores), (1, 1));
            if alpha {
                assert_eq!(source.coverage_calls.get(), 0);
                assert_eq!(renderer.clips, 0);
                assert_eq!(renderer.alpha_clips.len(), 1);
                assert_eq!(renderer.alpha_clips[0].len(), usize::from(painted));
                if painted {
                    assert_eq!(renderer.alpha_clips[0][0].opacity, 0.5);
                }
            } else {
                assert_eq!(source.coverage_calls.get(), 2);
                assert_eq!(renderer.clips, 1);
                assert!(renderer.alpha_clips.is_empty());
            }
        }
    }
}

#[test]
fn initially_empty_repeated_mask_invalidates_without_unrelated_input() {
    use pax_language::interpreter::{PaxExpression, PaxIdentifier, PaxPrimary};
    use pax_manifest::ExpressionInfo;
    use pax_runtime::api::{PaxValue, ToPaxValue, Variable};
    use pax_runtime::{RepeatInstance, RepeatProperties};

    let context = context();
    borrow_mut!(context.dirty_canvases).resize(1, false);
    let source = Property::new(Vec::<usize>::new().to_pax_value());
    let paint_opacity = Property::new(1.0_f64);
    let mut leaf_args = args();
    let paint = paint_opacity.clone();
    leaf_args.properties_scope = PropertiesScopeInit::Factory(Box::new(move |_| {
        [(
            "paint_opacity".to_string(),
            Variable::new_from_typed_property(paint.clone()),
        )]
        .into()
    }));
    let leaf = CountingSource::instantiate(leaf_args);
    let mut repeat_args = args();
    repeat_args.children = Some(RefCell::new(vec![leaf]));
    let repeat_source = source.clone();
    repeat_args.prototypical_properties = PropertiesInit::Factory(Box::new(move |_, _| {
        Some(Rc::new(RefCell::new(
            RepeatProperties {
                source_expression: repeat_source.clone(),
                iterator_i_symbol: Property::new(Some("i".to_string())),
                iterator_elem_symbol: Property::new(Some("item".to_string())),
                repeat_key_expression: Some(ExpressionInfo::new(PaxExpression::Primary(Box::new(
                    PaxPrimary::Identifier(PaxIdentifier::new("item"), Vec::new()),
                )))),
            }
            .to_pax_any(),
        )))
    }));
    let mut mask_args = args();
    mask_args.children = Some(RefCell::new(vec![
        CountingSource::instantiate(args()),
        RepeatInstance::instantiate(repeat_args),
    ]));
    mask_args.prototypical_properties = PropertiesInit::Factory(Box::new(|_, _| {
        Some(Rc::new(RefCell::new(
            Mask {
                alpha: Property::new(true),
                feather: Property::new(6.0),
            }
            .to_pax_any(),
        )))
    }));
    let mask = MaskInstance::instantiate(mask_args);
    let mut root_args = args();
    root_args.component_template = Some(RefCell::new(vec![mask.clone()]));
    let root = ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &context);
    root.recurse_update(&context);
    context.drain_node_effects();
    context.clear_all_dirty_canvases();

    source.set(vec![1usize].to_pax_value());
    context.drain_node_effects();
    assert!(
        context.has_canvas_render_work(),
        "first ring must invalidate an initially empty mask"
    );
    context.clear_all_dirty_canvases();

    context.drain_node_effects();
    assert!(
        !context.has_canvas_render_work(),
        "unchanged mask sources must not force continuous redraw"
    );

    // A new descendant's paint can change independently of repeat membership.
    paint_opacity.set(0.25);
    context.drain_node_effects();
    assert!(
        context.has_canvas_render_work(),
        "new mask leaves need paint subscriptions"
    );
    context.clear_all_dirty_canvases();

    let node = root.children.get()[0].clone();
    let repeat = borrow!(node.sidecar_children)[0].clone();
    let leaf = repeat.children.get()[0].clone();
    borrow!(leaf.get_common_properties())
        .opacity
        .set(Some(pax_runtime::api::Opacity::Alpha(0.5.into())));
    context.drain_node_effects();
    assert!(
        context.has_canvas_render_work(),
        "source common opacity must invalidate painted alpha"
    );
    context.clear_all_dirty_canvases();

    source.set(PaxValue::Vec(Vec::new()));
    context.drain_node_effects();
    assert!(
        context.has_canvas_render_work(),
        "last ring removal must clear the retained mask"
    );
    let mut renderer = RecordingRenderer::default();
    mask.handle_pre_render(&node, &context, &mut renderer);
    mask.handle_post_render(&node, &context, &mut renderer);
    assert_eq!(renderer.alpha_clips.len(), 1);
    assert!(renderer.alpha_clips[0].is_empty());
    context.clear_all_dirty_canvases();

    paint_opacity.set(0.75);
    context.drain_node_effects();
    assert!(
        !context.has_canvas_render_work(),
        "retired sources must release their subscriptions"
    );
}

#[test]
fn mask_source_opacity_is_relative_even_when_ancestor_is_zero() {
    let context = context();
    let parent_opacity = Property::new(Some(0.0.into()));
    let source_opacity = Property::new(Some(0.5.into()));
    let mut source_args = args();
    let opacity = source_opacity.clone();
    source_args.prototypical_common_properties =
        CommonPropertiesInit::Factory(Box::new(move |_, _| {
            Some(Rc::new(RefCell::new(CommonProperties {
                opacity: opacity.clone(),
                ..Default::default()
            })))
        }));
    let mut source = CountingSource::instantiate(source_args);
    Rc::get_mut(&mut source)
        .unwrap()
        .paints
        .push(AlphaMaskPaint {
            path: Rect::new(0.0, 0.0, 20.0, 20.0).to_path(0.1),
            transform: Affine::IDENTITY,
            fill: Fill::default(),
            opacity: 0.5,
        });
    let mut mask_args = args();
    mask_args.children = Some(RefCell::new(vec![
        CountingSource::instantiate(args()),
        source,
    ]));
    mask_args.prototypical_properties = PropertiesInit::Factory(Box::new(|_, _| {
        Some(Rc::new(RefCell::new(
            Mask {
                alpha: Property::new(true),
                feather: Property::new(0.0),
            }
            .to_pax_any(),
        )))
    }));
    let mask = MaskInstance::instantiate(mask_args);
    let mut root_args = args();
    root_args.component_template = Some(RefCell::new(vec![mask.clone()]));
    let opacity = parent_opacity.clone();
    root_args.prototypical_common_properties =
        CommonPropertiesInit::Factory(Box::new(move |_, _| {
            Some(Rc::new(RefCell::new(CommonProperties {
                opacity: opacity.clone(),
                ..Default::default()
            })))
        }));
    let root = ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &context);
    for (parent, source, expected) in [
        (0.0, 0.5, 0.25),
        (0.0, 0.25, 0.125),
        (0.5, 0.25, 0.125),
        (1.0, 0.5, 0.25),
    ] {
        parent_opacity.set(Some(parent.into()));
        source_opacity.set(Some(source.into()));
        root.recurse_update(&context);
        let node = root.children.get()[0].clone();
        let mut renderer = RecordingRenderer::default();
        mask.handle_pre_render(&node, &context, &mut renderer);
        mask.handle_post_render(&node, &context, &mut renderer);
        assert_eq!(renderer.alpha_clips[0][0].opacity, expected);
    }
}
