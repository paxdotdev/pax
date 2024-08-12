use std::{collections::HashMap, rc::Rc};

use pax_runtime_api::{
    functions::call_function, CoercionRules, Numeric, PaxValue, Percent, Rotation, Size,
};

use super::{
    property_resolution::IdentifierResolver, PaxAccessor, PaxExpression, PaxFunctionCall,
    PaxIdentifier, PaxInfix, PaxPostfix, PaxPrefix, PaxPrimary, PaxUnit,
};

/// Trait for expression types that can be computed to a value
pub trait Computable {
    fn compute(&self, idr: Rc<dyn IdentifierResolver>) -> Result<PaxValue, String>;
}

impl Computable for PaxExpression {
    fn compute(&self, idr: Rc<dyn IdentifierResolver>) -> Result<PaxValue, String> {
        match self {
            PaxExpression::Primary(p) => p.compute(idr),
            PaxExpression::Prefix(p) => p.compute(idr),
            PaxExpression::Infix(p) => p.compute(idr),
            PaxExpression::Postfix(p) => p.compute(idr),
        }
    }
}

impl Computable for PaxPrimary {
    fn compute(&self, idr: Rc<dyn IdentifierResolver>) -> Result<PaxValue, String> {
        match self {
            PaxPrimary::Literal(v) => Ok(v.clone()),
            PaxPrimary::Identifier(i, accessors) => {
                let mut value = i.compute(idr.clone())?;
                for accessor in accessors {
                    match accessor {
                        PaxAccessor::Tuple(index) => {
                            if let PaxValue::Vec(v) = value {
                                value = v[*index].clone();
                            } else {
                                return Err("Tuple access must be performed on a tuple".to_string());
                            }
                        }
                        PaxAccessor::List(index) => {
                            if let PaxValue::Vec(v) = value {
                                let index = Numeric::try_coerce(index.compute(idr.clone())?)?
                                    .to_int() as usize;
                                value = v[index].clone();
                            } else {
                                return Err("List access must be performed on a list".to_string());
                            }
                        }
                        PaxAccessor::Struct(field) => {
                            if let PaxValue::Object(obj) = value {
                                value = obj
                                    .get(field)
                                    .map(|v| v.clone())
                                    .ok_or(format!("Field not found: {}", field))?;
                            } else {
                                return Err(
                                    "Struct access must be performed on an object".to_string()
                                );
                            }
                        }
                    }
                }
                Ok(value)
            }
            PaxPrimary::FunctionCall(f) => f.compute(idr),
            PaxPrimary::Object(o) => {
                let mut obj = HashMap::new();
                for (k, v) in o.iter() {
                    obj.insert(k.clone(), v.compute(idr.clone())?);
                }
                Ok(PaxValue::Object(obj))
            }
            PaxPrimary::Range(start, end) => {
                let start = start.compute(idr.clone())?;
                let end = end.compute(idr)?;
                Ok(PaxValue::Range(Box::new(start), Box::new(end)))
            }
            PaxPrimary::Tuple(t) => {
                let tuple = t
                    .iter()
                    .map(|e| e.compute(idr.clone()))
                    .collect::<Result<Vec<PaxValue>, String>>()?;
                Ok(PaxValue::Vec(tuple))
            }
            PaxPrimary::List(l) => {
                let list = l
                    .iter()
                    .map(|e| e.compute(idr.clone()))
                    .collect::<Result<Vec<PaxValue>, String>>()?;
                Ok(PaxValue::Vec(list))
            }
            PaxPrimary::Grouped(expr, unit) => {
                let expr_val = expr.compute(idr.clone())?;
                if let Some(unit) = unit {
                    match Numeric::try_coerce(expr_val) {
                        Ok(n) => match unit {
                            PaxUnit::Percent => Ok(PaxValue::Percent(Percent(n))),
                            PaxUnit::Pixels => Ok(PaxValue::Size(Size::Pixels(n))),
                            PaxUnit::Radians => Ok(PaxValue::Rotation(Rotation::Radians(n))),
                            PaxUnit::Degrees => Ok(PaxValue::Rotation(Rotation::Degrees(n))),
                        },
                        Err(e) => Err(format!(
                            "A grouped expression with a unit must be of type numeric: {e:?}"
                        )),
                    }
                } else {
                    Ok(expr_val)
                }
            }
            PaxPrimary::Enum(variant, args) => {
                let args = args
                    .iter()
                    .map(|a| a.compute(idr.clone()))
                    .collect::<Result<Vec<PaxValue>, String>>()?;
                Ok(PaxValue::Enum(variant.clone(), args))
            }
        }
    }
}

impl Computable for PaxPrefix {
    fn compute(&self, idr: Rc<dyn IdentifierResolver>) -> Result<PaxValue, String> {
        let rhs = self.rhs.compute(idr)?;
        let operator = &self.operator.name;
        call_function("Math".to_string(), operator.to_string(), vec![rhs])
    }
}

impl Computable for PaxPostfix {
    fn compute(&self, idr: Rc<dyn IdentifierResolver>) -> Result<PaxValue, String> {
        let lhs = self.lhs.compute(idr)?;
        let operator = &self.operator.name;
        call_function("Math".to_string(), operator.to_string(), vec![lhs])
    }
}

impl Computable for PaxInfix {
    fn compute(&self, idr: Rc<dyn IdentifierResolver>) -> Result<PaxValue, String> {
        let lhs = self.lhs.compute(idr.clone())?;
        let rhs = self.rhs.compute(idr)?;
        let operator = &self.operator.name;
        call_function("Math".to_string(), operator.to_string(), vec![lhs, rhs])
    }
}

impl Computable for PaxIdentifier {
    fn compute(&self, idr: Rc<dyn IdentifierResolver>) -> Result<PaxValue, String> {
        idr.resolve(self.name.clone())
    }
}

impl Computable for PaxFunctionCall {
    fn compute(&self, idr: Rc<dyn IdentifierResolver>) -> Result<PaxValue, String> {
        let args = self
            .args
            .iter()
            .map(|a| a.compute(idr.clone()))
            .collect::<Result<Vec<PaxValue>, String>>()?;
        call_function(self.scope.clone(), self.function_name.clone(), args)
    }
}
