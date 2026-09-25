use super::*;
use std::cell::Cell;

#[derive(Debug, PartialEq)]
enum Op {
    Clear(usize),
    Image(usize, f64, f64),
    Blend(usize, usize, f64),
    Composite(usize, usize, f64),
}

struct Surface {
    id: usize,
    next_id: Rc<Cell<usize>>,
    ops: Rc<RefCell<Vec<Op>>>,
    context: piet::NullRenderContext,
}

impl Surface {
    fn root() -> Self {
        Self {
            id: 0,
            next_id: Rc::new(Cell::new(1)),
            ops: Rc::default(),
            context: piet::NullRenderContext::new(),
        }
    }
}

impl PietSurface for Surface {
    type Context = piet::NullRenderContext;
    fn context(&mut self) -> &mut Self::Context {
        &mut self.context
    }
    fn clear(&mut self) {
        self.ops.borrow_mut().push(Op::Clear(self.id));
    }
    fn configure(&mut self, _: (f32, f32), _: (u32, u32), _: [f32; 2]) {}
    fn create_group(&self) -> Self {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        Self {
            id,
            next_id: self.next_id.clone(),
            ops: self.ops.clone(),
            context: piet::NullRenderContext::new(),
        }
    }
    fn composite(&mut self, source: &Self, opacity: f64) {
        self.ops
            .borrow_mut()
            .push(Op::Composite(self.id, source.id, opacity));
    }
    fn draw_blend(&mut self, _: &kurbo::BezPath, terms: &[(Fill, f64)], opacity: f64) {
        self.ops
            .borrow_mut()
            .push(Op::Blend(self.id, terms.len(), opacity));
    }
    fn draw_image(&mut self, _: &piet::NullImage, rect: kurbo::Rect, opacity: f64) {
        self.ops
            .borrow_mut()
            .push(Op::Image(self.id, rect.x0, opacity));
    }
}

fn draw(label: f64, scopes: &[(u32, f32)]) -> PietDraw<piet::NullImage> {
    PietDraw {
        state: DrawState::default(),
        scopes: scopes
            .iter()
            .map(|&(node_id, opacity)| OpacityScope { node_id, opacity })
            .collect(),
        paint: PietPaint::Image(
            piet::NullImage,
            kurbo::Rect::new(label, 0.0, label + 1.0, 1.0),
            1.0,
        ),
    }
}

#[test]
fn groups_compose_after_children_and_preserve_interleaved_siblings() {
    let mut surface = Surface::root();
    let mut scratch = None;
    let draws = [
        draw(1.0, &[(10, 0.5)]),
        draw(2.0, &[(10, 0.5), (20, 0.25)]),
        draw(3.0, &[(10, 0.5)]),
        draw(4.0, &[]),
        draw(5.0, &[(10, 0.5)]),
    ];
    paint_draws(&mut surface, &mut scratch, &draws, 0);
    assert_eq!(
        *surface.ops.borrow(),
        [
            Op::Clear(1),
            Op::Image(1, 1.0, 1.0),
            Op::Clear(2),
            Op::Image(2, 2.0, 1.0),
            Op::Composite(1, 2, 0.25),
            Op::Image(1, 3.0, 1.0),
            Op::Composite(0, 1, 0.5),
            Op::Image(0, 4.0, 1.0),
            Op::Clear(1),
            Op::Image(1, 5.0, 1.0),
            Op::Composite(0, 1, 0.5),
        ]
    );
    assert_eq!(
        surface.next_id.get(),
        3,
        "sibling runs reuse the same two scratch surfaces"
    );
    surface.ops.borrow_mut().clear();
    paint_draws(&mut surface, &mut scratch, &draws, 0);
    assert_eq!(surface.next_id.get(), 3, "dirty frames reuse allocations");
}

#[test]
fn zero_skips_work_and_opaque_ancestors_do_not_allocate_or_swallow_child_opacity() {
    let mut surface = Surface::root();
    let mut scratch = None;
    paint_draws(
        &mut surface,
        &mut scratch,
        &[
            draw(1.0, &[(10, 0.0), (20, 0.5)]),
            draw(2.0, &[(30, 1.0)]),
            draw(3.0, &[(30, 1.0), (40, 0.5)]),
        ],
        0,
    );
    assert_eq!(
        *surface.ops.borrow(),
        [
            Op::Image(0, 2.0, 1.0),
            Op::Clear(1),
            Op::Image(1, 3.0, 1.0),
            Op::Composite(0, 1, 0.5),
        ]
    );
    assert_eq!(surface.next_id.get(), 2);
}

#[test]
fn image_paint_alpha_is_applied_inside_the_group() {
    let mut surface = Surface::root();
    let mut draw = draw(1.0, &[(10, 0.5)]);
    draw.paint = PietPaint::Image(piet::NullImage, kurbo::Rect::new(1.0, 0.0, 2.0, 1.0), 0.25);
    paint_draws(&mut surface, &mut None, &[draw], 0);
    assert_eq!(
        *surface.ops.borrow(),
        [
            Op::Clear(1),
            Op::Image(1, 1.0, 0.25),
            Op::Composite(0, 1, 0.5)
        ]
    );
}

#[test]
fn recorded_clips_keep_their_transform_and_restore_before_the_next_sibling() {
    use crate::api::RenderContext;
    use crate::engine::layer_surface::LayerSurfaceSize;
    let mut renderer = PietRenderer::new(|_| {
        let entry = LayerSurfaceEntry {
            key: "tile".into(),
            host_signature: "host".into(),
            origin_x: 0.0,
            origin_y: 0.0,
            replay_priority: 0,
            surface: LayerSurfaceSize {
                logical_width: 64.0,
                logical_height: 64.0,
                surface_width: 128,
                surface_height: 128,
                dpr: [2.0, 2.0],
            },
        };
        let target = PietLayerTarget::new(
            vec![PietLayerRenderer::new(
                entry.key.clone(),
                entry.host_signature.clone(),
                Surface::root(),
                &entry,
            )],
            true,
        );
        (
            target,
            Box::new(|| LayerSurfaceLayout {
                surfaces: vec![],
                active: true,
            }),
        )
    });
    renderer.resize_layers_to(1, Rc::new(RefCell::new(vec![true])));
    renderer.begin_node(0, 10, 0, 0);
    renderer.save(0);
    renderer.transform(0, Affine::translate((10.0, 20.0)));
    renderer.clip(0, kurbo::Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.1));
    renderer.end_node(0, 10);
    renderer.begin_node(0, 11, 1, 0);
    renderer.set_node_opacity_scopes(
        0,
        11,
        &[OpacityScope {
            node_id: 10,
            opacity: 0.5,
        }],
    );
    renderer.save(0);
    renderer.transform(0, Affine::translate((3.0, 4.0)));
    let path = kurbo::Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.1);
    let fill = Fill::Solid(pax_runtime_api::Color::rgba(
        1.0.into(),
        0.0.into(),
        0.0.into(),
        1.0.into(),
    ));
    renderer.fill_with_opacity(0, path.clone(), &fill, 1.0);
    renderer.restore(0);
    renderer.end_node(0, 11);
    renderer.restore(0);
    renderer.begin_node(0, 12, 2, 0);
    renderer.set_node_opacity_scopes(0, 12, &[]);
    renderer.fill_with_opacity(0, path, &fill, 1.0);
    renderer.end_node(0, 12);
    let draws = &renderer.layers[0].0.renderers[0].draws;
    assert_eq!(draws[0].state.transform, Affine::translate((13.0, 24.0)));
    assert_eq!(
        draws[0].state.clips[0].bounding_box(),
        kurbo::Rect::new(10.0, 20.0, 20.0, 30.0)
    );
    assert_eq!(draws[0].scopes[0].node_id, 10);
    assert_eq!(draws[1].state.transform, Affine::IDENTITY);
    assert!(draws[1].state.clips.is_empty());
    assert!(draws[1].scopes.is_empty());
}

#[test]
fn paint_mixture_is_accumulated_inside_its_opacity_group() {
    let mut surface = Surface::root();
    let mut draw = draw(1.0, &[(10, 0.5)]);
    draw.paint = PietPaint::Blend(
        kurbo::Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.1),
        vec![
            (Fill::Solid(pax_runtime_api::Color::BLACK), 0.25),
            (Fill::Solid(pax_runtime_api::Color::WHITE), 0.75),
        ],
        0.8,
    );
    paint_draws(&mut surface, &mut None, &[draw], 0);
    assert_eq!(
        *surface.ops.borrow(),
        [Op::Clear(1), Op::Blend(1, 2, 0.8), Op::Composite(0, 1, 0.5)]
    );
}
