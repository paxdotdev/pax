use serde::{Deserialize, Serialize};

use crate::Interpolatable;
#[derive(Clone, Serialize, Deserialize)]
#[serde(crate = "crate::serde")]
#[derive(Debug, Copy)]
pub enum Numeric {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F64(f64),
    F32(f32),
    ISize(isize),
    USize(usize),
}

impl Default for Numeric {
    fn default() -> Self {
        Self::F64(0.0)
    }
}

macro_rules! impl_numeric_arith {
    ($trait:ident, $method:ident, $op:tt) => {
        impl std::ops::$trait for &Numeric {
            type Output = Numeric;

            fn $method(self, rhs: Self) -> Self::Output {

                // TBD: might want to be more granular here at some point
                match (self.is_float(), rhs.is_float()) {
                    (false, false) => Numeric::I64(self.to_int() $op rhs.to_int()),
                    _ => Numeric::F64(self.to_float() $op rhs.to_float()),
                }
            }
        }
        impl std::ops::$trait for Numeric {
            type Output = Numeric;

            fn $method(self, rhs: Self) -> Self::Output {
                &self $op &rhs
            }
        }
    };
}

impl_numeric_arith!(Add, add, +);
impl_numeric_arith!(Sub, sub, -);
impl_numeric_arith!(Mul, mul, *);
impl_numeric_arith!(Div, div, /);

impl std::ops::Neg for Numeric {
    type Output = Self;

    fn neg(self) -> Self::Output {
        use Numeric::*;
        match self {
            I8(a) => I8(-a),
            I16(a) => I16(-a),
            I32(a) => I32(-a),
            I64(a) => I64(-a),
            F32(a) => F32(-a),
            F64(a) => F64(-a),
            ISize(a) => ISize(-a),
            _ => panic!("tried to negate numeric that is unsigned"),
        }
    }
}

macro_rules! impl_to_from {
    ($return_type:ty, $Variant:path) => {
        impl From<&Numeric> for $return_type {
            fn from(value: &Numeric) -> Self {
                use Numeric::*;
                match *value {
                    I8(a) => a as $return_type,
                    I16(a) => a as $return_type,
                    I32(a) => a as $return_type,
                    I64(a) => a as $return_type,
                    U8(a) => a as $return_type,
                    U16(a) => a as $return_type,
                    U32(a) => a as $return_type,
                    U64(a) => a as $return_type,
                    F32(a) => a as $return_type,
                    F64(a) => a as $return_type,
                    ISize(a) => a as $return_type,
                    USize(a) => a as $return_type,
                }
            }
        }

        impl From<Numeric> for $return_type {
            fn from(value: Numeric) -> Self {
                (&value).into()
            }
        }

        impl From<$return_type> for Numeric {
            fn from(value: $return_type) -> Self {
                $Variant(value)
            }
        }
        impl From<&$return_type> for Numeric {
            fn from(value: &$return_type) -> Self {
                $Variant(*value)
            }
        }
    };
}

impl_to_from!(f32, Numeric::F32);
impl_to_from!(f64, Numeric::F64);
impl_to_from!(i8, Numeric::I8);
impl_to_from!(i16, Numeric::I16);
impl_to_from!(i32, Numeric::I32);
impl_to_from!(i64, Numeric::I64);
impl_to_from!(u8, Numeric::U8);
impl_to_from!(u16, Numeric::U16);
impl_to_from!(u32, Numeric::U32);
impl_to_from!(u64, Numeric::U64);
impl_to_from!(isize, Numeric::ISize);
impl_to_from!(usize, Numeric::USize);

impl Numeric {
    pub fn to_float(&self) -> f64 {
        self.into()
    }

    pub fn to_int(&self) -> i64 {
        self.into()
    }

    pub fn is_float(&self) -> bool {
        match self {
            Numeric::F64(_) | Numeric::F32(_) => true,
            _ => false,
        }
    }
}

impl Interpolatable for Numeric {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Numeric::F64(Into::<f64>::into(self).interpolate(&other.into(), t))
    }
}
