use std::{
    cmp::Ordering,
    ops::{Add, Div, Mul, Neg, Not, Rem, Sub},
};

use crate::{Numeric, PaxValue, Percent, Size, ToPaxValue};

use super::{PaxAny, ToFromPaxAny};

const ANY_ARITH_UNSUPPORTED: &'static str =
    "types that are not representable as PaxValues are not supported in arithmetic expressions";
//----------------------------PaxValue Arithmetic-----------------------------
impl Add for PaxValue {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            // Basic types
            (PaxValue::Numeric(a), PaxValue::Numeric(b)) => (a + b).to_pax_value(),
            (PaxValue::String(a), PaxValue::String(b)) => (a + &b).to_pax_value(),
            (PaxValue::String(a), PaxValue::Numeric(b)) => (a + &b.to_string()).to_pax_value(),
            (PaxValue::Numeric(a), PaxValue::String(b)) => (a.to_string() + &b).to_pax_value(),

            // Size and Percent
            (PaxValue::Size(a), PaxValue::Size(b)) => (a + b).to_pax_value(),
            (PaxValue::Percent(a), PaxValue::Percent(b)) => (Percent(a.0 + b.0)).to_pax_value(),
            (PaxValue::Percent(a), PaxValue::Size(b))
            | (PaxValue::Size(b), PaxValue::Percent(a)) => (Size::Percent(a.0) + b).to_pax_value(),
            (PaxValue::Bool(a), PaxValue::Numeric(b))
            | (PaxValue::Numeric(b), PaxValue::Bool(a)) => {
                (Numeric::I64(a as i64) + b).to_pax_value()
            }
            (a, b) => panic!("can't add {:?} and {:?}", a, b),
        }
    }
}

impl Mul for PaxValue {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (PaxValue::Numeric(a), PaxValue::Numeric(b)) => (a * b).to_pax_value(),
            (PaxValue::Bool(a), PaxValue::Numeric(b))
            | (PaxValue::Numeric(b), PaxValue::Bool(a)) => {
                (Numeric::I64(a as i64) * b).to_pax_value()
            }
            (a, b) => panic!("can't multiply {:?} and {:?}", a, b),
        }
    }
}

impl Sub for PaxValue {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (PaxValue::Numeric(a), PaxValue::Numeric(b)) => (a - b).to_pax_value(),
            // Size and Percent
            (PaxValue::Size(a), PaxValue::Size(b)) => (a - b).to_pax_value(),
            (PaxValue::Percent(a), PaxValue::Percent(b)) => (Percent(a.0 - b.0)).to_pax_value(),
            (PaxValue::Percent(a), PaxValue::Size(b)) => (Size::Percent(a.0) - b).to_pax_value(),
            (PaxValue::Size(a), PaxValue::Percent(b)) => (a - Size::Percent(b.0)).to_pax_value(),
            (a, b) => panic!("can't subtract {:?} and {:?}", a, b),
        }
    }
}

impl Div for PaxValue {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (PaxValue::Numeric(a), PaxValue::Numeric(b)) => (a / b).to_pax_value(),
            (a, b) => panic!("can't divide {:?} and {:?}", a, b),
        }
    }
}

impl Neg for PaxValue {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            PaxValue::Numeric(a) => (-a).to_pax_value(),
            PaxValue::Size(a) => (-a).to_pax_value(),
            PaxValue::Percent(p) => (-p.0).to_pax_value(),
            a => panic!("can't negate {:?}", a),
        }
    }
}

impl PartialEq for PaxValue {
    fn eq(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (PaxValue::Bool(a), PaxValue::Bool(b)) => a == b,
            (PaxValue::Numeric(a), PaxValue::Numeric(b)) => a == b,
            (PaxValue::String(a), PaxValue::String(b)) => a == b,
            (PaxValue::Size(a), PaxValue::Size(b)) => a == b,
            (PaxValue::Percent(a), PaxValue::Percent(b)) => a == b,
            (PaxValue::Color(a), PaxValue::Color(b)) => a == b,
            (PaxValue::Rotation(a), PaxValue::Rotation(b)) => a == b,
            (PaxValue::Option(a), PaxValue::Option(b)) => a == b,
            (PaxValue::Vec(a), PaxValue::Vec(b)) => a == b,
            (PaxValue::Object(a), PaxValue::Object(b)) => a == b,
            (PaxValue::Enum(a, b), PaxValue::Enum(c, d)) => a == c && b == d,
            (PaxValue::Range(a, b), PaxValue::Range(c, d)) => a == c && b == d,
            (a, b) => panic!("can't compare {:?} and {:?}", a, b),
        }
    }
}

impl PartialOrd for PaxValue {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        match (self, rhs) {
            (PaxValue::Bool(a), PaxValue::Bool(b)) => a.partial_cmp(b),
            (PaxValue::Numeric(a), PaxValue::Numeric(b)) => a.partial_cmp(b),
            (PaxValue::String(a), PaxValue::String(b)) => a.partial_cmp(b),
            (a, b) => panic!("can't compare {:?} and {:?}", a, b),
        }
    }
}

impl Rem for PaxValue {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (PaxValue::Numeric(a), PaxValue::Numeric(b)) => (a % b).to_pax_value(),
            (a, b) => panic!("can't get remainder of {:?} and {:?}", a, b),
        }
    }
}

impl Not for PaxValue {
    type Output = PaxValue;

    fn not(self) -> Self::Output {
        match self {
            PaxValue::Bool(v) => PaxValue::Bool(!v),
            v => panic!("! operator not valid for {:?}", v),
        }
    }
}

impl PaxValue {
    // the operator && currently can't be overloaded, we use this function instead
    // NOTE: this does not short circuit in the normal way that && does
    pub fn op_and(self, rhs: Self) -> Self {
        match (self, rhs) {
            (PaxValue::Bool(a), PaxValue::Bool(b)) => PaxValue::Bool(a && b),
            (a, b) => panic!("&& operator not valid for {:?} and {:?}", a, b),
        }
    }
    // the operator || currently can't be overloaded, we use this function instead
    // NOTE: this does not short circuit in the normal way that && does
    pub fn op_or(self, rhs: Self) -> Self {
        match (self, rhs) {
            (PaxValue::Bool(a), PaxValue::Bool(b)) => PaxValue::Bool(a || b),
            (a, b) => panic!("&& operator not valid for {:?} and {:?}", a, b),
        }
    }

    pub fn op_not(self) -> Self {
        match self {
            PaxValue::Bool(v) => PaxValue::Bool(!v),
            v => panic!("! operator not valid for {:?}", v),
        }
    }

    // exponentiation, self ^ exp
    pub fn pow(self, exp: Self) -> Self {
        match (self, exp) {
            (PaxValue::Numeric(a), PaxValue::Numeric(b)) => a.pow(b).to_pax_value(),
            (a, b) => panic!("exponentiation not valid between {:?} and {:?}", a, b),
        }
    }

    pub fn min(self, rhs: Self) -> Self {
        match (self, rhs) {
            (PaxValue::Numeric(a), PaxValue::Numeric(b)) => a.min(b).to_pax_value(),
            (a, b) => panic!("min not valid between {:?} and {:?}", a, b),
        }
    }

    pub fn max(self, rhs: Self) -> Self {
        match (self, rhs) {
            (PaxValue::Numeric(a), PaxValue::Numeric(b)) => a.max(b).to_pax_value(),
            (a, b) => panic!("max not valid between {:?} and {:?}", a, b),
        }
    }
}

//----------------------------PaxAny Arithmetic-----------------------------
impl Add for PaxAny {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (PaxAny::Builtin(a), PaxAny::Builtin(b)) => (a + b).to_pax_any(),
            _ => panic!("{}", ANY_ARITH_UNSUPPORTED),
        }
    }
}

impl Mul for PaxAny {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (PaxAny::Builtin(a), PaxAny::Builtin(b)) => (a * b).to_pax_any(),
            _ => panic!("{}", ANY_ARITH_UNSUPPORTED),
        }
    }
}

impl Sub for PaxAny {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (PaxAny::Builtin(a), PaxAny::Builtin(b)) => (a - b).to_pax_any(),
            _ => panic!("{}", ANY_ARITH_UNSUPPORTED),
        }
    }
}

impl Div for PaxAny {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (PaxAny::Builtin(a), PaxAny::Builtin(b)) => (a / b).to_pax_any(),
            _ => panic!("{}", ANY_ARITH_UNSUPPORTED),
        }
    }
}

impl Rem for PaxAny {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (PaxAny::Builtin(a), PaxAny::Builtin(b)) => (a % b).to_pax_any(),
            _ => panic!("{}", ANY_ARITH_UNSUPPORTED),
        }
    }
}

impl Neg for PaxAny {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            PaxAny::Builtin(a) => (-a).to_pax_any(),
            _ => panic!("{}", ANY_ARITH_UNSUPPORTED),
        }
    }
}

impl PartialEq for PaxAny {
    fn eq(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (PaxAny::Builtin(a), PaxAny::Builtin(b)) => a == b,
            _ => panic!("{}", ANY_ARITH_UNSUPPORTED),
        }
    }
}

impl PartialOrd for PaxAny {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        match (self, rhs) {
            (PaxAny::Builtin(a), PaxAny::Builtin(b)) => a.partial_cmp(b),
            _ => panic!("{}", ANY_ARITH_UNSUPPORTED),
        }
    }
}

impl Not for PaxAny {
    type Output = PaxAny;

    fn not(self) -> Self::Output {
        match self {
            PaxAny::Builtin(v) => (!v).to_pax_any(),
            _ => panic!("{}", ANY_ARITH_UNSUPPORTED),
        }
    }
}

impl PaxAny {
    // the operator && currently can't be overloaded, we use this function instead
    // NOTE: this does not short circuit in the normal way that && does
    pub fn op_and(self, rhs: Self) -> Self {
        match (self, rhs) {
            (PaxAny::Builtin(a), PaxAny::Builtin(b)) => a.op_and(b).to_pax_any(),
            _ => panic!("{}", ANY_ARITH_UNSUPPORTED),
        }
    }
    // the operator || currently can't be overloaded, we use this function instead
    // NOTE: this does not short circuit in the normal way that && does
    pub fn op_or(self, rhs: Self) -> Self {
        match (self, rhs) {
            (PaxAny::Builtin(a), PaxAny::Builtin(b)) => a.op_or(b).to_pax_any(),
            _ => panic!("{}", ANY_ARITH_UNSUPPORTED),
        }
    }

    // exponentiation, self ^ exp
    pub fn pow(self, exp: Self) -> Self {
        match (self, exp) {
            (PaxAny::Builtin(a), PaxAny::Builtin(b)) => a.pow(b).to_pax_any(),
            _ => panic!("{}", ANY_ARITH_UNSUPPORTED),
        }
    }
}
