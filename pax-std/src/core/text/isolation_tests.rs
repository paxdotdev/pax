use super::*;
use crate::core::{frame::*, group::*, scroller::*};
use pax_message::NativeMessage;
use pax_runtime::api::{pax_value::ToFromPaxAny, CommonProperties, Platform, Rotation, OS};
use pax_runtime::{
    CommonPropertiesInit, ComponentInstance, PaxEngine, PropertiesInit, PropertiesScopeInit,
};

fn args<T: ToFromPaxAny + Clone + 'static>(
    props: T,
    children: Vec<Rc<dyn InstanceNode>>,
) -> InstantiationArgs {
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Factory(Box::new(|_, _| {
            let p = CommonProperties::default();
            p.width.set(Some(Size::Pixels(200.into())));
            p.height.set(Some(Size::Pixels(40.into())));
            Some(Rc::new(RefCell::new(p)))
        })),
        prototypical_properties: PropertiesInit::Factory(Box::new(move |_, _| {
            Some(Rc::new(RefCell::new(props.clone().to_pax_any())))
        })),
        properties_scope: PropertiesScopeInit::None,
        children: Some(RefCell::new(children)),
        component_template: None,
        component_settings: None,
        component_camera: None,
        handler_registry: None,
        template_node_identifier: None,
        template_node_type_id: None,
        template_node_selector_info: None,
        transition_config: Default::default(),
    }
}

struct Fixture {
    engine: PaxEngine,
    outer: Rc<ExpandedNode>,
    owner: Rc<ExpandedNode>,
    frame: Rc<ExpandedNode>,
    texts: Vec<Rc<ExpandedNode>>,
}
impl Fixture {
    fn new(count: usize, platform: Platform) -> Self {
        let texts = (0..count)
            .map(|_| {
                TextInstance::instantiate(args(Text::default(), vec![])) as Rc<dyn InstanceNode>
            })
            .collect();
        let frame = FrameInstance::instantiate(args(Frame::default(), texts));
        let scroll = ScrollerHostInstance::instantiate(args(ScrollerHost::default(), vec![frame]));
        let outer = GroupInstance::instantiate(args(Group::default(), vec![scroll]));
        let mut root_args = args(Frame::default(), vec![]);
        root_args.component_template = Some(RefCell::new(vec![outer]));
        let ctx = Rc::new(crate::test_support::runtime_context_with_target(
            (640., 480.),
            platform,
            OS::Mac,
        ));
        {
            let _graph = ctx.property_graph.enter();
            pax_runtime::api::properties::register_time(&ctx.globals().elapsed_frames);
            pax_runtime::api::properties::register_millis(&ctx.globals().elapsed_millis);
        }
        let mut engine = PaxEngine {
            runtime_context: ctx,
            root_expanded_node: None,
            scroller_tiling_policy: Default::default(),
        };
        let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
        let outer = root.children.get()[0].clone();
        let owner = outer.children.get()[0].clone();
        let frame = owner.children.get()[0].clone();
        let texts = frame.children.get();
        let mut f = Self {
            engine,
            outer,
            owner,
            frame,
            texts,
        };
        let initial = f.settle();
        assert!(
            initial.iter().any(|p| p.transform.is_some()),
            "missing initial text patch, attached={:?}",
            f.texts.iter().map(|t| t.attached.get()).collect::<Vec<_>>()
        );
        f
    }
    fn settle(&mut self) -> Vec<TextPatch> {
        self.settle_messages()
            .into_iter()
            .filter_map(|m| match m {
                NativeMessage::TextUpdate(p) => Some(p),
                _ => None,
            })
            .collect()
    }
    fn settle_messages(&mut self) -> Vec<NativeMessage> {
        let mut messages = Vec::new();
        for _ in 0..3 {
            messages.extend(self.engine.tick());
        }
        messages
    }
    fn assert_chart(&self) {
        let ctx = &self.engine.runtime_context;
        let owner = kurbo::Affine::new(
            ctx.native_presentation_matrix_for_node(&self.owner)
                .try_into()
                .unwrap(),
        );
        for text in &self.texts {
            let (id, local) = text.native_placement(ctx).get();
            assert_eq!(id, Some(self.owner.id.to_u32()));
            let actual = (owner * kurbo::Affine::new(local.try_into().unwrap())).as_coeffs();
            let expected = ctx.native_presentation_matrix_for_node(text);
            assert!(actual
                .iter()
                .zip(expected)
                .all(|(a, b)| (a - b).abs() < 1e-8));
        }
    }
}

#[test]
fn retained_frame_clip_changes_and_coordinate_handoffs_remain_live() {
    for platform in [Platform::Web, Platform::Native] {
        let mut f = Fixture::new(1, platform);
        let frame_id = f.frame.id.to_u32();
        f.frame
            .with_properties_unwrapped(|p: &mut Frame| p.corner_radius.set(8.));
        let messages = f.settle_messages();
        assert!(messages.iter().any(|m| matches!(m, NativeMessage::FrameUpdate(p)
            if p.id == frame_id && p.corner_radius == Some(8.) && p.clip_path.as_ref().is_some_and(|s| !s.is_empty()))));
        f.outer
            .get_common_properties()
            .borrow()
            .x
            .set(Some(Size::Pixels(37.into())));
        let messages = f.settle_messages();
        assert!(!messages
            .iter()
            .any(|m| matches!(m, NativeMessage::FrameUpdate(p)
            if p.id == frame_id && (p.transform.is_some() || p.clip_path.is_some()))));
        f.frame
            .get_common_properties()
            .borrow()
            .x
            .set(Some(Size::Pixels(11.into())));
        let messages = f.settle_messages();
        assert!(messages.iter().any(|m| matches!(m, NativeMessage::FrameUpdate(p)
            if p.id == frame_id && p.transform.as_ref().is_some_and(|t| t[4] == 11.) && p.clip_path.is_some())));
        #[cfg(feature = "3d")]
        for (angle, owner) in [(20., None), (0., Some(f.owner.id.to_u32()))] {
            f.outer
                .get_common_properties()
                .borrow()
                .rotate_x
                .set(Some(Rotation::Degrees(angle.into())));
            let messages = f.settle_messages();
            assert!(messages.iter().any(|m| matches!(m, NativeMessage::FrameUpdate(p)
                if p.id == frame_id && p.transform_scroller == Some(owner) && p.transform.is_some() && p.clip_path.is_some())),
                "coordinate changes must atomically publish owner, transform, and path");
        }
    }
}

#[test]
fn moving_native_host_retains_one_and_many_text_placements() {
    for platform in [Platform::Web, Platform::Native] {
        for count in [1, 128] {
            let mut f = Fixture::new(count, platform);
            for (x, rotate) in [(12., 7.), (-30., -12.), (0., 0.)] {
                let cp = f.outer.get_common_properties();
                cp.borrow().x.set(Some(Size::Pixels(x.into())));
                cp.borrow()
                    .rotate
                    .set(Some(Rotation::Degrees(rotate.into())));
                let patches = f.settle();
                assert!(
                    patches.is_empty(),
                    "host-only movement emitted {} text patches",
                    patches.len()
                );
                f.assert_chart();
            }
        }
    }
}

#[test]
fn local_movement_and_content_layout_edits_remain_live() {
    let mut f = Fixture::new(1, Platform::Web);
    let before = f.texts[0].native_placement(&f.engine.runtime_context).get();
    f.frame
        .get_common_properties()
        .borrow()
        .x
        .set(Some(Size::Pixels(15.into())));
    let patches = f.settle();
    assert!(
        patches.iter().any(|p| p.transform.is_some()),
        "patches={patches:?}, before={before:?}, after={:?}",
        f.texts[0].native_placement(&f.engine.runtime_context).get()
    );
    assert!(patches
        .iter()
        .all(|p| p.content.is_none() && p.style.is_none() && p.measure_generation.is_none()));
    f.assert_chart();
    let cp = f.texts[0].get_common_properties();
    cp.borrow().height.set(None);
    cp.borrow().width.set(Some(Size::Pixels(120.into())));
    let patches = f.settle();
    assert!(patches
        .iter()
        .any(|p| p.size_x == Some(120.) && p.measure_generation.is_some()));
    f.texts[0].with_properties_unwrapped(|p: &mut Text| p.text.set("a changed paragraph".into()));
    let patches = f.settle();
    assert!(patches
        .iter()
        .any(|p| p.content.as_deref() == Some("a changed paragraph")
            && p.measure_generation.is_some()));
    f.texts[0].with_properties_unwrapped(|p: &mut Text| {
        let style = p.style.get();
        style.font_size.set(Size::Pixels(25.into()));
        p.style.set(style);
    });
    assert!(f
        .settle()
        .iter()
        .any(|p| p.style.is_some() && p.measure_generation.is_some()));
}

#[test]
fn reparenting_out_of_a_scroller_and_home_switches_coordinate_modes() {
    let mut f = Fixture::new(1, Platform::Web);
    f.texts[0]
        .get_common_properties()
        .borrow()
        .id
        .set(Some("text".into()));
    f.outer
        .get_common_properties()
        .borrow()
        .id
        .set(Some("outer".into()));
    f.settle();
    let root = f
        .outer
        .get_node_context(&f.engine.runtime_context)
        .app_root()
        .unwrap();
    let node = root.query("#text").one().unwrap();
    let destination = root.query("#outer").one().unwrap();
    node.move_to_append_child(&destination).unwrap();
    assert!(f
        .settle()
        .iter()
        .any(|p| p.transform_scroller == Some(None) && p.transform.is_some()));
    {
        // Native handlers run inside the application's property graph; easing
        // must use that graph's frame clock rather than the test thread's clock.
        let _graph = f.engine.runtime_context.property_graph.enter();
        node.animated(
            pax_runtime::api::Duration::Frames(4.into()),
            pax_runtime::api::EasingCurve::Linear,
        )
        .move_to_home()
        .unwrap();
    }
    assert_eq!(
        f.texts[0]
            .native_placement(&f.engine.runtime_context)
            .get()
            .0,
        None,
        "a returning flight must retain world placement until it lands"
    );
    let mut patches = f.settle();
    patches.extend(f.settle());
    let owner = f.owner.id.to_u32();
    assert!(
        patches
            .iter()
            .any(|p| p.transform_scroller == Some(Some(owner)) && p.transform.is_some()),
        "patches={patches:?}, frame={}, placement={:?}",
        f.engine.runtime_context.globals().elapsed_frames.get(),
        f.texts[0].native_placement(&f.engine.runtime_context).get()
    );
    f.assert_chart();
}

#[cfg(feature = "3d")]
#[test]
fn spatial_entry_and_exit_publish_coordinate_mode_atomically() {
    let mut f = Fixture::new(1, Platform::Web);
    f.outer
        .get_common_properties()
        .borrow()
        .rotate_y
        .set(Some(Rotation::Degrees(20.into())));
    let patches = f.settle();
    assert!(
        patches
            .iter()
            .any(|p| p.transform_scroller == Some(None) && p.transform.is_some()),
        "patches={patches:?}, placement={:?}",
        f.texts[0].native_placement(&f.engine.runtime_context).get()
    );
    f.outer.get_common_properties().borrow().rotate_y.set(None);
    let owner = f.owner.id.to_u32();
    assert!(f
        .settle()
        .iter()
        .any(|p| p.transform_scroller == Some(Some(owner)) && p.transform.is_some()));
    f.assert_chart();
}
