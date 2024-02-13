//! Basic example of rendering in the browser

use js_sys::Uint8Array;
use pax_runtime::api::ArgsButtonClick;
use pax_runtime::api::ArgsCheckboxChange;
use pax_runtime::api::ArgsTextboxChange;
use pax_runtime::api::RenderContext;
use pax_runtime::ExpressionTable;
use std::cell::RefCell;

use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement};

use piet_web::WebRenderContext;

use pax_runtime::{PaxEngine, Renderer};

use pax_message::{ImageLoadInterruptArgs, NativeInterrupt};
use pax_runtime::api::{
    ArgsClap, ArgsClick, ArgsContextMenu, ArgsDoubleClick, ArgsKeyDown, ArgsKeyPress, ArgsKeyUp,
    ArgsMouseDown, ArgsMouseMove, ArgsMouseOut, ArgsMouseOver, ArgsMouseUp, ArgsScroll,
    ArgsTouchEnd, ArgsTouchMove, ArgsTouchStart, ArgsWheel, KeyboardEventArgs, ModifierKey,
    MouseButton, MouseEventArgs, Touch,
};
use serde_json;

#[cfg(feature = "designtime")]
use pax_designtime::DesigntimeManager;

#[cfg(feature = "designtime")]
const USERLAND_PROJECT_ID: &str = "userland_project";

// Console.log support, piped from `pax_engine::log`
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
pub fn wasm_memory() -> JsValue {
    wasm_bindgen::memory()
}

#[wasm_bindgen]
pub struct PaxChassisWeb {
    drawing_contexts: Renderer<WebRenderContext<'static>>,
    engine: Rc<RefCell<PaxEngine>>,
    #[cfg(feature = "designtime")]
    definition_to_instance_traverser: pax_cartridge::DefinitionToInstanceTraverser,
    #[cfg(feature = "designtime")]
    designtime_manager: Rc<RefCell<DesigntimeManager>>,
    #[cfg(feature = "designtime")]
    last_manifest_version_rendered: usize,
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

        let mut definition_to_instance_traverser =
            pax_cartridge::DefinitionToInstanceTraverser::new();
        let main_component_instance = definition_to_instance_traverser.get_main_component();
        let expression_table = ExpressionTable {
            table: pax_cartridge::instantiate_expression_table(),
        };

        #[cfg(feature = "designtime")]
        {
            let designtime_manager = definition_to_instance_traverser.get_designtime_manager();
            let engine = pax_runtime::PaxEngine::new_with_designtime(
                main_component_instance,
                expression_table,
                pax_runtime::api::PlatformSpecificLogger::Web(log_wrapper),
                (width, height),
                designtime_manager.clone(),
            );
            let engine_container: Rc<RefCell<PaxEngine>> = Rc::new(RefCell::new(engine));
            Self {
                engine: engine_container,
                drawing_contexts: Renderer::new(),
                definition_to_instance_traverser,
                designtime_manager: designtime_manager,
                last_manifest_version_rendered: 0,
            }
        }
        #[cfg(not(feature = "designtime"))]
        {
            let engine = pax_runtime::PaxEngine::new(
                main_component_instance,
                expression_table,
                pax_runtime::api::PlatformSpecificLogger::Web(log_wrapper),
                (width, height),
            );

            let engine_container: Rc<RefCell<PaxEngine>> = Rc::new(RefCell::new(engine));

            Self {
                engine: engine_container,
                drawing_contexts: Renderer::new(),
            }
        }
    }

    pub fn add_context(&mut self, id: String) {
        let window = window().unwrap();
        let dpr = window.device_pixel_ratio();
        let document = window.document().unwrap();
        let canvas = document
            .get_element_by_id(id.as_str())
            .unwrap()
            .dyn_into::<HtmlCanvasElement>()
            .unwrap();
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
        self.drawing_contexts.add_context(&id, render_context);
    }

    pub fn send_viewport_update(&mut self, width: f64, height: f64) {
        self.engine.borrow_mut().set_viewport_size((width, height));
    }
    pub fn remove_context(&mut self, id: String) {
        self.drawing_contexts.remove_context(&id);
    }

    pub fn interrupt(&mut self, native_interrupt: String, additional_payload: &JsValue) {
        let x: NativeInterrupt = serde_json::from_str(&native_interrupt).unwrap();

        let engine = self.engine.borrow_mut();
        let globals = engine.runtime_context.globals();
        match x {
            NativeInterrupt::Image(args) => match args {
                ImageLoadInterruptArgs::Reference(_ref_args) => {}
                ImageLoadInterruptArgs::Data(data_args) => {
                    let data = Uint8Array::new(additional_payload).to_vec();
                    self.drawing_contexts.load_image(
                        &data_args.path,
                        &data,
                        data_args.width,
                        data_args.height,
                    );
                }
            },
            NativeInterrupt::FormButtonClick(args) => {
                let node = engine
                    .get_expanded_node(args.id_chain[0])
                    .expect("button node exists in engine");
                node.dispatch_button_click(ArgsButtonClick {}, globals, &engine.runtime_context);
            }
            NativeInterrupt::FormTextboxChange(args) => {
                let node = engine
                    .get_expanded_node(args.id_chain[0])
                    .expect("textbox node exists in engine");
                node.dispatch_textbox_change(
                    ArgsTextboxChange { text: args.text },
                    globals,
                    &engine.runtime_context,
                );
            }
            NativeInterrupt::FormCheckboxToggle(args) => {
                let node = engine
                    .get_expanded_node(args.id_chain[0])
                    .expect("checkbox node exists in engine");
                node.dispatch_checkbox_change(
                    ArgsCheckboxChange {
                        checked: args.state,
                    },
                    globals,
                    &engine.runtime_context,
                );
            }

            NativeInterrupt::AddedLayer(_args) => {}
            NativeInterrupt::Click(args) => {
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray((args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_click = ArgsClick {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::from(args.button),
                            modifiers: args
                                .modifiers
                                .iter()
                                .map(|x| ModifierKey::from(x))
                                .collect(),
                        },
                    };
                    topmost_node.dispatch_click(args_click, globals, &engine.runtime_context);
                }
            }
            NativeInterrupt::Scroll(args) => {
                let prospective_hit = engine.get_focused_element();
                if let Some(topmost_node) = prospective_hit {
                    let args_scroll = ArgsScroll {
                        delta_x: args.delta_x,
                        delta_y: args.delta_y,
                    };
                    topmost_node.dispatch_scroll(args_scroll, globals, &engine.runtime_context);
                }
            }
            NativeInterrupt::Clap(args) => {
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray((args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_clap = ArgsClap {
                        x: args.x,
                        y: args.y,
                    };
                    topmost_node.dispatch_clap(args_clap, globals, &engine.runtime_context);
                }
            }
            NativeInterrupt::TouchStart(args) => {
                let first_touch = args.touches.get(0).unwrap();
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray((first_touch.x, first_touch.y));
                if let Some(topmost_node) = prospective_hit {
                    let touches = args.touches.iter().map(|x| Touch::from(x)).collect();
                    let args_touch_start = ArgsTouchStart { touches };
                    topmost_node.dispatch_touch_start(
                        args_touch_start,
                        globals,
                        &engine.runtime_context,
                    );
                }
            }
            NativeInterrupt::TouchMove(args) => {
                let first_touch = args.touches.get(0).unwrap();
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray((first_touch.x, first_touch.y));
                if let Some(topmost_node) = prospective_hit {
                    let touches = args.touches.iter().map(|x| Touch::from(x)).collect();
                    let args_touch_move = ArgsTouchMove { touches };
                    topmost_node.dispatch_touch_move(
                        args_touch_move,
                        globals,
                        &engine.runtime_context,
                    );
                }
            }
            NativeInterrupt::TouchEnd(args) => {
                let first_touch = args.touches.get(0).unwrap();
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray((first_touch.x, first_touch.y));
                if let Some(topmost_node) = prospective_hit {
                    let touches = args.touches.iter().map(|x| Touch::from(x)).collect();
                    let args_touch_end = ArgsTouchEnd { touches };
                    topmost_node.dispatch_touch_end(
                        args_touch_end,
                        globals,
                        &engine.runtime_context,
                    );
                }
            }
            NativeInterrupt::KeyDown(args) => {
                let prospective_hit = engine.get_focused_element();
                if let Some(topmost_node) = prospective_hit {
                    let modifiers = args
                        .modifiers
                        .iter()
                        .map(|x| ModifierKey::from(x))
                        .collect();
                    let args_key_down = ArgsKeyDown {
                        keyboard: KeyboardEventArgs {
                            key: args.key,
                            modifiers,
                            is_repeat: args.is_repeat,
                        },
                    };
                    topmost_node.dispatch_key_down(args_key_down, globals, &engine.runtime_context);
                }
            }
            NativeInterrupt::KeyUp(args) => {
                let prospective_hit = engine.get_focused_element();
                if let Some(topmost_node) = prospective_hit {
                    let modifiers = args
                        .modifiers
                        .iter()
                        .map(|x| ModifierKey::from(x))
                        .collect();
                    let args_key_up = ArgsKeyUp {
                        keyboard: KeyboardEventArgs {
                            key: args.key,
                            modifiers,
                            is_repeat: args.is_repeat,
                        },
                    };
                    topmost_node.dispatch_key_up(args_key_up, globals, &engine.runtime_context);
                }
            }
            NativeInterrupt::KeyPress(args) => {
                let prospective_hit = engine.get_focused_element();
                if let Some(topmost_node) = prospective_hit {
                    let modifiers = args
                        .modifiers
                        .iter()
                        .map(|x| ModifierKey::from(x))
                        .collect();
                    let args_key_press = ArgsKeyPress {
                        keyboard: KeyboardEventArgs {
                            key: args.key,
                            modifiers,
                            is_repeat: args.is_repeat,
                        },
                    };
                    topmost_node.dispatch_key_press(
                        args_key_press,
                        globals,
                        &engine.runtime_context,
                    );
                }
            }
            NativeInterrupt::DoubleClick(args) => {
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray((args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_double_click = ArgsDoubleClick {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::from(args.button),
                            modifiers: args
                                .modifiers
                                .iter()
                                .map(|x| ModifierKey::from(x))
                                .collect(),
                        },
                    };
                    topmost_node.dispatch_double_click(
                        args_double_click,
                        globals,
                        &engine.runtime_context,
                    );
                }
            }
            NativeInterrupt::MouseMove(args) => {
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray((args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_mouse_move = ArgsMouseMove {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::from(args.button),
                            modifiers: args
                                .modifiers
                                .iter()
                                .map(|x| ModifierKey::from(x))
                                .collect(),
                        },
                    };
                    topmost_node.dispatch_mouse_move(
                        args_mouse_move,
                        globals,
                        &engine.runtime_context,
                    );
                }
            }
            NativeInterrupt::Wheel(args) => {
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray((args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let modifiers = args
                        .modifiers
                        .iter()
                        .map(|x| ModifierKey::from(x))
                        .collect();
                    let args_wheel = ArgsWheel {
                        x: args.x,
                        y: args.y,
                        delta_x: args.delta_x,
                        delta_y: args.delta_y,
                        modifiers,
                    };
                    topmost_node.dispatch_wheel(args_wheel, globals, &engine.runtime_context);
                }
            }
            NativeInterrupt::MouseDown(args) => {
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray((args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_mouse_down = ArgsMouseDown {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::from(args.button),
                            modifiers: args
                                .modifiers
                                .iter()
                                .map(|x| ModifierKey::from(x))
                                .collect(),
                        },
                    };
                    topmost_node.dispatch_mouse_down(
                        args_mouse_down,
                        globals,
                        &engine.runtime_context,
                    );
                }
            }
            NativeInterrupt::MouseUp(args) => {
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray((args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_mouse_up = ArgsMouseUp {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::from(args.button),
                            modifiers: args
                                .modifiers
                                .iter()
                                .map(|x| ModifierKey::from(x))
                                .collect(),
                        },
                    };
                    topmost_node.dispatch_mouse_up(args_mouse_up, globals, &engine.runtime_context);
                }
            }
            NativeInterrupt::MouseOver(args) => {
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray((args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_mouse_over = ArgsMouseOver {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::from(args.button),
                            modifiers: args
                                .modifiers
                                .iter()
                                .map(|x| ModifierKey::from(x))
                                .collect(),
                        },
                    };
                    topmost_node.dispatch_mouse_over(
                        args_mouse_over,
                        globals,
                        &engine.runtime_context,
                    );
                }
            }
            NativeInterrupt::MouseOut(args) => {
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray((args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_mouse_out = ArgsMouseOut {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::from(args.button),
                            modifiers: args
                                .modifiers
                                .iter()
                                .map(|x| ModifierKey::from(x))
                                .collect(),
                        },
                    };
                    topmost_node.dispatch_mouse_out(
                        args_mouse_out,
                        globals,
                        &engine.runtime_context,
                    );
                }
            }
            NativeInterrupt::ContextMenu(args) => {
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray((args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_context_menu = ArgsContextMenu {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::from(args.button),
                            modifiers: args
                                .modifiers
                                .iter()
                                .map(|x| ModifierKey::from(x))
                                .collect(),
                        },
                    };
                    topmost_node.dispatch_context_menu(
                        args_context_menu,
                        globals,
                        &engine.runtime_context,
                    );
                }
            }
        };
    }

    pub fn deallocate(&mut self, slice: MemorySlice) {
        let layout = std::alloc::Layout::from_size_align(slice.len(), 1).unwrap();
        unsafe {
            std::alloc::dealloc(slice.ptr() as *mut u8, layout);
        }
    }

    #[cfg(feature = "designtime")]
    pub fn update_userland_component(&mut self) {
        let current_manifest_version = self.designtime_manager.borrow().get_manifest_version();
        if current_manifest_version != self.last_manifest_version_rendered {
            if let Some(instance_node) = self
                .definition_to_instance_traverser
                .get_template_node_by_id(USERLAND_PROJECT_ID)
            {
                let mut engine = self.engine.borrow_mut();
                engine.replace_by_id(USERLAND_PROJECT_ID, instance_node);
            }
            self.last_manifest_version_rendered = current_manifest_version;
        }
    }

    pub fn tick(&mut self) -> MemorySlice {
        #[cfg(feature = "designtime")]
        self.update_userland_component();

        let message_queue = self.engine.borrow_mut().tick();

        // Serialize data to a JSON string
        let json_string = serde_json::to_string(&message_queue).unwrap();

        // Convert the string into bytes
        let bytes = json_string.as_bytes();

        // Allocate space in the WebAssembly memory
        let layout = std::alloc::Layout::from_size_align(bytes.len(), 1).unwrap();
        let ptr = unsafe { std::alloc::alloc(layout) as *mut u8 };

        // Copy the data into the WebAssembly memory
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        }

        MemorySlice {
            ptr: ptr as *const u8,
            len: bytes.len(),
        }
    }

    pub fn render(&mut self) {
        self.engine
            .borrow_mut()
            .render((&mut self.drawing_contexts) as &mut dyn RenderContext);
    }

    pub fn image_loaded(&mut self, path: &str) -> bool {
        self.drawing_contexts.image_loaded(path)
    }
}

#[wasm_bindgen]
pub struct MemorySlice {
    ptr: *const u8,
    len: usize,
}

#[wasm_bindgen]
impl MemorySlice {
    pub fn ptr(&self) -> *const u8 {
        self.ptr
    }

    pub fn len(&self) -> usize {
        self.len
    }
}
