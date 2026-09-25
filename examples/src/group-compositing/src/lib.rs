use pax_kit::*;

mod compositing_card;
use compositing_card::CompositingCard;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub alpha: Property<f64>,
    pub moved: Property<bool>,
    pub show_image: Property<bool>,
    pub pixels: Property<Vec<u8>>,
    pub note: Property<String>,
    pub show_card: Property<bool>,
    pub alternate_theme: Property<bool>,
    pub mounts: Property<u64>,
    pub unmounts: Property<u64>,
    pub reverse_at: Property<u64>,
    pub artwork_at: Property<u64>,
    pub profile_at: Property<u64>,
    pub profile_step: Property<u64>,
}

impl Example {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.alpha.set(0.5);
        self.show_image.set(true);
        self.show_card.set(true);
        self.note.set("Edit me, then change opacity".into());
        // A tiny deterministic texture keeps this fixture independent of artwork
        // downloads and image decode timing while exercising the GPU Image path.
        self.pixels.set(vec![
            20, 160, 210, 255, 245, 130, 55, 255, 245, 130, 55, 255, 20, 160, 210, 255,
        ]);
        // Opt-in deterministic physical-device run. Normal interactive launches stay idle.
        if std::env::var("PAX_GROUP_COMPOSITING_PROFILE").as_deref() == Ok("1") {
            self.alpha.set(1.0);
            self.profile_at.set(ctx.elapsed_millis.get() + 3_000);
        }
    }

    pub fn step_alpha(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.alpha.set(if self.alpha.get() >= 1.0 {
            0.0
        } else {
            self.alpha.get() + 0.25
        });
    }

    pub fn move_card(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.moved.set(!self.moved.get());
    }

    pub fn toggle_image(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.show_image.set(!self.show_image.get());
    }

    pub fn toggle_card(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.reverse_at.set(0);
        self.show_card.set(!self.show_card.get());
    }

    pub fn reverse_exit(&mut self, ctx: &NodeContext, _event: Event<ButtonClick>) {
        if self.show_card.get() {
            self.show_card.set(false);
            self.reverse_at.set(ctx.elapsed_millis.get() + 300);
        }
    }

    pub fn toggle_theme(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.alternate_theme.set(!self.alternate_theme.get());
    }

    pub fn late_artwork(&mut self, ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.show_image.set(false);
        self.artwork_at.set(ctx.elapsed_millis.get() + 350);
    }

    pub fn pre_render(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_millis.get();
        if self.profile_at.get() != 0 && now >= self.profile_at.get() {
            let step = self.profile_step.get();
            if step < 12 {
                let shown = step % 2 == 1;
                self.show_card.set(shown);
                println!("[GroupProfile] step={step} shown={shown} time_ms={now}");
                self.profile_step.set(step + 1);
                self.profile_at.set(now + 1_400);
            } else {
                self.profile_at.set(0);
                println!(
                    "[GroupProfile] complete mounts={} exits={} time_ms={now}",
                    self.mounts.get(),
                    self.unmounts.get()
                );
            }
        }
        if self.reverse_at.get() != 0 && now >= self.reverse_at.get() {
            self.reverse_at.set(0);
            self.show_card.set(true);
        }
        if self.artwork_at.get() != 0 && now >= self.artwork_at.get() {
            self.artwork_at.set(0);
            // Simulate decoded pixels arriving after the surrounding group is cached.
            self.pixels.set(vec![
                180, 60, 200, 255, 250, 205, 60, 255, 250, 205, 60, 255, 180, 60, 200, 255,
            ]);
            self.show_image.set(true);
        }
    }
}
