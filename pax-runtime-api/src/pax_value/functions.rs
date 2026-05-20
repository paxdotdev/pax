use crate::{
    math::{Transform2, Vector2},
    Color, ColorChannel, Fill, PaxValue, Rotation,
};
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use super::{CoercionRules, ToPaxValue};

type FunctionType = Arc<dyn Fn(Vec<PaxValue>) -> Result<PaxValue, String> + Send + Sync>;

static FUNCTIONS: Lazy<Arc<RwLock<HashMap<String, HashMap<String, FunctionType>>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

// Dumps registered PAXEL helper functions for diagnostics.
pub fn print_all_functions() {
    let functions = FUNCTIONS.read().unwrap();
    log::warn!("Total scopes: {}", functions.len());
    for (scope, funcs) in functions.iter() {
        log::warn!("Scope: {}", scope);
        for (name, _) in funcs.iter() {
            log::warn!("  |{}|", name);
        }
    }
}

// Registers one PAXEL helper function under a scope/name pair.
pub fn register_function(scope: String, name: String, func: FunctionType) {
    let mut functions = FUNCTIONS.write().unwrap();
    functions
        .entry(scope)
        .or_insert_with(HashMap::new)
        .insert(name, func);
}

// Dispatches a registered PAXEL helper function.
pub fn call_function(scope: String, name: String, args: Vec<PaxValue>) -> Result<PaxValue, String> {
    let functions = FUNCTIONS.read().unwrap();
    let scope_funcs = functions
        .get(&scope)
        .ok_or_else(|| format!("Scope {} not found", scope))?;
    let func = scope_funcs
        .get(&name)
        .ok_or_else(|| format!("Function {} not found in scope {}", name, scope))?;
    func(args)
}

/// Helper function for adding two PaxValues
fn add(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function add".to_string());
    }
    let mut itr = args.into_iter();
    Ok(itr.next().unwrap() + itr.next().unwrap())
}

/// Helper function for subtracting two PaxValues
fn sub_or_neg(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() == 1 {
        Ok(-args.into_iter().next().unwrap())
    } else if args.len() == 2 {
        let mut itr = args.into_iter();
        Ok(itr.next().unwrap() - itr.next().unwrap())
    } else {
        Err("Expected 1 or 2 arguments for function sub_or_neg".to_string())
    }
}

/// Helper function for multiplying two PaxValues
fn mul(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function mul".to_string());
    }
    let mut itr = args.into_iter();
    Ok(itr.next().unwrap() * itr.next().unwrap())
}

/// Helper function for dividing two PaxValues
fn div(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function div".to_string());
    }
    let mut itr = args.into_iter();
    Ok(itr.next().unwrap() / itr.next().unwrap())
}

/// Helper function for exponentiating one PaxValue by another
fn exp(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function exp".to_string());
    }
    let mut itr = args.into_iter();
    Ok(itr.next().unwrap().pow(itr.next().unwrap()))
}

/// Helper function for taking the modulus of one PaxValue by another
fn mod_(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function mod_".to_string());
    }
    let mut itr = args.into_iter();
    Ok(itr.next().unwrap() % itr.next().unwrap())
}

/// Helper function for checking if two PaxValues are equal
fn rel_eq(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function rel_eq".to_string());
    }
    Ok(PaxValue::Bool(args[0] == args[1]))
}

/// Helper function for checking if one PaxValue is greater than another
fn rel_gt(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function rel_gt".to_string());
    }
    Ok(PaxValue::Bool(args[0] > args[1]))
}

/// Helper function for checking if one PaxValue is greater than or equal to another
fn rel_gte(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function rel_gte".to_string());
    }
    Ok(PaxValue::Bool(args[0] >= args[1]))
}

/// Helper function for checking if one PaxValue is less than another
fn rel_lt(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function rel_lt".to_string());
    }
    Ok(PaxValue::Bool(args[0] < args[1]))
}

/// Helper function for checking if one PaxValue is less than or equal to another
fn rel_lte(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function rel_lte".to_string());
    }
    Ok(PaxValue::Bool(args[0] <= args[1]))
}

/// Helper function for checking if two PaxValues are not equal
fn rel_neq(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function rel_neq".to_string());
    }
    Ok(PaxValue::Bool(args[0] != args[1]))
}

/// Helper function for performing a boolean AND operation on two PaxValues
fn bool_and(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function bool_and".to_string());
    }
    let mut itr = args.into_iter();
    Ok(itr.next().unwrap().op_and(itr.next().unwrap()))
}

/// Helper function for performing a boolean OR operation on two PaxValues
fn bool_or(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function bool_or".to_string());
    }
    let mut itr = args.into_iter();
    Ok(itr.next().unwrap().op_or(itr.next().unwrap()))
}

/// Helper function for performing a boolean NOT operation on a PaxValue
fn bool_not(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 1 {
        return Err("Expected 1 argument for function bool_not".to_string());
    }
    Ok(args.into_iter().next().unwrap().op_not())
}

/// Helper function for taking the minimum of two PaxValues
fn min(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function min".to_string());
    }
    let mut itr = args.into_iter();
    Ok(itr.next().unwrap().min(itr.next().unwrap()))
}

/// Helper function for taking the maximum of two PaxValues
fn max(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function max".to_string());
    }
    let mut itr = args.into_iter();
    Ok(itr.next().unwrap().max(itr.next().unwrap()))
}

/// Helper function for getting the length of a PaxValue vector
fn len(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 1 {
        return Err("len function takes a single argument".to_string());
    }
    match args.into_iter().next().unwrap() {
        PaxValue::Vec(vec) => Ok(vec.len().to_pax_value()),
        e => Err(format!("can't get length of {e:?}")),
    }
}

/// Helper function for creating an RGB color from three PaxValues representing the red, green, and blue channels
fn rgb(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 3 {
        return Err("Expected 3 arguments for function rgb".to_string());
    }
    let mut itr = args.into_iter();
    let r = ColorChannel::try_coerce(itr.next().unwrap())?;
    let g = ColorChannel::try_coerce(itr.next().unwrap())?;
    let b = ColorChannel::try_coerce(itr.next().unwrap())?;
    Ok(Color::rgb(r, g, b).to_pax_value())
}

/// Helper function for creating an RGBA color from four PaxValues representing the red, green, blue, and alpha channels
fn rgba(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 4 {
        return Err("Expected 4 arguments for function rgba".to_string());
    }
    let mut itr = args.into_iter();
    let r = ColorChannel::try_coerce(itr.next().unwrap())?;
    let g = ColorChannel::try_coerce(itr.next().unwrap())?;
    let b = ColorChannel::try_coerce(itr.next().unwrap())?;
    let a = ColorChannel::try_coerce(itr.next().unwrap())?;
    Ok(Color::rgba(r, g, b, a).to_pax_value())
}

/// Helper function for creating an HSL color from three PaxValues representing the hue, saturation, and lightness channels
fn hsl(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 3 {
        return Err("Expected 3 arguments for function hsl".to_string());
    }
    let mut itr = args.into_iter();
    let h = Rotation::try_coerce(itr.next().unwrap())?;
    let s = ColorChannel::try_coerce(itr.next().unwrap())?;
    let l = ColorChannel::try_coerce(itr.next().unwrap())?;
    Ok(Color::hsl(h, s, l).to_pax_value())
}

/// Helper function for creating an HSLA color from four PaxValues representing the hue, saturation, lightness, and alpha channels
fn hsla(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 4 {
        return Err("Expected 4 arguments for function hsla".to_string());
    }
    let mut itr = args.into_iter();
    let h = Rotation::try_coerce(itr.next().unwrap())?;
    let s = ColorChannel::try_coerce(itr.next().unwrap())?;
    let l = ColorChannel::try_coerce(itr.next().unwrap())?;
    let a = ColorChannel::try_coerce(itr.next().unwrap())?;
    Ok(Color::hsla(h, s, l, a).to_pax_value())
}

/// Helper function for creating a color from a hexadecimal string representation
fn hex(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 1 {
        return Err("Expected 1 argument for function hex".to_string());
    }
    let hex = String::try_coerce(args.into_iter().next().unwrap())?;
    Ok(Color::from_hex(&hex).to_pax_value())
}

/// Registers helper functions made available to PAXEL scopes.
pub trait HelperFunctions {
    /// Registers all helper functions for this type.
    fn register_all_functions() {}
}

/// Registry bootstrap for built-in PAXEL helper functions.
pub struct Functions;

impl Functions {
    /// Registers built-in math, color, and transform helper functions.
    pub fn register_all_functions() {
        // Math
        register_function("Math".to_string(), "+".to_string(), Arc::new(add));
        register_function("Math".to_string(), "-".to_string(), Arc::new(sub_or_neg));
        register_function("Math".to_string(), "*".to_string(), Arc::new(mul));
        register_function("Math".to_string(), "/".to_string(), Arc::new(div));
        register_function("Math".to_string(), "^".to_string(), Arc::new(exp));
        register_function("Math".to_string(), "%%".to_string(), Arc::new(mod_));
        register_function("Math".to_string(), "==".to_string(), Arc::new(rel_eq));
        register_function("Math".to_string(), ">".to_string(), Arc::new(rel_gt));
        register_function("Math".to_string(), ">=".to_string(), Arc::new(rel_gte));
        register_function("Math".to_string(), "<".to_string(), Arc::new(rel_lt));
        register_function("Math".to_string(), "<=".to_string(), Arc::new(rel_lte));
        register_function("Math".to_string(), "!=".to_string(), Arc::new(rel_neq));
        register_function("Math".to_string(), "&&".to_string(), Arc::new(bool_and));
        register_function("Math".to_string(), "||".to_string(), Arc::new(bool_or));
        register_function("Math".to_string(), "!".to_string(), Arc::new(bool_not));
        register_function("Math".to_string(), "min".to_string(), Arc::new(min));
        register_function("Math".to_string(), "max".to_string(), Arc::new(max));
        register_function("Math".to_string(), "len".to_string(), Arc::new(len));
        // Colors
        register_function("Color".to_string(), "rgb".to_string(), Arc::new(rgb));
        register_function("Color".to_string(), "rgba".to_string(), Arc::new(rgba));
        register_function("Color".to_string(), "hsl".to_string(), Arc::new(hsl));
        register_function("Color".to_string(), "hsla".to_string(), Arc::new(hsla));
        register_function("Color".to_string(), "#".to_string(), Arc::new(hex));
        // Transform2D
        crate::Transform2D::register_all_functions();
    }

    /// Returns true if a helper function exists in the named scope.
    pub fn has_function(scope: &str, name: &str) -> bool {
        let functions = FUNCTIONS.read().unwrap();
        if let Some(scope_funcs) = functions.get(scope) {
            scope_funcs.contains_key(name)
        } else {
            false
        }
    }
}

impl HelperFunctions for crate::Size {}

impl HelperFunctions for crate::Color {}

impl HelperFunctions for crate::Rotation {}

impl HelperFunctions for crate::Duration {}

impl HelperFunctions for String {}

impl HelperFunctions for crate::Numeric {}

impl HelperFunctions for crate::UnitValue {}

impl HelperFunctions for bool {}

impl HelperFunctions for Fill {}

impl HelperFunctions for crate::PaxValue {}

impl HelperFunctions for crate::ColorChannel {}

impl HelperFunctions for crate::Stroke {}

impl HelperFunctions for crate::PathSmoothing {}

impl HelperFunctions for crate::StrokeCap {}

impl HelperFunctions for crate::StrokeJoin {}

impl HelperFunctions for u8 {}
impl HelperFunctions for u16 {}
impl HelperFunctions for u32 {}
impl HelperFunctions for u64 {}
impl HelperFunctions for u128 {}
impl HelperFunctions for usize {}

impl HelperFunctions for i8 {}
impl HelperFunctions for i16 {}
impl HelperFunctions for i32 {}
impl HelperFunctions for i64 {}
impl HelperFunctions for i128 {}
impl HelperFunctions for isize {}

impl HelperFunctions for f32 {}
impl HelperFunctions for f64 {}
impl<T> HelperFunctions for Vec<T> {}

impl<T: HelperFunctions> HelperFunctions for Option<T> {}

impl HelperFunctions for crate::Transform2D {
    fn register_all_functions() {
        register_function(
            "Transform2D".to_string(),
            "scale".to_string(),
            Arc::new(|args| {
                if args.len() != 2 {
                    return Err("Expected 2 arguments for function scale".to_string());
                }
                let mut itr = args.into_iter();
                let x = crate::Size::try_coerce(itr.next().unwrap())?;
                let y = crate::Size::try_coerce(itr.next().unwrap())?;
                Ok(crate::Transform2D::scale(x, y).to_pax_value())
            }),
        );
        register_function(
            "Transform2D".to_string(),
            "rotate".to_string(),
            Arc::new(|args| {
                if args.len() != 1 {
                    return Err("Expected 1 argument for function rotate".to_string());
                }
                let z = crate::Rotation::try_coerce(args.into_iter().next().unwrap())?;
                Ok(crate::Transform2D::rotate(z).to_pax_value())
            }),
        );
        register_function(
            "Transform2D".to_string(),
            "translate".to_string(),
            Arc::new(|args| {
                if args.len() != 2 {
                    return Err("Expected 2 arguments for function translate".to_string());
                }
                let mut itr = args.into_iter();
                let x = crate::Size::try_coerce(itr.next().unwrap())?;
                let y = crate::Size::try_coerce(itr.next().unwrap())?;
                Ok(crate::Transform2D::translate(x, y).to_pax_value())
            }),
        );
        register_function(
            "Transform2D".to_string(),
            "anchor".to_string(),
            Arc::new(|args| {
                if args.len() != 2 {
                    return Err("Expected 2 arguments for function anchor".to_string());
                }
                let mut itr = args.into_iter();
                let x = crate::Size::try_coerce(itr.next().unwrap())?;
                let y = crate::Size::try_coerce(itr.next().unwrap())?;
                Ok(crate::Transform2D::anchor(x, y).to_pax_value())
            }),
        );
    }
}

impl HelperFunctions for Transform2 {
    fn register_all_functions() {
        register_function(
            "Transform2".to_string(),
            "identity".to_string(),
            Arc::new(|args| {
                if args.len() != 0 {
                    return Err("Expected 0 arguments for function identity".to_string());
                }
                Ok(Transform2::identity().to_pax_value())
            }),
        );
        register_function(
            "Transform2".to_string(),
            "scale".to_string(),
            Arc::new(|args| {
                if args.len() != 1 {
                    return Err("Expected 1 argument for function scale".to_string());
                }
                let s = f64::try_coerce(args.into_iter().next().unwrap())?;
                Ok(Transform2::scale(s).to_pax_value())
            }),
        );

        register_function(
            "Transform2".to_string(),
            "translate".to_string(),
            Arc::new(|args| {
                if args.len() != 1 {
                    return Err("Expected 1 argument for function scale".to_string());
                }
                let s = Vector2::try_coerce(args.into_iter().next().unwrap())?;
                Ok(Transform2::translate(s).to_pax_value())
            }),
        );

        register_function(
            "Transform2".to_string(),
            "rotate".to_string(),
            Arc::new(|args| {
                if args.len() != 1 {
                    return Err("Expected 1 argument for function rotate".to_string());
                }
                let s = f64::try_coerce(args.into_iter().next().unwrap())?;
                Ok(Transform2::rotate(s).to_pax_value())
            }),
        );
        register_function(
            "Transform2".to_string(),
            "skew".to_string(),
            Arc::new(|args| {
                if args.len() != 1 {
                    return Err("Expected 1 argument for function skew".to_string());
                }
                let s = Vector2::try_coerce(args.into_iter().next().unwrap())?;
                Ok(Transform2::skew(s).to_pax_value())
            }),
        );
    }
}
