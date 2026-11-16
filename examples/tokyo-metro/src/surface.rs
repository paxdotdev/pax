use crate::{geometry::Point, AtlasStore, LineView, SearchHit};
use pax_kit::*;

#[pax]
#[inlined(
    <Rectangle width=100% height=100% fill=TRANSPARENT/>
    @settings { @click: select @touch_start: touch_start @touch_move: touch_move @touch_end: touch_end @touch_cancel: touch_cancel }
)]
pub struct AtlasSurface {
    pub selected: Property<u32>,
    pub selected_train: Property<i64>,
    pub view_scale: Property<f64>,
    pub choices: Property<Vec<LineView>>,
    pub touch_active: Property<bool>,
    pub touch_x: Property<f64>,
    pub touch_y: Property<f64>,
    pub last_tap: Property<u64>,
}
impl AtlasSurface {
    pub fn select(&mut self, ctx: &NodeContext, e: Event<Click>) {
        if self.last_tap.get() > 0
            && ctx.elapsed_millis.get().saturating_sub(self.last_tap.get()) < 600
        {
            return;
        }
        self.select_at(ctx, e.mouse.x, e.mouse.y);
    }
    pub fn touch_start(&mut self, _: &NodeContext, e: Event<TouchStart>) {
        self.touch_active.set(e.touches.len() == 1);
        if let Some(t) = e.touches.first() {
            self.touch_x.set(t.x);
            self.touch_y.set(t.y);
        }
    }
    pub fn touch_move(&mut self, _: &NodeContext, e: Event<TouchMove>) {
        if e.touches.len() != 1
            || e.touches
                .iter()
                .any(|t| (t.x - self.touch_x.get()).hypot(t.y - self.touch_y.get()) > 8.)
        {
            self.touch_active.set(false);
        }
    }
    pub fn touch_end(&mut self, ctx: &NodeContext, e: Event<TouchEnd>) {
        if self.touch_active.get() {
            if let Some(t) = e.touches.first() {
                self.select_at(ctx, t.x, t.y);
                self.last_tap.set(ctx.elapsed_millis.get());
            }
        }
        self.touch_active.set(false);
    }
    pub fn touch_cancel(&mut self, _: &NodeContext, _: Event<TouchCancel>) {
        self.touch_active.set(false);
    }
    fn select_at(&mut self, ctx: &NodeContext, x: f64, y: f64) {
        let p = ctx.local_point(math::Point2::new(x, y));
        let (w, h) = ctx.bounds_self.get();
        let p = Point::new(p.x * w, p.y * h);
        let scale = self.view_scale.get().max(0.01);
        let _ = ctx.with_store(|s: &mut AtlasStore| {
            let nearest = s
                .layers
                .iter()
                .enumerate()
                .filter(|(i, _)| s.layout.alpha[*i] > 0.5)
                .flat_map(|(_, layer)| &layer.trains)
                .map(|t| (Point::new(t.x, t.y).distance(p), t.id))
                .min_by(|a, b| a.0.total_cmp(&b.0));
            if let Some((d, id)) = nearest {
                if d < 20. / scale {
                    self.selected_train.set(id as i64);
                    self.choices.set(vec![]);
                    return;
                }
            }
            if self.selected.get() > 0 {
                self.selected_train.set(-1);
                return;
            }
            let mut distances: Vec<_> = s
                .layout
                .samples
                .iter()
                .enumerate()
                .map(|(i, line)| {
                    (
                        line.iter()
                            .map(|c| c.distance(p))
                            .fold(f64::INFINITY, f64::min),
                        i,
                    )
                })
                .filter(|(d, _)| *d < 13. / scale)
                .collect();
            distances.sort_by(|a, b| a.0.total_cmp(&b.0));
            if let Some(&(best, line)) = distances.first() {
                let tied: Vec<_> = distances
                    .iter()
                    .filter(|(d, _)| *d - best < 4. / scale)
                    .collect();
                if tied.len() > 1 {
                    self.choices.set(
                        tied.iter()
                            .map(|&&(_, i)| {
                                let l = &s.network.lines[i];
                                LineView {
                                    id: i,
                                    code: l.code.into(),
                                    title: l.name.into(),
                                    color: crate::color(l.color),
                                }
                            })
                            .collect(),
                    );
                } else {
                    self.selected.set(line as u32 + 1);
                }
            }
        });
    }
}

#[pax]
#[inlined(
    <Text x=3px y=10px width=24px height=24px selectable=false _raycastable=false
        style={font: {family: "Arial", weight: 700}, font_size: 11px, fill: WHITE, align_horizontal: TextAlignHorizontal::Center, align_vertical: TextAlignVertical::Center}>{line.code}</Text>
    <Ellipse x=3px y=10px width=24px height=24px fill={line.color} _raycastable=false/>
    <Button x=34px width={100% - 34px} height=44px @button_click=self.choose
        color={selected == line.id + 1 ? rgb(229, 234, 227) : rgb(247, 247, 240)}
        hover_color=rgb(229, 234, 227) corner_radius=6
        outline={paint: TRANSPARENT, width: 0px}
        style={font: "Arial", font_size: 14px, fill: rgb(40, 53, 47), align_horizontal: TextAlignHorizontal::Left, align_vertical: TextAlignVertical::Center}>{line.title}</Button>
    @settings { }
)]
pub struct LineChoice {
    pub line: Property<LineView>,
    pub selected: Property<u32>,
}
impl LineChoice {
    pub fn choose(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.selected.set(self.line.get().id as u32 + 1);
    }
}

#[pax]
#[inlined(
    <Button width=100% height=44px color=rgb(255, 255, 250) hover_color=rgb(229, 234, 227)
        outline={paint: TRANSPARENT, width: 0px} @button_click=self.choose
        style={font: "Arial", font_size: 13px, fill: rgb(36, 53, 43), align_horizontal: TextAlignHorizontal::Left, align_vertical: TextAlignVertical::Center}>{hit.label}</Button>
)]
pub struct StationChoice {
    pub hit: Property<SearchHit>,
    pub selected: Property<usize>,
}
impl StationChoice {
    pub fn choose(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.selected.set(self.hit.get().id + 1);
    }
}
