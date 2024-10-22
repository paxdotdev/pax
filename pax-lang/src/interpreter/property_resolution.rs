use std::collections::{hash_set, HashMap};

use pax_runtime_api::{PaxValue, Property, Variable};

use super::{PaxExpression, PaxInfix, PaxPostfix, PaxPrefix, PaxPrimary};

/// Trait for resolving identifiers to values
/// This is implemented by RuntimePropertyStackFrame
pub trait IdentifierResolver {
    fn resolve(&self, name: String) -> Result<Variable, String>;
}

pub trait DependencyCollector {
    fn collect_dependencies(&self) -> Vec<String>;
}

impl DependencyCollector for PaxExpression {
    fn collect_dependencies(&self) -> Vec<String> {
        match self {
            PaxExpression::Primary(p) => p.collect_dependencies(),
            PaxExpression::Prefix(p) => p.collect_dependencies(),
            PaxExpression::Infix(p) => p.collect_dependencies(),
            PaxExpression::Postfix(p) => p.collect_dependencies(),
        }
    }
}

impl DependencyCollector for PaxPrimary {
    fn collect_dependencies(&self) -> Vec<String> {
        let ret = match self {
            PaxPrimary::Literal(_) => vec![],
            PaxPrimary::Grouped(expr, _) => expr.collect_dependencies(),
            PaxPrimary::Identifier(i, _) => vec![i.name.clone()],
            PaxPrimary::Object(o) => o
                .iter()
                .flat_map(|(_, v)| v.collect_dependencies())
                .collect(),
            PaxPrimary::FunctionOrEnum(_, _, args) => {
                args.iter().flat_map(|a| a.collect_dependencies()).collect()
            }
            PaxPrimary::Range(start, end) => {
                let mut deps = start.collect_dependencies();
                deps.extend(end.collect_dependencies());
                deps
            }
            PaxPrimary::Tuple(t) => t.iter().flat_map(|e| e.collect_dependencies()).collect(),
            PaxPrimary::List(l) => l.iter().flat_map(|e| e.collect_dependencies()).collect(),
        };
        let deduped_deps: std::collections::HashSet<String> = ret.into_iter().collect();
        deduped_deps.into_iter().collect()
    }
}

impl DependencyCollector for PaxPrefix {
    fn collect_dependencies(&self) -> Vec<String> {
        self.rhs.collect_dependencies()
    }
}

impl DependencyCollector for PaxInfix {
    fn collect_dependencies(&self) -> Vec<String> {
        let mut deps = self.lhs.collect_dependencies();
        deps.extend(self.rhs.collect_dependencies());
        deps
    }
}

impl DependencyCollector for PaxPostfix {
    fn collect_dependencies(&self) -> Vec<String> {
        self.lhs.collect_dependencies()
    }
}

impl IdentifierResolver for HashMap<String, PaxValue> {
    fn resolve(&self, name: String) -> Result<Variable, String> {
        self.get(&name)
            .map(|v| Variable::new_from_typed_property(Property::new(v.clone())))
            .ok_or(format!("Identifier not found: {}", name))
    }
}
