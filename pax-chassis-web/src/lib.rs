//! Basic example of rendering in the browser
#![allow(non_snake_case)]

use js_sys::Uint8Array;
use pax_message::ImageLoadInterruptArgs;
use pax_runtime::api::borrow;
use pax_runtime::api::math::Point2;
use pax_runtime::api::use_RefCell;
use pax_runtime::api::ButtonClick;
use pax_runtime::api::Platform;
use pax_runtime::api::RenderContext;
use pax_runtime::api::TextboxChange;
use pax_runtime::api::OS;
use pax_runtime::DefinitionToInstanceTraverser;
use pax_runtime_api::borrow_mut;
use pax_runtime_api::Event;
use pax_runtime_api::Focus;
use pax_runtime_api::SelectStart;
use web_time::Instant;
use_RefCell!();

use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement};

use piet_web::WebRenderContext;

use pax_runtime::{PaxEngine, Renderer};

pub use {console_error_panic_hook, console_log};

use pax_message::NativeInterrupt;
use pax_runtime::api::{
    Clap, Click, ContextMenu, DoubleClick, Drop, KeyDown, KeyPress, KeyUp, KeyboardEventArgs,
    ModifierKey, MouseButton, MouseDown, MouseEventArgs, MouseMove, MouseUp, Touch, TouchEnd,
    TouchMove, TouchStart, Wheel,
};
use serde_json;

#[cfg(any(feature = "designtime", feature = "designer"))]
use {pax_designtime::orm::ReloadType, pax_designtime::DesigntimeManager};

const USERLAND_COMPONENT_ROOT: &str = "USERLAND_COMPONENT_ROOT";
#[cfg(any(feature = "designtime", feature = "designer"))]
const DESIGNER_COMPONENT_ROOT: &str = "DESIGNER_COMPONENT_ROOT";

#[wasm_bindgen]
pub fn wasm_memory() -> JsValue {
    wasm_bindgen::memory()
}

#[wasm_bindgen]
pub struct PaxChassisWeb {
    drawing_contexts: Renderer<WebRenderContext<'static>>,
    engine: Rc<RefCell<PaxEngine>>,
    #[cfg(any(feature = "designtime", feature = "designer"))]
    userland_definition_to_instance_traverser:
        Box<dyn pax_runtime::cartridge::DefinitionToInstanceTraverser>,
    #[cfg(any(feature = "designtime", feature = "designer"))]
    designtime_manager: Rc<RefCell<DesigntimeManager>>,
}

#[wasm_bindgen]
pub struct InterruptResult {
    pub prevent_default: bool,
}

// Two impl blocks: one for "private" functions,
//                  the second for FFI-exposed functions

impl PaxChassisWeb {
    #[cfg(any(feature = "designtime", feature = "designer"))]
    pub async fn new(
        userland_definition_to_instance_traverser: Box<dyn DefinitionToInstanceTraverser>,
        designer_definition_to_instance_traverser: Box<dyn DefinitionToInstanceTraverser>,
    ) -> Self {
        let (width, height, os_info, get_elapsed_millis) = Self::init_common();
        let query_string = window()
            .unwrap()
            .location()
            .search()
            .expect("no search exists");

        let main_component_instance =
            designer_definition_to_instance_traverser.get_main_component(DESIGNER_COMPONENT_ROOT);
        let userland_main_component_instance =
            userland_definition_to_instance_traverser.get_main_component(USERLAND_COMPONENT_ROOT);

        let designtime_manager = userland_definition_to_instance_traverser
            .get_designtime_manager(query_string)
            .unwrap();
        let engine = pax_runtime::PaxEngine::new_with_designtime(
            main_component_instance,
            userland_main_component_instance,
            (width, height),
            designtime_manager.clone(),
            Platform::Web,
            os_info,
            get_elapsed_millis,
        );
        let engine_container: Rc<RefCell<PaxEngine>> = Rc::new(RefCell::new(engine));
        Self {
            engine: engine_container,
            drawing_contexts: Renderer::new(),
            userland_definition_to_instance_traverser,
            designtime_manager,
        }
    }

    #[cfg(not(any(feature = "designtime", feature = "designer")))]
    pub async fn new(
        definition_to_instance_traverser: Box<dyn DefinitionToInstanceTraverser>,
    ) -> Self {
        let (width, height, os_info, get_time) = Self::init_common();

        let main_component_instance =
            definition_to_instance_traverser.get_main_component(USERLAND_COMPONENT_ROOT);
        let engine = pax_runtime::PaxEngine::new(
            main_component_instance,
            (width, height),
            Platform::Web,
            os_info,
            get_time,
        );

        let engine_container: Rc<RefCell<PaxEngine>> = Rc::new(RefCell::new(engine));

        Self {
            engine: engine_container,
            drawing_contexts: Renderer::new(),
        }
    }

    fn init_common() -> (f64, f64, OS, Box<dyn Fn() -> u128>) {
        let window = window().unwrap();
        let user_agent_str = window.navigator().user_agent().ok();
        let os_info = user_agent_str
            .and_then(|s| parse_user_agent_str(&s))
            .unwrap_or_default();

        let width = window.inner_width().unwrap().as_f64().unwrap();
        let height = window.inner_height().unwrap().as_f64().unwrap();
        let start = Instant::now();
        let get_time = Box::new(move || start.elapsed().as_millis());
        (width, height, os_info, get_time)
    }
}

#[wasm_bindgen]
impl PaxChassisWeb {
    pub fn add_context(&mut self, id: usize) {
        let window = window().unwrap();
        let dpr = window.device_pixel_ratio();
        let document = window.document().unwrap();
        let canvas = document
            .get_element_by_id(id.to_string().as_str())
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
        self.drawing_contexts.add_context(id, render_context);
        self.engine.borrow().runtime_context.add_canvas(id);
    }

    pub fn send_viewport_update(&mut self, width: f64, height: f64) {
        self.engine
            .borrow()
            .runtime_context
            .set_all_canvases_dirty();
        borrow_mut!(self.engine).set_viewport_size((width, height));
    }
    pub fn remove_context(&mut self, id: usize) {
        self.drawing_contexts.remove_context(id);
        self.engine.borrow().runtime_context.remove_canvas(&id);
    }

    pub fn get_dirty_canvases(&self) -> Vec<usize> {
        let ret = self.engine.borrow().runtime_context.get_dirty_canvases();
        ret
    }

    pub fn interrupt(
        &mut self,
        native_interrupt: String,
        additional_payload: &JsValue,
    ) -> InterruptResult {
        let x: NativeInterrupt = serde_json::from_str(&native_interrupt).unwrap();

        let engine = borrow_mut!(self.engine);
        let ctx = &engine.runtime_context;
        let globals = ctx.globals();
        let prevent_default = match &x {
            NativeInterrupt::Focus(_args) => engine.global_dispatch_focus(Focus {}),
            NativeInterrupt::DropFile(args) => {
                let data = Uint8Array::new(additional_payload).to_vec();
                let topmost_node = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y));
                let args_drop = Drop {
                    x: args.x,
                    y: args.y,
                    name: args.name.clone(),
                    mime_type: args.mime_type.clone(),
                    data,
                };
                topmost_node.dispatch_drop(Event::new(args_drop), &globals, &engine.runtime_context)
            }
            NativeInterrupt::FormRadioSetChange(args) => {
                let node = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id));
                if let Some(node) = node {
                    borrow!(node.instance_node).handle_native_interrupt(&node, &x);
                }
                false
            }
            NativeInterrupt::FormSliderChange(args) => {
                let node = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id));
                if let Some(node) = node {
                    borrow!(node.instance_node).handle_native_interrupt(&node, &x);
                }
                false
            }
            NativeInterrupt::FormDropdownChange(args) => {
                let node = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id));
                if let Some(node) = node {
                    borrow!(node.instance_node).handle_native_interrupt(&node, &x);
                }
                false
            }
            NativeInterrupt::ChassisResizeRequestCollection(collection) => {
                for args in collection {
                    let node =
                        engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id));
                    if let Some(node) = node {
                        node.chassis_resize_request(args.width, args.height);
                    }
                }
                false
            }
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
                if let Some(node) =
                    engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
                {
                    node.dispatch_button_click(
                        Event::new(ButtonClick {}),
                        &globals,
                        &engine.runtime_context,
                    )
                } else {
                    log::warn!(
                        "tried to dispatch event for button click after node already removed"
                    );
                    false
                }
            }
            NativeInterrupt::FormTextboxInput(args) => {
                if let Some(node) =
                    engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
                {
                    borrow!(node.instance_node).handle_native_interrupt(&node, &x);
                } else {
                    log::warn!(
                        "tried to dispatch event for textbox input after node already removed"
                    );
                }
                false
            }
            NativeInterrupt::TextInput(args) => {
                if let Some(node) =
                    engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
                {
                    borrow!(node.instance_node).handle_native_interrupt(&node, &x);
                } else {
                    log::warn!("tried to dispatch event for text input after node already removed");
                }
                false
            }
            NativeInterrupt::FormTextboxChange(args) => {
                if let Some(node) =
                    engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
                {
                    node.dispatch_textbox_change(
                        Event::new(TextboxChange {
                            text: args.text.clone(),
                        }),
                        &globals,
                        &engine.runtime_context,
                    )
                } else {
                    log::warn!(
                        "tried to dispatch event for textbox change after node already removed"
                    );
                    false
                }
            }
            NativeInterrupt::FormCheckboxToggle(args) => {
                if let Some(node) =
                    engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
                {
                    borrow!(node.instance_node).handle_native_interrupt(&node, &x);
                } else {
                    log::warn!(
                        "tried to dispatch event for checkbox toggle after node already removed"
                    );
                }
                false
            }

            NativeInterrupt::AddedLayer(_args) => false,
            NativeInterrupt::Click(args) => {
                let topmost_node = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y));
                let args_click = Click {
                    mouse: MouseEventArgs {
                        x: args.x,
                        y: args.y,
                        button: MouseButton::from(args.button.clone()),
                        modifiers: args
                            .modifiers
                            .iter()
                            .map(|x| ModifierKey::from(x))
                            .collect(),
                    },
                };
                topmost_node.dispatch_click(
                    Event::new(args_click),
                    &globals,
                    &engine.runtime_context,
                )
            }
            NativeInterrupt::Scrollbar(args) => {
                let node = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id));
                if let Some(node) = node {
                    borrow!(node.instance_node).handle_native_interrupt(&node, &x);
                }
                false
            }
            NativeInterrupt::Scroll(_) => false,
            NativeInterrupt::Clap(args) => {
                let topmost_node = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y));
                let args_clap = Clap {
                    x: args.x,
                    y: args.y,
                };
                topmost_node.dispatch_clap(Event::new(args_clap), &globals, &engine.runtime_context)
            }
            NativeInterrupt::TouchStart(args) => {
                let first_touch = args.touches.get(0).unwrap();
                let topmost_node = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(first_touch.x, first_touch.y));
                let touches = args.touches.iter().map(|x| Touch::from(x)).collect();
                let args_touch_start = TouchStart { touches };
                topmost_node.dispatch_touch_start(
                    Event::new(args_touch_start),
                    &globals,
                    &engine.runtime_context,
                )
            }
            NativeInterrupt::TouchMove(args) => {
                let first_touch = args.touches.get(0).unwrap();
                let topmost_node = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(first_touch.x, first_touch.y));
                let touches = args.touches.iter().map(|x| Touch::from(x)).collect();
                let args_touch_move = TouchMove { touches };
                topmost_node.dispatch_touch_move(
                    Event::new(args_touch_move),
                    &globals,
                    &engine.runtime_context,
                )
            }
            NativeInterrupt::TouchEnd(args) => {
                let first_touch = args.touches.get(0).unwrap();
                let topmost_node = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(first_touch.x, first_touch.y));
                let touches = args.touches.iter().map(|x| Touch::from(x)).collect();
                let args_touch_end = TouchEnd { touches };
                topmost_node.dispatch_touch_end(
                    Event::new(args_touch_end),
                    &globals,
                    &engine.runtime_context,
                )
            }
            NativeInterrupt::KeyDown(args) => {
                let modifiers = args
                    .modifiers
                    .iter()
                    .map(|x| ModifierKey::from(x))
                    .collect();
                let args_key_down = KeyDown {
                    keyboard: KeyboardEventArgs {
                        key: args.key.clone(),
                        modifiers,
                        is_repeat: args.is_repeat,
                    },
                };
                engine.global_dispatch_key_down(args_key_down)
            }
            NativeInterrupt::KeyUp(args) => {
                let modifiers = args
                    .modifiers
                    .iter()
                    .map(|x| ModifierKey::from(x))
                    .collect();
                let args_key_up = KeyUp {
                    keyboard: KeyboardEventArgs {
                        key: args.key.clone(),
                        modifiers,
                        is_repeat: args.is_repeat,
                    },
                };
                engine.global_dispatch_key_up(args_key_up)
            }
            NativeInterrupt::KeyPress(args) => {
                let modifiers = args
                    .modifiers
                    .iter()
                    .map(|x| ModifierKey::from(x))
                    .collect();
                let args_key_press = KeyPress {
                    keyboard: KeyboardEventArgs {
                        key: args.key.clone(),
                        modifiers,
                        is_repeat: args.is_repeat,
                    },
                };
                engine.global_dispatch_key_press(args_key_press)
            }
            NativeInterrupt::DoubleClick(args) => {
                let topmost_node = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y));
                let args_double_click = DoubleClick {
                    mouse: MouseEventArgs {
                        x: args.x,
                        y: args.y,
                        button: MouseButton::from(args.button.clone()),
                        modifiers: args
                            .modifiers
                            .iter()
                            .map(|x| ModifierKey::from(x))
                            .collect(),
                    },
                };
                topmost_node.dispatch_double_click(
                    Event::new(args_double_click),
                    &globals,
                    &engine.runtime_context,
                )
            }
            NativeInterrupt::SelectStart(_args) => {
                engine.global_dispatch_select_start(SelectStart {})
            }
            NativeInterrupt::MouseMove(args) => {
                let topmost_node = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y));
                let args_mouse_move = MouseMove {
                    mouse: MouseEventArgs {
                        x: args.x,
                        y: args.y,
                        button: MouseButton::from(args.button.clone()),
                        modifiers: args
                            .modifiers
                            .iter()
                            .map(|x| ModifierKey::from(x))
                            .collect(),
                    },
                };
                topmost_node.dispatch_mouse_move(
                    Event::new(args_mouse_move),
                    &globals,
                    &engine.runtime_context,
                )
            }
            NativeInterrupt::Wheel(args) => {
                let topmost_node = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y));
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
                topmost_node.dispatch_wheel(
                    Event::new(args_wheel),
                    &globals,
                    &engine.runtime_context,
                )
            }
            NativeInterrupt::MouseDown(args) => {
                let topmost_node = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y));
                let args_mouse_down = MouseDown {
                    mouse: MouseEventArgs {
                        x: args.x,
                        y: args.y,
                        button: MouseButton::from(args.button.clone()),
                        modifiers: args
                            .modifiers
                            .iter()
                            .map(|x| ModifierKey::from(x))
                            .collect(),
                    },
                };
                topmost_node.dispatch_mouse_down(
                    Event::new(args_mouse_down),
                    &globals,
                    &engine.runtime_context,
                )
            }
            NativeInterrupt::MouseUp(args) => {
                let topmost_node = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y));
                let args_mouse_up = MouseUp {
                    mouse: MouseEventArgs {
                        x: args.x,
                        y: args.y,
                        button: MouseButton::from(args.button.clone()),
                        modifiers: args
                            .modifiers
                            .iter()
                            .map(|x| ModifierKey::from(x))
                            .collect(),
                    },
                };
                topmost_node.dispatch_mouse_up(
                    Event::new(args_mouse_up),
                    &globals,
                    &engine.runtime_context,
                )
            }
            NativeInterrupt::ContextMenu(args) => {
                let topmost_node = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y));
                let args_context_menu = ContextMenu {
                    mouse: MouseEventArgs {
                        x: args.x,
                        y: args.y,
                        button: MouseButton::from(args.button.clone()),
                        modifiers: args
                            .modifiers
                            .iter()
                            .map(|x| ModifierKey::from(x))
                            .collect(),
                    },
                };
                topmost_node.dispatch_context_menu(
                    Event::new(args_context_menu),
                    &globals,
                    &engine.runtime_context,
                )
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

    #[cfg(any(feature = "designtime", feature = "designer"))]
    pub fn update_userland_component(&mut self) {
        let current_manifest_version =
            borrow!(self.designtime_manager).get_last_written_manifest_version();
        let mut reload_queue = borrow_mut!(self.designtime_manager).take_reload_queue();
        // erase unnecessary reloads
        if reload_queue.contains(&ReloadType::FullEdit) {
            reload_queue = vec![ReloadType::FullEdit];
        };
        if current_manifest_version.get()
            != self
                .designtime_manager
                .borrow()
                .get_last_rendered_manifest_version()
                .get()
        {
            for reload_type in reload_queue {
                match reload_type {
                    // This and FullPlay are now the same: TODO join?
                    ReloadType::FullEdit => {
                        let mut engine = borrow_mut!(self.engine);
                        let root = self
                            .userland_definition_to_instance_traverser
                            .get_main_component(USERLAND_COMPONENT_ROOT)
                            as Rc<dyn pax_runtime::InstanceNode>;
                        engine.full_reload_userland(root);
                    }
                    ReloadType::FullPlay => {
                        let root = self
                            .userland_definition_to_instance_traverser
                            .get_main_component(USERLAND_COMPONENT_ROOT)
                            as Rc<dyn pax_runtime::InstanceNode>;
                        let mut engine = borrow_mut!(self.engine);
                        engine.full_reload_userland(root);
                    }
                    ReloadType::Partial(uni) => {
                        let manifest = self
                            .userland_definition_to_instance_traverser
                            .get_manifest();
                        let containing_component = manifest
                            .components
                            .get(&uni.get_containing_component_type_id())
                            .unwrap();
                        let containing_template = containing_component.template.as_ref().unwrap();
                        let tnd = containing_template
                            .get_node(&uni.get_template_node_id())
                            .unwrap();
                        let pax_type = tnd.type_id.get_pax_type();
                        let instance_node = match pax_type {
                            pax_manifest::PaxType::If
                            | pax_manifest::PaxType::Slot
                            | pax_manifest::PaxType::Repeat => self
                                .userland_definition_to_instance_traverser
                                .build_control_flow(
                                    &uni.get_containing_component_type_id(),
                                    &uni.get_template_node_id(),
                                ),
                            _ => self
                                .userland_definition_to_instance_traverser
                                .build_template_node(
                                    &uni.get_containing_component_type_id(),
                                    &uni.get_template_node_id(),
                                ),
                        };
                        let mut engine = borrow_mut!(self.engine);
                        engine.partial_update_expanded_node(Rc::clone(&instance_node));
                    }
                }
            }
            self.designtime_manager
                .borrow_mut()
                .set_last_rendered_manifest_version(current_manifest_version.get());
        }
    }

    #[cfg(any(feature = "designtime", feature = "designer"))]
    pub fn handle_recv_designtime(&mut self) {
        borrow_mut!(self.designtime_manager)
            .handle_recv()
            .expect("couldn't handle recv");
    }

    #[cfg(any(feature = "designtime", feature = "designer"))]
    pub fn designtime_tick(&mut self) {
        self.handle_recv_designtime();
        self.update_userland_component();
    }

    pub fn tick(&mut self) -> MemorySlice {
        #[cfg(any(feature = "designtime", feature = "designer"))]
        self.designtime_tick();

        let message_queue = borrow_mut!(self.engine).tick();

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
        borrow_mut!(self.engine).render((&mut self.drawing_contexts) as &mut dyn RenderContext);
        self.engine
            .borrow()
            .runtime_context
            .clear_all_dirty_canvases();
    }

    pub fn image_loaded(&mut self, path: &str) -> bool {
        self.drawing_contexts.image_loaded(path)
    }
}

// parsing of user_agent strings could most likely be done more robustly, possibly copy some of the logic
// used in https://crates.io/crates/woothee (used server side normally, to large dep?)
// list of common user agent strings: https://deviceatlas.com/blog/list-of-user-agent-strings
fn parse_user_agent_str(user_agent: &str) -> Option<OS> {
    // example:
    //              /-----------we are cutting out this part------------\
    // Mozilla/5.0 (Linux; Android 12; SM-X906C Build/QP1A.190711.020; wv) AppleWebKit/537.36 (KHTML, like Gecko)
    // Version/4.0 Chrome/80.0.3987.119 Mobile Safari/537.36
    let platform_start = user_agent.find('(')?;
    let platform_end = platform_start + user_agent[platform_start..].find(')')?;
    let platform_str = user_agent.get(platform_start + 1..platform_end - 1)?;

    // NOTE: the ordering here is important: Android/iOS can contain Linux/MacOS strings
    const STR_PLATFORM_PAIRS: &[(&str, OS)] = &[
        ("Android", OS::Android),
        ("iPhone", OS::IPhone),
        ("Windows", OS::Windows),
        ("Mac", OS::Mac),
        ("Linux", OS::Linux),
    ];
    for (needle, plat) in STR_PLATFORM_PAIRS {
        if platform_str.contains(needle) {
            return Some(*plat);
        }
    }
    None
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
