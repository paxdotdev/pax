#![cfg(not(feature = "designtime"))]

// Exercises retained rendering without a GPU so a one-frame branch gap is deterministic.
use pax_runtime::api::*;
use pax_runtime::*;
use pax_runtime_api::pax_value::{PaxAny, ToFromPaxAny};
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
};

fn args() -> InstantiationArgs {
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Default,
        prototypical_properties: PropertiesInit::Factory(Box::new(|_, node| {
            node.is_none()
                .then(|| Rc::new(RefCell::new(PaxAny::Builtin(Default::default()))))
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

struct Leaf {
    base: BaseInstance,
}
impl InstanceNode for Leaf {
    fn instantiate(args: InstantiationArgs) -> Rc<Self> {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    layer: Layer::Canvas,
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    is_component: false,
                    is_slot: false,
                },
            ),
        })
    }
    fn base(&self) -> &BaseInstance {
        &self.base
    }
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Leaf").finish()
    }
    fn render(&self, node: &ExpandedNode, ctx: &Rc<RuntimeContext>, rc: &mut dyn RenderContext) {
        rc.begin_node(0, node.id.to_u32(), node.occlusion.get().z_index, 0);
        rc.end_node(0, node.id.to_u32());
        ctx.clear_canvas_node_dirty(&node.id);
    }
}

#[derive(Default)]
struct Recorder {
    nodes: HashSet<u32>,
    layers: usize,
}
impl RenderContext for Recorder {
    fn begin_node(&mut self, _: usize, id: u32, _: i32, _: u32) -> bool {
        self.nodes.insert(id);
        true
    }
    fn remove_node(&mut self, _: usize, id: u32) -> bool {
        self.nodes.remove(&id);
        true
    }
    fn save(&mut self, _: usize) {}
    fn restore(&mut self, _: usize) {}
    fn clip(&mut self, _: usize, _: kurbo::BezPath) {}
    fn layers(&self) -> usize {
        self.layers
    }
    fn fill_with_opacity(&mut self, _: usize, _: kurbo::BezPath, _: &Fill, _: f64) {}
    fn stroke_with_opacity(&mut self, _: usize, _: kurbo::BezPath, _: &Stroke, _: f64) {}
    fn stroke_with_draw_range_and_material_and_opacity(
        &mut self,
        _: usize,
        _: kurbo::BezPath,
        _: &Stroke,
        _: &Material,
        _: f64,
        _: f64,
        _: f64,
    ) {
    }
    fn transform(&mut self, _: usize, _: kurbo::Affine) {}
    fn load_image(&mut self, _: &str, _: &[u8], _: usize, _: usize) {}
    fn draw_image(&mut self, _: usize, _: &str, _: kurbo::Rect) {}
    fn get_image_size(&mut self, _: &str) -> Option<(usize, usize)> {
        None
    }
    fn image_loaded(&self, _: &str) -> bool {
        false
    }
    fn resize_layers_to(&mut self, n: usize, _: Rc<RefCell<Vec<bool>>>) {
        self.layers = n;
    }
    fn clear(&mut self, _: usize) {
        self.nodes.clear();
    }
    fn flush(&mut self, _: usize, _: Rc<RefCell<Vec<bool>>>) {}
    fn resize(&mut self, _: usize, _: usize) {}
    fn refresh_layers(&mut self, _: &[usize]) {}
}

fn handoff_frames(remove_sibling: bool, force_replay: bool) -> Vec<(u128, usize)> {
    let clock = Rc::new(Cell::new(0u128));
    let clock_source = clock.clone();
    let mut engine = PaxEngine::new_empty(
        (320.0, 240.0),
        Platform::Web,
        OS::Mac,
        Box::new(move || clock_source.get()),
        Default::default(),
    );
    let progress = Property::new(0.0f64);
    progress.ease_to(
        1.0,
        Duration::Milliseconds(1440.into()),
        EasingCurve::Linear,
    );
    let condition = Property::computed(
        {
            let p = progress.clone();
            move || p.get() < 0.999
        },
        &[progress.untyped()],
    );
    let mut conditional_args = args();
    conditional_args.children = Some(RefCell::new(vec![
        Leaf::instantiate(args()),
        Leaf::instantiate(args()),
    ]));
    conditional_args.prototypical_properties = PropertiesInit::Factory(Box::new(move |_, node| {
        node.is_none().then(|| {
            Rc::new(RefCell::new(
                ConditionalProperties {
                    boolean_expression: Property::new(true),
                    conditional_branches: vec![condition.clone(), Property::new(true)],
                }
                .to_pax_any(),
            ))
        })
    }));
    let conditional = ConditionalInstance::instantiate_with_branch_child_ranges(
        conditional_args,
        vec![0..1, 1..2],
    );
    let sibling_present = Property::new(true);
    let sibling_condition = sibling_present.clone();
    let mut sibling_args = args();
    sibling_args.children = Some(RefCell::new(vec![Leaf::instantiate(args())]));
    sibling_args.prototypical_properties = PropertiesInit::Factory(Box::new(move |_, node| {
        node.is_none().then(|| {
            Rc::new(RefCell::new(
                ConditionalProperties {
                    boolean_expression: sibling_condition.clone(),
                    conditional_branches: Vec::new(),
                }
                .to_pax_any(),
            ))
        })
    }));
    let sibling = ConditionalInstance::instantiate(sibling_args);
    let mut root_args = args();
    root_args.component_template = Some(RefCell::new(vec![conditional, sibling]));
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    // Each test has its own renderer; the engine's layer-count optimization is
    // process-wide, so initialize this context's dirty-layer storage explicitly.
    engine.runtime_context.resize_canvas_layers_to(1);
    let mut renderer = Recorder::default();
    let mut frames = Vec::new();
    for time in [0, 16, 1400, 1420, 1440, 1460, 1480, 1500] {
        clock.set(time);
        if time == 1440 && remove_sibling {
            sibling_present.set(false);
        }
        engine.tick();
        // A moving sibling forces a render, as an active ring does in the example.
        if force_replay {
            engine.runtime_context.set_canvas_dirty(0);
            engine.runtime_context.mark_canvas_nodes_on_layer_dirty(0);
        }
        engine.render(&mut renderer);
        let active_branch = root.children.get()[0].children.get()[0].id.to_u32();
        assert!(
            renderer.nodes.contains(&active_branch),
            "the current branch must be drawn in the same frame at {time}ms"
        );
        frames.push((time, renderer.nodes.len()));
    }
    frames
}

#[test]
fn clock_driven_handoff_survives_sibling_removal_and_retained_replay() {
    let frames = handoff_frames(true, true);
    assert_eq!(
        frames,
        vec![
            (0, 2),
            (16, 2),
            (1400, 2),
            (1420, 2),
            (1440, 1),
            (1460, 1),
            (1480, 1),
            (1500, 1)
        ]
    );
}

#[test]
fn clock_driven_handoff_preserves_visibility_without_sibling_removal() {
    assert!(handoff_frames(false, true)
        .iter()
        .all(|(_, count)| *count == 2));
}

#[test]
fn clock_alone_replaces_an_idle_retained_branch() {
    assert!(handoff_frames(false, false)
        .iter()
        .all(|(_, count)| *count == 2));
}

#[test]
fn clock_driven_handoff_does_not_need_unrelated_redraws() {
    let frames = handoff_frames(true, false);
    assert_eq!(
        frames,
        vec![
            (0, 2),
            (16, 2),
            (1400, 2),
            (1420, 2),
            (1440, 1),
            (1460, 1),
            (1480, 1),
            (1500, 1)
        ]
    );
}
