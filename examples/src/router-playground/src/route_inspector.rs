#![allow(unused_imports)]

use pax_kit::pax_engine::api::{PaxValue, Variable};
use pax_kit::*;

#[pax]
#[file("route_inspector.pax")]
pub struct RouteInspector {
    pub title: Property<String>,
    pub scope_path: Property<String>,
    pub global_path: Property<String>,
    pub params_summary: Property<String>,
    pub query_summary: Property<String>,
    pub remainder_summary: Property<String>,
    pub fragment_summary: Property<String>,
    pub exact_summary: Property<String>,
    pub consumed_summary: Property<String>,
}

impl RouteInspector {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        let Some(route) = ctx.local_stack_frame.resolve_symbol("route") else {
            self.scope_path.set("route unavailable".to_string());
            self.global_path.set("route unavailable".to_string());
            self.params_summary.set("none".to_string());
            self.query_summary.set("none".to_string());
            self.remainder_summary.set("none".to_string());
            self.fragment_summary.set("none".to_string());
            self.exact_summary.set("unknown".to_string());
            self.consumed_summary.set("unknown".to_string());
            return;
        };

        bind_string(&self.scope_path, &route, route_scope_path);
        bind_string(&self.global_path, &route, route_global_path);
        bind_string(&self.params_summary, &route, route_params_summary);
        bind_string(&self.query_summary, &route, route_query_summary);
        bind_string(&self.remainder_summary, &route, route_remainder_summary);
        bind_string(&self.fragment_summary, &route, route_fragment_summary);
        bind_string(&self.exact_summary, &route, route_exact_summary);
        bind_string(&self.consumed_summary, &route, route_consumed_summary);
    }
}

fn bind_string(target: &Property<String>, route: &Variable, formatter: fn(&PaxValue) -> String) {
    let route = route.clone();
    let deps = [route.get_untyped_property().clone()];
    target.replace_with(Property::computed(
        move || formatter(&route.get_as_pax_value()),
        &deps,
    ));
}

fn route_scope_path(route: &PaxValue) -> String {
    format_path(
        nested_field(field(route, "location"), "path_segments"),
        true,
    )
}

fn route_global_path(route: &PaxValue) -> String {
    format_path(
        nested_field(field(route, "global_location"), "path_segments"),
        true,
    )
}

fn route_params_summary(route: &PaxValue) -> String {
    format_named_map(field(route, "params"))
}

fn route_query_summary(route: &PaxValue) -> String {
    format_named_map(nested_field(field(route, "global_location"), "query"))
}

fn route_remainder_summary(route: &PaxValue) -> String {
    format_path(field(route, "remainder"), false)
}

fn route_fragment_summary(route: &PaxValue) -> String {
    match nested_field(field(route, "global_location"), "fragment") {
        Some(PaxValue::Option(fragment)) => match fragment.as_ref() {
            Some(PaxValue::String(value)) => format!("#{value}"),
            Some(value) => format!("{value}"),
            None => "none".to_string(),
        },
        Some(PaxValue::String(value)) => format!("#{value}"),
        _ => "none".to_string(),
    }
}

fn route_exact_summary(route: &PaxValue) -> String {
    match field(route, "is_exact") {
        Some(PaxValue::Bool(true)) => "true".to_string(),
        Some(PaxValue::Bool(false)) => "false".to_string(),
        _ => "unknown".to_string(),
    }
}

fn route_consumed_summary(route: &PaxValue) -> String {
    match field(route, "consumed_segments") {
        Some(PaxValue::Numeric(value)) => value.to_string(),
        Some(value) => format!("{value}"),
        None => "unknown".to_string(),
    }
}

fn field<'a>(value: &'a PaxValue, key: &str) -> Option<&'a PaxValue> {
    let PaxValue::Object(entries) = value else {
        return None;
    };
    entries
        .iter()
        .find_map(|(name, value)| (name == key).then_some(value))
}

fn nested_field<'a>(value: Option<&'a PaxValue>, key: &str) -> Option<&'a PaxValue> {
    value.and_then(|value| field(value, key))
}

fn format_path(value: Option<&PaxValue>, leading_slash: bool) -> String {
    let Some(PaxValue::Vec(segments)) = value else {
        return "none".to_string();
    };

    let mut parts = Vec::new();
    for segment in segments {
        match segment {
            PaxValue::String(value) => parts.push(value.clone()),
            other => parts.push(format!("{other}")),
        }
    }

    if parts.is_empty() {
        if leading_slash {
            "/".to_string()
        } else {
            "(empty)".to_string()
        }
    } else if leading_slash {
        format!("/{}", parts.join("/"))
    } else {
        parts.join("/")
    }
}

fn format_named_map(value: Option<&PaxValue>) -> String {
    let Some(PaxValue::Object(entries)) = value else {
        return "none".to_string();
    };

    if entries.is_empty() {
        return "none".to_string();
    }

    let mut parts = entries
        .iter()
        .map(|(name, value)| format!("{name}={}", format_scalar(value)))
        .collect::<Vec<_>>();
    parts.sort();
    parts.join(", ")
}

fn format_scalar(value: &PaxValue) -> String {
    match value {
        PaxValue::String(value) => value.clone(),
        PaxValue::Numeric(value) => value.to_string(),
        PaxValue::Bool(value) => value.to_string(),
        PaxValue::Vec(values) => {
            let values = values.iter().map(format_scalar).collect::<Vec<_>>();
            format!("[{}]", values.join(", "))
        }
        PaxValue::Option(value) => match value.as_ref() {
            Some(value) => format_scalar(value),
            None => "none".to_string(),
        },
        _ => format!("{value}"),
    }
}
