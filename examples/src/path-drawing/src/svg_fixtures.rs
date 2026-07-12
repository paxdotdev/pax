use pax_kit::*;

#[pax]
#[custom(Default)]
#[svg("assets/svg/signature-strokes.svg")]
pub struct SvgSignatureFixture {
    pub draw_start: Property<UnitValue>,
    pub draw_end: Property<UnitValue>,
}

impl Default for SvgSignatureFixture {
    fn default() -> Self {
        Self {
            draw_start: Property::new(unit(0.0)),
            draw_end: Property::new(unit(1.0)),
        }
    }
}

#[pax]
#[custom(Default)]
#[svg("assets/svg/cellar-door.svg")]
pub struct SvgCellarDoorFixture {
    pub draw_start: Property<UnitValue>,
    pub draw_end: Property<UnitValue>,
}

impl Default for SvgCellarDoorFixture {
    fn default() -> Self {
        Self {
            draw_start: Property::new(unit(0.0)),
            draw_end: Property::new(unit(1.0)),
        }
    }
}

fn unit(value: f64) -> UnitValue {
    UnitValue::Unitless(Numeric::F64(value))
}
