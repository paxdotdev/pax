//! Authored sheet coordinates and orthogonal routes, in logical pixels.
//! Endpoint IDs refer to content.rs; source/target ports are resolved from these boxes.

pub const WIDTH: f64 = 3120.0;
pub const HEIGHT: f64 = 2160.0;

pub struct Placement {
    pub id: &'static str,
    pub rect: [f64; 4],
    pub band: usize,
}

/// Fixed card vocabulary; all nodes share one height and chip treatment.
pub enum CardSize {
    Compact,
    Single,
    Double,
}
impl CardSize {
    pub const fn width(self) -> f64 {
        match self {
            Self::Compact => 200.,
            Self::Single => 300.,
            Self::Double => 640.,
        }
    }
}
pub const CARD_HEIGHT: f64 = 88.;
pub const HEADING_BOTTOM: f64 = 75.;

macro_rules! place {
    ($id:literal, $x:literal, $y:literal, $size:ident, $band:literal) => {
        Placement {
            id: $id,
            rect: [$x as f64, $y as f64, CardSize::$size.width(), CARD_HEIGHT],
            band: $band,
        }
    };
}

pub const PLACEMENTS: &[Placement] = &[
    place!("source", 88, 246, Single, 0),
    place!("construct.analyze", 88, 466, Single, 0),
    place!("construct.connect", 88, 686, Single, 0),
    place!("cartridge", 88, 950, Single, 0),
    place!("construct.instantiate", 528, 246, Single, 1),
    place!("instance.behavior", 528, 466, Single, 1),
    place!("construct.expand", 528, 719, Single, 1),
    place!("scene", 528, 1027, Single, 1),
    place!("operate.drive", 1000, 200, Double, 2),
    place!("operate.react.properties", 1000, 420, Single, 2),
    place!("operate.react.evaluate", 1340, 420, Single, 2),
    place!("operate.react.dirty", 1000, 622, Single, 2),
    place!("operate.react.effects", 1340, 622, Single, 2),
    place!("operate.structure", 1000, 842, Double, 2),
    place!("operate.geometry", 1000, 1049, Single, 2),
    place!("operate.geometry.index", 1340, 1049, Single, 2),
    place!("operate.opacity", 1000, 1280, Single, 2),
    place!("operate.project", 1340, 1280, Single, 2),
    place!("present.plan.layers", 1840, 262, Compact, 3),
    place!("present.plan.coverage", 2070, 262, Compact, 3),
    place!("present.plan.tiles", 2300, 262, Compact, 3),
    place!("present.plan.occlusion", 1840, 482, Compact, 3),
    place!("present.work.replay", 2070, 482, Compact, 3),
    place!("present.work.dirty", 2300, 482, Compact, 3),
    place!("present.lights", 1840, 702, Compact, 3),
    place!("present.work.cull", 2070, 702, Compact, 3),
    place!("present.patch", 2300, 702, Compact, 3),
    place!("present.draw.adapter", 1840, 922, Single, 3),
    place!("present.draw.retained", 2200, 922, Single, 3),
    place!("present.draw.vector", 1840, 1142, Compact, 3),
    place!("present.draw.textures", 2070, 1142, Compact, 3),
    place!("present.draw.stencil", 2300, 1142, Compact, 3),
    place!("present.draw.materials", 1840, 1362, Single, 3),
    place!("present.draw.capture", 1840, 1582, Single, 3),
    place!("present.draw.alpha", 2200, 1582, Single, 3),
    place!("present.draw.capture.opacity", 1840, 1802, Single, 3),
    place!("present.draw.submit", 2200, 1802, Single, 3),
    place!("feedback.clock", 2700, 246, Single, 4),
    place!("feedback.input", 2700, 444, Single, 4),
    place!("chassis.bridge", 2700, 658, Single, 4),
    place!("chassis.native", 2700, 851, Single, 4),
    place!("chassis.masks", 2700, 1005, Single, 4),
    place!("chassis.surfaces", 2700, 1168, Single, 4),
    place!("present.compose", 2700, 1344, Single, 4),
    place!("feedback.resource", 2700, 1465, Single, 4),
];

pub struct AssemblySpec {
    pub title: &'static str,
    pub rect: [f64; 4],
}

pub const ASSEMBLIES: &[AssemblySpec] = &[
    AssemblySpec {
        title: "DIRTY DAG",
        rect: [982., 341., 676., 418.],
    },
    AssemblySpec {
        title: "COMPOSITOR",
        rect: [1822., 193., 696., 463.],
    },
    AssemblySpec {
        title: "RENDERER",
        rect: [1822., 852., 696., 1062.],
    },
];

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Port {
    Left,
    Right,
    Top,
    Bottom,
}

pub struct Connection {
    pub id: &'static str,
    pub source: &'static str,
    pub target: &'static str,
    pub from: Port,
    pub to: Port,
    pub via: &'static [(f64, f64)],
    pub label: &'static str,
    pub topology: &'static str,
}

use Port::*;
macro_rules! edge {
    ($id:literal, $source:literal, $from:ident, $target:literal, $to:ident, $via:expr, $label:literal, $topology:literal) => {
        Connection {
            id: $id,
            source: $source,
            target: $target,
            from: $from,
            to: $to,
            via: $via,
            label: $label,
            topology: $topology,
        }
    };
}

pub const CONNECTIONS: &[Connection] = &[
    edge!(
        "property-layout",
        "operate.react.effects",
        Left,
        "operate.geometry",
        Top,
        &[
            (1320., 666.),
            (1320., 781.),
            (983., 781.),
            (983., 1023.),
            (1150., 1023.)
        ],
        "layout values",
        "d02"
    ),
    edge!(
        "property-primitives",
        "operate.react.effects",
        Right,
        "operate.project",
        Left,
        &[
            (1665., 666.),
            (1665., 1247.),
            (1323., 1247.),
            (1323., 1324.)
        ],
        "primitive updates",
        "e12"
    ),
    edge!(
        "primitive-native",
        "operate.project",
        Right,
        "present.patch",
        Bottom,
        &[(1708., 1324.), (1708., 830.), (2400., 830.)],
        "native updates",
        "e16"
    ),
    edge!(
        "direct-vectors",
        "present.draw.vector",
        Left,
        "present.draw.submit",
        Left,
        &[
            (1805., 1182.),
            (1805., 1725.),
            (2189., 1725.),
            (2189., 1846.)
        ],
        "direct vectors",
        "d17"
    ),
    edge!(
        "paint",
        "present.draw.materials",
        Top,
        "present.draw.vector",
        Right,
        &[(1993., 1230.), (2055., 1230.), (2055., 1182.)],
        "",
        "d12"
    ),
    edge!(
        "image-records",
        "present.draw.retained",
        Bottom,
        "present.draw.textures",
        Top,
        &[(2347., 1103.), (2170., 1103.)],
        "",
        "d10"
    ),
    edge!(
        "clip-records",
        "present.draw.retained",
        Right,
        "present.draw.stencil",
        Top,
        &[(2513., 966.), (2513., 1107.), (2400., 1107.)],
        "",
        "d04"
    ),
    edge!(
        "direct-draw",
        "present.draw.textures",
        Bottom,
        "present.draw.submit",
        Top,
        &[(2170., 1740.), (2350., 1740.)],
        "direct draws",
        "d17"
    ),
    edge!(
        "stencil-test",
        "present.draw.stencil",
        Right,
        "present.draw.submit",
        Top,
        &[(2525., 1182.), (2525., 1747.), (2347., 1747.)],
        "",
        "d17"
    ),
    edge!(
        "opacity-boundary",
        "operate.opacity",
        Bottom,
        "present.draw.capture.opacity",
        Left,
        &[(1150., 1420.), (1768., 1420.), (1768., 1846.)],
        "opacity scopes",
        "d13"
    ),
    edge!(
        "surface-return",
        "chassis.surfaces",
        Left,
        "present.work.replay",
        Right,
        &[(2614., 1591.), (2614., 636.), (2285., 636.), (2285., 526.)],
        "ready / retarget",
        "d07"
    ),
    edge!(
        "sources",
        "source",
        Bottom,
        "construct.analyze",
        Top,
        &[],
        "analyzes",
        "e01"
    ),
    edge!(
        "definitions",
        "construct.analyze",
        Bottom,
        "construct.connect",
        Top,
        &[],
        "definitions",
        "e01"
    ),
    edge!(
        "cartridge",
        "construct.connect",
        Bottom,
        "cartridge",
        Top,
        &[],
        "generates",
        "e02"
    ),
    edge!(
        "startup",
        "cartridge",
        Right,
        "construct.instantiate",
        Left,
        &[(458., 998.), (458., 294.)],
        "startup",
        "e02"
    ),
    edge!(
        "behaviors",
        "construct.instantiate",
        Bottom,
        "instance.behavior",
        Top,
        &[],
        "constructs",
        "e03"
    ),
    edge!(
        "expand",
        "instance.behavior",
        Bottom,
        "construct.expand",
        Top,
        &[],
        "expands",
        "e03"
    ),
    edge!(
        "mount",
        "construct.expand",
        Bottom,
        "scene",
        Top,
        &[],
        "mounts instances",
        "e05"
    ),
    edge!(
        "wire",
        "construct.expand",
        Right,
        "operate.react.properties",
        Left,
        &[(913., 766.), (913., 464.)],
        "binds",
        "e04"
    ),
    edge!(
        "writes",
        "operate.drive",
        Bottom,
        "operate.react.properties",
        Top,
        &[(1320., 317.), (1161., 317.)],
        "writes",
        "e07"
    ),
    edge!(
        "dirty",
        "operate.react.properties",
        Bottom,
        "operate.react.dirty",
        Top,
        &[],
        "invalidates",
        "d01"
    ),
    edge!(
        "queue",
        "operate.react.dirty",
        Right,
        "operate.react.effects",
        Left,
        &[],
        "queues",
        "d01"
    ),
    edge!(
        "evaluate",
        "operate.react.effects",
        Top,
        "operate.react.evaluate",
        Bottom,
        &[],
        "demands values",
        "d02"
    ),
    edge!(
        "read",
        "operate.react.evaluate",
        Left,
        "operate.react.properties",
        Right,
        &[],
        "reads",
        "e04"
    ),
    edge!(
        "structure",
        "operate.react.effects",
        Bottom,
        "operate.structure",
        Top,
        &[(1484., 798.), (1320., 798.)],
        "resolves",
        "e08"
    ),
    edge!(
        "remount",
        "operate.structure",
        Left,
        "construct.expand",
        Right,
        &[(879., 891.), (879., 766.)],
        "creates / retires",
        "e09"
    ),
    edge!(
        "layout",
        "operate.structure",
        Bottom,
        "operate.geometry",
        Top,
        &[(1320., 988.), (1150., 988.)],
        "membership",
        "e13"
    ),
    edge!(
        "geometry",
        "operate.geometry",
        Right,
        "operate.geometry.index",
        Left,
        &[],
        "prepares",
        "d23"
    ),
    edge!(
        "hooks",
        "operate.geometry.index",
        Bottom,
        "operate.project",
        Top,
        &[],
        "geometry",
        "e13"
    ),
    edge!(
        "opacity",
        "scene",
        Right,
        "operate.opacity",
        Left,
        &[(900., 1084.), (900., 1324.)],
        "opacity ancestry",
        "d13"
    ),
    edge!(
        "order",
        "scene",
        Right,
        "present.plan.layers",
        Left,
        &[(870., 1230.), (870., 144.), (1770., 144.), (1770., 306.)],
        "presentation order",
        "e14"
    ),
    edge!(
        "clips",
        "present.plan.layers",
        Right,
        "present.plan.coverage",
        Left,
        &[],
        "clips",
        "d04"
    ),
    edge!(
        "tile",
        "present.plan.coverage",
        Right,
        "present.plan.tiles",
        Left,
        &[],
        "extents",
        "d05"
    ),
    edge!(
        "occlusion",
        "present.plan.coverage",
        Bottom,
        "present.plan.occlusion",
        Top,
        &[(2170., 416.), (1940., 416.)],
        "coverage",
        "d04"
    ),
    edge!(
        "replay",
        "present.plan.tiles",
        Bottom,
        "present.work.replay",
        Top,
        &[(2400., 433.), (2170., 433.)],
        "regions",
        "d06"
    ),
    edge!(
        "replay-query",
        "present.work.replay",
        Left,
        "operate.geometry.index",
        Right,
        &[(2055., 526.), (2055., 630.), (1722., 630.), (1722., 1362.)],
        "region query",
        "d21"
    ),
    edge!(
        "dirty-nodes",
        "operate.geometry.index",
        Right,
        "present.work.dirty",
        Bottom,
        &[(1750., 1093.), (1750., 626.), (2400., 626.)],
        "selected instances",
        "d22"
    ),
    edge!(
        "cull",
        "present.work.dirty",
        Bottom,
        "present.work.cull",
        Top,
        &[(2400., 668.), (2170., 668.)],
        "traversal",
        "d08"
    ),
    edge!(
        "patch",
        "present.plan.occlusion",
        Bottom,
        "present.patch",
        Top,
        &[(1940., 602.), (2400., 602.)],
        "native masks",
        "d15"
    ),
    edge!(
        "draw",
        "operate.project",
        Right,
        "present.draw.adapter",
        Left,
        &[(1692., 1752.), (1692., 966.)],
        "draw commands",
        "e15"
    ),
    edge!(
        "surface-cull",
        "present.work.cull",
        Bottom,
        "present.draw.adapter",
        Top,
        &[(2170., 811.), (1993., 811.)],
        "surface scope",
        "d08"
    ),
    edge!(
        "retain",
        "present.draw.adapter",
        Right,
        "present.draw.retained",
        Left,
        &[],
        "records",
        "d09"
    ),
    edge!(
        "vectors",
        "present.draw.retained",
        Bottom,
        "present.draw.vector",
        Top,
        &[(2347., 1077.), (1940., 1077.)],
        "reuses geometry / resources",
        "d10"
    ),
    edge!(
        "lighting",
        "present.lights",
        Left,
        "present.draw.materials",
        Left,
        &[(1800., 746.), (1800., 1395.)],
        "lighting",
        "d12"
    ),
    edge!(
        "capture",
        "present.draw.vector",
        Left,
        "present.draw.capture",
        Left,
        &[(1825., 1182.), (1825., 1626.)],
        "capture draws",
        "d19"
    ),
    edge!(
        "mask-source",
        "present.draw.capture",
        Right,
        "present.draw.alpha",
        Left,
        &[],
        "source alpha",
        "d20"
    ),
    edge!(
        "fade",
        "present.draw.capture",
        Bottom,
        "present.draw.capture.opacity",
        Top,
        &[],
        "cached pixels",
        "d13"
    ),
    edge!(
        "compose",
        "present.draw.capture.opacity",
        Right,
        "present.draw.submit",
        Left,
        &[],
        "composes",
        "d17"
    ),
    edge!(
        "alpha-sample",
        "present.draw.alpha",
        Bottom,
        "present.draw.submit",
        Top,
        &[],
        "coverage",
        "d14"
    ),
    edge!(
        "messages",
        "present.patch",
        Right,
        "chassis.bridge",
        Left,
        &[],
        "patches",
        "e18"
    ),
    edge!(
        "native",
        "chassis.bridge",
        Bottom,
        "chassis.native",
        Top,
        &[],
        "updates",
        "e19"
    ),
    edge!(
        "native-mask",
        "chassis.native",
        Bottom,
        "chassis.masks",
        Top,
        &[],
        "clip / mask",
        "d15"
    ),
    edge!(
        "surfaces",
        "present.plan.tiles",
        Right,
        "chassis.surfaces",
        Left,
        &[(2634., 306.), (2634., 1591.)],
        "surface plan",
        "d06"
    ),
    edge!(
        "present",
        "present.draw.submit",
        Right,
        "present.compose",
        Left,
        &[],
        "presents",
        "d18"
    ),
    edge!(
        "host-surfaces",
        "chassis.surfaces",
        Bottom,
        "present.compose",
        Top,
        &[],
        "surfaces",
        "e20"
    ),
    edge!(
        "native-compose",
        "chassis.masks",
        Right,
        "present.compose",
        Right,
        &[(3041., 1049.), (3041., 1388.)],
        "native views",
        "d18"
    ),
    edge!(
        "clock",
        "feedback.clock",
        Left,
        "operate.drive",
        Right,
        &[(2596., 290.), (2596., 120.), (1702., 120.), (1702., 244.)],
        "tick / clock",
        "e24"
    ),
    edge!(
        "input",
        "feedback.input",
        Left,
        "operate.drive",
        Right,
        &[(2620., 488.), (2620., 96.), (1702., 96.), (1702., 244.)],
        "input / events",
        "e06"
    ),
    edge!(
        "measurement",
        "chassis.native",
        Left,
        "operate.geometry",
        Bottom,
        &[
            (2620., 895.),
            (2620., 864.),
            (1730., 864.),
            (1730., 1186.),
            (1150., 1186.)
        ],
        "measurements / scroll",
        "e23"
    ),
    edge!(
        "resources",
        "feedback.resource",
        Left,
        "present.draw.textures",
        Bottom,
        &[(2170., 1509.)],
        "decoded pixels",
        "d16"
    ),
];
