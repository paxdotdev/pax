use crate::{Color, ColorChannel, Numeric, PaxValue, Rotation};
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

fn sub(args: Vec<PaxValue>) -> Result<PaxValue, String> {
    if args.len() != 2 {
        return Err("Expected 2 arguments for function sub".to_string());
    }
    Ok(args[0].clone() - args[1].clone())
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
    fn register_all_functions();
}


pub struct GlobalFunctions;

impl HelperFunctions for GlobalFunctions {
    fn register_all_functions() {
        // Math
        register_function("Math".to_string(), "+".to_string(), Arc::new(add));
        register_function("Math".to_string(), "-".to_string(), Arc::new(sub));
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
        register_function("Math".to_string(), "min".to_string(), Arc::new(min));
        register_function("Math".to_string(), "max".to_string(), Arc::new(max));
        // Colors
        register_function("Color".to_string(), "rgb".to_string(), Arc::new(rgb));
        register_function("Color".to_string(), "rgba".to_string(), Arc::new(rgba));
        register_function("Color".to_string(), "hsl".to_string(), Arc::new(hsl));
        register_function("Color".to_string(), "hsla".to_string(), Arc::new(hsla));
        register_function("Color".to_string(), "#".to_string(), Arc::new(hex));
    }
}
