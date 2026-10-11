//! Pre-traced hero lettering. Ordinary builds need no font parser or shaper.
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

#[cfg(test)]
mod tests {
    use super::*;
    use pax_text::{OutlineCommand, OutlineFont};

    #[test]
    fn baked_runs_match_live_shaping_and_every_outline_coordinate() {
        let mut font = OutlineFont::new(
            include_bytes!("../assets/fonts/Manrope.ttf").as_slice(),
            800.0,
        )
        .unwrap();
        assert_eq!(OUTLINES.len(), 17);
        for (word, baked) in WORDS.into_iter().zip(hero_runs()) {
            let live = font.shape_line(word).unwrap();
            assert_eq!(baked.advance, live.advance);
            assert_eq!(baked.cap_height, live.cap_height);
            assert_eq!(
                baked.baseline,
                (1.5 - (live.ascent - live.descent)) / 2.0 + live.ascent
            );
            assert_eq!(baked.clusters.len(), live.clusters.len());
            for (a, b) in baked.clusters.iter().zip(&live.clusters) {
                assert_eq!(
                    (a.byte_start, a.byte_end, a.ordinal),
                    (b.byte_start, b.byte_end, b.ordinal)
                );
                assert_eq!(a.glyph_ids, b.glyph_ids);
                assert_eq!((a.x, a.advance), (b.x, b.advance));
                let mut elements = Vec::new();
                for command in &b.outline {
                    match *command {
                        OutlineCommand::Move(x, y) => Command::Move(x, y),
                        OutlineCommand::Line(x, y) => Command::Line(x, y),
                        OutlineCommand::Quad(a, b, x, y) => Command::Quad(a, b, x, y),
                        OutlineCommand::Cubic(a, b, c, d, x, y) => Command::Cubic(a, b, c, d, x, y),
                        OutlineCommand::Close => Command::Close,
                    }
                    .append(&mut elements);
                }
                assert_eq!(
                    a.elements, elements,
                    "outline differs for {word} at {}",
                    a.byte_start
                );
            }
        }
    }
}
