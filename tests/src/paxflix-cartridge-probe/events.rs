use pax_runtime_api::*;
use serde_json::{json, Value};
pub trait WireEvent {
    fn encode(&self) -> Value;
    fn decode(v: Value) -> Self;
}
fn modifiers(v: &[ModifierKey]) -> Vec<u32> {
    v.iter()
        .map(|m| match m {
            ModifierKey::Shift => 0,
            ModifierKey::Control => 1,
            ModifierKey::Alt => 2,
            ModifierKey::Command => 3,
        })
        .collect()
}
fn decode_modifiers(v: &Value) -> Vec<ModifierKey> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|m| match m.as_u64().unwrap() {
            0 => ModifierKey::Shift,
            1 => ModifierKey::Control,
            2 => ModifierKey::Alt,
            3 => ModifierKey::Command,
            _ => unreachable!(),
        })
        .collect()
}
macro_rules! empty {($($ty:ident),*)=>{$(impl WireEvent for $ty{fn encode(&self)->Value{Value::Null}fn decode(_:Value)->Self{Self{}}})*}}
empty!(MouseOver, MouseOut);
impl WireEvent for KeyDown {
    fn encode(&self) -> Value {
        json!({"key":self.keyboard.key,"modifiers":modifiers(&self.keyboard.modifiers),"repeat":self.keyboard.is_repeat})
    }
    fn decode(v: Value) -> Self {
        Self {
            keyboard: KeyboardEventArgs {
                key: v["key"].as_str().unwrap().into(),
                modifiers: decode_modifiers(&v["modifiers"]),
                is_repeat: v["repeat"].as_bool().unwrap(),
            },
        }
    }
}
impl WireEvent for Click {
    fn encode(&self) -> Value {
        json!({"x":self.mouse.x,"y":self.mouse.y,"button":match self.mouse.button{MouseButton::Left=>0,MouseButton::Right=>1,MouseButton::Middle=>2,MouseButton::Unknown=>3},"modifiers":modifiers(&self.mouse.modifiers)})
    }
    fn decode(v: Value) -> Self {
        Self {
            mouse: MouseEventArgs {
                x: v["x"].as_f64().unwrap(),
                y: v["y"].as_f64().unwrap(),
                button: match v["button"].as_u64().unwrap() {
                    0 => MouseButton::Left,
                    1 => MouseButton::Right,
                    2 => MouseButton::Middle,
                    _ => MouseButton::Unknown,
                },
                modifiers: decode_modifiers(&v["modifiers"]),
            },
        }
    }
}
impl WireEvent for Wheel {
    fn encode(&self) -> Value {
        json!({"x":self.x,"y":self.y,"dx":self.delta_x,"dy":self.delta_y,"modifiers":modifiers(&self.modifiers)})
    }
    fn decode(v: Value) -> Self {
        Self {
            x: v["x"].as_f64().unwrap(),
            y: v["y"].as_f64().unwrap(),
            delta_x: v["dx"].as_f64().unwrap(),
            delta_y: v["dy"].as_f64().unwrap(),
            modifiers: decode_modifiers(&v["modifiers"]),
        }
    }
}
impl WireEvent for TouchStart {
    fn encode(&self) -> Value {
        json!(self
            .touches
            .iter()
            .map(|t| json!({"x":t.x,"y":t.y,"id":t.identifier,"dx":t.delta_x,"dy":t.delta_y}))
            .collect::<Vec<_>>())
    }
    fn decode(v: Value) -> Self {
        Self {
            touches: v
                .as_array()
                .unwrap()
                .iter()
                .map(|t| Touch {
                    x: t["x"].as_f64().unwrap(),
                    y: t["y"].as_f64().unwrap(),
                    identifier: t["id"].as_i64().unwrap(),
                    delta_x: t["dx"].as_f64().unwrap(),
                    delta_y: t["dy"].as_f64().unwrap(),
                })
                .collect(),
        }
    }
}
fn snapshot(s: &ViewportProximitySnapshot) -> Value {
    json!({"near":s.in_proximity,"bounds":[s.bounds.x0,s.bounds.y0,s.bounds.x1,s.bounds.y1],"intersection":s.viewport_intersection.map(|r|[r.x0,r.y0,r.x1,r.y1])})
}
fn decode_snapshot(v: &Value) -> ViewportProximitySnapshot {
    let b = &v["bounds"];
    let i = &v["intersection"];
    ViewportProximitySnapshot {
        in_proximity: v["near"].as_bool().unwrap(),
        bounds: kurbo::Rect::new(
            b[0].as_f64().unwrap(),
            b[1].as_f64().unwrap(),
            b[2].as_f64().unwrap(),
            b[3].as_f64().unwrap(),
        ),
        viewport_intersection: (!i.is_null()).then(|| {
            kurbo::Rect::new(
                i[0].as_f64().unwrap(),
                i[1].as_f64().unwrap(),
                i[2].as_f64().unwrap(),
                i[3].as_f64().unwrap(),
            )
        }),
    }
}
impl WireEvent for ViewportProximityChange {
    fn encode(&self) -> Value {
        json!({"current":snapshot(&self.current),"previous":self.previous.as_ref().map(snapshot)})
    }
    fn decode(v: Value) -> Self {
        Self {
            current: decode_snapshot(&v["current"]),
            previous: (!v["previous"].is_null()).then(|| decode_snapshot(&v["previous"])),
        }
    }
}
impl WireEvent for ViewportProximityExit {
    fn encode(&self) -> Value {
        json!({"current":snapshot(&self.current),"previous":snapshot(&self.previous)})
    }
    fn decode(v: Value) -> Self {
        Self {
            current: decode_snapshot(&v["current"]),
            previous: decode_snapshot(&v["previous"]),
        }
    }
}
