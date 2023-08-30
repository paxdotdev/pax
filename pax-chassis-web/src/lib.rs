//! Basic example of rendering in the browser

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement};
use js_sys::Uint8Array;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

use piet_web::WebRenderContext;

use pax_core::{InstanceRegistry, PaxEngine};

use serde_json;
use pax_message::{ImageLoadInterruptArgs, NativeInterrupt};
use pax_runtime_api::{ArgsClick, ArgsContextMenu, ArgsDoubleClick, ArgsJab, ArgsKeyDown, ArgsKeyPress, ArgsKeyUp, ArgsMouseDown, ArgsMouseMove, ArgsMouseOut, ArgsMouseOver, ArgsMouseUp, ArgsScroll, ArgsTouchEnd, ArgsTouchMove, ArgsTouchStart, ArgsWheel, KeyboardEventArgs, ZIndex, ModifierKey, MouseButton, MouseEventArgs, Touch};

// Console.log support, piped from `pax_lang::log`
#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    // The `console.log` is quite polymorphic, so we can bind it with multiple
    // signatures. Note that we need to use `js_name` to ensure we always call
    // `log` in JS.
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_u32(a: u32);

    // Multiple arguments too!
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_many(a: &str, b: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

pub fn log_wrapper(msg: &str) {
    console_log!("{}", msg);
}

#[wasm_bindgen]
pub struct PaxChassisWeb {
    engine: Rc<RefCell<PaxEngine<WebRenderContext<'static>>>>,
    drawing_contexts: HashMap<String, WebRenderContext<'static>>,
}

#[wasm_bindgen]
impl PaxChassisWeb {
    //called from JS, this is essentially `main`
    pub fn new() -> Self {

        #[cfg(feature = "console_error_panic_hook")]
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        let window = window().unwrap();
        let width = window.inner_width().unwrap().as_f64().unwrap();
        let height = window.inner_height().unwrap().as_f64().unwrap();

        let instance_registry : Rc<RefCell<InstanceRegistry<WebRenderContext>>> = Rc::new(RefCell::new(InstanceRegistry::new()));
        let main_component_instance = pax_cartridge::instantiate_main_component(Rc::clone(&instance_registry));
        let expression_table = pax_cartridge::instantiate_expression_table();

        let engine = pax_core::PaxEngine::new(main_component_instance, expression_table, pax_runtime_api::PlatformSpecificLogger::Web(log_wrapper), (width, height), instance_registry);

        let engine_container : Rc<RefCell<PaxEngine<WebRenderContext>>> = Rc::new(RefCell::new(engine));

        Self {
            engine: engine_container,
            drawing_contexts: HashMap::new()
        }
    }

    pub fn add_context(&mut self, scroller_id: Option<Vec<u64>>, z_index: u32) {
        let id = PaxChassisWeb::generate_location_id(scroller_id, z_index);
        let window = window().unwrap();
        let dpr = window.device_pixel_ratio();
        let document = window.document().unwrap();
        let canvas = document.get_element_by_id(id.as_str()).unwrap().dyn_into::<HtmlCanvasElement>().unwrap();
        let context: web_sys::CanvasRenderingContext2d = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        let width = canvas.offset_width() as f64 * dpr;
        let height = canvas.offset_height() as f64 * dpr;

        canvas.set_width(width as u32);
        canvas.set_height(height as u32);
        let _ = context.scale(dpr, dpr);

        let render_context = WebRenderContext::new(context, window.clone());

        self.drawing_contexts.insert(id, render_context);
    }

    pub fn sendViewportUpdate(&mut self, width: f64, height: f64){
        self.engine.borrow_mut().set_viewport_size((width, height));
    }

    pub fn remove_context(&mut self, scroller_id: Option<Vec<u64>>, z_index: u32) {
        let id = PaxChassisWeb::generate_location_id(scroller_id, z_index);
        self.drawing_contexts.remove(&id);
    }

    pub fn interrupt(&mut self, native_interrupt: String, additional_payload: &JsValue) {
        let x : NativeInterrupt = serde_json::from_str(&native_interrupt).unwrap();
        match x {
            NativeInterrupt::Image(args) => {
                match args {
                    ImageLoadInterruptArgs::Reference(ref_args) => {}
                    ImageLoadInterruptArgs::Data(data_args) => {
                        let data = Uint8Array::new(additional_payload).to_vec();
                        (*self.engine).borrow_mut().loadImage(data_args.id_chain, data, data_args.width, data_args.height);
                    }
                }
            },
            NativeInterrupt::AddedLayer(args) => {},
            NativeInterrupt::Click(args) => {
                let prospective_hit = (*self.engine).borrow().get_topmost_element_beneath_ray((args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_click = ArgsClick {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::from(args.button),
                            modifiers: args.modifiers.iter().map(|x| { ModifierKey::from(x) }).collect(),
                        }
                    };
                    topmost_node.dispatch_click(args_click);
                }
            },
            NativeInterrupt::Scroll(args) => {
                let prospective_hit = (*self.engine).borrow().get_focused_element();
                if let Some(topmost_node) = prospective_hit {
                    let args_scroll = ArgsScroll { delta_x: args.delta_x, delta_y: args.delta_y };
                    topmost_node.dispatch_scroll(args_scroll);
                }
            },
            NativeInterrupt::Jab(args) => {
                let prospective_hit = (*self.engine).borrow().get_topmost_element_beneath_ray((args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_jab = ArgsJab { x: args.x, y: args.y };
                    topmost_node.dispatch_jab(args_jab);
                }
            }
            NativeInterrupt::TouchStart(args) => {
                let first_touch = args.touches.get(0).unwrap();
                let prospective_hit = (*self.engine).borrow().get_topmost_element_beneath_ray((first_touch.x, first_touch.y));
                if let Some(topmost_node) = prospective_hit {
                    let touches = args.touches.iter().map(|x|{Touch::from(x)}).collect();
                    let args_touch_start = ArgsTouchStart { touches };
                    topmost_node.dispatch_touch_start(args_touch_start);
                }
            }
            NativeInterrupt::TouchMove(args) => {
                let first_touch = args.touches.get(0).unwrap();
                let prospective_hit = (*self.engine).borrow().get_topmost_element_beneath_ray((first_touch.x, first_touch.y));
                if let Some(topmost_node) = prospective_hit {
                    let touches = args.touches.iter().map(|x|{Touch::from(x)}).collect();
                    let args_touch_move = ArgsTouchMove { touches };
                    topmost_node.dispatch_touch_move(args_touch_move);
                }
            }
            NativeInterrupt::TouchEnd(args) => {
                let first_touch = args.touches.get(0).unwrap();
                let prospective_hit = (*self.engine).borrow().get_topmost_element_beneath_ray((first_touch.x, first_touch.y));
                if let Some(topmost_node) = prospective_hit {
                    let touches = args.touches.iter().map(|x|{Touch::from(x)}).collect();
                    let args_touch_end = ArgsTouchEnd { touches };
                    topmost_node.dispatch_touch_end(args_touch_end);
                }
            }
            NativeInterrupt::KeyDown(args) => {
                let prospective_hit = (*self.engine).borrow().get_focused_element();
                if let Some(topmost_node) = prospective_hit {
                    let modifiers = args.modifiers.iter().map(|x|{ModifierKey::from(x)}).collect();
                    let args_key_down = ArgsKeyDown { keyboard: KeyboardEventArgs {
                        key: args.key,
                        modifiers,
                        is_repeat: args.is_repeat,
                    } };
                    topmost_node.dispatch_key_down(args_key_down);
                }
            }
            NativeInterrupt::KeyUp(args) => {
                let prospective_hit = (*self.engine).borrow().get_focused_element();
                if let Some(topmost_node) = prospective_hit {
                    let modifiers = args.modifiers.iter().map(|x|{ModifierKey::from(x)}).collect();
                    let args_key_up = ArgsKeyUp { keyboard: KeyboardEventArgs {
                        key: args.key,
                        modifiers,
                        is_repeat: args.is_repeat,
                    } };
                    topmost_node.dispatch_key_up(args_key_up);
                }
            }
            NativeInterrupt::KeyPress(args) => {
                let prospective_hit = (*self.engine).borrow().get_focused_element();
                if let Some(topmost_node) = prospective_hit {
                    let modifiers = args.modifiers.iter().map(|x|{ModifierKey::from(x)}).collect();
                    let args_key_press = ArgsKeyPress { keyboard: KeyboardEventArgs {
                        key: args.key,
                        modifiers,
                        is_repeat: args.is_repeat,
                    } };
                    topmost_node.dispatch_key_press(args_key_press);
                }
            }
            NativeInterrupt::DoubleClick(args) => {
                let prospective_hit = (*self.engine).borrow().get_topmost_element_beneath_ray((args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_double_click = ArgsDoubleClick {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::from(args.button),
                            modifiers: args.modifiers.iter().map(|x| { ModifierKey::from(x) }).collect(),
                        }
                    };
                    topmost_node.dispatch_double_click(args_double_click);
                }
            }
            NativeInterrupt::MouseMove(args) => {
                let prospective_hit = (*self.engine).borrow().get_topmost_element_beneath_ray((args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_mouse_move = ArgsMouseMove {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::from(args.button),
                            modifiers: args.modifiers.iter().map(|x| { ModifierKey::from(x) }).collect(),
                        }
                    };
                    topmost_node.dispatch_mouse_move(args_mouse_move);
                }
            }
            NativeInterrupt::Wheel(args) => {
                let prospective_hit = (*self.engine).borrow().get_topmost_element_beneath_ray((args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let modifiers = args.modifiers.iter().map(|x|{ModifierKey::from(x)}).collect();
                    let args_wheel = ArgsWheel {
                        x: args.x,
                        y: args.y,
                        delta_x: args.delta_x,
                        delta_y: args.delta_y,
                        modifiers,
                    };
                    topmost_node.dispatch_wheel(args_wheel);
                }
            }
            NativeInterrupt::MouseDown(args) => {
                let prospective_hit = (*self.engine).borrow().get_topmost_element_beneath_ray((args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_mouse_down = ArgsMouseDown {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::from(args.button),
                            modifiers: args.modifiers.iter().map(|x| { ModifierKey::from(x) }).collect(),
                        }
                    };
                    topmost_node.dispatch_mouse_down(args_mouse_down);
                }
            }
            NativeInterrupt::MouseUp(args) => {
                let prospective_hit = (*self.engine).borrow().get_topmost_element_beneath_ray((args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_mouse_up = ArgsMouseUp {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::from(args.button),
                            modifiers: args.modifiers.iter().map(|x| { ModifierKey::from(x) }).collect(),
                        }
                    };
                    topmost_node.dispatch_mouse_up(args_mouse_up);
                }
            }
            NativeInterrupt::MouseOver(args) => {
                let prospective_hit = (*self.engine).borrow().get_topmost_element_beneath_ray((args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_mouse_over = ArgsMouseOver {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::from(args.button),
                            modifiers: args.modifiers.iter().map(|x| { ModifierKey::from(x) }).collect(),
                        }
                    };
                    topmost_node.dispatch_mouse_over(args_mouse_over);
                }
            }
            NativeInterrupt::MouseOut(args) => {
                let prospective_hit = (*self.engine).borrow().get_topmost_element_beneath_ray((args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_mouse_out = ArgsMouseOut {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::from(args.button),
                            modifiers: args.modifiers.iter().map(|x| { ModifierKey::from(x) }).collect(),
                        }
                    };
                    topmost_node.dispatch_mouse_out(args_mouse_out);
                }
            }
            NativeInterrupt::ContextMenu(args) => {
                let prospective_hit = (*self.engine).borrow().get_topmost_element_beneath_ray((args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_context_menu = ArgsContextMenu {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::from(args.button),
                            modifiers: args.modifiers.iter().map(|x| { ModifierKey::from(x) }).collect(),
                        }
                    };
                    topmost_node.dispatch_context_menu(args_context_menu);
                }
            }
        };

    }

    pub fn tick(&mut self) -> String {
        let message_queue = self.engine.borrow_mut().tick(&mut self.drawing_contexts);

        //Note that this approach likely carries some CPU overhead, but may be suitable.
        //See zb lab journal `On robust message-passing to web` May 11 2022
        serde_json::to_string(&message_queue).unwrap()
    }

    pub fn generate_location_id(scroller_id: Option<Vec<u64>>, z_index: u32) -> String {
        ZIndex::generate_location_id(scroller_id, z_index)
    }

}
