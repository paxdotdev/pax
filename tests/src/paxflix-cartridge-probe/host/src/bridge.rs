use crate::*;
use fields::Field;
use std::rc::Rc;
use wire::*;
thread_local! {
 static HANDLER:Cell<u32>=const{Cell::new(0)};
 static OUTPUT:RefCell<Vec<u8>>=const{RefCell::new(Vec::new())};
 static VALUES:RefCell<arena::Arena<(u32,Rc<RefCell<Field>>) >> =RefCell::new(Default::default());
 static CONTEXT:RefCell<Vec<(NodeContext,serde_json::Value)>>=const{RefCell::new(Vec::new())};
 static CALLS:Cell<u32>=const{Cell::new(0)};
 static BYTES:Cell<u32>=const{Cell::new(0)};
}
pub fn call(op: u32, a: u32, b: u32) -> u32 {
    let slot = HANDLER.with(Cell::get);
    assert_ne!(slot, 0);
    let f: unsafe extern "C" fn(u32, u32, u32) -> u32 =
        unsafe { std::mem::transmute(slot as usize) };
    unsafe { f(op, a, b) }
}
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn probe_table() -> wasm_bindgen::JsValue {
    wasm_bindgen::function_table()
}
#[no_mangle]
pub extern "C" fn probe_register(slot: u32) -> u32 {
    HANDLER.with(|h| {
        if slot == 0 || h.get() != 0 {
            return 0;
        }
        h.set(slot);
        1
    })
}
pub fn add(value: Field) -> u32 {
    VALUES.with(|v| v.borrow_mut().insert((1, Rc::new(RefCell::new(value)))))
}
pub fn get(id: u32) -> Rc<RefCell<Field>> {
    VALUES.with(|v| v.borrow().get(id).1.clone())
}
pub fn retain(id: u32) {
    VALUES.with(|v| v.borrow_mut().get_mut(id).0 += 1)
}
pub fn release(id: u32) {
    let removed = VALUES.with(|v| {
        let mut v = v.borrow_mut();
        let value = v.get_mut(id);
        value.0 -= 1;
        if value.0 == 0 {
            Some(v.remove(id))
        } else {
            None
        }
    });
    drop(removed);
}
pub fn receive<T: serde::de::DeserializeOwned>(len: u32) -> T {
    let mut bytes = Vec::with_capacity(len as usize + 3);
    for i in 0..len.div_ceil(4) {
        bytes.extend_from_slice(&call(6, i, 0).to_le_bytes())
    }
    bytes.truncate(len as usize);
    BYTES.with(|b| b.set(b.get() + len));
    serde_json::from_slice(&bytes).unwrap()
}
fn output<T: serde::Serialize>(value: &T) -> u32 {
    let bytes = serde_json::to_vec(value).unwrap();
    let len = bytes.len() as u32;
    BYTES.with(|b| b.set(b.get() + len));
    OUTPUT.with(|o| *o.borrow_mut() = bytes);
    len
}
pub struct Callback(pub u32);
impl Callback {
    pub fn evaluate(&self) -> u32 {
        call(3, self.0, 0)
    }
}
impl std::ops::Drop for Callback {
    fn drop(&mut self) {
        call(4, self.0, 0);
    }
}
pub fn with_context<T>(ctx: &NodeContext, event: serde_json::Value, f: impl FnOnce() -> T) -> T {
    CONTEXT.with(|c| c.borrow_mut().push((ctx.clone(), event)));
    let result = f();
    let last = CONTEXT.with(|c| c.borrow_mut().pop());
    drop(last);
    result
}
#[no_mangle]
pub extern "C" fn module_host_bridge(op: u32, a: u32, b: u32) -> u32 {
    CALLS.with(|c| c.set(c.get() + 1));
    match op {
        READ => {
            let field = get(a).borrow().clone();
            output(&field.get())
        }
        WRITE => {
            let value = receive(b);
            let field = get(a).borrow().clone();
            field.set(value);
            0
        }
        NEW => add(Field::new(a, receive(b))),
        COMPUTED => {
            let c: Computation = receive(b);
            let deps = c
                .dependencies
                .iter()
                .map(|id| get(*id).borrow().untyped())
                .collect::<Vec<_>>();
            add(Field::computed(a, c.callback, &deps))
        }
        REPLACE => {
            let x = get(a).borrow().clone();
            let y = get(b).borrow().clone();
            x.replace(&y);
            0
        }
        SUBSCRIBE => {
            let ids: Vec<u32> = receive(b);
            let deps = ids
                .iter()
                .map(|id| get(*id).borrow().untyped())
                .collect::<Vec<_>>();
            let ctx = CONTEXT.with(|c| c.borrow().last().unwrap().0.clone());
            let callback = Callback(a);
            ctx.subscribe(&deps, move || {
                callback.evaluate();
            });
            0
        }
        CURSOR => {
            let ctx = CONTEXT.with(|c| c.borrow().last().unwrap().0.clone());
            ctx.set_cursor(if a == 0 {
                pax_engine::api::cursor::CursorStyle::Auto
            } else {
                pax_engine::api::cursor::CursorStyle::Pointer
            });
            0
        }
        RETAIN => {
            retain(a);
            0
        }
        RELEASE => {
            release(a);
            0
        }
        CANCEL => {
            let f = get(a).borrow().clone();
            f.cancel();
            0
        }
        EASE => {
            let e = receive(b);
            let f = get(a).borrow().clone();
            f.ease(e);
            0
        }
        HANDLE => {
            let id = bundle::field_handle(a, b);
            retain(id);
            id
        }
        WORD => OUTPUT.with(|o| {
            let o = o.borrow();
            let mut word = [0; 4];
            for (i, v) in word.iter_mut().enumerate() {
                *v = *o.get(a as usize * 4 + i).unwrap_or(&0)
            }
            u32::from_le_bytes(word)
        }),
        REQUEST => {
            let (ctx, event) = CONTEXT.with(|c| c.borrow().last().unwrap().clone());
            let request = Request {
                bounds: add(Field::Bounds(ctx.bounds_self.clone())),
                clock: add(Field::U64(ctx.elapsed_millis.clone())),
                event,
            };
            output(&request)
        }
        _ => panic!("bridge operation"),
    }
}
#[no_mangle]
pub extern "C" fn probe_stats(which: u32) -> u32 {
    match which {
        0 => VALUES.with(|v| v.borrow().len() as u32),
        1 => call(7, 0, 0),
        2 => call(8, 0, 0),
        3 => CALLS.with(Cell::get),
        4 => BYTES.with(Cell::get),
        5 => bundle::instances(),
        _ => 0,
    }
}
