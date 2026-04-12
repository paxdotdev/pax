//! Basic example of rendering in the browser
#![allow(non_snake_case)]

use js_sys::{Uint32Array, Uint8Array};
use pax_message::ImageLoadInterruptArgs;
use pax_message::ScreenshotData;
use pax_runtime::api::borrow;
use pax_runtime::api::borrow_mut;
use pax_runtime::api::math::Point2;
use pax_runtime::api::use_RefCell;
use pax_runtime::api::ButtonClick;
use pax_runtime::api::Event;
use pax_runtime::api::Focus;
use pax_runtime::api::Platform;
use pax_runtime::api::RenderContext;
use pax_runtime::api::SelectStart;
use pax_runtime::api::TextboxChange;
use pax_runtime::api::OS;
use pax_runtime::DefinitionToInstanceTraverser;
use web_time::Instant;
use_RefCell!();

mod browser_surface_policy;
pub mod web_render_contexts;

use pax_runtime::PaxEngine;
#[cfg(feature = "designtime")]
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::window;

pub use {console_error_panic_hook, console_log};

use pax_message::NativeInterrupt;
use pax_runtime::api::{
    Clap, Click, ContextMenu, DoubleClick, Drop, KeyDown, KeyPress, KeyUp, KeyboardEventArgs,
    ModifierKey, MouseButton, MouseDown, MouseEventArgs, MouseMove, MouseUp, Touch, TouchEnd,
    TouchMove, TouchStart, Wheel,
};
use serde_json;

#[cfg(feature = "designtime")]
use pax_designtime::DesigntimeManager;

const USERLAND_COMPONENT_ROOT: &str = "USERLAND_COMPONENT_ROOT";
#[cfg(feature = "designtime")]
const DESIGNER_COMPONENT_ROOT: &str = "DESIGNER_COMPONENT_ROOT";

#[cfg(feature = "designtime")]
mod dev;

#[wasm_bindgen(inline_js = r#"
const PAX_DEV_CONSOLE_TAP_KEY = "__paxDevConsoleTap";

function paxDevStringifyArg(value) {
    try {
        if (typeof value === "string") {
            return value;
        }
        if (value instanceof Error) {
            return value.stack || `${value.name}: ${value.message}`;
        }
        if (value === undefined || value === null) {
            return String(value);
        }
        if (typeof value === "object") {
            return JSON.stringify(value);
        }
        return String(value);
    } catch (_err) {
        return String(value);
    }
}

export function install_dev_console_tap(maxEntries) {
    const global = globalThis;
    const existing = global[PAX_DEV_CONSOLE_TAP_KEY];
    if (existing) {
        if (typeof maxEntries === "number" && maxEntries > 0) {
            existing.maxEntries = Math.max(existing.maxEntries, maxEntries);
        }
        return;
    }

    const methods = ["debug", "info", "log", "warn", "error"];
    const originals = {};
    const state = {
        entries: [],
        nextSeq: 1,
        maxEntries: typeof maxEntries === "number" && maxEntries > 0 ? maxEntries : 2000,
    };

    const pushEntry = (level, args) => {
        const message = args.map(paxDevStringifyArg).join(" ");
        state.entries.push({
            seq: state.nextSeq,
            level,
            message,
            timestamp_ms: Date.now(),
        });
        state.nextSeq += 1;
        if (state.entries.length > state.maxEntries) {
            state.entries.splice(0, state.entries.length - state.maxEntries);
        }
    };

    for (const level of methods) {
        const original = typeof console[level] === "function"
            ? console[level].bind(console)
            : console.log.bind(console);
        originals[level] = original;
        console[level] = (...args) => {
            pushEntry(level, args);
            original(...args);
        };
    }

    state.getEntries = (sinceSeq, limit) => {
        const filtered = typeof sinceSeq === "number" && Number.isFinite(sinceSeq) && sinceSeq >= 0
            ? state.entries.filter((entry) => entry.seq > sinceSeq)
            : state.entries.slice();
        const entries = typeof limit === "number" && limit > 0
            ? filtered.slice(-limit)
            : filtered;
        return {
            entries,
            next_seq: state.nextSeq,
            oldest_seq: state.entries.length > 0 ? state.entries[0].seq : null,
        };
    };

    global[PAX_DEV_CONSOLE_TAP_KEY] = state;
}

export function get_dev_console_entries_json(sinceSeq, limit) {
    const state = globalThis[PAX_DEV_CONSOLE_TAP_KEY];
    if (!state || typeof state.getEntries !== "function") {
        return JSON.stringify({
            entries: [],
            next_seq: 1,
            oldest_seq: null,
        });
    }
    return JSON.stringify(state.getEntries(sinceSeq, limit));
}
"#)]
extern "C" {
    fn install_dev_console_tap(max_entries: u32);
    fn get_dev_console_entries_json(since_seq: f64, limit: u32) -> String;
}

#[wasm_bindgen]
pub fn wasm_memory() -> JsValue {
    wasm_bindgen::memory()
}

fn window_location_search() -> String {
    window()
        .and_then(|window| window.location().search().ok())
        .unwrap_or_default()
}

fn query_param_value(search: &str, name: &str) -> Option<String> {
    search.trim_start_matches('?').split('&').find_map(|entry| {
        let (key, value) = entry.split_once('=')?;
        (key == name).then(|| value.to_ascii_lowercase())
    })
}

#[wasm_bindgen]
pub fn init_console_logging() {
    install_dev_console_tap(2_000);
    let search = window_location_search();
    let level = match query_param_value(&search, "pax_log").as_deref() {
        Some("trace") => log::Level::Trace,
        Some("debug") => log::Level::Debug,
        Some("info") => log::Level::Info,
        Some("warn") => log::Level::Warn,
        Some("error") => log::Level::Error,
        _ if cfg!(debug_assertions) => log::Level::Warn,
        _ => log::Level::Error,
    };
    console_log::init_with_level(level)
        .expect("console_log::init_with_level initialized correctly");
}

pub(crate) fn read_dev_console_entries_json(since_seq: Option<u64>, limit: usize) -> String {
    let since_seq = since_seq.map(|value| value as f64).unwrap_or(-1.0);
    get_dev_console_entries_json(since_seq, limit as u32)
}

#[wasm_bindgen]
pub struct PaxChassisWeb {
    render_context: Box<dyn RenderContext>,
    engine: Rc<RefCell<PaxEngine>>,
    #[cfg(feature = "designtime")]
    userland_definition_to_instance_traverser:
        Box<dyn pax_runtime::cartridge::DefinitionToInstanceTraverser>,
    #[cfg(feature = "designtime")]
    designtime_manager: Rc<RefCell<DesigntimeManager>>,
    #[cfg(feature = "designtime")]
    pending_dev_look_requests: HashMap<String, dev::PendingWebDevLookRequest>,
    #[cfg(feature = "designtime")]
    next_dev_capture_id: u32,
}

#[wasm_bindgen]
pub struct InterruptResult {
    pub prevent_default: bool,
}

// Two impl blocks: one for "private" functions,
//                  the second for FFI-exposed functions

impl PaxChassisWeb {
    #[cfg(feature = "designtime")]
    pub async fn new_designer(
        userland_definition_to_instance_traverser: Box<dyn DefinitionToInstanceTraverser>,
        designer_definition_to_instance_traverser: Box<dyn DefinitionToInstanceTraverser>,
    ) -> Self {
        let (width, height, os_info, get_elapsed_millis, renderer) = Self::init_common();
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
        let engine = pax_runtime::PaxEngine::new_with_designer(
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
            render_context: renderer,
            userland_definition_to_instance_traverser,
            designtime_manager,
            pending_dev_look_requests: HashMap::new(),
            next_dev_capture_id: 1_000_000,
        }
    }

    #[cfg(feature = "designtime")]
    pub async fn new_designtime(
        userland_definition_to_instance_traverser: Box<dyn DefinitionToInstanceTraverser>,
    ) -> Self {
        let (width, height, os_info, get_time, renderer) = Self::init_common();
        let query_string = window()
            .unwrap()
            .location()
            .search()
            .expect("no search exists");

        let userland_main_component_instance =
            userland_definition_to_instance_traverser.get_main_component(USERLAND_COMPONENT_ROOT);
        let designtime_manager = userland_definition_to_instance_traverser
            .get_designtime_manager(query_string)
            .unwrap();
        let engine = pax_runtime::PaxEngine::new_with_designtime(
            userland_main_component_instance,
            (width, height),
            designtime_manager.clone(),
            Platform::Web,
            os_info,
            get_time,
        );

        let engine_container: Rc<RefCell<PaxEngine>> = Rc::new(RefCell::new(engine));

        Self {
            engine: engine_container,
            render_context: renderer,
            userland_definition_to_instance_traverser,
            designtime_manager,
            pending_dev_look_requests: HashMap::new(),
            next_dev_capture_id: 1_000_000,
        }
    }

    #[cfg(not(feature = "designtime"))]
    pub async fn new(
        definition_to_instance_traverser: Box<dyn DefinitionToInstanceTraverser>,
    ) -> Self {
        let (width, height, os_info, get_time, renderer) = Self::init_common();

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
            render_context: renderer,
        }
    }

    fn init_common() -> (f64, f64, OS, Box<dyn Fn() -> u128>, Box<dyn RenderContext>) {
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();
        let window = window().unwrap();
        let user_agent_str = window.navigator().user_agent().ok();
        let os_info = user_agent_str
            .and_then(|s| parse_user_agent_str(&s))
            .unwrap_or_default();

        let width = window.inner_width().unwrap().as_f64().unwrap();
        let height = window.inner_height().unwrap().as_f64().unwrap();
        let start = Instant::now();
        let renderer = web_render_contexts::get_render_context(window);
        let get_time = Box::new(move || start.elapsed().as_millis());
        (width, height, os_info, get_time, Box::new(renderer))
    }

    #[cfg(feature = "designtime")]
    pub fn handle_recv_designtime(&mut self) {
        self.designtime_manager
            .borrow_mut()
            .handle_recv(self.engine.borrow().runtime_context.get_screenshot_map())
            .expect("couldn't handle recv");
    }

    #[cfg(feature = "designtime")]
    pub fn designtime_tick(&mut self) {
        self.handle_recv_designtime();
        self.collect_completed_dev_look_captures();
        self.update_userland_component();
        self.process_pending_dev_client_requests();
        self.schedule_due_dev_look_captures();
    }
}

#[wasm_bindgen]
impl PaxChassisWeb {
    pub fn send_viewport_update(&mut self, width: f64, height: f64) {
        self.engine
            .borrow()
            .runtime_context
            .set_all_canvases_dirty();
        self.render_context.resize(width as usize, height as usize);
        borrow_mut!(self.engine).set_viewport_size((width, height));
    }

    pub fn refresh_render_surfaces(&mut self) {
        let window = window().unwrap();
        let width = window.inner_width().unwrap().as_f64().unwrap_or(0.0);
        let height = window.inner_height().unwrap().as_f64().unwrap_or(0.0);
        {
            let engine = self.engine.borrow();
            engine.runtime_context.set_all_canvases_dirty();
            engine.runtime_context.mark_all_canvas_nodes_dirty();
        }
        self.render_context.resize(width as usize, height as usize);
    }

    pub fn refresh_render_surfaces_for_layers(&mut self, layer_ids: Uint32Array) {
        let mut layers = vec![0u32; layer_ids.length() as usize];
        layer_ids.copy_to(&mut layers);
        let layers: Vec<usize> = layers.into_iter().map(|layer| layer as usize).collect();
        {
            let engine = self.engine.borrow();
            for layer in &layers {
                engine.runtime_context.set_canvas_dirty(*layer);
                engine.runtime_context.mark_canvas_nodes_on_layer_dirty(*layer);
            }
        }
        self.render_context.refresh_layers(&layers);
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
                    self.render_context.load_image(
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

            NativeInterrupt::AddedLayer(args) => {
                if let Some(layer_id) = args.layer_id.map(|layer| layer as usize) {
                    engine.runtime_context.set_canvas_dirty(layer_id);
                    engine
                        .runtime_context
                        .mark_canvas_nodes_on_layer_dirty(layer_id);
                } else {
                    engine.runtime_context.set_all_canvases_dirty();
                    engine.runtime_context.mark_all_canvas_nodes_dirty();
                }
                false
            }
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
            NativeInterrupt::BrowserConfig(args) => {
                globals
                    .browser_allows_scroller_vector_layers
                    .set(args.allow_scroller_vector_layers);
                globals
                    .browser_allows_nested_scroller_vector_layers
                    .set(args.allow_nested_scroller_vector_layers);
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
            NativeInterrupt::Screenshot(args) => {
                let data = Uint8Array::new(additional_payload).to_vec();
                if let ImageLoadInterruptArgs::Data(args) = args {
                    let screenshot_data: ScreenshotData = ScreenshotData {
                        id: args.id,
                        data,
                        width: args.width,
                        height: args.height,
                    };
                    engine
                        .runtime_context
                        .load_screenshot(args.id, screenshot_data)
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

    pub fn tick(&mut self) -> MemorySlice {
        #[cfg(feature = "designtime")]
        self.designtime_tick();

        for layer_id in self.render_context.take_ready_canvas_layers() {
            {
                let engine = borrow!(self.engine);
                engine
                    .runtime_context
                    .mark_canvas_nodes_on_layer_dirty(layer_id);
                engine.runtime_context.set_canvas_dirty(layer_id);
            }
            let native_interrupt =
                format!(r#"{{"AddedLayer":{{"num_layers_added":1,"layer_id":{layer_id}}}}}"#);
            let _ = self.interrupt(native_interrupt, &JsValue::UNDEFINED);
        }
        for layer_id in self.render_context.take_replay_canvas_layers() {
            // A retained surface was reused for a different tile origin; force the runtime to
            // replay that layer's canvas nodes so the browser tile does not stay blank.
            let engine = borrow!(self.engine);
            engine
                .runtime_context
                .mark_canvas_nodes_on_layer_dirty(layer_id);
            engine.runtime_context.set_canvas_dirty(layer_id);
        }

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
        borrow_mut!(self.engine).render(self.render_context.as_mut());
    }

    pub fn request_layer_screenshot(&mut self, layer: usize, request_id: u32) {
        self.render_context
            .request_layer_screenshot(layer, request_id);
    }

    pub fn take_layer_screenshot(&mut self, layer: usize, request_id: u32) -> JsValue {
        self.render_context
            .take_layer_screenshot(layer, request_id)
            .and_then(|capture| serde_wasm_bindgen::to_value(&capture).ok())
            .unwrap_or(JsValue::NULL)
    }

    pub fn image_loaded(&mut self, path: &str) -> bool {
        self.render_context.image_loaded(path)
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
