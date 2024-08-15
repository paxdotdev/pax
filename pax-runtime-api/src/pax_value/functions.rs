use crate::{math::{Transform2, Vector2}, Color, ColorChannel, Fill, Numeric, PaxValue, Rotation};
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use super::{CoercionRules, ToPaxValue};

type FunctionType = Arc<dyn Fn(Vec<PaxValue>) -> Result<PaxValue, String> + Send + Sync>;

static FUNCTIONS: Lazy<Arc<RwLock<HashMap<String, HashMap<String, FunctionType>>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

pub fn print_all_functions() {
    let functions = FUNCTIONS.read().unwrap();
    println!("Total scopes: {}", functions.len());
    for (scope, funcs) in functions.iter() {
        println!("Scope: {}", scope);
        for (name, _) in funcs.iter() {
            println!("  |{}|", name);
        }
    }
}

pub fn register_function(scope: String, name: String, func: FunctionType) {
    let mut functions = FUNCTIONS.write().unwrap();
    functions
        .entry(scope)
        .or_insert_with(HashMap::new)
        .insert(name, func);
}

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

// Helper functions
fn add(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function add".to_string());
    }
    Ok(args[0].clone() + args[1].clone())
}

fn sub_or_neg(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() == 1 {
        Ok(-args[0].clone())
    } else if args.len() == 2 {
        Ok(args[0].clone() - args[1].clone())
    } else {
        Err("Expected 1 or 2 arguments for function sub_or_neg".to_string())
    }
}

fn mul(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function mul".to_string());
    }
    Ok(args[0].clone() * args[1].clone())
}

fn div(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function div".to_string());
    }
    Ok(args[0].clone() / args[1].clone())
}

fn exp(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function exp".to_string());
    }
    Ok(args[0].clone().pow(args[1].clone()))
}

fn mod_(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function mod_".to_string());
    }
    Ok(args[0].clone() % args[1].clone())
}

fn rel_eq(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function rel_eq".to_string());
    }
    Ok(PaxValue::Bool(args[0] == args[1]))
}

fn rel_gt(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function rel_gt".to_string());
    }
    Ok(PaxValue::Bool(args[0] > args[1]))
}

fn rel_gte(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function rel_gte".to_string());
    }
    Ok(PaxValue::Bool(args[0] >= args[1]))
}

fn rel_lt(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function rel_lt".to_string());
    }
    Ok(PaxValue::Bool(args[0] < args[1]))
}

fn rel_lte(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function rel_lte".to_string());
    }
    Ok(PaxValue::Bool(args[0] <= args[1]))
}

fn rel_neq(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function rel_neq".to_string());
    }
    Ok(PaxValue::Bool(args[0] != args[1]))
}

fn bool_and(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function bool_and".to_string());
    }
    Ok(args[0].clone().op_and(args[1].clone()))
}

fn bool_or(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function bool_or".to_string());
    }
    Ok(args[0].clone().op_or(args[1].clone()))
}

fn bool_not(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 1 {
        return Err("Expected 1 argument for function bool_not".to_string());
    }
    Ok(args[0].clone().op_not())
}

fn min(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function min".to_string());
    }
    Ok(args[0].clone().min(args[1].clone()))
}

fn max(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function max".to_string());
    }
    Ok(args[0].clone().max(args[1].clone()))
}

fn len(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 1 {
        return Err("len function takes a single argument".to_string());
    }
    match args.into_iter().next().unwrap() {
        PaxValue::Vec(vec) => Ok(vec.len().to_pax_value()),
        e => Err(format!("can't get length of {e:?}")),
    }
}

fn rgb(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 3 {
        return Err("Expected 3 arguments for function rgb".to_string());
    }
    let r = ColorChannel::try_coerce(args[0].clone())?;
    let g = ColorChannel::try_coerce(args[1].clone())?;
    let b = ColorChannel::try_coerce(args[2].clone())?;
    Ok(Color::rgb(r, g, b).to_pax_value())
}

fn rgba(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 4 {
        return Err("Expected 4 arguments for function rgba".to_string());
    }
    let r = ColorChannel::try_coerce(args[0].clone())?;
    let g = ColorChannel::try_coerce(args[1].clone())?;
    let b = ColorChannel::try_coerce(args[2].clone())?;
    let a = ColorChannel::try_coerce(args[3].clone())?;
    Ok(Color::rgba(r, g, b, a).to_pax_value())
}

fn hsl(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 3 {
        return Err("Expected 3 arguments for function hsl".to_string());
    }
    let h = Rotation::try_coerce(args[0].clone())?;
    let s = ColorChannel::try_coerce(args[1].clone())?;
    let l = ColorChannel::try_coerce(args[2].clone())?;
    Ok(Color::hsl(h, s, l).to_pax_value())
}

fn hsla(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 4 {
        return Err("Expected 4 arguments for function hsla".to_string());
    }
    let h = Rotation::try_coerce(args[0].clone())?;
    let s = ColorChannel::try_coerce(args[1].clone())?;
    let l = ColorChannel::try_coerce(args[2].clone())?;
    let a = ColorChannel::try_coerce(args[3].clone())?;
    Ok(Color::hsla(h, s, l, a).to_pax_value())
}

fn hex(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 1 {
        return Err("Expected 1 argument for function hex".to_string());
    }
    let hex = String::try_coerce(args[0].clone())?;
    Ok(Color::from_hex(&hex).to_pax_value())
}

pub trait HelperFunctions {
    fn register_all_functions() {}
}

pub struct Functions;

impl Functions {
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

impl HelperFunctions for String {}

impl HelperFunctions for crate::Numeric {}

impl HelperFunctions for bool {}

impl HelperFunctions for Fill {}

impl HelperFunctions for crate::PaxValue {}

impl HelperFunctions for crate::ColorChannel {}

impl HelperFunctions for crate::Stroke {}

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
                let x = crate::Size::try_coerce(args[0].clone())?;
                let y = crate::Size::try_coerce(args[1].clone())?;
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
                let z = crate::Rotation::try_coerce(args[0].clone())?;
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
                let x = crate::Size::try_coerce(args[0].clone())?;
                let y = crate::Size::try_coerce(args[1].clone())?;
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
                let x = crate::Size::try_coerce(args[0].clone())?;
                let y = crate::Size::try_coerce(args[1].clone())?;
                Ok(crate::Transform2D::anchor(x, y).to_pax_value())
            }),
        );
    }
}



// use crate::Interpolatable;

// use super::{Generic, Point2, Space, Vector2};
// use std::{marker::PhantomData, ops::Mul};

// //-----------------------------------------------------------
// // Pax matrix/transform class heavily borrows from kurbos
// // transform impl (copy/pasted initially with some modifications)
// // curbo crate: https://www.michaelfbryan.com/arcs/kurbo/index.html
// // original source code: https://www.michaelfbryan.com/arcs/src/kurbo/affine.rs.html#10
// // Kurbo is distributed under an MIT license.
// //-----------------------------------------------------------

// impl<W: Space, T: Space> Interpolatable for Transform2<W, T> {}

// pub struct Transform2<WFrom = Generic, WTo = WFrom> {
//     m: [f64; 6],
//     _panthom_from: PhantomData<WFrom>,
//     _panthom_to: PhantomData<WTo>,
// }

// // Implement Clone, Copy, PartialEq, etc manually, as
// // to not require the Space to implement these.

// impl<F, T> Clone for Transform2<F, T> {
//     fn clone(&self) -> Self {
//         Self {
//             m: self.m,
//             _panthom_from: PhantomData,
//             _panthom_to: PhantomData,
//         }
//     }
// }

// impl<F, T> std::fmt::Debug for Transform2<F, T> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         writeln!(f, "{} {} {}", self.m[0], self.m[2], self.m[4])?;
//         write!(f, "{} {} {}", self.m[1], self.m[3], self.m[5])
//     }
// }

// impl<F, T> PartialEq for Transform2<F, T> {
//     fn eq(&self, other: &Self) -> bool {
//         self.m == other.m
//     }
// }

// impl<F, T> Copy for Transform2<F, T> {}

// impl<F: Space, T: Space> Default for Transform2<F, T> {
//     fn default() -> Self {
//         Self::identity()
//     }
// }

// impl<WFrom: Space, WTo: Space> Transform2<WFrom, WTo> {
//     pub fn new(m: [f64; 6]) -> Self {
//         Self {
//             m,
//             _panthom_from: PhantomData,
//             _panthom_to: PhantomData,
//         }
//     }

//     pub fn identity() -> Self {
//         Self::new([1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
//     }

//     pub fn scale(s: f64) -> Self {
//         Self::scale_sep(Vector2::new(s, s))
//     }

//     pub fn scale_sep(s: Vector2<WTo>) -> Self {
//         Self::new([s.x, 0.0, 0.0, s.y, 0.0, 0.0])
//     }

//     pub fn skew(k: Vector2<WTo>) -> Self {
//         Self::new([1.0, k.y, k.x, 1.0, 0.0, 0.0])
//     }

//     pub fn rotate(th: f64) -> Self {
//         let (s, c) = th.sin_cos();
//         Self::new([c, s, -s, c, 0.0, 0.0])
//     }

//     pub fn translate(p: Vector2<WTo>) -> Self {
//         Self::new([1.0, 0.0, 0.0, 1.0, p.x, p.y])
//     }

//     pub fn determinant(self) -> f64 {
//         self.m[0] * self.m[3] - self.m[1] * self.m[2]
//     }

//     pub fn coeffs(&self) -> [f64; 6] {
//         self.m
//     }

//     pub fn get_translation(self) -> Vector2<WFrom> {
//         (self * Point2::<WFrom>::default()).cast_space().to_vector()
//     }

//     pub fn get_scale(self) -> Vector2<WTo> {
//         self * Vector2::<WFrom>::new(1.0, 1.0)
//     }

//     pub fn cast_spaces<W: Space, T: Space>(self) -> Transform2<W, T> {
//         Transform2::new(self.m)
//     }

//     /// Produces NaN values when the determinant is zero.
//     pub fn inverse(self) -> Transform2<WTo, WFrom> {
//         let inv_det = self.determinant().recip();
//         Transform2::<WTo, WFrom>::new([
//             inv_det * self.m[3],
//             -inv_det * self.m[1],
//             -inv_det * self.m[2],
//             inv_det * self.m[0],
//             inv_det * (self.m[2] * self.m[5] - self.m[3] * self.m[4]),
//             inv_det * (self.m[1] * self.m[4] - self.m[0] * self.m[5]),
//         ])
//     }

//     pub fn compose(p: Point2<WTo>, vx: Vector2<WTo>, vy: Vector2<WTo>) -> Self {
//         Self::new([vx.x, vx.y, vy.x, vy.y, p.x, p.y])
//     }

//     // Decomposes the transform into translation point + unit vector transforms
//     // (ie. where (0, 1) and (1, 0) end up)
//     pub fn decompose(&self) -> (Point2<WTo>, Vector2<WTo>, Vector2<WTo>) {
//         let [v1x, v1y, v2x, v2y, px, py] = self.m;
//         (
//             Point2::new(px, py),
//             Vector2::new(v1x, v1y),
//             Vector2::new(v2x, v2y),
//         )
//     }

//     pub fn contains_point(&self, point: Point2<WTo>) -> bool {
//         let unit = self.inverse() * point;
//         unit.x > 0.0 && unit.y > 0.0 && unit.x < 1.0 && unit.y < 1.0
//     }
// }


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
                let s = f64::try_coerce(args[0].clone())?;
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
                let s = Vector2::try_coerce(args[0].clone())?;
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
                let s = f64::try_coerce(args[0].clone())?;
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
                let s = Vector2::try_coerce(args[0].clone())?;
                Ok(Transform2::skew(s).to_pax_value())
            }),
        );
    }
}