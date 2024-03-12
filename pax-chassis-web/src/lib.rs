//! Basic example of rendering in the browser

use js_sys::Uint8Array;
use log::Level;
use pax_message::ImageLoadInterruptArgs;
use pax_runtime::api::math::Point2;
use pax_runtime::api::ButtonClick;
use pax_runtime::api::CheckboxChange;
use pax_runtime::api::RenderContext;
use pax_runtime::api::TextboxChange;
use pax_runtime::api::TextboxInput;
use pax_runtime::ExpressionTable;
use std::cell::RefCell;

use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement};

use piet_web::WebRenderContext;

use pax_runtime::{PaxEngine, Renderer};

use pax_message::NativeInterrupt;
use pax_runtime::api::{
    Clap, Click, ContextMenu, DoubleClick, KeyDown, KeyPress, KeyUp, KeyboardEventArgs,
    ModifierKey, MouseButton, MouseDown, MouseEventArgs, MouseMove, MouseOut, MouseOver, MouseUp,
    Touch, TouchEnd, TouchMove, TouchStart, Wheel,
};
use serde_json;

#[cfg(feature = "designtime")]
use pax_designtime::DesigntimeManager;

#[cfg(feature = "designtime")]
const USERLAND_PROJECT_ID: &str = "userland_project";
#[cfg(feature = "designtime")]
const RUNNING_PROJECT_ID: &str = "running_project";

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
pub struct InterruptResult {
    pub prevent_default: bool,
}

#[wasm_bindgen]
impl PaxChassisWeb {
    //called from JS, this is essentially `main`
    pub fn new() -> Self {
        #[cfg(feature = "console_error_panic_hook")]
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));

        #[cfg(debug_assertions)]
        console_log::init_with_level(Level::Debug)
            .expect("console_log::init_with_level initialized correctly");
        #[cfg(not(debug_assertions))]
        console_log::init_with_level(Level::Error)
            .expect("console_log::init_with_level initialized correctly");

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
                (width, height),
                designtime_manager.clone(),
            );
            let engine_container: Rc<RefCell<PaxEngine>> = Rc::new(RefCell::new(engine));
            Self {
                engine: engine_container,
                drawing_contexts: Renderer::new(),
                definition_to_instance_traverser,
                designtime_manager,
                last_manifest_version_rendered: 0,
            }
        }
        #[cfg(not(feature = "designtime"))]
        {
            let engine = pax_runtime::PaxEngine::new(
                main_component_instance,
                expression_table,
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

    pub fn interrupt(
        &mut self,
        native_interrupt: String,
        additional_payload: &JsValue,
    ) -> InterruptResult {
        let x: NativeInterrupt = serde_json::from_str(&native_interrupt).unwrap();

        let engine = self.engine.borrow_mut();
        let globals = engine.runtime_context.globals();
        let prevent_default = match x {
            NativeInterrupt::Image(args) => match args {
                ImageLoadInterruptArgs::Reference(_ref_args) => false,
                ImageLoadInterruptArgs::Data(data_args) => {
                    let data = Uint8Array::new(additional_payload).to_vec();
                    self.drawing_contexts.load_image(
                        &data_args.path,
                        &data,
                        data_args.width,
                        data_args.height,
                    );
                    false
                }
            },
            NativeInterrupt::FormButtonClick(args) => {
                let node = engine
                    .get_expanded_node(args.id_chain[0])
                    .expect("button node exists in engine");
                node.dispatch_button_click(ButtonClick {}, globals, &engine.runtime_context)
            }
            NativeInterrupt::FormTextboxInput(args) => {
                let node = engine
                    .get_expanded_node(args.id_chain[0])
                    .expect("textbox node exists in engine");
                node.dispatch_textbox_input(
                    TextboxInput { text: args.text },
                    globals,
                    &engine.runtime_context,
                )
            }
            NativeInterrupt::FormTextboxChange(args) => {
                let node = engine
                    .get_expanded_node(args.id_chain[0])
                    .expect("textbox node exists in engine");
                node.dispatch_textbox_change(
                    TextboxChange { text: args.text },
                    globals,
                    &engine.runtime_context,
                )
            }
            NativeInterrupt::FormCheckboxToggle(args) => {
                let node = engine
                    .get_expanded_node(args.id_chain[0])
                    .expect("checkbox node exists in engine");
                node.dispatch_checkbox_change(
                    CheckboxChange {
                        checked: args.state,
                    },
                    globals,
                    &engine.runtime_context,
                )
            }

            NativeInterrupt::AddedLayer(_args) => false,
            NativeInterrupt::Click(args) => {
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_click = Click {
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
                    topmost_node.dispatch_click(args_click, globals, &engine.runtime_context)
                } else {
                    false
                }
            }
            NativeInterrupt::Scroll(_args) => false,
            NativeInterrupt::Clap(args) => {
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_clap = Clap {
                        x: args.x,
                        y: args.y,
                    };
                    topmost_node.dispatch_clap(args_clap, globals, &engine.runtime_context)
                } else {
                    false
                }
            }
            NativeInterrupt::TouchStart(args) => {
                let first_touch = args.touches.get(0).unwrap();
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(first_touch.x, first_touch.y));
                if let Some(topmost_node) = prospective_hit {
                    let touches = args.touches.iter().map(|x| Touch::from(x)).collect();
                    let args_touch_start = TouchStart { touches };
                    topmost_node.dispatch_touch_start(
                        args_touch_start,
                        globals,
                        &engine.runtime_context,
                    )
                } else {
                    false
                }
            }
            NativeInterrupt::TouchMove(args) => {
                let first_touch = args.touches.get(0).unwrap();
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(first_touch.x, first_touch.y));
                if let Some(topmost_node) = prospective_hit {
                    let touches = args.touches.iter().map(|x| Touch::from(x)).collect();
                    let args_touch_move = TouchMove { touches };
                    topmost_node.dispatch_touch_move(
                        args_touch_move,
                        globals,
                        &engine.runtime_context,
                    )
                } else {
                    false
                }
            }
            NativeInterrupt::TouchEnd(args) => {
                let first_touch = args.touches.get(0).unwrap();
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(first_touch.x, first_touch.y));
                if let Some(topmost_node) = prospective_hit {
                    let touches = args.touches.iter().map(|x| Touch::from(x)).collect();
                    let args_touch_end = TouchEnd { touches };
                    topmost_node.dispatch_touch_end(
                        args_touch_end,
                        globals,
                        &engine.runtime_context,
                    )
                } else {
                    false
                }
            }
            NativeInterrupt::KeyDown(args) => {
                let modifiers = args
                    .modifiers
                    .iter()
                    .map(|x| ModifierKey::from(x))
                    .collect();
                let args_key_down = KeyDown {
                    keyboard: KeyboardEventArgs {
                        key: args.key,
                        modifiers,
                        is_repeat: args.is_repeat,
                    },
                };
                engine.global_dispatch_key_down(args_key_down);
                false
            }
            NativeInterrupt::KeyUp(args) => {
                let modifiers = args
                    .modifiers
                    .iter()
                    .map(|x| ModifierKey::from(x))
                    .collect();
                let args_key_up = KeyUp {
                    keyboard: KeyboardEventArgs {
                        key: args.key,
                        modifiers,
                        is_repeat: args.is_repeat,
                    },
                };
                engine.global_dispatch_key_up(args_key_up);
                false
            }
            NativeInterrupt::KeyPress(args) => {
                let modifiers = args
                    .modifiers
                    .iter()
                    .map(|x| ModifierKey::from(x))
                    .collect();
                let args_key_press = KeyPress {
                    keyboard: KeyboardEventArgs {
                        key: args.key,
                        modifiers,
                        is_repeat: args.is_repeat,
                    },
                };
                engine.global_dispatch_key_press(args_key_press);
                false
            }
            NativeInterrupt::DoubleClick(args) => {
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_double_click = DoubleClick {
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
                    )
                } else {
                    false
                }
            }
            NativeInterrupt::MouseMove(args) => {
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_mouse_move = MouseMove {
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
                    )
                } else {
                    false
                }
            }
            NativeInterrupt::Wheel(args) => {
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let modifiers = args
                        .modifiers
                        .iter()
                        .map(|x| ModifierKey::from(x))
                        .collect();
                    let args_wheel = Wheel {
                        x: args.x,
                        y: args.y,
                        delta_x: args.delta_x,
                        delta_y: args.delta_y,
                        modifiers,
                    };
                    topmost_node.dispatch_wheel(args_wheel, globals, &engine.runtime_context)
                } else {
                    false
                }
            }
            NativeInterrupt::MouseDown(args) => {
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_mouse_down = MouseDown {
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
                    )
                } else {
                    false
                }
            }
            NativeInterrupt::MouseUp(args) => {
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_mouse_up = MouseUp {
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
                    topmost_node.dispatch_mouse_up(args_mouse_up, globals, &engine.runtime_context)
                } else {
                    false
                }
            }
            NativeInterrupt::MouseOver(args) => {
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_mouse_over = MouseOver {
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
                    )
                } else {
                    false
                }
            }
            NativeInterrupt::MouseOut(args) => {
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_mouse_out = MouseOut {
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
                    )
                } else {
                    false
                }
            }
            NativeInterrupt::ContextMenu(args) => {
                let prospective_hit = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y));
                if let Some(topmost_node) = prospective_hit {
                    let args_context_menu = ContextMenu {
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
                    )
                } else {
                    false
                }
            }
        };

        InterruptResult { prevent_default }
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
            let mut new_instance_nodes = vec![];
            if let Some(instance_node) = self
                .definition_to_instance_traverser
                .get_template_node_by_id(USERLAND_PROJECT_ID)
            {
                new_instance_nodes.push(instance_node);
            }
            if let Some(instance_node) = self
                .definition_to_instance_traverser
                .get_template_node_by_id(RUNNING_PROJECT_ID)
            {
                new_instance_nodes.push(instance_node);
            }
            let mut engine = self.engine.borrow_mut();
            engine.replace_instance_nodes(&new_instance_nodes);
            self.last_manifest_version_rendered = current_manifest_version;
        }
    }

    #[cfg(feature = "designtime")]
    pub fn handle_recv_designtime(&mut self) {
        self.designtime_manager
            .borrow_mut()
            .handle_recv()
            .expect("couldn't handle recv");
    }

    #[cfg(feature = "designtime")]
    pub fn designtime_tick(&mut self) {
        self.handle_recv_designtime();
        self.update_userland_component();
    }

    pub fn tick(&mut self) -> MemorySlice {
        #[cfg(feature = "designtime")]
        self.designtime_tick();

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
