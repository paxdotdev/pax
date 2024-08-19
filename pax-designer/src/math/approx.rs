use pax_engine::api::{Numeric, Rotation};
use pax_std::Size;

const EPS: f64 = 1e-2;

pub trait ApproxEq {
    fn approx_eq(&self, other: &Self) -> bool;
}

impl ApproxEq for Size {
    fn approx_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Size::Pixels(a), Size::Pixels(b)) => a.approx_eq(b),
            (Size::Percent(a), Size::Percent(b)) => a.approx_eq(b),
            (Size::Combined(a1, a2), Size::Combined(b1, b2)) => {
                a1.approx_eq(b1) && a2.approx_eq(b2)
            }
            _ => false,
        }
    }
}

impl ApproxEq for Rotation {
    fn approx_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Rotation::Radians(a), Rotation::Radians(b)) => a.approx_eq(b),
            (Rotation::Degrees(a), Rotation::Degrees(b)) => a.approx_eq(b),
            (Rotation::Percent(a), Rotation::Percent(b)) => a.approx_eq(b),
            _ => false,
        }
    }
}

impl ApproxEq for Numeric {
    fn approx_eq(&self, other: &Self) -> bool {
        (self.to_float() - other.to_float()).abs() < EPS
    }
}

impl<T: ApproxEq> ApproxEq for Option<&T> {
    fn approx_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (None, None) => true,
            (Some(a), Some(b)) => a.approx_eq(b),
            _ => false,
        }
    }
}
