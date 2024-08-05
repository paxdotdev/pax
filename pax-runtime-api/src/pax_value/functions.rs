use crate::{Numeric, PaxValue};
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

macro_rules! register_scoped_func {
    ($scope:expr, $func:ident) => {
        register_scoped_func!($scope, $func, stringify!($func));
    };
    ($scope:expr, $func:ident, $name:expr) => {
        const _: () = {
            #[ctor::ctor]
            fn _generated_func() {
                let scope = $scope.to_string();
                let name = $name.to_string();
                let boxed_func: FunctionType = Arc::new(move |args| $func(args));
                register_function(scope, name, boxed_func);
            }
        };
    };
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

// Register functions with scopes
register_scoped_func!("Math", add, "+");
register_scoped_func!("Math", sub, "-");
register_scoped_func!("Math", mul, "*");
register_scoped_func!("Math", div, "/");
register_scoped_func!("Math", exp, "^");
register_scoped_func!("Math", mod_, "%%");
register_scoped_func!("Math", rel_eq, "==");
register_scoped_func!("Math", rel_gt, ">");
register_scoped_func!("Math", rel_gte, ">=");
register_scoped_func!("Math", rel_lt, "<");
register_scoped_func!("Math", rel_lte, "<=");
register_scoped_func!("Math", rel_neq, "!=");
register_scoped_func!("Math", bool_and, "&&");
register_scoped_func!("Math", bool_or, "||");
register_scoped_func!("Math", min);
register_scoped_func!("Math", max);
