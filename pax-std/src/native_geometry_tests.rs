use crate::core::{event_blocker::*, frame::*, group::*, mask::*, scroller::*, text::*};
use crate::drawing::rectangle::{Rectangle, RectangleInstance};
use crate::forms::{
    button::*, checkbox::*, dropdown::*, photo_picker::*, radio_list::*, slider::*, textbox::*,
};
use crate::media::{native_image::*, youtube_video::*};
use pax_message::NativeMessage;
use pax_runtime::api::pax_value::ToFromPaxAny;
use pax_runtime::api::{CommonProperties, NativeLiquidGlassScope, Platform, Size, OS};
use pax_runtime::{
    CommonPropertiesInit, ComponentInstance, ExpandedNode, InstanceNode, InstantiationArgs,
    PropertiesInit, PropertiesScopeInit, RuntimeContext,
};
use std::{cell::RefCell, rc::Rc};

fn args<T: ToFromPaxAny + Clone + 'static>(
    props: T,
    common: CommonProperties,
) -> InstantiationArgs {
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Factory(Box::new(move |_, _| {
            Some(Rc::new(RefCell::new(common.clone())))
        })),
        prototypical_properties: PropertiesInit::Factory(Box::new(move |_, _| {
            Some(Rc::new(RefCell::new(props.clone().to_pax_any())))
        })),
        properties_scope: PropertiesScopeInit::None,
        handler_registry: None,
        children: None,
        component_template: None,
        component_settings: None,
        component_camera: None,
        template_node_identifier: None,
        template_node_type_id: None,
        template_node_selector_info: None,
        transition_config: Default::default(),
    }
}

fn child_common() -> CommonProperties {
    let common = CommonProperties::default();
    common.width.set(Some(Size::Percent(50.into())));
    common.height.set(Some(Size::Percent(50.into())));
    common
}

struct Fixture {
    context: Rc<RuntimeContext>,
    _root: Rc<ExpandedNode>,
    parent: CommonProperties,
    node: Rc<ExpandedNode>,
}

impl Fixture {
    fn mount(instance: Rc<dyn InstanceNode>) -> Self {
        let context = Rc::new(crate::test_support::runtime_context_with_target(
            (320.0, 240.0),
            Platform::Web,
            OS::Mac,
        ));
        let parent = CommonProperties::default();
        let mut parent_args = args(Group::default(), parent.clone());
        parent_args.children = Some(RefCell::new(vec![instance]));
        let mut root_args = args(Group::default(), CommonProperties::default());
        root_args.component_template =
            Some(RefCell::new(vec![GroupInstance::instantiate(parent_args)]));
        let root =
            ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &context);
        root.recurse_update(&context);
        context.drain_node_effects();
        let node = root.children.get()[0].children.get()[0].clone();
        context.take_native_messages();
        Self {
            context,
            _root: root,
            parent,
            node,
        }
    }

    fn messages(&self) -> Vec<NativeMessage> {
        // Deliberately do not mutate the leaf, dirty its occlusion, or run a
        // full frame: inherited geometry must wake the native patch listener.
        self.context.drain_node_effects();
        self.context.take_native_messages()
    }
}

#[test]
fn drawer_preview_keeps_its_padded_layout_width_during_motion() {
    let content = CommonProperties::default();
    content.width.set(Some(Size::Percent(100.into())));
    let fixture = Fixture::mount(GroupInstance::instantiate(args(Group::default(), content)));
    let _graph = fixture.context.property_graph.enter();
    fixture.parent.width.set(Some(Size::Pixels(320.into())));
    fixture.parent.padding_x.set(Some(Size::Pixels(20.into())));
    let mut host = crate::ExampleHost::default();
    let ctx = fixture.node.get_node_context(&fixture.context);
    assert_eq!(ctx.bounds_self.get().0, 280.);
    host.bind_preview_width(&ctx);
    let preview_width = host._preview_layout_width_px.local();
    assert_eq!(preview_width.get(), 280.);
    host._drawer_progress.local().set(0.5);
    assert_eq!(preview_width.get(), 280.);
    host._preview_reserved_width_px.local().set(80.);
    assert_eq!(preview_width.get(), 200.);
    fixture.parent.padding_x.set(Some(Size::Percent(10.into())));
    fixture.context.drain_node_effects();
    assert_eq!(preview_width.get(), 176.);
}

#[test]
fn settings_importer_keeps_current_geometry_for_explicit_queries() {
    use crate::core::import_settings::{ImportSettings, ImportSettingsInstance};
    use pax_runtime::api::math::Point2;

    let common = CommonProperties::default();
    common.width.set(Some(Size::Pixels(30.into())));
    common.height.set(Some(Size::Pixels(20.into())));
    let importer = ImportSettingsInstance::instantiate(args(ImportSettings::default(), common));
    let mut scroller = args(ScrollerHost::default(), CommonProperties::default());
    scroller.children = Some(RefCell::new(vec![importer]));
    let fixture = Fixture::mount(ScrollerHostInstance::instantiate(scroller));
    let mut engine = pax_runtime::PaxEngine {
        runtime_context: fixture.context.clone(),
        root_expanded_node: Some(fixture._root.clone()),
        scroller_tiling_policy: Default::default(),
    };
    engine.tick();
    let importer = fixture.node.children.get()[0].clone();
    let hits = |x, include_invisible| {
        fixture
            .context
            .get_elements_beneath_ray(
                Some(fixture._root.clone()),
                Point2::new(x, 10.),
                false,
                vec![],
                include_invisible,
            )
            .iter()
            .any(|node| node.id == importer.id)
    };
    assert!(hits(15., true));
    fixture.parent.x.set(Some(Size::Pixels(70.into())));
    let pose = importer.presentation_transform_and_bounds.get();
    assert_eq!(kurbo::Affine::from(pose.transform).translation().x, 70.);
    engine.tick();
    assert!(!hits(15., true));
    assert!(hits(85., true));
    assert!(!hits(85., false));
}

#[derive(Debug)]
struct GeometryPatch {
    transform: Option<Vec<f64>>,
    width: Option<f64>,
    height: Option<f64>,
}

fn exercise_inherited_geometry(
    fixture: Fixture,
    extract: impl Fn(NativeMessage, u32) -> Option<GeometryPatch>,
) {
    let id = fixture.node.id.to_u32();
    let check = |expect_transform: bool, expect_size: bool| {
        let patches: Vec<_> = fixture
            .messages()
            .into_iter()
            .filter_map(|m| extract(m, id))
            .collect();
        assert!(
            !patches.is_empty(),
            "ancestor-only change did not emit a native patch"
        );
        let expected = fixture
            .context
            .native_presentation_matrix_for_node(&fixture.node);
        let bounds = fixture.node.presentation_transform_and_bounds.get().bounds;
        if expect_transform {
            assert!(
                patches
                    .iter()
                    .any(|p| p.transform.as_ref() == Some(&expected)),
                "{patches:?}"
            );
        }
        if expect_size {
            assert!(
                patches
                    .iter()
                    .any(|p| p.width == Some(bounds.0) && p.height == Some(bounds.1)),
                "{patches:?}"
            );
        }
        assert!(
            fixture
                .messages()
                .into_iter()
                .all(|m| extract(m, id).is_none()),
            "unchanged geometry must not resend patches"
        );
    };

    // Includes returning a fixed nav from its initially offscreen position.
    for y in [-96.0, 0.0, 45.0] {
        fixture.parent.y.set(Some(Size::Pixels(y.into())));
        check(true, false);
    }
    fixture.parent.scale_x.set(Some(Size::Percent(150.into())));
    fixture.parent.scale_y.set(Some(Size::Percent(75.into())));
    check(true, false);

    #[cfg(feature = "3d")]
    {
        fixture
            .parent
            .rotate_y
            .set(Some(pax_runtime::api::Rotation::Degrees(25.into())));
        check(true, false);
    }

    fixture
        .context
        .globals()
        .viewport
        .update(|v| v.bounds = (688.0, 900.0));
    check(false, true);
    fixture.parent.width.set(Some(Size::Pixels(200.into())));
    fixture.parent.height.set(Some(Size::Pixels(160.into())));
    check(false, true);
}

macro_rules! native_geometry_test {
    ($name:ident, $instance:ident, $props:ident, $message:ident) => {
        #[test]
        fn $name() {
            let instance = $instance::instantiate(args($props::default(), child_common()));
            exercise_inherited_geometry(Fixture::mount(instance), |message, id| match message {
                NativeMessage::$message(p) if p.id == id => Some(GeometryPatch {
                    transform: p.transform,
                    width: p.size_x,
                    height: p.size_y,
                }),
                _ => None,
            });
        }
    };
}

native_geometry_test!(
    text_follows_inherited_geometry,
    TextInstance,
    Text,
    TextUpdate
);
native_geometry_test!(
    image_follows_inherited_geometry,
    NativeImageInstance,
    NativeImage,
    NativeImageUpdate
);
native_geometry_test!(
    video_follows_inherited_geometry,
    YoutubeVideoInstance,
    YoutubeVideo,
    YoutubeVideoUpdate
);
native_geometry_test!(
    button_follows_inherited_geometry,
    ButtonInstance,
    Button,
    ButtonUpdate
);
native_geometry_test!(
    checkbox_follows_inherited_geometry,
    CheckboxInstance,
    Checkbox,
    CheckboxUpdate
);
native_geometry_test!(
    dropdown_follows_inherited_geometry,
    DropdownInstance,
    Dropdown,
    DropdownUpdate
);
native_geometry_test!(
    radio_list_follows_inherited_geometry,
    RadioListInstance,
    RadioList,
    RadioListUpdate
);
native_geometry_test!(
    slider_follows_inherited_geometry,
    SliderInstance,
    Slider,
    SliderUpdate
);
native_geometry_test!(
    textbox_follows_inherited_geometry,
    TextboxInstance,
    Textbox,
    TextboxUpdate
);
native_geometry_test!(
    photo_picker_follows_inherited_geometry,
    PhotoPickerInputInstance,
    PhotoPickerInput,
    PhotoPickerUpdate
);
native_geometry_test!(
    event_blocker_follows_inherited_geometry,
    EventBlockerInstance,
    EventBlocker,
    EventBlockerUpdate
);
native_geometry_test!(
    frame_follows_inherited_geometry,
    FrameInstance,
    Frame,
    FrameUpdate
);
native_geometry_test!(
    scroller_follows_inherited_geometry,
    ScrollerHostInstance,
    ScrollerHost,
    ScrollerUpdate
);

#[test]
fn glass_group_follows_inherited_geometry() {
    let fixture = Fixture::mount(GroupInstance::instantiate(args(
        Group::default(),
        child_common(),
    )));
    fixture
        .node
        .liquid_glass_scope
        .set(Some(NativeLiquidGlassScope::default()));
    fixture.messages();
    exercise_inherited_geometry(fixture, |message, id| match message {
        NativeMessage::GlassSurfaceUpdate(p) if p.id == id => Some(GeometryPatch {
            transform: p.transform,
            width: p.size_x,
            height: p.size_y,
        }),
        _ => None,
    });
}

#[test]
fn mask_follows_inherited_geometry() {
    let mut mask_args = args(Mask::default(), child_common());
    mask_args.children = Some(RefCell::new(vec![
        RectangleInstance::instantiate(args(Rectangle::default(), CommonProperties::default())),
        RectangleInstance::instantiate(args(Rectangle::default(), CommonProperties::default())),
    ]));
    exercise_inherited_geometry(
        Fixture::mount(MaskInstance::instantiate(mask_args)),
        |message, id| match message {
            NativeMessage::FrameUpdate(p) if p.id == id => Some(GeometryPatch {
                transform: p.transform,
                width: p.size_x,
                height: p.size_y,
            }),
            _ => None,
        },
    );
}

#[test]
fn text_measurement_moves_native_text_without_changing_its_properties() {
    let common = child_common();
    common.height.set(None);
    common.y.set(Some(Size::Percent(50.into())));
    let fixture = Fixture::mount(TextInstance::instantiate(args(Text::default(), common)));
    let before = fixture
        .context
        .native_presentation_matrix_for_node(&fixture.node);
    fixture.node.measured_size.set(Some((100.0, 48.0)));
    let messages = fixture.messages();
    let after = fixture
        .context
        .native_presentation_matrix_for_node(&fixture.node);
    assert_ne!(before, after);
    assert!(messages
        .into_iter()
        .any(|m| matches!(m, NativeMessage::TextUpdate(p)
        if p.id == fixture.node.id.to_u32() && p.transform.as_ref() == Some(&after))));
}

#[test]
fn inherited_width_change_requests_fresh_native_text_measurement() {
    let common = child_common();
    common.height.set(None);
    let fixture = Fixture::mount(TextInstance::instantiate(args(Text::default(), common)));
    fixture.node.measured_size.set(Some((160.0, 48.0)));
    fixture.messages();
    let bounds = fixture.node.presentation_bounds();
    let mut previous_generation = 0;
    for viewport_width in [688.0, 320.0] {
        fixture
            .context
            .globals()
            .viewport
            .update(|v| v.bounds.0 = viewport_width);
        assert_eq!(bounds.get().0, viewport_width * 0.5);
        let patch = fixture
            .messages()
            .into_iter()
            .find_map(|m| match m {
                NativeMessage::TextUpdate(p) if p.id == fixture.node.id.to_u32() => Some(p),
                _ => None,
            })
            .expect("resized text must receive its new width");
        assert_eq!(patch.size_x, Some(viewport_width * 0.5));
        let generation = patch
            .measure_generation
            .expect("changed wrapping width needs remeasurement");
        assert!(generation > previous_generation);
        previous_generation = generation;
    }
}

#[test]
fn image_placement_and_source_changes_redraw_without_reloading_for_motion() {
    use crate::media::image::{Image, ImageInstance, ImageSource};
    let image = Image::default();
    let source = image.source.clone();
    let fixture = Fixture::mount(ImageInstance::instantiate(args(image, child_common())));
    let node = &fixture.node;
    let initial = node.canvas_transform_and_bounds.get();
    fixture.context.clear_canvas_node_dirty(&node.id);
    fixture.parent.x.set(Some(Size::Pixels(10.into())));
    assert_ne!(node.canvas_transform_and_bounds.get(), initial);
    let image_load = |m: &NativeMessage| matches!(m, NativeMessage::ImageLoad(_));
    assert!(!fixture.messages().iter().any(image_load));
    assert!(fixture.context.is_canvas_node_dirty(&node.id));

    fixture.context.clear_canvas_node_dirty(&node.id);
    source.set(ImageSource::Url("cover.png".into()));
    let messages = fixture.messages();
    assert_eq!(
        messages
            .iter()
            .filter(|m| matches!(m, NativeMessage::ImageLoad(p)
                if p.id == node.id.to_u32() && p.path.as_deref() == Some("cover.png")))
            .count(),
        1
    );
    assert!(fixture.context.is_canvas_node_dirty(&node.id));
    fixture.context.clear_canvas_node_dirty(&node.id);
    source.set(ImageSource::Data(1, 1, vec![255; 4]));
    assert!(!fixture.messages().iter().any(image_load));
    assert!(fixture.context.is_canvas_node_dirty(&node.id));
}

#[test]
fn retained_rectangle_coverage_tracks_nested_radius_edits_and_replacement() {
    use crate::drawing::rectangle::CornerRadii;
    let radii = CornerRadii::radii(0.into(), 0.into(), 0.into(), 0.into());
    let rectangle = Rectangle::default();
    rectangle.corner_radius.set(radii.clone());
    let mut group = args(Group::default(), CommonProperties::default());
    group.children = Some(RefCell::new(vec![
        RectangleInstance::instantiate(args(rectangle.clone(), child_common())),
        TextInstance::instantiate(args(Text::default(), child_common())),
    ]));
    let fixture = Fixture::mount(GroupInstance::instantiate(group));
    let native = fixture.node.children.get()[1].clone();
    let hash = || {
        fixture.context.drain_node_effects();
        pax_runtime::engine::occlusion::update_node_occlusion(&fixture._root, &fixture.context);
        native.native_mask_hash.get()
    };
    let square = hash();
    assert_ne!(square, 0);
    radii.top_left.set(20.into());
    let rounded = hash();
    assert_ne!(rounded, square);
    radii.top_left.set(0.into());
    assert_eq!(hash(), square);
    rectangle
        .corner_radius
        .set(CornerRadii::radii(0.into(), 30.into(), 0.into(), 0.into()));
    let replaced = hash();
    assert_ne!(replaced, rounded);
    assert_ne!(replaced, square);
    radii.top_left.set(45.into());
    assert_eq!(
        hash(),
        replaced,
        "replaced radii no longer own the retained outline"
    );
}
