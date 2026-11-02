use pax_kit::pax_engine::api::{NavigationTarget, PaxValue};
use pax_kit::*;

pax_kit::runnable! { root: crate::NativeNavigation }

#[pax]
#[file("lib.pax")]
pub struct NativeNavigation {}

impl NativeNavigation {
    pub fn absolute(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        ctx.navigate_to(
            "/teams/42/members?lane=alpha&tag=a&tag=b#details",
            NavigationTarget::Current,
        );
    }
    pub fn query(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        ctx.navigate_to("?lane=beta#inspect", NavigationTarget::Current);
    }
    pub fn sibling(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        ctx.navigate_to("./settings", NavigationTarget::Current);
    }
    pub fn parent(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        ctx.navigate_to("../overview", NavigationTarget::Current);
    }
    pub fn unicode(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        ctx.navigate_to(
            "/caf%C3%A9/hello%20world?tag=%E2%9C%93&tag=two#r%C3%A9sum%C3%A9",
            NavigationTarget::Current,
        );
    }
    pub fn fragment(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        ctx.navigate_to("#changed", NavigationTarget::Current);
    }
}

#[pax]
#[file("route_status.pax")]
pub struct RouteStatus {
    pub snapshot: Property<String>,
    pub last_recorded: Property<String>,
}

impl RouteStatus {
    pub fn mount(&mut self, ctx: &NodeContext) {
        let route = ctx
            .expression_stack
            .resolve_symbol("route")
            .expect("route scope");
        let dependencies = [route.get_untyped_property().clone()];
        self.snapshot.replace_with(LocalProperty::computed(
            move || {
                let value = route.get_as_pax_value();
                let location = match &value {
                    PaxValue::Object(fields) => fields
                        .iter()
                        .find(|(key, _)| key == "global_location")
                        .map(|(_, value)| value),
                    _ => None,
                };
                location
                    .map(route_json)
                    .unwrap_or(serde_json::Value::Null)
                    .to_string()
            },
            &dependencies,
        ));
    }

    pub fn record(&mut self, _: &NodeContext) {
        let snapshot = self.snapshot.get();
        if snapshot == self.last_recorded.get() {
            return;
        }
        self.last_recorded.set(snapshot.clone());
        eprintln!("pax-navigation: {snapshot}");
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = std::env::var_os("PAX_NAVIGATION_LOG") {
            use std::io::Write;
            let mut log = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .expect("open PAX_NAVIGATION_LOG");
            writeln!(log, "{snapshot}").expect("write PAX_NAVIGATION_LOG");
        }
    }
}

fn route_json(value: &PaxValue) -> serde_json::Value {
    match value {
        PaxValue::String(value) => value.clone().into(),
        PaxValue::Vec(values) => values.iter().map(route_json).collect(),
        PaxValue::Object(fields) => fields
            .iter()
            .map(|(key, value)| (key.clone(), route_json(value)))
            .collect::<serde_json::Map<_, _>>()
            .into(),
        PaxValue::Option(value) => value
            .as_ref()
            .as_ref()
            .map(route_json)
            .unwrap_or(serde_json::Value::Null),
        _ => panic!("unexpected route data: {value:?}"),
    }
}
