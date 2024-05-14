use std::ops::{Add, Div, Mul, Neg, Sub};

use crate::{PaxValue, ToFromPaxValue};

use super::{PaxAny, ToFromPaxAny};

const ANY_ARITH_UNSUPPORTED: &'static str =
    "types that are not representable as PaxValues are not supported in arithmetic expressions";
//----------------------------PaxValue Arithmetic-----------------------------
impl Add for PaxValue {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (PaxValue::Numeric(a), PaxValue::Numeric(b)) => (a + b).to_pax_value(),
            (PaxValue::String(a), PaxValue::String(b)) => (a + &b).to_pax_value(),
            (a, b) => panic!("can't add {:?} and {:?}", a, b),
        }
    }
}

impl Mul for PaxValue {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (PaxValue::Numeric(a), PaxValue::Numeric(b)) => (a * b).to_pax_value(),
            (a, b) => panic!("can't multiply {:?} and {:?}", a, b),
        }
    }
}

impl Sub for PaxValue {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (PaxValue::Numeric(a), PaxValue::Numeric(b)) => (a - b).to_pax_value(),
            (a, b) => panic!("can't subtract {:?} and {:?}", a, b),
        }
    }
}

impl Div for PaxValue {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (PaxValue::Numeric(a), PaxValue::Numeric(b)) => (a / b).to_pax_value(),
            (a, b) => panic!("can't subtract {:?} and {:?}", a, b),
        }
    }
}

impl Neg for PaxValue {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            PaxValue::Numeric(a) => (-a).to_pax_value(),
            a => panic!("can't negate {:?}", a),
        }
    }
}

impl PartialEq for PaxValue {
    fn eq(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (PaxValue::Numeric(a), PaxValue::Numeric(b)) => a == b,
            (a, b) => panic!("can't multiply {:?} and {:?}", a, b),
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
