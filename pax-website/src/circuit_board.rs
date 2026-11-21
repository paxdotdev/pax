use crate::circuit_logo::CircuitLogo;
use crate::circuit_package::CircuitPackage;
use crate::circuit_pin::CircuitPin;
use pax_kit::*;
use rand::Rng;
#[path = "circuit_layout.rs"]
mod layout;
#[path = "circuit_motion.rs"]
mod motion;
#[path = "circuit_traffic.rs"]
mod traffic;

#[pax]
pub struct CircuitChip {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub label: String,
    pub reference: String,
    pub pins_per_side: usize,
}

#[pax]
pub struct CircuitTrace {
    pub elements: Vec<PathElement>,
}

#[pax]
pub struct CircuitSignal {
    pub active: bool,
    pub x: f64,
    pub y: f64,
    pub color: Color,
    pub material: Material,
    pub casts_light: bool,
}

/// A retained hardware illustration; signal accents never rebuild its geometry.
#[pax]
#[file("circuit_board.pax")]
pub struct CircuitBoard {
    pub light_mode: Property<bool>,
    pub compact: Property<bool>,
    pub board_width: Property<f64>,
    pub board_height: Property<f64>,
    pub board_scale: Property<f64>,
    pub chips: Property<Vec<CircuitChip>>,
    pub traces: Property<Vec<CircuitTrace>>,
    pub signals: Property<Vec<CircuitSignal>>,
    pub visible: Property<bool>,
    pub rotation_y: Property<f64>,
    pub face_opacity: Property<f64>,
    #[pax(rust)]
    pub motion: motion::Motion,
    #[pax(rust)]
    pub motion_enabled: bool,
    #[pax(rust)]
    pub wiring: layout::Wiring,
    #[pax(rust)]
    pub tracks: Vec<traffic::Track>,
    #[pax(rust)]
    pub traffic: traffic::Traffic,
    #[pax(rust)]
    pub emitters: [Material; 3],
    #[pax(rust)]
    pub last_ms: f64,
    #[pax(rust)]
    pub geometry_compact: Option<bool>,
}

impl CircuitBoard {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.resize(ctx);
        self.publish_pose();
        self.last_ms = ctx.elapsed_time_millis() as f64;
    }

    pub fn enable_motion(&mut self, _: &NodeContext) {
        self.motion_enabled = true;
    }

    pub fn resize(&mut self, ctx: &NodeContext) {
        let (width, height) = ctx.bounds_self.get();
        let compact = width < 600.0;
        let (bw, bh) = layout::dimensions(compact);
        self.board_scale
            .set_if_neq((width / (bw + 36.0)).min(height / (bh + 46.0)).min(1.25) * 100.0);
        if self.geometry_compact == Some(compact) {
            return;
        }
        self.geometry_compact = Some(compact);
        self.compact.set(compact);
        self.board_width.set(bw);
        self.board_height.set(bh);
        let (chips, routes, wiring) = layout::build(compact);
        let chips = chips
            .into_iter()
            .map(|chip| CircuitChip {
                x: chip.center.0,
                y: chip.center.1,
                width: chip.size.0,
                height: chip.size.1,
                label: chip.label.into(),
                reference: chip.reference.into(),
                pins_per_side: chip.pins_per_side,
            })
            .collect();
        self.traces.set(
            routes
                .iter()
                .map(|points| CircuitTrace {
                    elements: path(points),
                })
                .collect(),
        );
        self.wiring = wiring;
        self.tracks = routes.into_iter().map(traffic::Track::new).collect();
        self.emitters =
            ["76F8FF", "FF82EA", "FFF475"].map(|rgb| Material::emissive(Color::from_hex(rgb), 1.1));
        self.chips.set(chips);
        self.update_signals(0);
    }

    pub fn visibility(&mut self, ctx: &NodeContext, event: Event<ViewportProximityChange>) {
        self.visible.set_if_neq(event.current.is_in_viewport());
        if !self.visible.get() && self.motion.cancel() {
            ctx.set_cursor(cursor::CursorStyle::Auto);
        }
        self.last_ms = ctx.elapsed_time_millis() as f64;
    }

    pub fn tick(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        let dt = now - self.last_ms;
        self.last_ms = now;
        if !self.visible.get() || !(0.0..1000.0).contains(&dt) {
            return;
        }
        self.motion.advance(dt);
        self.publish_pose();
        self.update_signals(dt as u64);
    }

    fn publish_pose(&mut self) {
        self.rotation_y
            .set_if_neq(self.motion.angle.rem_euclid(360.0));
        // Keep native labels mounted for capture, but omit reversed rear-facing ink.
        self.face_opacity
            .set_if_neq(if self.motion.angle.to_radians().cos() > 0.001 {
                1.0
            } else {
                0.0
            });
    }

    fn pointer(ctx: &NodeContext, x: f64, y: f64) -> (f64, f64) {
        // Handlers belong to the fixed stage, so rotating the board cannot feed back into drag distance.
        let local = ctx.local_point(pax_kit::math::Point2::new(x, y));
        let (width, height) = ctx.bounds_self.get();
        (local.x * width, local.y * height)
    }

    pub fn down(&mut self, ctx: &NodeContext, event: Event<MouseDown>) {
        if self.motion_enabled
            && event.mouse.button == MouseButton::Left
            && self.motion.begin(
                None,
                Self::pointer(ctx, event.mouse.x, event.mouse.y),
                ctx.bounds_self.get().0,
            )
        {
            event.prevent_default();
            ctx.set_cursor(cursor::CursorStyle::Grabbing);
        }
    }

    pub fn mouse_move(&mut self, ctx: &NodeContext, event: Event<MouseMove>) {
        if self
            .motion
            .move_to(None, Self::pointer(ctx, event.mouse.x, event.mouse.y))
        {
            self.publish_pose();
            event.prevent_default();
        }
    }

    pub fn up(&mut self, ctx: &NodeContext, event: Event<MouseUp>) {
        if event.mouse.button == MouseButton::Left {
            self.motion
                .move_to(None, Self::pointer(ctx, event.mouse.x, event.mouse.y));
            self.finish_drag(ctx, None);
        }
    }

    pub fn mouse_cancel(&mut self, ctx: &NodeContext, event: Event<MouseCancel>) {
        if event.mouse.button == MouseButton::Left {
            self.finish_drag(ctx, None);
        }
    }

    pub fn touch_start(&mut self, ctx: &NodeContext, event: Event<TouchStart>) {
        if self.motion_enabled {
            if let Some(touch) = event.touches.first() {
                self.motion.begin(
                    Some(touch.identifier),
                    Self::pointer(ctx, touch.x, touch.y),
                    ctx.bounds_self.get().0,
                );
            }
        }
    }

    pub fn touch_move(&mut self, ctx: &NodeContext, event: Event<TouchMove>) {
        for touch in &event.touches {
            if self
                .motion
                .move_to(Some(touch.identifier), Self::pointer(ctx, touch.x, touch.y))
            {
                self.publish_pose();
                event.prevent_default();
            }
        }
    }

    pub fn touch_end(&mut self, ctx: &NodeContext, event: Event<TouchEnd>) {
        for touch in &event.touches {
            self.motion
                .move_to(Some(touch.identifier), Self::pointer(ctx, touch.x, touch.y));
            self.finish_drag(ctx, Some(touch.identifier));
        }
    }

    pub fn touch_cancel(&mut self, ctx: &NodeContext, event: Event<TouchCancel>) {
        for touch in &event.touches {
            self.finish_drag(ctx, Some(touch.identifier));
        }
    }

    fn finish_drag(&mut self, ctx: &NodeContext, owner: Option<i64>) {
        if self.motion.end(owner) {
            self.publish_pose();
            self.last_ms = ctx.elapsed_time_millis() as f64;
            ctx.set_cursor(cursor::CursorStyle::Auto);
        }
    }

    fn update_signals(&mut self, dt: u64) {
        self.traffic
            .advance(dt, &self.wiring, &self.tracks, |count| {
                rand::thread_rng().gen_range(0..count)
            });
        // Reuse the same ten paint nodes and five lights across emissions.
        let mut signals: Vec<_> = (0..traffic::PACKET_SLOTS)
            .map(|slot| CircuitSignal {
                casts_light: slot % 2 == 0,
                ..Default::default()
            })
            .collect();
        for packet in &self.traffic.packets {
            let channel = packet.id % 3;
            let signal = &mut signals[packet.id % traffic::PACKET_SLOTS];
            (signal.x, signal.y) = self.tracks[packet.route].point(self.traffic.progress(packet));
            signal.active = true;
            signal.color = [Color::CYAN, Color::FUCHSIA, Color::YELLOW][channel].clone();
            signal.material = self.emitters[channel].clone();
        }
        self.signals.set(signals);
    }
}

fn path(points: &[(f64, f64)]) -> Vec<PathElement> {
    let mut elements = Vec::new();
    for (i, &(x, y)) in points.iter().enumerate() {
        if i > 0 {
            elements.push(PathElement::Line);
        }
        elements.push(PathElement::Point(
            Size::Pixels(x.into()),
            Size::Pixels(y.into()),
        ));
    }
    elements
}
