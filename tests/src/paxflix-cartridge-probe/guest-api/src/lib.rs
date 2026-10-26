//! Fixture guest bindings. All property evaluation and animation use the host graph.
pub use pax_runtime_api::{
    CoercionRules, Duration, EasingCurve, Interpolatable, PaxValue, ToPaxValue,
};
use std::{cell::RefCell, marker::PhantomData, rc::Rc};
#[path = "../../arena.rs"]
mod arena;
#[path = "../../events.rs"]
pub mod events;
#[path = "../../wire.rs"]
pub mod wire;
pub use pax_macro::__pax_expand as pax;
use wire::*;
#[link(wasm_import_module = "pax_module")]
extern "C" {
    fn module_host_bridge(op: u32, a: u32, b: u32) -> u32;
}
fn bridge(op: u32, a: u32, b: u32) -> u32 {
    unsafe { module_host_bridge(op, a, b) }
}
thread_local! {
 static OUTPUT:RefCell<Vec<u8>>=const {RefCell::new(Vec::new())};
 static CALLBACKS:RefCell<arena::Arena<Rc<dyn Fn()->PaxValue>>>=RefCell::new(Default::default());
}
pub fn output<T: serde::Serialize>(value: &T) -> u32 {
    let bytes = serde_json::to_vec(value).unwrap();
    let len = bytes.len() as u32;
    OUTPUT.with(|b| *b.borrow_mut() = bytes);
    len
}
pub fn output_word(index: u32) -> u32 {
    OUTPUT.with(|b| {
        let b = b.borrow();
        let mut word = [0; 4];
        for (i, w) in word.iter_mut().enumerate() {
            *w = *b.get(index as usize * 4 + i).unwrap_or(&0);
        }
        u32::from_le_bytes(word)
    })
}
fn receive<T: serde::de::DeserializeOwned>(len: u32) -> T {
    let mut bytes = Vec::with_capacity(len as usize + 3);
    for i in 0..len.div_ceil(4) {
        bytes.extend_from_slice(&bridge(WORD, i, 0).to_le_bytes());
    }
    bytes.truncate(len as usize);
    serde_json::from_slice(&bytes).unwrap()
}
pub fn request() -> Request {
    receive(bridge(REQUEST, 0, 0))
}
pub fn evaluate(id: u32) -> u32 {
    let f = CALLBACKS.with(|c| c.borrow().get(id).clone());
    output(&f())
}
pub fn drop_callback(id: u32) {
    let f = CALLBACKS.with(|c| c.borrow_mut().remove(id));
    drop(f);
}
pub fn callbacks() -> u32 {
    CALLBACKS.with(|c| c.borrow().len() as u32)
}
fn callback(f: impl Fn() -> PaxValue + 'static) -> u32 {
    CALLBACKS.with(|c| c.borrow_mut().insert(Rc::new(f)))
}
pub trait Value: Default + Clone + Interpolatable + ToPaxValue + CoercionRules + 'static {}
impl<T: Default + Clone + Interpolatable + ToPaxValue + CoercionRules + 'static> Value for T {}
fn kind<T>() -> u32 {
    match std::any::type_name::<T>() {
        "bool" => 1,
        "usize" => 2,
        "u64" => 3,
        "f64" => 4,
        "alloc::string::String" => 5,
        "(f64, f64)" => 6,
        _ => 0,
    }
}
#[derive(Debug)]
pub struct UntypedProperty {
    id: u32,
}
impl Clone for UntypedProperty {
    fn clone(&self) -> Self {
        bridge(RETAIN, self.id, 0);
        Self { id: self.id }
    }
}
impl std::ops::Drop for UntypedProperty {
    fn drop(&mut self) {
        bridge(RELEASE, self.id, 0);
    }
}
pub struct Property<T: Value> {
    handle: UntypedProperty,
    marker: PhantomData<T>,
}
pub type LocalProperty<T> = Property<T>;
impl<T: Value> Clone for Property<T> {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
            marker: PhantomData,
        }
    }
}
impl<T: Value> Property<T> {
    pub fn from_owned_handle(id: u32) -> Self {
        Self {
            handle: UntypedProperty { id },
            marker: PhantomData,
        }
    }
    pub fn field(instance: u32, index: u32) -> Self {
        Self::from_owned_handle(bridge(HANDLE, instance, index))
    }
    pub fn new(value: T) -> Self {
        let len = output(&value.to_pax_value());
        Self::from_owned_handle(bridge(NEW, kind::<T>(), len))
    }
    pub fn get(&self) -> T {
        T::try_coerce(receive(bridge(READ, self.handle.id, 0))).unwrap()
    }
    pub fn read<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.get())
    }
    pub fn set(&self, value: T) {
        let len = output(&value.to_pax_value());
        bridge(WRITE, self.handle.id, len);
    }
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        let mut value = self.get();
        f(&mut value);
        self.set(value);
    }
    pub fn local(&self) -> Self {
        self.clone()
    }
    pub fn untyped(&self) -> UntypedProperty {
        self.handle.clone()
    }
    pub fn replace_with(&self, source: Self) {
        bridge(REPLACE, self.handle.id, source.handle.id);
    }
    pub fn computed(f: impl Fn() -> T + 'static, deps: &[UntypedProperty]) -> Self {
        let callback = callback(move || f().to_pax_value());
        let len = output(&Computation {
            callback,
            dependencies: deps.iter().map(|d| d.id).collect(),
        });
        Self::from_owned_handle(bridge(COMPUTED, kind::<T>(), len))
    }
    pub fn cancel_transitions(&self) {
        bridge(CANCEL, self.handle.id, 0);
    }
    pub fn ease_to<D: Into<Duration>>(&self, v: T, d: D, c: EasingCurve) {
        self.ease(v, d.into(), c, false)
    }
    pub fn ease_to_later<D: Into<Duration>>(&self, v: T, d: D, c: EasingCurve) {
        self.ease(v, d.into(), c, true)
    }
    fn ease(&self, v: T, d: Duration, c: EasingCurve, append: bool) {
        let len = output(&Ease {
            value: v.to_pax_value(),
            duration: d,
            curve: match c {
                EasingCurve::Linear => 0,
                EasingCurve::Hold => 1,
                EasingCurve::InQuad => 2,
                EasingCurve::OutQuad => 3,
                EasingCurve::InOutQuad => 4,
                EasingCurve::InBack => 5,
                EasingCurve::OutBack => 6,
                EasingCurve::InOutBack => 7,
                EasingCurve::Custom(_) => panic!("custom curve callbacks not in this fixture"),
            },
            append,
        });
        bridge(EASE, self.handle.id, len);
    }
}
impl<T: Value> Default for Property<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}
impl<T: Value> std::fmt::Debug for Property<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_tuple("RemoteProperty")
            .field(&self.handle.id)
            .finish()
    }
}
impl<T: Value> Interpolatable for Property<T> {}
impl<T: Value + serde::Serialize> serde::Serialize for Property<T> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.get().serialize(s)
    }
}
impl<'de, T: Value + serde::Deserialize<'de>> serde::Deserialize<'de> for Property<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        T::deserialize(d).map(Self::new)
    }
}
pub trait PropertyBinding<T: Value> {
    fn from_value(value: T) -> Self;
    fn value_for_conversion(&self) -> T;
}
impl<T: Value> PropertyBinding<T> for Property<T> {
    fn from_value(value: T) -> Self {
        Self::new(value)
    }
    fn value_for_conversion(&self) -> T {
        self.get()
    }
}
#[derive(Clone)]
pub struct BoundsProperty {
    handle: UntypedProperty,
}
impl BoundsProperty {
    pub fn get(&self) -> (f64, f64) {
        <(f64, f64)>::try_coerce(receive(bridge(READ, self.handle.id, 0))).unwrap()
    }
    pub fn untyped(&self) -> UntypedProperty {
        self.handle.clone()
    }
}
pub struct NodeContext {
    pub bounds_self: BoundsProperty,
    pub elapsed_millis: LocalProperty<u64>,
}
impl NodeContext {
    pub fn from_request(r: &Request) -> Self {
        Self {
            bounds_self: BoundsProperty {
                handle: UntypedProperty { id: r.bounds },
            },
            elapsed_millis: Property::from_owned_handle(r.clock),
        }
    }
    pub fn subscribe(&self, deps: &[UntypedProperty], f: impl Fn() + 'static) {
        let id = callback(move || {
            f();
            PaxValue::default()
        });
        let len = output(&deps.iter().map(|d| d.id).collect::<Vec<_>>());
        bridge(SUBSCRIBE, id, len);
    }
    pub fn set_cursor(&self, c: pax_runtime_api::cursor::CursorStyle) {
        let value = match c {
            pax_runtime_api::cursor::CursorStyle::Pointer => 1,
            pax_runtime_api::cursor::CursorStyle::Auto => 0,
            _ => panic!("unsupported cursor"),
        };
        bridge(CURSOR, value, 0);
    }
}
pub mod pax_engine {
    pub use pax_runtime_api::{serde, PaxValue};
    pub mod api {
        pub use crate::{LocalProperty, NodeContext, Property, PropertyBinding, UntypedProperty};
        pub use pax_runtime_api::*;
    }
}
pub use pax_engine::api::*;
