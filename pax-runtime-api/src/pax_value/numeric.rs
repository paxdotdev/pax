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
    I128(i128),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
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
        impl std::ops::$trait for Numeric {
            type Output = Self;

            fn $method(self, rhs: Self) -> Self::Output {
                use Numeric::*;
                match (self, rhs) {
                    (I8(a), I8(b)) => I8(a $op b),
                    (I16(a), I16(b)) => I16(a $op b),
                    (I32(a), I32(b)) => I32(a $op b),
                    (I64(a), I64(b)) => I64(a $op b),
                    (U8(a), U8(b)) => U8(a $op b),
                    (U16(a), U16(b)) => U16(a $op b),
                    (U32(a), U32(b)) => U32(a $op b),
                    (U64(a), U64(b)) => U64(a $op b),
                    (F32(a), F32(b)) => F32(a $op b),
                    (F64(a), F64(b)) => F64(a $op b),
                    (ISize(a), ISize(b)) => ISize(a $op b),
                    (USize(a), USize(b)) => USize(a $op b),
                    _ => panic!("tried to perform operation between incompatible Numeric types"),
                }
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
                    I128(a) => a as $return_type,
                    U8(a) => a as $return_type,
                    U16(a) => a as $return_type,
                    U32(a) => a as $return_type,
                    U64(a) => a as $return_type,
                    U128(a) => a as $return_type,
                    F32(a) => a as $return_type,
                    F64(a) => a as $return_type,
                    ISize(a) => a as $return_type,
                    USize(a) => a as $return_type,
                }
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

impl_to_from!(f64, Numeric::F64);
impl_to_from!(i32, Numeric::I32);
impl_to_from!(isize, Numeric::ISize);
impl_to_from!(usize, Numeric::USize);

impl Numeric {
    pub fn to_float(&self) -> f64 {
        self.into()
    }

    pub fn to_int(&self) -> i32 {
        self.into()
    }
}

impl Interpolatable for Numeric {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Numeric::F64(Into::<f64>::into(self).interpolate(&other.into(), t))
    }
}
