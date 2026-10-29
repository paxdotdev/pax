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
    commands: &'static [Command],
}

include!("acrostic_outlines_data.rs");

pub fn acrostic_glyphs() -> Vec<(char, Vec<PathElement>)> {
    ACROSTIC_GLYPHS
        .iter()
        .map(|glyph| {
            // This decorative grid uses equal ink boxes, not proportional text
            // advances. Include curve overshoot instead of clipping at cap height.
            let points: Vec<_> = glyph
                .commands
                .iter()
                .flat_map(|command| match *command {
                    Command::Move(x, y) | Command::Line(x, y) => vec![(x, y)],
                    Command::Quad(a, b, x, y) => vec![(a, b), (x, y)],
                    Command::Cubic(a, b, c, d, x, y) => vec![(a, b), (c, d), (x, y)],
                    Command::Close => vec![],
                })
                .collect();
            let bounds = points.iter().fold(
                [
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                ],
                |b, &(x, y)| [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)],
            );
            let x = |v: f64| (v - bounds[0]) / (bounds[2] - bounds[0]);
            let y = |v: f64| (v - bounds[1]) / (bounds[3] - bounds[1]);
            let mut elements = Vec::new();
            for command in glyph.commands {
                let normalized = match *command {
                    Command::Move(a, b) => Command::Move(x(a), y(b)),
                    Command::Line(a, b) => Command::Line(x(a), y(b)),
                    Command::Quad(a, b, c, d) => Command::Quad(x(a), y(b), x(c), y(d)),
                    Command::Cubic(a, b, c, d, e, f) => {
                        Command::Cubic(x(a), y(b), x(c), y(d), x(e), y(f))
                    }
                    Command::Close => Command::Close,
                };
                normalized.append(&mut elements);
            }
            (glyph.letter, elements)
        })
        .collect()
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
