use pax_kit::*;
use pax_kit::math::*;

const PALETTE_IMAGE_SIZE: usize = 64;

#[pax]
#[main]
#[file("color_picker.pax")]
pub struct ColorPickerExample {
    pub color: Property<Color>,

    // ---------color palette picker------------
    pub saturation_and_lightness_image_data: Property<Vec<u8>>,
    pub hue_slider_image_data: Property<Vec<u8>>,
    pub alpha_slider_image_data: Property<Vec<u8>>,

    // 0.0-1.0 coord in palette
    pub hue: Property<f64>,
    pub saturation: Property<f64>,
    pub lightness: Property<f64>,
    pub alpha: Property<f64>,

    pub mouse_is_down_on_palette: Property<bool>,

    // All the below props should be private: never set by user, used for internal state
    // TODO hook up
    pub red: Property<String>,
    pub green: Property<String>,
    pub blue: Property<String>,
    pub alpha_text: Property<String>,

    pub property_listener: Property<bool>,
}

impl ColorPickerExample {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        let color = self.color.clone();
        let initial_hsla = color.get().to_hsla_0_1();
        self.hue.set(initial_hsla[0]);
        self.saturation.set(initial_hsla[1]);
        self.lightness.set(initial_hsla[2]);
        self.alpha.set(initial_hsla[3]);
        self.hue_slider_image_data.set(hue_slider().to_vec());

        fn get_color_channel(color: &Property<Color>, i: usize) -> String {
            ((color.get().to_rgba_0_1()[i] * 255.0) as u8).to_string()
        }

        let color = self.color.clone();
        let deps = [color.untyped()];
        self.red.replace_with(Property::computed(
            move || get_color_channel(&color, 0),
            &deps,
        ));

        let color = self.color.clone();
        self.green.replace_with(Property::computed(
            move || get_color_channel(&color, 1),
            &deps,
        ));

        let color = self.color.clone();
        self.blue.replace_with(Property::computed(
            move || get_color_channel(&color, 2),
            &deps,
        ));

        let color = self.color.clone();
        self.alpha_text.replace_with(Property::computed(
            move || get_color_channel(&color, 3),
            &deps,
        ));

        let hue = self.hue.clone();
        let deps = [hue.untyped()];
        self.saturation_and_lightness_image_data
            .replace_with(Property::computed(
                move || {
                    // saturated/max brightness color with this hue
                    palette(hue.get()).to_vec()
                },
                &deps,
            ));

        let hue = self.hue.clone();
        let saturation = self.saturation.clone();
        let lightness = self.lightness.clone();
        let alpha = self.alpha.clone();
        let color = self.color.clone();
        let deps = [
            hue.untyped(),
            saturation.untyped(),
            lightness.untyped(),
            alpha.untyped(),
        ];
        self.property_listener.replace_with(Property::computed(
            move || {
                let new_col = Color::hsla(
                    Rotation::Percent(hue.get().into()),
                    (saturation.get() * 255.0).into(),
                    (lightness.get() * 255.0).into(),
                    (alpha.get() * 255.0).into(),
                );
                color.set(new_col);
                true
            },
            &deps,
        ));
        let color = self.color.clone();
        self.alpha_slider_image_data
            .replace_with(Property::computed(
                move || alpha_slider(color.get()).to_vec(),
                &deps,
            ));
    }

    pub fn palette_mouse_down(&mut self, ctx: &NodeContext, event: Event<MouseDown>) {
        self.mouse_is_down_on_palette.set(true);
        self.palette_set_from_mouse(ctx, &event.mouse)
    }
    pub fn palette_mouse_move(&mut self, ctx: &NodeContext, event: Event<MouseMove>) {
        if self.mouse_is_down_on_palette.get() {
            self.palette_set_from_mouse(ctx, &event.mouse);
        }
    }

    pub fn palette_mouse_up(&mut self, _ctx: &NodeContext, _event: Event<MouseUp>) {
        self.mouse_is_down_on_palette.set(false);
    }

    pub fn palette_touch_start(&mut self, ctx: &NodeContext, event: Event<TouchStart>) {
        if let Some(touch) = event.touches.first() {
            self.mouse_is_down_on_palette.set(true);
            self.palette_set_xy(ctx, touch.x, touch.y);
        }
    }

    pub fn palette_touch_move(&mut self, ctx: &NodeContext, event: Event<TouchMove>) {
        if self.mouse_is_down_on_palette.get() {
            if let Some(touch) = event.touches.first() {
                self.palette_set_xy(ctx, touch.x, touch.y);
            }
        }
    }

    pub fn palette_touch_end(&mut self, ctx: &NodeContext, event: Event<TouchEnd>) {
        if let Some(touch) = event.touches.first() {
            self.palette_set_xy(ctx, touch.x, touch.y);
        }
        self.mouse_is_down_on_palette.set(false);
    }

    pub fn palette_set_from_mouse(&mut self, ctx: &NodeContext, mouse: &MouseEventArgs) {
        self.palette_set_xy(ctx, mouse.x, mouse.y);
    }

    fn palette_set_xy(&mut self, ctx: &NodeContext, x: f64, y: f64) {
        let p = Point2::new(x, y);
        let local = ctx.local_point(p);
        self.saturation.set(local.x.clamp(0.0, 1.0));
        self.lightness.set(1.0 - local.y.clamp(0.0, 1.0));
    }

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        self.property_listener.get();
    }

    pub fn red_change(&mut self, _ctx: &NodeContext, event: Event<TextboxChange>) {
        if let Some(val) = color_channel(&event.text) {
            self.rgba_text_change(0, val);
        }
    }

    pub fn green_change(&mut self, _ctx: &NodeContext, event: Event<TextboxChange>) {
        if let Some(val) = color_channel(&event.text) {
            self.rgba_text_change(1, val);
        }
    }

    pub fn blue_change(&mut self, _ctx: &NodeContext, event: Event<TextboxChange>) {
        if let Some(val) = color_channel(&event.text) {
            self.rgba_text_change(2, val);
        }
    }

    pub fn alpha_change(&mut self, _ctx: &NodeContext, event: Event<TextboxChange>) {
        if let Some(val) = color_channel(&event.text) {
            self.rgba_text_change(3, val);
        }
    }

    pub fn rgba_text_change(&mut self, chan: usize, val: u8) {
        let curr = self.color.get();
        let mut rgba = curr.to_rgba_0_1();
        rgba[chan] = val as f64 / 255.0;
        let hsla = Color::rgba(
            (rgba[0] * 255.0).into(),
            (rgba[1] * 255.0).into(),
            (rgba[2] * 255.0).into(),
            (rgba[3] * 255.0).into(),
        )
        .to_hsla_0_1();
        self.hue.set(hsla[0]);
        self.saturation.set(hsla[1]);
        self.lightness.set(hsla[2]);
        self.alpha.set(hsla[3]);
    }
}

#[rustfmt::skip]
fn palette(hue: f64) -> Vec<u8> {
    let mut res = Vec::with_capacity(PALETTE_IMAGE_SIZE * PALETTE_IMAGE_SIZE * 4);
    let max_index = (PALETTE_IMAGE_SIZE - 1) as i32;
    for y in 0..PALETTE_IMAGE_SIZE as i32 {
        for x in 0..PALETTE_IMAGE_SIZE as i32 {
            let c = Color::hsl(
                Rotation::Percent(hue.into()),
                (x * 255 / max_index).into(),
                (255 - y * 255 / max_index).into(),
            )
            .to_rgba_0_1();
           res.extend(c.map(|v| (v * 255.0) as u8)) 
        }
    }
    res
}

fn hue_slider() -> [u8; 10 * 4] {
    let mut hues = [0u8; 10 * 4];
    for i in (0..40).step_by(4) {
        let c = Color::hsl(
            Rotation::Percent((i as f64 / 40.0).into()),
            255.into(),
            (255 / 2).into(),
        )
        .to_rgba_0_1();
        hues[i..(i + 4)].copy_from_slice(&c.map(|v| (v * 255.0) as u8));
    }
    hues
}

fn alpha_slider(color: Color) -> [u8; 10 * 4] {
    let mut alphas = [0u8; 10 * 4];
    let rgba = color.to_rgba_0_1();
    for i in (0..40).step_by(4) {
        let c = Color::rgba(
            (rgba[0] * 255.0).into(),
            (rgba[1] * 255.0).into(),
            (rgba[2] * 255.0).into(),
            (i as f64 / 40.0 * 255.0).into(),
        )
        .to_rgba_0_1();
        alphas[i..(i + 4)].copy_from_slice(&c.map(|v| (v * 255.0) as u8));
    }
    alphas
}

fn color_channel(text: &str) -> Option<u8> {
    if text.is_empty() {
        return Some(0);
    }
    text.parse::<u8>().ok()
}
