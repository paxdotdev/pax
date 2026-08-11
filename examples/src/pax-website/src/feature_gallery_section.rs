#![allow(unused_imports)]

use pax_kit::*;

const COMPACT_BREAKPOINT_PX: f64 = 760.0;
const DESKTOP_CARD_WIDTH_PX: f64 = 350.0;
const COMPACT_CARD_WIDTH_PX: f64 = 286.0;
const CARD_GAP_PX: f64 = 14.0;

#[pax]
#[custom(Defaults)]
pub struct FeatureRecord {
    pub ordinal: String,
    pub category: String,
    pub title: String,
    pub summary: String,
    pub docs_label: String,
    pub docs_url: String,
    pub visual_kind: usize,
}

#[pax]
#[file("feature_gallery_section.pax")]
pub struct FeatureGallerySection {
    pub compact: Property<bool>,
    pub features: Property<Vec<FeatureRecord>>,
    pub card_width_px: Property<f64>,
    pub rail_width_px: Property<f64>,
    pub scroll_x: Property<f64>,
    pub snap_positions: Property<Vec<Size>>,
}

impl FeatureGallerySection {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        // Temporary records keep visual/interaction work independent from the
        // evidence-backed PAX-869 content pass running in another worktree.
        self.features.set(placeholder_features());
        self.sync_layout(ctx);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
    }

    pub fn previous_card(&mut self, _ctx: &NodeContext, _args: Event<Click>) {
        let stride = self.card_width_px.get() + CARD_GAP_PX;
        self.scroll_x.set((self.scroll_x.get() - stride).max(0.0));
    }

    pub fn next_card(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        let stride = self.card_width_px.get() + CARD_GAP_PX;
        let viewport_width =
            (ctx.bounds_self.get().0 * if self.compact.get() { 1.0 } else { 0.94 }).max(0.0);
        let maximum = (self.rail_width_px.get() - viewport_width).max(0.0);
        self.scroll_x
            .set((self.scroll_x.get() + stride).min(maximum));
    }

    fn sync_layout(&mut self, ctx: &NodeContext) {
        let compact = ctx.bounds_self.get().0 < COMPACT_BREAKPOINT_PX;
        self.compact.set_if_neq(compact);

        let card_width = if compact {
            COMPACT_CARD_WIDTH_PX
        } else {
            DESKTOP_CARD_WIDTH_PX
        };
        self.card_width_px.set_if_neq(card_width);

        let feature_count = self.features.get().len();
        let rail_width = if feature_count == 0 {
            0.0
        } else {
            feature_count as f64 * card_width + (feature_count - 1) as f64 * CARD_GAP_PX
        };
        self.rail_width_px.set_if_neq(rail_width);

        self.snap_positions.set_if_neq(
            (0..feature_count)
                .map(|index| Size::Pixels(Numeric::F64(index as f64 * (card_width + CARD_GAP_PX))))
                .collect(),
        );
    }
}

fn placeholder_features() -> Vec<FeatureRecord> {
    [
        ("01", "RENDERING", "Lighting & materials", "Compose light, color, and surface treatments in the same declarative scene as the rest of your interface.", 0),
        ("02", "NATIVE / PLATFORM", "Native scrolling", "Keep familiar platform scrolling behavior while Pax coordinates the surrounding rendered interface.", 1),
        ("03", "LANGUAGE", "PAXEL operators", "Turn state into layout, style, and content through concise, side-effect-free expressions embedded directly in templates.", 2),
        ("04", "ANIMATION", "Declarative timelines", "Stage authored motion beside the component it belongs to, with keyframes and easing kept legible in source.", 3),
        ("05", "LAYOUT", "Responsive settings", "Shift hierarchy, navigation, and spacing at deliberate breakpoints without splitting the interface into separate implementations.", 1),
        ("06", "DEV TOOLING", "Hot reload lanes", "Iterate on templates and application logic with explicit reload boundaries across supported development targets.", 2),
        ("07", "RENDERING", "Path drawing", "Build precise vector forms from reusable path primitives and animate them as first-class pieces of the scene.", 0),
        ("08", "APPLICATION", "Declarative routing", "Map nested paths to Pax components while keeping browser history and not-found behavior inside the application structure.", 2),
        ("09", "LANGUAGE", "Reactive properties", "Let dependent values update like a spreadsheet graph while Rust owns state changes and side effects.", 3),
        ("10", "NATIVE / PLATFORM", "Native element composition", "Place platform controls and rendered content in one coordinated layout and transformation space.", 1),
    ]
    .into_iter()
    .map(|(ordinal, category, title, summary, visual_kind)| FeatureRecord {
        ordinal: ordinal.to_string(),
        category: category.to_string(),
        title: title.to_string(),
        summary: summary.to_string(),
        docs_label: "EXPLORE DOCS".to_string(),
        docs_url: "https://docs.pax.dev".to_string(),
        visual_kind,
    })
    .collect()
}
