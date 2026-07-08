use pax_kit::*;

const SIGNATURE_WIDTH: f64 = 640.0;
const SIGNATURE_HEIGHT: f64 = 160.0;
const CELLAR_DOOR_WIDTH: f64 = 1390.81;
const CELLAR_DOOR_HEIGHT: f64 = 195.24;

#[pax]
#[custom(Default)]
#[file("svg_signature_fixture.pax")]
pub struct SvgSignatureFixture {
    pub draw_start: Property<UnitValue>,
    pub draw_end: Property<UnitValue>,
    pub elements: Property<Vec<PathElement>>,
}

impl Default for SvgSignatureFixture {
    fn default() -> Self {
        Self {
            draw_start: Property::new(unit(0.0)),
            draw_end: Property::new(unit(1.0)),
            elements: Property::new(signature_elements()),
        }
    }
}

#[pax]
#[custom(Default)]
#[file("svg_cellar_door_fixture.pax")]
pub struct SvgCellarDoorFixture {
    pub draw_start: Property<UnitValue>,
    pub draw_end: Property<UnitValue>,
    pub elements: Property<Vec<PathElement>>,
}

impl Default for SvgCellarDoorFixture {
    fn default() -> Self {
        Self {
            draw_start: Property::new(unit(0.0)),
            draw_end: Property::new(unit(1.0)),
            elements: Property::new(cellar_door_elements()),
        }
    }
}

fn signature_elements() -> Vec<PathElement> {
    vec![
        p(18.0, 102.0, SIGNATURE_WIDTH, SIGNATURE_HEIGHT),
        c(72.0, 40.0, 135.0, 132.0, SIGNATURE_WIDTH, SIGNATURE_HEIGHT),
        p(186.0, 84.0, SIGNATURE_WIDTH, SIGNATURE_HEIGHT),
        c(222.0, 50.0, 278.0, 54.0, SIGNATURE_WIDTH, SIGNATURE_HEIGHT),
        p(316.0, 92.0, SIGNATURE_WIDTH, SIGNATURE_HEIGHT),
        c(
            345.0,
            122.0,
            392.0,
            130.0,
            SIGNATURE_WIDTH,
            SIGNATURE_HEIGHT,
        ),
        p(414.0, 82.0, SIGNATURE_WIDTH, SIGNATURE_HEIGHT),
        c(432.0, 43.0, 376.0, 42.0, SIGNATURE_WIDTH, SIGNATURE_HEIGHT),
        p(382.0, 78.0, SIGNATURE_WIDTH, SIGNATURE_HEIGHT),
        c(
            390.0,
            130.0,
            490.0,
            120.0,
            SIGNATURE_WIDTH,
            SIGNATURE_HEIGHT,
        ),
        p(546.0, 68.0, SIGNATURE_WIDTH, SIGNATURE_HEIGHT),
        c(578.0, 40.0, 610.0, 52.0, SIGNATURE_WIDTH, SIGNATURE_HEIGHT),
        p(624.0, 82.0, SIGNATURE_WIDTH, SIGNATURE_HEIGHT),
        p(118.0, 126.0, SIGNATURE_WIDTH, SIGNATURE_HEIGHT),
        c(
            158.0,
            104.0,
            194.0,
            102.0,
            SIGNATURE_WIDTH,
            SIGNATURE_HEIGHT,
        ),
        p(235.0, 122.0, SIGNATURE_WIDTH, SIGNATURE_HEIGHT),
    ]
}

fn cellar_door_elements() -> Vec<PathElement> {
    vec![
        cd_p(180.89, 57.41),
        PathElement::Line,
        cd_p(128.82, 70.3),
        cd_c(121.34, 53.8, 112.32, 46.33),
        cd_p(95.57, 46.33),
        cd_c(73.14, 46.33, 58.71, 63.86),
        cd_p(58.71, 97.11),
        cd_c(58.71, 130.36, 72.89, 148.92),
        cd_p(96.6, 148.92),
        cd_c(115.93, 148.92, 125.99, 139.38),
        cd_p(133.72, 120.82),
        PathElement::Line,
        cd_p(184.76, 135.77),
        cd_c(169.55, 172.38, 136.04, 192.74),
        cd_p(94.54, 192.74),
        cd_c(44.27, 192.74, 2.51, 162.84),
        cd_p(2.51, 97.36),
        cd_c(2.51, 31.88, 46.58, 2.5),
        cd_p(95.04, 2.5),
        cd_c(132.68, 2.5, 164.9, 17.71),
        cd_p(180.88, 57.41),
        PathElement::Close,
        cd_p(333.75, 122.37),
        PathElement::Line,
        cd_p(333.75, 131.13),
        PathElement::Line,
        cd_p(240.17, 131.13),
        cd_c(241.72, 146.85, 252.54, 153.81),
        cd_p(266.46, 153.81),
        cd_c(279.35, 153.81, 286.57, 147.62),
        cd_p(292.5, 138.6),
        PathElement::Line,
        cd_p(332.71, 157.16),
        cd_c(318.27, 178.81, 296.11, 191.45),
        cd_p(263.37, 191.45),
        cd_c(216.97, 191.45, 187.07, 161.29),
        cd_p(187.07, 118.24),
        cd_c(187.07, 75.19, 214.91, 44.26),
        cd_p(260.28, 44.26),
        cd_c(305.65, 44.26, 333.75, 72.36),
        cd_p(333.75, 122.37),
        PathElement::Close,
        cd_p(239.92, 102.26),
        PathElement::Line,
        cd_p(281.68, 102.26),
        cd_c(280.91, 84.99, 272.92, 78.8),
        cd_p(260.28, 78.8),
        cd_c(250.23, 78.8, 240.69, 84.47),
        cd_p(239.92, 102.26),
        PathElement::Close,
        cd_p(399.22, 187.85),
        PathElement::Line,
        cd_p(347.66, 187.85),
        PathElement::Line,
        cd_p(347.66, 7.4),
        PathElement::Line,
        cd_p(399.22, 7.4),
        PathElement::Line,
        cd_p(399.22, 187.85),
        PathElement::Close,
        cd_p(471.4, 187.85),
        PathElement::Line,
        cd_p(419.84, 187.85),
        PathElement::Line,
        cd_p(419.84, 7.4),
        PathElement::Line,
        cd_p(471.4, 7.4),
        PathElement::Line,
        cd_p(471.4, 187.85),
        PathElement::Close,
        cd_p(620.4, 88.34),
        PathElement::Line,
        cd_p(620.4, 156.39),
        cd_c(620.4, 168.76, 620.92, 177.79),
        cd_p(625.04, 187.84),
        PathElement::Line,
        cd_p(575.55, 187.84),
        cd_c(572.97, 183.97, 571.17, 179.33),
        cd_p(570.14, 174.44),
        cd_c(561.12, 186.81, 548.49, 191.45),
        cd_p(531.21, 191.45),
        cd_c(502.08, 191.45, 485.58, 173.66),
        cd_p(485.58, 148.66),
        cd_c(485.58, 121.08, 501.3, 105.35),
        cd_p(546.42, 99.68),
        PathElement::Line,
        cd_p(568.85, 96.84),
        PathElement::Line,
        cd_p(568.85, 91.43),
        cd_c(568.85, 81.89, 566.27, 78.02),
        cd_p(556.22, 78.02),
        cd_c(546.17, 78.02, 542.56, 82.14),
        cd_p(542.3, 90.91),
        PathElement::Line,
        cd_p(491.0, 90.91),
        cd_c(492.03, 61.01, 515.23, 44.25),
        cd_p(556.99, 44.25),
        cd_c(602.36, 44.25, 620.4, 60.23),
        cd_p(620.4, 88.33),
        PathElement::Close,
        cd_p(557.5, 127.78),
        cd_c(542.81, 130.1, 537.65, 135.0),
        cd_p(537.65, 142.73),
        cd_c(537.65, 150.46, 542.55, 154.07),
        cd_p(549.77, 154.07),
        cd_c(561.11, 154.07, 568.85, 146.34),
        cd_p(568.85, 135.77),
        PathElement::Line,
        cd_p(568.85, 125.98),
        PathElement::Line,
        cd_p(557.51, 127.78),
        PathElement::Close,
        cd_p(736.91, 45.03),
        PathElement::Line,
        cd_p(736.91, 93.24),
        cd_c(731.5, 92.98, 729.95, 92.98),
        cd_p(728.15, 92.98),
        cd_c(707.79, 92.98, 691.29, 100.71),
        cd_p(691.29, 138.35),
        PathElement::Line,
        cd_p(691.29, 187.84),
        PathElement::Line,
        cd_p(639.73, 187.84),
        PathElement::Line,
        cd_p(639.73, 48.13),
        PathElement::Line,
        cd_p(688.71, 48.13),
        PathElement::Line,
        cd_p(688.71, 70.56),
        cd_c(698.51, 50.45, 711.14, 44.27),
        cd_p(725.57, 44.27),
        cd_c(729.69, 44.27, 732.79, 44.53),
        cd_p(736.91, 45.04),
        PathElement::Close,
        cd_p(939.52, 187.85),
        PathElement::Line,
        cd_p(890.54, 187.85),
        PathElement::Line,
        cd_p(890.54, 173.93),
        cd_c(881.0, 185.27, 867.34, 191.46),
        cd_p(850.58, 191.46),
        cd_c(813.98, 191.46, 789.74, 160.53),
        cd_p(789.74, 117.73),
        cd_c(789.74, 71.85, 819.64, 44.26),
        cd_p(852.9, 44.26),
        cd_c(867.85, 44.26, 879.45, 48.9),
        cd_p(887.96, 56.38),
        PathElement::Line,
        cd_p(887.96, 7.4),
        PathElement::Line,
        cd_p(939.52, 7.4),
        PathElement::Line,
        cd_p(939.52, 187.85),
        PathElement::Close,
        cd_p(844.14, 118.24),
        cd_c(844.14, 141.44, 853.16, 152.01),
        cd_p(868.11, 152.01),
        cd_c(883.06, 152.01, 891.83, 141.18),
        cd_p(891.83, 118.24),
        cd_c(891.83, 94.01, 882.81, 83.18),
        cd_p(868.11, 83.18),
        cd_c(852.39, 83.18, 844.14, 94.52),
        cd_p(844.14, 118.24),
        PathElement::Close,
        cd_p(1111.46, 117.73),
        cd_c(1111.46, 162.33, 1080.53, 191.46),
        cd_p(1032.58, 191.46),
        cd_c(984.63, 191.46, 953.7, 161.3),
        cd_p(953.7, 117.73),
        cd_c(953.7, 74.16, 983.09, 44.26),
        cd_p(1032.58, 44.26),
        cd_c(1082.07, 44.26, 1111.46, 75.71),
        cd_p(1111.46, 117.73),
        PathElement::Close,
        cd_p(1008.09, 117.21),
        cd_c(1008.09, 141.44, 1017.11, 152.27),
        cd_p(1032.58, 152.27),
        cd_c(1048.05, 152.27, 1057.07, 141.44),
        cd_p(1057.07, 117.21),
        cd_c(1057.07, 92.98, 1047.53, 83.44),
        cd_p(1032.58, 83.44),
        cd_c(1017.63, 83.44, 1008.09, 92.46),
        cd_p(1008.09, 117.21),
        PathElement::Close,
        cd_p(1276.95, 117.73),
        cd_c(1276.95, 162.33, 1246.02, 191.46),
        cd_p(1198.07, 191.46),
        cd_c(1150.12, 191.46, 1119.19, 161.3),
        cd_p(1119.19, 117.73),
        cd_c(1119.19, 74.16, 1148.58, 44.26),
        cd_p(1198.07, 44.26),
        cd_c(1247.56, 44.26, 1276.95, 75.71),
        cd_p(1276.95, 117.73),
        PathElement::Close,
        cd_p(1173.58, 117.21),
        cd_c(1173.58, 141.44, 1182.6, 152.27),
        cd_p(1198.07, 152.27),
        cd_c(1213.54, 152.27, 1222.56, 141.44),
        cd_p(1222.56, 117.21),
        cd_c(1222.56, 92.98, 1213.02, 83.44),
        cd_p(1198.07, 83.44),
        cd_c(1183.12, 83.44, 1173.58, 92.46),
        cd_p(1173.58, 117.21),
        PathElement::Close,
        cd_p(1388.31, 45.03),
        PathElement::Line,
        cd_p(1388.31, 93.24),
        cd_c(1382.9, 92.98, 1381.35, 92.98),
        cd_p(1379.55, 92.98),
        cd_c(1359.19, 92.98, 1342.69, 100.71),
        cd_p(1342.69, 138.35),
        PathElement::Line,
        cd_p(1342.69, 187.84),
        PathElement::Line,
        cd_p(1291.13, 187.84),
        PathElement::Line,
        cd_p(1291.13, 48.13),
        PathElement::Line,
        cd_p(1340.11, 48.13),
        PathElement::Line,
        cd_p(1340.11, 70.56),
        cd_c(1349.91, 50.45, 1362.54, 44.27),
        cd_p(1376.97, 44.27),
        cd_c(1381.09, 44.27, 1384.19, 44.53),
        cd_p(1388.31, 45.04),
        PathElement::Close,
    ]
}

fn unit(value: f64) -> UnitValue {
    UnitValue::Unitless(Numeric::F64(value))
}

fn cd_p(x: f64, y: f64) -> PathElement {
    p(x, y, CELLAR_DOOR_WIDTH, CELLAR_DOOR_HEIGHT)
}

fn cd_c(x1: f64, y1: f64, x2: f64, y2: f64) -> PathElement {
    c(x1, y1, x2, y2, CELLAR_DOOR_WIDTH, CELLAR_DOOR_HEIGHT)
}

fn p(x: f64, y: f64, width: f64, height: f64) -> PathElement {
    PathElement::Point(pct(x, width), pct(y, height))
}

fn c(x1: f64, y1: f64, x2: f64, y2: f64, width: f64, height: f64) -> PathElement {
    PathElement::Cubic(
        pct(x1, width),
        pct(y1, height),
        pct(x2, width),
        pct(y2, height),
    )
}

fn pct(value: f64, extent: f64) -> Size {
    Size::Percent(Numeric::F64(value / extent * 100.0))
}
