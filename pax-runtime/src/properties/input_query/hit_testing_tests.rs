use crate::{
    BaseInstance, CommonPropertiesInit, ComponentInstance, ExpandedNode, InstanceFlags,
    InstanceNode, InstantiationArgs, PaxEngine, PropertiesInit, PropertiesScopeInit,
};
use pax_runtime_api::{
    math::Point2,
    pax_value::{PaxAny, PaxValue},
    Layer, Platform, OS,
};
use std::{cell::RefCell, rc::Rc};

struct HitProbe {
    base: BaseInstance,
    inside: bool,
    clips: bool,
}

fn args(children: Vec<Rc<dyn InstanceNode>>) -> InstantiationArgs {
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Default,
        prototypical_properties: PropertiesInit::Factory(Box::new(|_, existing| {
            existing
                .is_none()
                .then(|| Rc::new(RefCell::new(PaxAny::Builtin(PaxValue::default()))))
        })),
        children: Some(RefCell::new(children)),
        component_template: None,
        handler_registry: None,
        component_settings: None,
        component_camera: None,
        template_node_identifier: None,
        template_node_type_id: None,
        template_node_selector_info: None,
        transition_config: Default::default(),
        properties_scope: PropertiesScopeInit::None,
    }
}

impl HitProbe {
    fn new(
        inside: bool,
        clips: bool,
        invisible: bool,
        children: Vec<Rc<dyn InstanceNode>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: BaseInstance::new(
                args(children),
                InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: invisible,
                    is_component: false,
                    is_slot: false,
                    layer: Layer::DontCare,
                },
            ),
            inside,
            clips,
        })
    }
}

impl InstanceNode for HitProbe {
    fn instantiate(_: InstantiationArgs) -> Rc<Self> {
        unreachable!()
    }
    fn base(&self) -> &BaseInstance {
        &self.base
    }
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("HitProbe").finish()
    }
    fn clips_content(&self, _: &ExpandedNode) -> bool {
        self.clips
    }
    fn prepare_hit_shape(&self, _: &ExpandedNode) -> pax_runtime_api::HitShape {
        use kurbo::Shape;
        let rect = kurbo::Rect::new(0., 0., 100., 100.);
        if self.inside {
            pax_runtime_api::HitShape::Rect(rect)
        } else {
            // The ray lies in a hole but inside conservative coverage, so both
            // indexed and fallback queries must use the exact clip geometry.
            let mut path = rect.to_path(0.1);
            path.move_to((10., 10.));
            path.line_to((10., 30.));
            path.line_to((30., 30.));
            path.line_to((30., 10.));
            path.close_path();
            pax_runtime_api::HitShape::Path {
                path: Rc::new(path),
                tolerance: 0.,
            }
        }
    }
}

fn mount(children: Vec<Rc<dyn InstanceNode>>, indexed: bool) -> (PaxEngine, Rc<ExpandedNode>) {
    let mut engine = crate::test_support::empty_engine(
        (100.0, 100.0),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    let _graph = engine.runtime_context.property_graph.enter();
    let mut root_args = args(Vec::new());
    root_args.children = None;
    root_args.component_template = Some(RefCell::new(children));
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    engine.tick();
    engine.runtime_context.input_query_unindexed.set(!indexed);
    (engine, root)
}

fn hits(engine: &PaxEngine, root: &Rc<ExpandedNode>, invisible: bool) -> Vec<u32> {
    engine
        .runtime_context
        .get_elements_beneath_ray(
            Some(root.clone()),
            Point2::new(20.0, 20.0),
            false,
            Vec::new(),
            invisible,
        )
        .iter()
        .map(|node| node.id.to_u32())
        .collect()
}

#[test]
fn clipped_descendants_do_not_evaluate_hit_geometry() {
    for indexed in [false, true] {
        let leaves: Vec<_> = (0..1000)
            .map(|_| HitProbe::new(true, false, false, vec![]))
            .collect();
        let clip = HitProbe::new(
            false,
            true,
            true,
            leaves
                .iter()
                .map(|node| node.clone() as Rc<dyn InstanceNode>)
                .collect(),
        );
        let (engine, root) = mount(vec![clip.clone()], indexed);
        assert!(hits(&engine, &root, false).is_empty());
        assert_eq!(engine.runtime_context.input_query_stats().exact_tests, 1);
        assert_eq!(
            engine.runtime_context.input_query_stats().indexed_queries,
            u64::from(indexed)
        );
    }
}

#[test]
fn invisible_wrappers_skip_geometry_but_clips_still_test_the_ray() {
    for indexed in [false, true] {
        let leaf = HitProbe::new(true, false, false, vec![]);
        let clip = HitProbe::new(true, true, true, vec![leaf.clone()]);
        let wrapper = HitProbe::new(false, false, true, vec![clip.clone()]);
        let (engine, root) = mount(vec![wrapper.clone()], indexed);
        let leaf_node = root.children.get()[0].children.get()[0].children.get()[0].clone();
        assert_eq!(hits(&engine, &root, false), vec![leaf_node.id.to_u32()]);
        assert_eq!(engine.runtime_context.input_query_stats().exact_tests, 2);
    }
}

#[test]
fn unclippable_grandchild_can_escape_without_an_engine_tick() {
    for indexed in [false, true] {
        let escape = HitProbe::new(true, false, false, vec![]);
        let inner_clip = HitProbe::new(false, true, true, vec![escape.clone()]);
        let outer_clip = HitProbe::new(false, true, true, vec![inner_clip.clone()]);
        let (engine, root) = mount(vec![outer_clip], indexed);
        let escape_node = root.children.get()[0].children.get()[0].children.get()[0].clone();
        assert!(hits(&engine, &root, false).is_empty());
        escape_node
            .get_common_properties()
            .borrow()
            .unclippable
            .set(Some(true));
        assert_eq!(hits(&engine, &root, false), vec![escape_node.id.to_u32()]);
        assert_eq!(engine.runtime_context.input_query_stats().exact_tests, 3);
        escape_node
            .get_common_properties()
            .borrow()
            .unclippable
            .set(Some(false));
        assert!(hits(&engine, &root, false).is_empty());
    }
}

#[test]
fn escaped_descendants_remain_subject_to_their_own_clips() {
    for indexed in [false, true] {
        let leaf = HitProbe::new(true, false, false, vec![]);
        let inner_clip = HitProbe::new(false, true, true, vec![leaf.clone()]);
        let outer_clip = HitProbe::new(false, true, true, vec![inner_clip.clone()]);
        let (engine, root) = mount(vec![outer_clip], indexed);
        root.children.get()[0].children.get()[0]
            .get_common_properties()
            .borrow()
            .unclippable
            .set(Some(true));
        assert!(hits(&engine, &root, false).is_empty());
        assert_eq!(engine.runtime_context.input_query_stats().exact_tests, 2);
    }
}

#[test]
fn hit_invisible_still_queries_transparent_nodes_and_preserves_order() {
    for indexed in [false, true] {
        let first = HitProbe::new(true, false, true, vec![]);
        let second = HitProbe::new(true, false, false, vec![]);
        let wrapper = HitProbe::new(true, false, true, vec![first.clone(), second.clone()]);
        let (engine, root) = mount(vec![wrapper.clone()], indexed);
        let wrapper_node = root.children.get()[0].clone();
        let nodes = wrapper_node.children.get();
        assert_eq!(
            hits(&engine, &wrapper_node, false),
            vec![nodes[1].id.to_u32()]
        );
        assert_eq!(engine.runtime_context.input_query_stats().exact_tests, 1);
        assert_eq!(
            hits(&engine, &wrapper_node, true),
            vec![
                wrapper_node.id.to_u32(),
                nodes[0].id.to_u32(),
                nodes[1].id.to_u32()
            ]
        );
        let top = engine.runtime_context.get_elements_beneath_ray(
            Some(wrapper_node),
            Point2::new(20.0, 20.0),
            true,
            Vec::new(),
            false,
        );
        assert_eq!(top[0].id, nodes[1].id);
    }
}
