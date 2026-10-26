use crate::*;
use pax_engine::api::properties::{PropertyValue, UntypedProperty};
use pax_engine::api::{pax_value::PaxValue, CoercionRules, ToPaxValue, Variable};
use pax_engine::pax_manifest as manifest;
use pax_engine::pax_runtime as rt;
use std::{collections::HashMap, rc::Rc};
#[derive(Clone)]
pub enum Field {
    Value(LocalProperty<PaxValue>),
    Bool(LocalProperty<bool>),
    Usize(LocalProperty<usize>),
    U64(LocalProperty<u64>),
    F64(LocalProperty<f64>),
    String(LocalProperty<String>),
    Bounds(LocalProperty<(f64, f64)>),
}
macro_rules! field {
    ($v:expr,$p:ident=>$body:expr) => {
        match $v {
            Field::Value($p) => $body,
            Field::Bool($p) => $body,
            Field::Usize($p) => $body,
            Field::U64($p) => $body,
            Field::F64($p) => $body,
            Field::String($p) => $body,
            Field::Bounds(_) => panic!("bounds are read-only context"),
        }
    };
}
impl Field {
    pub fn new(kind: u32, value: PaxValue) -> Self {
        match kind {
            0 => Self::Value(LocalProperty::new(value)),
            1 => Self::Bool(LocalProperty::new(bool::try_coerce(value).unwrap())),
            2 => Self::Usize(LocalProperty::new(usize::try_coerce(value).unwrap())),
            3 => Self::U64(LocalProperty::new(u64::try_coerce(value).unwrap())),
            4 => Self::F64(LocalProperty::new(f64::try_coerce(value).unwrap())),
            5 => Self::String(LocalProperty::new(String::try_coerce(value).unwrap())),
            6 => Self::Bounds(LocalProperty::new(<(f64, f64)>::try_coerce(value).unwrap())),
            _ => panic!("field kind"),
        }
    }
    pub fn get(&self) -> PaxValue {
        if let Field::Bounds(p) = self {
            let (x, y) = p.get();
            return PaxValue::Vec(vec![x.to_pax_value(), y.to_pax_value()]);
        }
        field!(self,p=>p.get().to_pax_value())
    }
    pub fn set(&self, value: PaxValue) {
        field!(self,p=>p.set(CoercionRules::try_coerce(value).unwrap()))
    }
    pub fn untyped(&self) -> UntypedProperty {
        if let Field::Bounds(p) = self {
            return p.untyped();
        }
        field!(self,p=>p.untyped())
    }
    pub fn variable(&self) -> Variable {
        field!(self,p=>Variable::new_from_typed_property(p.clone()))
    }
    pub fn replace(&self, other: &Field) {
        field!(self,p=>p.replace_with(LocalProperty::new_from_untyped(other.untyped())))
    }
    pub fn cancel(&self) {
        field!(self,p=>p.cancel_transitions())
    }
    pub fn ease(&self, e: wire::Ease) {
        let curve = match e.curve {
            0 => EasingCurve::Linear,
            1 => EasingCurve::Hold,
            2 => EasingCurve::InQuad,
            3 => EasingCurve::OutQuad,
            4 => EasingCurve::InOutQuad,
            5 => EasingCurve::InBack,
            6 => EasingCurve::OutBack,
            7 => EasingCurve::InOutBack,
            _ => panic!("curve"),
        };
        field!(self,p=>{let v=CoercionRules::try_coerce(e.value).unwrap();if e.append{p.ease_to_later(v,e.duration,curve)}else{p.ease_to(v,e.duration,curve)}})
    }
    pub fn computed(kind: u32, callback: u32, deps: &[UntypedProperty]) -> Self {
        let guard = super::bridge::Callback(callback);
        macro_rules! make {
            ($t:ty,$v:ident) => {
                Self::$v(LocalProperty::computed(
                    move || {
                        let len = guard.evaluate();
                        <$t>::try_coerce(super::bridge::receive(len)).unwrap()
                    },
                    deps,
                ))
            };
        }
        match kind {
            0 => make!(PaxValue, Value),
            1 => make!(bool, Bool),
            2 => make!(usize, Usize),
            3 => make!(u64, U64),
            4 => make!(f64, F64),
            5 => make!(String, String),
            _ => panic!("kind"),
        }
    }
    pub fn apply(
        &mut self,
        name: &str,
        default: PaxValue,
        entries: &[rt::RuntimeResolvedPropertyEntry],
        stack: &Rc<rt::RuntimePropertiesStackFrame>,
        node: Option<&rt::ExpandedNode>,
        scope: &HashMap<String, Variable>,
    ) {
        field!(self,p=>apply(p,name,default,entries,stack,node,scope))
    }
}
fn apply<T: PropertyValue + ToPaxValue + CoercionRules>(
    property: &mut LocalProperty<T>,
    name: &str,
    default: PaxValue,
    entries: &[rt::RuntimeResolvedPropertyEntry],
    stack: &Rc<rt::RuntimePropertiesStackFrame>,
    node: Option<&rt::ExpandedNode>,
    scope: &HashMap<String, Variable>,
) {
    let mut layered = LocalProperty::new(T::try_coerce(default).unwrap());
    let mut direct = None;
    for entry in entries {
        let previous = layered.clone();
        let source = entry.source_stack.as_ref().unwrap_or(stack);
        let base = rt::stack_with_base(source, previous.clone());
        let local = match &entry.value {
            manifest::ValueDefinition::Timeline(t) => t.use_local_property_scope,
            manifest::ValueDefinition::Transition(t) => {
                t.enter.as_ref().is_some_and(|t| t.use_local_property_scope)
                    || t.exit.as_ref().is_some_and(|t| t.use_local_property_scope)
            }
            _ => false,
        };
        let timeline = if local {
            let mut s = scope.clone();
            s.insert(
                name.into(),
                Variable::new_from_typed_property(layered.clone()),
            );
            base.push(s)
        } else {
            base.clone()
        };
        let candidate =
            rt::build_component_property(name, &entry.value, &base, timeline, |_, _| {
                panic!("Paxflix custom properties do not use literal blocks")
            });
        direct = if entry.condition.is_none()
            && matches!(entry.value, manifest::ValueDefinition::DoubleBinding(_))
        {
            Some(candidate.clone())
        } else {
            None
        };
        layered = rt::apply_runtime_settings_condition(
            name,
            entry.condition.as_ref(),
            source,
            previous,
            candidate,
        );
    }
    if let Some(bound) = direct {
        rt::clear_settings_motion(node, name);
        *property = bound
    } else {
        rt::bind_settings_property(property, layered, name, entries, stack, node)
    }
}
