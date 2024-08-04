use std::{collections::HashMap, sync::{Arc, RwLock}};
use once_cell::sync::Lazy;
use crate::PaxValue;


static FUNCTIONS: Lazy<Arc<RwLock<HashMap<String, Arc<dyn Fn(Vec<PaxValue>) -> Result<PaxValue, String> + Send + Sync>>>>> = Lazy::new(|| {
    Arc::new(RwLock::new(HashMap::new()))
});

pub fn print_all_functions() {
    let functions = FUNCTIONS.read().unwrap();
    println!("len: {}", functions.len());
    for (name, _) in functions.iter() {
        println!("|{}|", name);
    }
}

pub fn register_function(name: String, func: Arc<dyn Fn(Vec<PaxValue>) -> Result<PaxValue, String> + Send + Sync>) {
    let mut functions = FUNCTIONS.write().unwrap();
    functions.insert(name, func);
}

pub fn call_function(name: String, args: Vec<PaxValue>) -> Result<PaxValue, String> {
    let functions = FUNCTIONS.read().unwrap();
    let func = functions.get(&name).ok_or_else(|| format!("Function {} not found", name))?;
    func(args)
}

macro_rules! register_func {
    ($func:ident) => {
        register_func!($func, stringify!($func));
    };
    ($func:ident, $name:expr) => {
        paste::paste! {
            #[ctor::ctor]
            fn [<generated_ $func>]() {
                let name = $name.to_string();
                let boxed_func: Arc<dyn Fn(Vec<PaxValue>) -> Result<PaxValue, String> + Send + Sync> = Arc::new(move |args| {
                    $func(args)
                });
                register_function(name.clone(), boxed_func);
            }
        }
    };
}


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

register_func!(add, "+");
register_func!(sub, "-");
register_func!(mul, "*");
register_func!(div, "/");
register_func!(exp, "^");
register_func!(mod_, "%%");
register_func!(rel_eq, "==");
register_func!(rel_gt, ">");
register_func!(rel_gte, ">=");
register_func!(rel_lt, "<");
register_func!(rel_lte, "<=");
register_func!(rel_neq, "!=");
register_func!(bool_and, "&&");
register_func!(bool_or, "||");