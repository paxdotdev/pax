//! Pre-traced website lettering. Ordinary builds need no font parser or shaper.
use crate::HeroCluster;
use pax_kit::{PathElement, Size};

pub const WORDS: [&str; 3] = [RUNS[0].text, RUNS[1].text, RUNS[2].text];

pub struct HeroRun {
    pub clusters: Vec<HeroCluster>,
    pub advance: f64,
    pub baseline: f64,
    pub cap_height: f64,
}

struct BakedRun {
    text: &'static str,
    advance: f64,
    baseline: f64,
    cap_height: f64,
    clusters: &'static [BakedCluster],
}

struct BakedCluster {
    byte_start: usize,
    byte_end: usize,
    x: f64,
    advance: f64,
    glyph_ids: &'static [u32],
    outline: usize,
}

// Compact source-asset representation; not a new Pax property or binary format.
enum Command {
    Move(f64, f64),
    Line(f64, f64),
    Quad(f64, f64, f64, f64),
    #[allow(dead_code)] // The pinned TrueType font uses quadratic curves.
    Cubic(f64, f64, f64, f64, f64, f64),
    Close,
}

impl Command {
    fn append(&self, out: &mut Vec<PathElement>) {
        let p = |v: f64| Size::Percent((v * 100.0).into());
        match *self {
            Self::Move(x, y) => out.push(PathElement::Point(p(x), p(y))),
            Self::Line(x, y) => {
                out.push(PathElement::Line);
                out.push(PathElement::Point(p(x), p(y)));
            }
            Self::Quad(a, b, x, y) => {
                out.push(PathElement::Quadratic(p(a), p(b)));
                out.push(PathElement::Point(p(x), p(y)));
            }
            Self::Cubic(a, b, c, d, x, y) => {
                out.push(PathElement::Cubic(p(a), p(b), p(c), p(d)));
                out.push(PathElement::Point(p(x), p(y)));
            }
            Self::Close => out.push(PathElement::Close),
        }
    }
}

include!("hero_outlines_data.rs");

struct BakedGlyph {
    letter: char,
    advance: f64,
    commands: &'static [Command],
}

include!("acrostic_outlines_data.rs");

pub fn acrostic_glyphs() -> (f64, Vec<(char, f64, Vec<PathElement>)>) {
    let glyphs = ACROSTIC_GLYPHS
        .iter()
        .map(|glyph| {
            let mut elements = Vec::new();
            for command in glyph.commands {
                command.append(&mut elements);
            }
            (glyph.letter, glyph.advance, elements)
        })
        .collect();
    (RUNS[0].cap_height, glyphs)
}

pub fn hero_runs() -> [HeroRun; 3] {
    let outlines: Vec<Vec<PathElement>> = OUTLINES
        .iter()
        .map(|commands| {
            let mut elements = Vec::new();
            for command in *commands {
                command.append(&mut elements);
            }
            elements
        })
        .collect();
    std::array::from_fn(|index| {
        let run = &RUNS[index];
        HeroRun {
            advance: run.advance,
            baseline: run.baseline,
            cap_height: run.cap_height,
            clusters: run
                .clusters
                .iter()
                .enumerate()
                .map(|(ordinal, cluster)| HeroCluster {
                    byte_start: cluster.byte_start,
                    byte_end: cluster.byte_end,
                    ordinal,
                    glyph_ids: cluster.glyph_ids.to_vec(),
                    x: cluster.x,
                    advance: cluster.advance,
                    elements: outlines[cluster.outline].clone(),
                })
                .collect(),
        }
    })
}
