use super::*;
use crate::core::{
    group::{Group, GroupInstance},
    text::{Text, TextInstance},
};
use crate::layout::stacker::{Stacker, StackerInstance};
use pax_runtime::api::{pax_value::ToFromPaxAny, CommonProperties};
use pax_runtime::{
    CommonPropertiesInit, ComponentInstance, PropertiesInit, PropertiesScopeInit, Slot,
    SlotInstance,
};

fn args<T: ToFromPaxAny + Clone + 'static>(
    props: T,
    common: CommonProperties,
    children: Vec<Rc<dyn InstanceNode>>,
) -> InstantiationArgs {
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Factory(Box::new(move |_, _| {
            Some(Rc::new(RefCell::new(common.clone())))
        })),
        prototypical_properties: PropertiesInit::Factory(Box::new(move |_, _| {
            Some(Rc::new(RefCell::new(props.clone().to_pax_any())))
        })),
        properties_scope: PropertiesScopeInit::None,
        children: Some(RefCell::new(children)),
        component_template: None,
        handler_registry: None,
        component_settings: None,
        component_camera: None,
        template_node_identifier: None,
        template_node_type_id: None,
        template_node_selector_info: None,
        transition_config: Default::default(),
    }
}

fn px(value: f64) -> Size {
    Size::Pixels(value.into())
}

fn common(width: Size, height: Option<Size>, y: f64) -> CommonProperties {
    let common = CommonProperties::default();
    common.width.set(Some(width));
    common.height.set(height);
    common.y.set(Some(px(y)));
    common
}

// Exercise the same public-component -> native-host -> projected-content seam
// as Scroller's template, including its actual content-autosize subscription.
fn scroller(
    props: &Scroller,
    common: CommonProperties,
    child: Rc<dyn InstanceNode>,
) -> Rc<ComponentInstance> {
    let mut slot = args(Group::default(), CommonProperties::default(), vec![]);
    slot.prototypical_properties = PropertiesInit::Factory(Box::new(|_, _| {
        Some(Rc::new(RefCell::new(Slot::default().to_pax_any())))
    }));
    let host = ScrollerHostInstance::instantiate(args(
        ScrollerHost {
            scroll_pos_x: props.scroll_pos_x.clone(),
            scroll_pos_y: props.scroll_pos_y.clone(),
            scroll_width: props._resolved_scroll_width.clone(),
            scroll_height: props._resolved_scroll_height.clone(),
            ..Default::default()
        },
        CommonProperties::default(),
        vec![SlotInstance::instantiate(slot)],
    ));
    let mut component = args(props.clone(), common, vec![child]);
    component.component_template = Some(RefCell::new(vec![host]));
    ComponentInstance::instantiate(component)
}

struct Fixture {
    context: Rc<RuntimeContext>,
    root: Rc<ExpandedNode>,
    stack: Rc<ExpandedNode>,
    section: Rc<ExpandedNode>,
    next: Rc<ExpandedNode>,
    inner: Rc<ExpandedNode>,
    text: Rc<ExpandedNode>,
    inner_props: Scroller,
    outer_props: Scroller,
}

impl Fixture {
    fn new() -> Self {
        let context = Rc::new(crate::test_support::runtime_context_with_target(
            (1000.0, 800.0),
            pax_runtime::api::Platform::Web,
            pax_runtime::api::OS::Mac,
        ));
        let text_common = CommonProperties::default();
        text_common.x.set(Some(px(14.0)));
        text_common.y.set(Some(px(14.0)));
        let text = TextInstance::instantiate(args(Text::default(), text_common, vec![]));
        let mut inner_props = Scroller::default();
        inner_props.autosize.set(true);
        inner_props.autosize_x.set(Some(true));
        let inner = scroller(
            &inner_props,
            common(
                px(430.0),
                Some(Size::Combined((-120).into(), 100.into())),
                120.0,
            ),
            text,
        );
        let section = GroupInstance::instantiate(args(
            Group::default(),
            common(px(1000.0), Some(px(600.0)), 0.0),
            vec![inner],
        ));
        let next = GroupInstance::instantiate(args(
            Group::default(),
            common(px(1000.0), Some(px(80.0)), 0.0),
            vec![],
        ));
        let stack_props = Stacker::default();
        stack_props.autosize.set(true);
        let stack = StackerInstance::instantiate(args(
            stack_props,
            common(px(1000.0), None, 0.0),
            vec![section, next],
        ));
        let mut outer_props = Scroller::default();
        outer_props.autosize.set(true);
        let outer = scroller(
            &outer_props,
            common(px(1000.0), Some(px(800.0)), 0.0),
            stack,
        );
        let mut root_args = args(Group::default(), CommonProperties::default(), vec![]);
        root_args.component_template = Some(RefCell::new(vec![outer]));
        let root =
            ExpandedNode::initialize_root(ComponentInstance::instantiate(root_args), &context);
        root.recurse_update(&context);
        context.drain_node_effects();
        let outer = root.children.get()[0].clone();
        let stack = outer.get_node_context(&context).received_children.get()[0].clone();
        let section = stack.children.get()[0].clone();
        let next = stack.children.get()[1].clone();
        let inner = section.children.get()[0].clone();
        let text = inner.get_node_context(&context).received_children.get()[0].clone();
        {
            let _graph = context.property_graph.enter();
            inner_props.on_mount(&inner.get_node_context(&context));
            outer_props.on_mount(&outer.get_node_context(&context));
        }
        context.drain_node_effects();
        Self {
            context,
            root,
            stack,
            section,
            next,
            inner,
            text,
            inner_props,
            outer_props,
        }
    }

    fn measure_text(&self, width: f64, height: f64) {
        self.text.set_measured_size(width, height);
        self.settle();
        // Native text still changes the inner scroll range in both axes.
        assert_eq!(
            self.inner_props._resolved_scroll_width.get(),
            px(width + 14.0)
        );
        assert_eq!(
            self.inner_props._resolved_scroll_height.get(),
            px(height + 14.0)
        );
    }

    fn settle(&self) {
        self.root.recurse_update(&self.context);
        self.context.drain_node_effects();
    }

    fn assert_section_height(&self, height: f64) {
        assert_eq!(
            self.section.subtree_layout_hull.get().forward_extent_x(),
            Some(1000.0)
        );
        assert_eq!(
            self.section.subtree_layout_hull.get().forward_extent_y(),
            Some(height)
        );
        assert_eq!(
            self.stack.transform_and_bounds.get().bounds.1,
            height + 80.0
        );
        assert_eq!(
            self.next.container_frame.get().unwrap().transform.m[5],
            height
        );
        assert_eq!(
            self.outer_props._resolved_scroll_height.get(),
            px(height + 80.0)
        );
    }
}

#[test]
fn source_drawer_measurements_do_not_reflow_outer_sections() {
    let f = Fixture::new();
    for (width, height) in [(400.0, 300.0), (9000.0, 8000.0), (400.0, 600.0), (0.0, 0.0)] {
        f.measure_text(width, height);
        assert_eq!(f.inner.transform_and_bounds.get().bounds, (430.0, 480.0));
        f.assert_section_height(600.0);
    }
}

#[test]
fn viewport_resize_updates_ancestors_without_exporting_scroll_content() {
    let f = Fixture::new();
    f.measure_text(9000.0, 8000.0);
    for height in [800.0, 400.0, 600.0] {
        f.section
            .get_common_properties()
            .borrow()
            .height
            .set(Some(px(height)));
        f.settle();
        assert_eq!(f.inner.transform_and_bounds.get().bounds.1, height - 120.0);
        f.assert_section_height(height);
    }
}

#[test]
fn scroller_boundary_preserves_parent_dependent_axis_safeguards() {
    let f = Fixture::new();
    f.measure_text(9000.0, 8000.0);
    let common = f.inner.get_common_properties();
    common.borrow().width.set(Some(Size::Percent(100.into())));
    f.settle();
    let hull = f.inner.subtree_layout_hull.get();
    assert_eq!(hull.forward_extent_x(), None);
    assert_eq!(hull.forward_extent_y(), None);
    f.assert_section_height(600.0);
    common.borrow().width.set(Some(px(430.0)));
    common.borrow().height.set(Some(px(480.0)));
    f.settle();
    assert_eq!(
        f.inner.subtree_layout_hull.get().forward_extent_x(),
        Some(430.0)
    );
    assert_eq!(
        f.inner.subtree_layout_hull.get().forward_extent_y(),
        Some(480.0)
    );
    f.assert_section_height(600.0);
}

#[test]
fn layout_boundary_does_not_depend_on_temporary_render_clipping() {
    let f = Fixture::new();
    f.measure_text(400.0, 8000.0);
    let host = f.inner.children.get()[0].clone();
    for clip in [false, true] {
        host.with_properties_unwrapped(|p: &mut ScrollerHost| p._clip_content.set(clip));
        f.settle();
        assert_eq!(
            host.instance_node
                .borrow()
                .resolve_local_clip_path(&host)
                .is_some(),
            clip
        );
        f.assert_section_height(600.0);
    }
}

#[test]
fn scrolling_nested_content_does_not_change_outer_layout() {
    let f = Fixture::new();
    f.measure_text(9000.0, 8000.0);
    for position in [100.0, 4000.0, 0.0] {
        f.inner_props.scroll_pos_x.set(position);
        f.inner_props.scroll_pos_y.set(position);
        f.settle();
        f.assert_section_height(600.0);
        assert_eq!(f.inner_props._resolved_scroll_width.get(), px(9014.0));
        assert_eq!(f.inner_props._resolved_scroll_height.get(), px(8014.0));
    }
}

#[test]
fn ordinary_group_overflow_still_contributes_to_ancestor_measurement() {
    let f = Fixture::new();
    // Add ordinary non-scrolling overflow beside the source drawer.
    // Existing Group semantics must not become a blanket fixed-size clamp.
    // Use a real Text as an overflowing descendant of the fixed-height section.
    let text = TextInstance::instantiate(args(
        Text::default(),
        common(px(400.0), Some(px(2000.0)), 0.0),
        vec![],
    ));
    let children = f.section.generate_children(
        std::iter::once((text as Rc<dyn InstanceNode>, f.section.stack.clone())),
        &f.context,
        &f.section.parent_frame,
        true,
    );
    let mut all = f.section.children.get();
    all.extend(children);
    f.section.children.set(all);
    f.settle();
    assert_eq!(
        f.section.subtree_layout_hull.get().forward_extent_y(),
        Some(2000.0)
    );
}
