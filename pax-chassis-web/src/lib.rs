//! Basic example of rendering in the browser
#![allow(non_snake_case)]

use js_sys::{Array, Object, Reflect, Uint32Array, Uint8Array};
use pax_message::{
    AccelInterruptArgs, AddedLayerArgs, BrowserConfigInterruptArgs, ChassisResizeRequestArgs,
    ClickInterruptArgs, ContextMenuInterruptArgs, DoubleClickInterruptArgs, DropFileArgs,
    FocusInterruptArgs, FormButtonClickArgs, FormCheckboxToggleArgs, FormDropdownChangeArgs,
    FormRadioListChangeArgs, FormSliderChangeArgs, FormTextboxChangeArgs, FormTextboxInputArgs,
    GyroInterruptArgs, ImageDataArgs, ImageLoadInterruptArgs, ImagePointerArgs,
    KeyDownInterruptArgs, KeyPressInterruptArgs, KeyUpInterruptArgs, ModifierKeyMessage,
    MouseButtonMessage, MouseDownInterruptArgs, MouseMoveInterruptArgs, MouseUpInterruptArgs,
    NativeInterrupt, PhotoPickerAssetArgs, PhotoPickerInterruptArgs, RenderSurfaceUpdateArgs,
    RouteChangeInterruptArgs, ScreenshotData, ScrollInterruptArgs, ScrollerPositionInterruptArgs,
    SelectStartArgs, TapInterruptArgs, TextInputArgs, TouchEndInterruptArgs, TouchMessage,
    TouchMoveInterruptArgs, TouchStartInterruptArgs, ViewportResizeArgs, VisualViewportUpdateArgs,
    WheelInterruptArgs,
};
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
use pax_runtime::api::OS;
use pax_runtime::api::{Accel, Gyro};
use pax_runtime::api::{PhotoPickerChange, TextboxChange, TextboxInput};
use pax_runtime::engine::layer_tiling::scroller_canvas_plan_with_policy;
use pax_runtime::DefinitionToInstanceTraverser;
use web_time::Instant;
use_RefCell!();

mod browser_surface_policy;
pub mod web_render_contexts;

use crate::browser_surface_policy::BrowserSurfacePolicy;
use pax_runtime::PaxEngine;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::window;

pub use {console_error_panic_hook, console_log};

use pax_runtime::api::{
    Click, ContextMenu, DoubleClick, Drop, KeyDown, KeyPress, KeyUp, KeyboardEventArgs,
    ModifierKey, MouseButton, MouseDown, MouseEventArgs, MouseMove, MouseUp, Scroll, Touch,
    TouchEnd, TouchMove, TouchStart, Wheel,
};

#[cfg(feature = "designtime")]
use pax_designtime::DesigntimeManager;

const USERLAND_COMPONENT_ROOT: &str = "USERLAND_COMPONENT_ROOT";
#[cfg(feature = "designtime")]
const DESIGNER_COMPONENT_ROOT: &str = "DESIGNER_COMPONENT_ROOT";

#[cfg(feature = "designtime")]
mod dev;

#[derive(Clone, Copy)]
struct SyntheticScrollGesture {
    touch_identifier: i64,
    target_id: pax_runtime::ExpandedNodeIdentifier,
}

fn node_is_in_scroller_subtree(node: &Rc<pax_runtime::ExpandedNode>) -> bool {
    let mut current = Some(Rc::clone(node));
    while let Some(candidate) = current {
        if borrow!(candidate.instance_node).scrolls_content(&candidate) {
            return true;
        }
        current = candidate.template_parent.upgrade();
    }
    false
}

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

#[cfg(feature = "designtime")]
pub(crate) fn read_dev_console_entries_json(since_seq: Option<u64>, limit: usize) -> String {
    let since_seq = since_seq.map(|value| value as f64).unwrap_or(-1.0);
    get_dev_console_entries_json(since_seq, limit as u32)
}

#[wasm_bindgen]
pub struct PaxChassisWeb {
    render_context: Box<dyn RenderContext>,
    engine: Rc<RefCell<PaxEngine>>,
    native_scroller_positions: HashMap<u32, (f64, f64)>,
    synthetic_scroll_gesture: Option<SyntheticScrollGesture>,
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
    fn drain_render_surface_updates(&mut self) {
        for layer_id in self.render_context.take_ready_canvas_layers() {
            let engine = borrow!(self.engine);
            engine
                .runtime_context
                .mark_canvas_nodes_on_layer_dirty(layer_id);
            engine.runtime_context.set_canvas_dirty(layer_id);
        }
        for update in self.render_context.take_replay_canvas_layer_updates() {
            // A retained surface was reused for a different tile origin or resized host; force the
            // runtime to replay affected canvas nodes before the frame renders.
            let engine = borrow!(self.engine);
            if let Some(node_ids) = update.node_ids {
                engine
                    .runtime_context
                    .mark_canvas_nodes_on_layer_dirty_by_id(update.layer, &node_ids);
            } else {
                engine
                    .runtime_context
                    .mark_canvas_nodes_on_layer_dirty(update.layer);
            }
            engine.runtime_context.set_canvas_dirty(update.layer);
        }
    }

    fn refresh_render_surface(&mut self, layer_id: Option<u32>) {
        let engine = borrow!(self.engine);
        if let Some(layer_id) = layer_id.map(|layer| layer as usize) {
            self.render_context.refresh_layers(&[layer_id]);
            engine.runtime_context.set_canvas_dirty(layer_id);
            engine
                .runtime_context
                .mark_canvas_nodes_on_layer_dirty(layer_id);
        } else {
            let window = window().unwrap();
            let width = window.inner_width().unwrap().as_f64().unwrap_or(0.0);
            let height = window.inner_height().unwrap().as_f64().unwrap_or(0.0);
            self.render_context.resize(width as usize, height as usize);
            engine.runtime_context.set_all_canvases_dirty();
            engine.runtime_context.mark_all_canvas_nodes_dirty();
        }
        drop(engine);
        self.drain_render_surface_updates();
    }

    #[cfg(feature = "designtime")]
    pub async fn new_designer(
        userland_definition_to_instance_traverser: Box<dyn DefinitionToInstanceTraverser>,
        designer_definition_to_instance_traverser: Box<dyn DefinitionToInstanceTraverser>,
    ) -> Self {
        let (width, height, os_info, surface_policy, get_elapsed_millis, renderer) =
            Self::init_common();
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
            surface_policy.scroller_tiling_policy(),
        );
        let engine_container: Rc<RefCell<PaxEngine>> = Rc::new(RefCell::new(engine));
        Self {
            engine: engine_container,
            render_context: renderer,
            native_scroller_positions: HashMap::new(),
            synthetic_scroll_gesture: None,
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
        let (width, height, os_info, surface_policy, get_time, renderer) = Self::init_common();
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
            surface_policy.scroller_tiling_policy(),
        );

        let engine_container: Rc<RefCell<PaxEngine>> = Rc::new(RefCell::new(engine));

        Self {
            engine: engine_container,
            render_context: renderer,
            native_scroller_positions: HashMap::new(),
            synthetic_scroll_gesture: None,
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
        let (width, height, os_info, surface_policy, get_time, renderer) = Self::init_common();

        let main_component_instance =
            definition_to_instance_traverser.get_main_component(USERLAND_COMPONENT_ROOT);
        let engine = pax_runtime::PaxEngine::new(
            main_component_instance,
            (width, height),
            Platform::Web,
            os_info,
            get_time,
            surface_policy.scroller_tiling_policy(),
        );

        let engine_container: Rc<RefCell<PaxEngine>> = Rc::new(RefCell::new(engine));

        Self {
            engine: engine_container,
            render_context: renderer,
            native_scroller_positions: HashMap::new(),
            synthetic_scroll_gesture: None,
        }
    }

    fn init_common() -> (
        f64,
        f64,
        OS,
        BrowserSurfacePolicy,
        Box<dyn Fn() -> u128>,
        Box<dyn RenderContext>,
    ) {
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();
        let window = window().unwrap();
        let user_agent_str = window.navigator().user_agent().ok();
        let os_info = user_agent_str
            .and_then(|s| parse_user_agent_str(&s))
            .unwrap_or_default();

        let width = window.inner_width().unwrap().as_f64().unwrap();
        let height = window.inner_height().unwrap().as_f64().unwrap();
        let surface_policy = BrowserSurfacePolicy::detect(&window);
        let start = Instant::now();
        let renderer = web_render_contexts::get_render_context(window, surface_policy);
        let get_time = Box::new(move || start.elapsed().as_millis());
        (
            width,
            height,
            os_info,
            surface_policy,
            get_time,
            Box::new(renderer),
        )
    }

    #[cfg(feature = "designtime")]
    pub fn handle_recv_designtime(&mut self) {
        self.designtime_manager
            .borrow_mut()
            .handle_recv(self.engine.borrow().runtime_context.get_screenshot_map())
            .unwrap_or_else(|err| log::warn!("designtime receive failed: {err:?}"));
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
                engine
                    .runtime_context
                    .mark_canvas_nodes_on_layer_dirty(*layer);
            }
        }
        self.render_context.refresh_layers(&layers);
    }

    pub fn interrupt(
        &mut self,
        native_interrupt: JsValue,
        additional_payload: &JsValue,
    ) -> InterruptResult {
        let x = native_interrupt_from_js(native_interrupt);

        let mut engine = borrow_mut!(self.engine);
        let ctx = &engine.runtime_context;
        let globals = ctx.globals();
        let prevent_default = match &x {
            NativeInterrupt::Focus(_args) => engine.global_dispatch_focus(Focus {}),
            NativeInterrupt::DropFile(args) => {
                let data = Uint8Array::new(additional_payload).to_vec();
                if let Some(topmost_node) = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y))
                {
                    let args_drop = Drop {
                        x: args.x,
                        y: args.y,
                        name: args.name.clone(),
                        mime_type: args.mime_type.clone(),
                        data,
                    };
                    topmost_node.dispatch_drop(
                        Event::new(args_drop),
                        &globals,
                        &engine.runtime_context,
                    )
                } else {
                    false
                }
            }
            NativeInterrupt::FormRadioListChange(args) => {
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
            NativeInterrupt::PhotoPicker(args) => {
                let mut args = args.clone();
                fill_photo_picker_bytes_from_js(&mut args, additional_payload);
                if let Some(node) =
                    engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
                {
                    node.dispatch_photo_picker_change(
                        Event::new(PhotoPickerChange::from(&args)),
                        &globals,
                        &engine.runtime_context,
                    )
                } else {
                    log::warn!(
                        "tried to dispatch event for photo picker after node already removed"
                    );
                    false
                }
            }
            NativeInterrupt::FormTextboxInput(args) => {
                if let Some(node) =
                    engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
                {
                    borrow!(node.instance_node).handle_native_interrupt(&node, &x);
                    node.dispatch_textbox_input(
                        Event::new(TextboxInput {
                            text: args.text.clone(),
                        }),
                        &globals,
                        &engine.runtime_context,
                    )
                } else {
                    log::warn!(
                        "tried to dispatch event for textbox input after node already removed"
                    );
                    false
                }
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
                drop(engine);
                self.refresh_render_surface(args.layer_id);
                false
            }
            NativeInterrupt::RenderSurfaceUpdate(args) => {
                drop(engine);
                self.refresh_render_surface(args.layer_id);
                false
            }
            NativeInterrupt::Click(args) => {
                if let Some(topmost_node) = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y))
                {
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
                } else {
                    false
                }
            }
            NativeInterrupt::ScrollerPosition(args) => {
                let node = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id));
                if let Some(node) = node {
                    let presentation_scroll_x = args.presentation_scroll_x.unwrap_or(args.scroll_x);
                    let presentation_scroll_y = args.presentation_scroll_y.unwrap_or(args.scroll_y);
                    engine.runtime_context.update_scroller_surface_scroll(
                        args.id,
                        args.scroll_x,
                        args.scroll_y,
                        presentation_scroll_x,
                        presentation_scroll_y,
                    );
                    let previous = self
                        .native_scroller_positions
                        .insert(args.id, (presentation_scroll_x, presentation_scroll_y));
                    borrow!(node.instance_node).handle_native_interrupt(&node, &x);
                    if let Some((previous_x, previous_y)) = previous {
                        let delta_x = presentation_scroll_x - previous_x;
                        let delta_y = presentation_scroll_y - previous_y;
                        if delta_x.abs() > f64::EPSILON || delta_y.abs() > f64::EPSILON {
                            node.dispatch_scroll(
                                Event::new(Scroll { delta_x, delta_y }),
                                &globals,
                                &engine.runtime_context,
                            )
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    self.native_scroller_positions.remove(&args.id);
                    false
                }
            }
            NativeInterrupt::BrowserConfig(args) => {
                globals
                    .browser_allows_scroller_vector_layers
                    .set(args.allow_scroller_vector_layers);
                globals
                    .browser_allows_nested_scroller_vector_layers
                    .set(args.allow_nested_scroller_vector_layers);
                engine.runtime_context.mark_occlusion_dirty();
                false
            }
            NativeInterrupt::ViewportResize(args) => {
                engine.runtime_context.set_all_canvases_dirty();
                self.render_context
                    .resize(args.width as usize, args.height as usize);
                engine.set_viewport_size((args.width, args.height));
                false
            }
            NativeInterrupt::RouteChange(args) => {
                globals.route_location.set(args.into());
                false
            }
            NativeInterrupt::VisualViewportUpdate(args) => {
                engine.runtime_context.set_visual_viewport_state(
                    pax_runtime::VisualViewportState {
                        width: args.width,
                        height: args.height,
                        offset_x: args.offset_x,
                        offset_y: args.offset_y,
                        page_scroll_x: args.page_scroll_x,
                        page_scroll_y: args.page_scroll_y,
                    },
                );
                false
            }
            NativeInterrupt::Gyro(args) => {
                let gyro = Gyro {
                    x: args.x,
                    y: args.y,
                    z: args.z,
                };
                globals.gyro.set_if_neq(gyro);
                engine.global_dispatch_gyro(gyro)
            }
            NativeInterrupt::Accel(args) => {
                let accel = Accel {
                    x: args.x,
                    y: args.y,
                    z: args.z,
                };
                globals.accel.set_if_neq(accel);
                engine.global_dispatch_accel(accel)
            }
            NativeInterrupt::Scroll(args) => {
                if let Some(topmost_node) = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y))
                {
                    topmost_node.dispatch_scroll(
                        Event::new(Scroll {
                            delta_x: args.delta_x,
                            delta_y: args.delta_y,
                        }),
                        &globals,
                        &engine.runtime_context,
                    )
                } else {
                    false
                }
            }
            NativeInterrupt::Tap(args) => {
                if let Some(topmost_node) = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y))
                {
                    let args_tap = Click {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::Left,
                            modifiers: vec![],
                        },
                    };
                    topmost_node.dispatch_tap(
                        Event::new(args_tap),
                        &globals,
                        &engine.runtime_context,
                    )
                } else {
                    false
                }
            }
            NativeInterrupt::TouchStart(args) => {
                self.synthetic_scroll_gesture = None;
                if let Some(first_touch) = args.touches.first() {
                    if let Some(topmost_node) = engine
                        .runtime_context
                        .get_topmost_element_beneath_ray(Point2::new(first_touch.x, first_touch.y))
                    {
                        if args.touches.len() == 1 && !node_is_in_scroller_subtree(&topmost_node) {
                            self.synthetic_scroll_gesture = Some(SyntheticScrollGesture {
                                touch_identifier: first_touch.identifier,
                                target_id: topmost_node.id,
                            });
                        }
                        let touches = args.touches.iter().map(|x| Touch::from(x)).collect();
                        let args_touch_start = TouchStart { touches };
                        topmost_node.dispatch_touch_start(
                            Event::new(args_touch_start),
                            &globals,
                            &engine.runtime_context,
                        )
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            NativeInterrupt::TouchMove(args) => {
                if let Some(first_touch) = args.touches.first() {
                    let mut prevented = if let Some(topmost_node) = engine
                        .runtime_context
                        .get_topmost_element_beneath_ray(Point2::new(first_touch.x, first_touch.y))
                    {
                        let touches = args.touches.iter().map(|x| Touch::from(x)).collect();
                        let args_touch_move = TouchMove { touches };
                        topmost_node.dispatch_touch_move(
                            Event::new(args_touch_move),
                            &globals,
                            &engine.runtime_context,
                        )
                    } else {
                        false
                    };
                    if args.touches.len() == 1 {
                        if let Some(gesture) = self.synthetic_scroll_gesture {
                            if let Some(active_touch) = args
                                .touches
                                .iter()
                                .find(|touch| touch.identifier == gesture.touch_identifier)
                            {
                                if let Some(target_node) =
                                    engine.get_expanded_node(gesture.target_id)
                                {
                                    prevented |= target_node.dispatch_scroll(
                                        Event::new(Scroll {
                                            delta_x: active_touch.delta_x,
                                            delta_y: active_touch.delta_y,
                                        }),
                                        &globals,
                                        &engine.runtime_context,
                                    );
                                } else {
                                    self.synthetic_scroll_gesture = None;
                                }
                            } else {
                                self.synthetic_scroll_gesture = None;
                            }
                        }
                    } else {
                        self.synthetic_scroll_gesture = None;
                    }
                    prevented
                } else {
                    self.synthetic_scroll_gesture = None;
                    false
                }
            }
            NativeInterrupt::TouchEnd(args) => {
                if let Some(gesture) = self.synthetic_scroll_gesture {
                    if args
                        .touches
                        .iter()
                        .any(|touch| touch.identifier == gesture.touch_identifier)
                    {
                        self.synthetic_scroll_gesture = None;
                    }
                }
                if let Some(first_touch) = args.touches.first() {
                    if let Some(topmost_node) = engine
                        .runtime_context
                        .get_topmost_element_beneath_ray(Point2::new(first_touch.x, first_touch.y))
                    {
                        let touches = args.touches.iter().map(|x| Touch::from(x)).collect();
                        let args_touch_end = TouchEnd { touches };
                        topmost_node.dispatch_touch_end(
                            Event::new(args_touch_end),
                            &globals,
                            &engine.runtime_context,
                        )
                    } else {
                        false
                    }
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
                if let Some(topmost_node) = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y))
                {
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
                } else {
                    false
                }
            }
            NativeInterrupt::SelectStart(_args) => {
                engine.global_dispatch_select_start(SelectStart {})
            }
            NativeInterrupt::MouseMove(args) => {
                if let Some(topmost_node) = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y))
                {
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
                } else {
                    false
                }
            }
            NativeInterrupt::Wheel(args) => {
                if let Some(topmost_node) = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y))
                {
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
                    let mut prevented = topmost_node.dispatch_wheel(
                        Event::new(args_wheel),
                        &globals,
                        &engine.runtime_context,
                    );
                    if !node_is_in_scroller_subtree(&topmost_node) {
                        prevented |= topmost_node.dispatch_scroll(
                            Event::new(Scroll {
                                delta_x: args.delta_x,
                                delta_y: args.delta_y,
                            }),
                            &globals,
                            &engine.runtime_context,
                        );
                    }
                    prevented
                } else {
                    false
                }
            }
            NativeInterrupt::MouseDown(args) => {
                if let Some(topmost_node) = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y))
                {
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
                } else {
                    false
                }
            }
            NativeInterrupt::MouseUp(args) => {
                if let Some(topmost_node) = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y))
                {
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
                } else {
                    false
                }
            }
            NativeInterrupt::ContextMenu(args) => {
                if let Some(topmost_node) = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y))
                {
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
                } else {
                    false
                }
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

    pub fn tick(&mut self) -> JsValue {
        #[cfg(feature = "designtime")]
        self.designtime_tick();

        self.drain_render_surface_updates();

        let message_queue = borrow_mut!(self.engine).tick();
        js_value_serde::to_value(&message_queue)
    }

    pub fn render(&mut self) {
        borrow_mut!(self.engine).render(self.render_context.as_mut());
    }

    pub fn layer_canvas_plan_generation(&self) -> u32 {
        self.engine
            .borrow()
            .runtime_context
            .layer_canvas_plan_generation()
    }

    pub fn get_layer_canvas_plan(&self, layer: usize) -> JsValue {
        let engine = self.engine.borrow();
        let ctx = &engine.runtime_context;
        let window = window().unwrap();
        let dpr = window.device_pixel_ratio().max(1.0);

        let plan = if let Some(owner) = ctx.get_layer_scroller_owner(layer) {
            let scroller_id = owner.to_u32();
            let Some(state) = ctx.get_scroller_surface_state(scroller_id) else {
                return JsValue::NULL;
            };
            let host_signature = format!("scroller:{scroller_id}");
            let mut viewport_width = state.viewport_width;
            let mut viewport_height = state.viewport_height;
            let mut scroll_x = if state.presentation_scroll_x.is_finite() {
                state.presentation_scroll_x
            } else {
                state.scroll_x
            };
            let mut scroll_y = if state.presentation_scroll_y.is_finite() {
                state.presentation_scroll_y
            } else {
                state.scroll_y
            };
            if ctx.get_root_scroller_id() == Some(scroller_id) {
                if let Some(visual) = ctx.get_visual_viewport_state() {
                    viewport_width = visual.width;
                    viewport_height = visual.height;
                    scroll_x = visual.page_scroll_x;
                    scroll_y = visual.page_scroll_y;
                }
            }
            scroller_canvas_plan_with_policy(
                layer,
                host_signature,
                state.content_width,
                state.content_height,
                viewport_width,
                viewport_height,
                scroll_x,
                scroll_y,
                dpr,
                engine.scroller_tiling_policy,
            )
        } else if layer == 0 {
            let viewport = ctx.globals().viewport.get();
            let host_signature = "root".to_string();
            let width = viewport.bounds.0;
            let height = viewport.bounds.1;
            scroller_canvas_plan_with_policy(
                layer,
                host_signature,
                width,
                height,
                width,
                height,
                0.0,
                0.0,
                dpr,
                engine.scroller_tiling_policy,
            )
        } else {
            return JsValue::NULL;
        };

        js_value_serde::to_value(&plan)
    }

    pub fn request_layer_screenshot(&mut self, layer: usize, request_id: u32) {
        self.render_context
            .request_layer_screenshot(layer, request_id);
    }

    pub fn take_layer_screenshot(&mut self, layer: usize, request_id: u32) -> JsValue {
        self.render_context
            .take_layer_screenshot(layer, request_id)
            .map(|capture| js_value_serde::to_value(&capture))
            .unwrap_or(JsValue::NULL)
    }

    pub fn take_layer_surface_screenshots(&mut self, layer: usize, request_id: u32) -> JsValue {
        serde_wasm_bindgen::to_value(
            &self
                .render_context
                .take_layer_surface_screenshots(layer, request_id),
        )
        .unwrap_or(JsValue::NULL)
    }

    pub fn take_reload_app_requests(&mut self) -> JsValue {
        #[cfg(feature = "designtime")]
        {
            return js_value_serde::to_value(
                &self
                    .designtime_manager
                    .borrow_mut()
                    .take_reload_app_requests(),
            );
        }

        #[cfg(not(feature = "designtime"))]
        {
            JsValue::NULL
        }
    }

    pub fn image_loaded(&mut self, path: &str) -> bool {
        self.render_context.image_loaded(path)
    }
}

fn js_field(value: &JsValue, field: &str) -> JsValue {
    Reflect::get(value, &JsValue::from_str(field)).unwrap()
}

fn js_variant(value: &JsValue, variant: &str) -> Option<JsValue> {
    let payload = js_field(value, variant);
    if payload.is_undefined() {
        None
    } else {
        Some(payload)
    }
}

fn js_f64(value: &JsValue, field: &str) -> f64 {
    js_field(value, field).as_f64().unwrap()
}

fn js_u32(value: &JsValue, field: &str) -> u32 {
    js_f64(value, field) as u32
}

fn js_u64(value: &JsValue, field: &str) -> u64 {
    js_f64(value, field) as u64
}

fn js_usize(value: &JsValue, field: &str) -> usize {
    js_f64(value, field) as usize
}

fn js_i64(value: &JsValue, field: &str) -> i64 {
    js_f64(value, field) as i64
}

fn js_bool(value: &JsValue, field: &str) -> bool {
    js_field(value, field).as_bool().unwrap()
}

fn js_string(value: &JsValue, field: &str) -> String {
    js_field(value, field).as_string().unwrap()
}

fn js_optional_string(value: &JsValue, field: &str) -> Option<String> {
    let field = js_field(value, field);
    if field.is_undefined() || field.is_null() {
        None
    } else {
        field.as_string()
    }
}

fn js_optional_f64(value: &JsValue, field: &str) -> Option<f64> {
    let field = js_field(value, field);
    if field.is_undefined() || field.is_null() {
        None
    } else {
        field.as_f64()
    }
}

fn js_optional_u32(value: &JsValue, field: &str) -> Option<u32> {
    js_optional_f64(value, field).map(|value| value as u32)
}

fn js_array(value: &JsValue, field: &str) -> Array {
    Array::from(&js_field(value, field))
}

fn js_string_vec(value: &JsValue, field: &str) -> Vec<String> {
    let values = js_array(value, field);
    let mut out = Vec::with_capacity(values.length() as usize);
    for index in 0..values.length() {
        out.push(values.get(index).as_string().unwrap());
    }
    out
}

fn js_string_multimap(
    value: &JsValue,
    field: &str,
) -> std::collections::HashMap<String, Vec<String>> {
    let query = js_field(value, field);
    let keys = Object::keys(&Object::from(query.clone()));
    let mut out = std::collections::HashMap::new();

    for index in 0..keys.length() {
        let key = keys.get(index).as_string().unwrap();
        let raw_values = Reflect::get(&query, &JsValue::from_str(&key)).unwrap();
        let values = Array::from(&raw_values);
        let mut parsed_values = Vec::with_capacity(values.length() as usize);
        for value_index in 0..values.length() {
            parsed_values.push(values.get(value_index).as_string().unwrap());
        }
        out.insert(key, parsed_values);
    }

    out
}

fn parse_mouse_button(value: &JsValue) -> MouseButtonMessage {
    match value.as_string().unwrap().as_str() {
        "Left" => MouseButtonMessage::Left,
        "Right" => MouseButtonMessage::Right,
        "Middle" => MouseButtonMessage::Middle,
        "Unknown" => MouseButtonMessage::Unknown,
        _ => panic!("unknown mouse button"),
    }
}

fn parse_modifier(value: &JsValue) -> ModifierKeyMessage {
    match value.as_string().unwrap().as_str() {
        "Shift" => ModifierKeyMessage::Shift,
        "Control" => ModifierKeyMessage::Control,
        "Alt" => ModifierKeyMessage::Alt,
        "Command" => ModifierKeyMessage::Command,
        _ => panic!("unknown modifier"),
    }
}

fn parse_modifiers(payload: &JsValue) -> Vec<ModifierKeyMessage> {
    let modifiers = js_array(payload, "modifiers");
    let mut out = Vec::with_capacity(modifiers.length() as usize);
    for index in 0..modifiers.length() {
        out.push(parse_modifier(&modifiers.get(index)));
    }
    out
}

fn parse_touch(value: JsValue) -> TouchMessage {
    TouchMessage {
        x: js_f64(&value, "x"),
        y: js_f64(&value, "y"),
        identifier: js_i64(&value, "identifier"),
        delta_x: js_f64(&value, "delta_x"),
        delta_y: js_f64(&value, "delta_y"),
    }
}

fn parse_touches(payload: &JsValue) -> Vec<TouchMessage> {
    let touches = js_array(payload, "touches");
    let mut out = Vec::with_capacity(touches.length() as usize);
    for index in 0..touches.length() {
        out.push(parse_touch(touches.get(index)));
    }
    out
}

fn parse_mouse_event(payload: &JsValue) -> (f64, f64, MouseButtonMessage, Vec<ModifierKeyMessage>) {
    (
        js_f64(payload, "x"),
        js_f64(payload, "y"),
        parse_mouse_button(&js_field(payload, "button")),
        parse_modifiers(payload),
    )
}

fn parse_image_load_interrupt(payload: &JsValue) -> ImageLoadInterruptArgs {
    if let Some(data) = js_variant(payload, "Data") {
        ImageLoadInterruptArgs::Data(ImageDataArgs {
            id: js_u32(&data, "id"),
            path: js_string(&data, "path"),
            width: js_usize(&data, "width"),
            height: js_usize(&data, "height"),
        })
    } else if let Some(reference) = js_variant(payload, "Reference") {
        ImageLoadInterruptArgs::Reference(ImagePointerArgs {
            id: js_u32(&reference, "id"),
            path: js_string(&reference, "path"),
            image_data: js_u64(&reference, "image_data"),
            image_data_length: js_usize(&reference, "image_data_length"),
            width: js_usize(&reference, "width"),
            height: js_usize(&reference, "height"),
        })
    } else {
        panic!("unknown image interrupt payload")
    }
}

fn parse_photo_picker_asset(value: JsValue) -> PhotoPickerAssetArgs {
    PhotoPickerAssetArgs {
        temp_id: js_string(&value, "temp_id"),
        file_name: js_optional_string(&value, "file_name"),
        mime_type: js_string(&value, "mime_type"),
        byte_size: js_u64(&value, "byte_size"),
        width: js_optional_u32(&value, "width"),
        height: js_optional_u32(&value, "height"),
        source_kind: js_string(&value, "source_kind"),
        handle: js_optional_string(&value, "handle"),
        data: Vec::new(),
    }
}

fn parse_photo_picker_interrupt(payload: &JsValue) -> PhotoPickerInterruptArgs {
    let photos = js_array(payload, "photos");
    let mut out = Vec::with_capacity(photos.length() as usize);
    for index in 0..photos.length() {
        out.push(parse_photo_picker_asset(photos.get(index)));
    }
    PhotoPickerInterruptArgs {
        id: js_u32(payload, "id"),
        request_id: js_u64(payload, "request_id"),
        status: js_string(payload, "status"),
        message: js_optional_string(payload, "message"),
        photos: out,
    }
}

fn fill_photo_picker_bytes_from_js(args: &mut PhotoPickerInterruptArgs, payload: &JsValue) {
    if payload.is_undefined() || payload.is_null() {
        return;
    }
    let byte_arrays = Array::from(payload);
    for index in 0..byte_arrays.length() {
        if let Some(photo) = args.photos.get_mut(index as usize) {
            let bytes = byte_arrays.get(index);
            if !bytes.is_undefined() && !bytes.is_null() {
                photo.data = Uint8Array::new(&bytes).to_vec();
            }
        }
    }
}

fn native_interrupt_from_js(value: JsValue) -> NativeInterrupt {
    if let Some(payload) = js_variant(&value, "ChassisResizeRequestCollection") {
        let collection = Array::from(&payload);
        let mut out = Vec::with_capacity(collection.length() as usize);
        for index in 0..collection.length() {
            let item = collection.get(index);
            out.push(ChassisResizeRequestArgs {
                id: js_u32(&item, "id"),
                width: js_f64(&item, "width"),
                height: js_f64(&item, "height"),
            });
        }
        NativeInterrupt::ChassisResizeRequestCollection(out)
    } else if js_variant(&value, "SelectStart").is_some() {
        NativeInterrupt::SelectStart(SelectStartArgs {})
    } else if js_variant(&value, "Focus").is_some() {
        NativeInterrupt::Focus(FocusInterruptArgs {})
    } else if let Some(payload) = js_variant(&value, "Tap") {
        NativeInterrupt::Tap(TapInterruptArgs {
            x: js_f64(&payload, "x"),
            y: js_f64(&payload, "y"),
        })
    } else if let Some(payload) = js_variant(&value, "Scroll") {
        NativeInterrupt::Scroll(ScrollInterruptArgs {
            x: js_f64(&payload, "x"),
            y: js_f64(&payload, "y"),
            delta_x: js_f64(&payload, "delta_x"),
            delta_y: js_f64(&payload, "delta_y"),
        })
    } else if let Some(payload) = js_variant(&value, "TouchStart") {
        NativeInterrupt::TouchStart(TouchStartInterruptArgs {
            touches: parse_touches(&payload),
        })
    } else if let Some(payload) = js_variant(&value, "TouchMove") {
        NativeInterrupt::TouchMove(TouchMoveInterruptArgs {
            touches: parse_touches(&payload),
        })
    } else if let Some(payload) = js_variant(&value, "TouchEnd") {
        NativeInterrupt::TouchEnd(TouchEndInterruptArgs {
            touches: parse_touches(&payload),
        })
    } else if let Some(payload) = js_variant(&value, "KeyDown") {
        NativeInterrupt::KeyDown(KeyDownInterruptArgs {
            key: js_string(&payload, "key"),
            modifiers: parse_modifiers(&payload),
            is_repeat: js_bool(&payload, "is_repeat"),
        })
    } else if let Some(payload) = js_variant(&value, "KeyUp") {
        NativeInterrupt::KeyUp(KeyUpInterruptArgs {
            key: js_string(&payload, "key"),
            modifiers: parse_modifiers(&payload),
            is_repeat: js_bool(&payload, "is_repeat"),
        })
    } else if let Some(payload) = js_variant(&value, "KeyPress") {
        NativeInterrupt::KeyPress(KeyPressInterruptArgs {
            key: js_string(&payload, "key"),
            modifiers: parse_modifiers(&payload),
            is_repeat: js_bool(&payload, "is_repeat"),
        })
    } else if let Some(payload) = js_variant(&value, "Click") {
        let (x, y, button, modifiers) = parse_mouse_event(&payload);
        NativeInterrupt::Click(ClickInterruptArgs {
            x,
            y,
            button,
            modifiers,
        })
    } else if let Some(payload) = js_variant(&value, "DoubleClick") {
        let (x, y, button, modifiers) = parse_mouse_event(&payload);
        NativeInterrupt::DoubleClick(DoubleClickInterruptArgs {
            x,
            y,
            button,
            modifiers,
        })
    } else if let Some(payload) = js_variant(&value, "MouseMove") {
        let (x, y, button, modifiers) = parse_mouse_event(&payload);
        NativeInterrupt::MouseMove(MouseMoveInterruptArgs {
            x,
            y,
            button,
            modifiers,
        })
    } else if let Some(payload) = js_variant(&value, "Wheel") {
        NativeInterrupt::Wheel(WheelInterruptArgs {
            x: js_f64(&payload, "x"),
            y: js_f64(&payload, "y"),
            delta_x: js_f64(&payload, "delta_x"),
            delta_y: js_f64(&payload, "delta_y"),
            modifiers: parse_modifiers(&payload),
        })
    } else if let Some(payload) = js_variant(&value, "MouseDown") {
        let (x, y, button, modifiers) = parse_mouse_event(&payload);
        NativeInterrupt::MouseDown(MouseDownInterruptArgs {
            x,
            y,
            button,
            modifiers,
        })
    } else if let Some(payload) = js_variant(&value, "MouseUp") {
        let (x, y, button, modifiers) = parse_mouse_event(&payload);
        NativeInterrupt::MouseUp(MouseUpInterruptArgs {
            x,
            y,
            button,
            modifiers,
        })
    } else if let Some(payload) = js_variant(&value, "ContextMenu") {
        let (x, y, button, modifiers) = parse_mouse_event(&payload);
        NativeInterrupt::ContextMenu(ContextMenuInterruptArgs {
            x,
            y,
            button,
            modifiers,
        })
    } else if let Some(payload) = js_variant(&value, "Image") {
        NativeInterrupt::Image(parse_image_load_interrupt(&payload))
    } else if let Some(payload) = js_variant(&value, "AddedLayer") {
        NativeInterrupt::AddedLayer(AddedLayerArgs {
            num_layers_added: js_u32(&payload, "num_layers_added"),
            layer_id: js_optional_u32(&payload, "layer_id"),
        })
    } else if let Some(payload) = js_variant(&value, "TextInput") {
        NativeInterrupt::TextInput(TextInputArgs {
            text: js_string(&payload, "text"),
            id: js_u32(&payload, "id"),
        })
    } else if let Some(payload) = js_variant(&value, "FormCheckboxToggle") {
        NativeInterrupt::FormCheckboxToggle(FormCheckboxToggleArgs {
            state: js_bool(&payload, "state"),
            id: js_u32(&payload, "id"),
        })
    } else if let Some(payload) = js_variant(&value, "FormDropdownChange") {
        NativeInterrupt::FormDropdownChange(FormDropdownChangeArgs {
            id: js_u32(&payload, "id"),
            selected_id: js_u32(&payload, "selected_id"),
        })
    } else if let Some(payload) = js_variant(&value, "FormSliderChange") {
        NativeInterrupt::FormSliderChange(FormSliderChangeArgs {
            id: js_u32(&payload, "id"),
            value: js_f64(&payload, "value"),
        })
    } else if let Some(payload) = js_variant(&value, "FormRadioListChange") {
        NativeInterrupt::FormRadioListChange(FormRadioListChangeArgs {
            id: js_u32(&payload, "id"),
            selected_id: js_u32(&payload, "selected_id"),
        })
    } else if let Some(payload) = js_variant(&value, "FormTextboxChange") {
        NativeInterrupt::FormTextboxChange(FormTextboxChangeArgs {
            text: js_string(&payload, "text"),
            id: js_u32(&payload, "id"),
        })
    } else if let Some(payload) = js_variant(&value, "FormTextboxInput") {
        NativeInterrupt::FormTextboxInput(FormTextboxInputArgs {
            text: js_string(&payload, "text"),
            id: js_u32(&payload, "id"),
        })
    } else if let Some(payload) = js_variant(&value, "FormButtonClick") {
        NativeInterrupt::FormButtonClick(FormButtonClickArgs {
            id: js_u32(&payload, "id"),
        })
    } else if let Some(payload) = js_variant(&value, "PhotoPicker") {
        NativeInterrupt::PhotoPicker(parse_photo_picker_interrupt(&payload))
    } else if let Some(payload) =
        js_variant(&value, "ScrollerPosition").or_else(|| js_variant(&value, "Scrollbar"))
    {
        NativeInterrupt::ScrollerPosition(ScrollerPositionInterruptArgs {
            id: js_u32(&payload, "id"),
            scroll_x: js_f64(&payload, "scroll_x"),
            scroll_y: js_f64(&payload, "scroll_y"),
            presentation_scroll_x: js_optional_f64(&payload, "presentation_scroll_x"),
            presentation_scroll_y: js_optional_f64(&payload, "presentation_scroll_y"),
        })
    } else if let Some(payload) = js_variant(&value, "BrowserConfig") {
        NativeInterrupt::BrowserConfig(BrowserConfigInterruptArgs {
            allow_scroller_vector_layers: js_bool(&payload, "allow_scroller_vector_layers"),
            allow_nested_scroller_vector_layers: js_bool(
                &payload,
                "allow_nested_scroller_vector_layers",
            ),
        })
    } else if let Some(payload) = js_variant(&value, "RenderSurfaceUpdate") {
        NativeInterrupt::RenderSurfaceUpdate(RenderSurfaceUpdateArgs {
            layer_id: js_optional_u32(&payload, "layer_id"),
        })
    } else if let Some(payload) = js_variant(&value, "ViewportResize") {
        NativeInterrupt::ViewportResize(ViewportResizeArgs {
            width: js_f64(&payload, "width"),
            height: js_f64(&payload, "height"),
        })
    } else if let Some(payload) = js_variant(&value, "RouteChange") {
        NativeInterrupt::RouteChange(RouteChangeInterruptArgs {
            path_segments: js_string_vec(&payload, "path_segments"),
            query: js_string_multimap(&payload, "query"),
            fragment: js_optional_string(&payload, "fragment"),
        })
    } else if let Some(payload) = js_variant(&value, "VisualViewportUpdate") {
        NativeInterrupt::VisualViewportUpdate(VisualViewportUpdateArgs {
            width: js_f64(&payload, "width"),
            height: js_f64(&payload, "height"),
            offset_x: js_f64(&payload, "offset_x"),
            offset_y: js_f64(&payload, "offset_y"),
            page_scroll_x: js_f64(&payload, "page_scroll_x"),
            page_scroll_y: js_f64(&payload, "page_scroll_y"),
        })
    } else if let Some(payload) = js_variant(&value, "Gyro") {
        NativeInterrupt::Gyro(GyroInterruptArgs {
            x: js_f64(&payload, "x"),
            y: js_f64(&payload, "y"),
            z: js_f64(&payload, "z"),
        })
    } else if let Some(payload) = js_variant(&value, "Accel") {
        NativeInterrupt::Accel(AccelInterruptArgs {
            x: js_f64(&payload, "x"),
            y: js_f64(&payload, "y"),
            z: js_f64(&payload, "z"),
        })
    } else if let Some(payload) = js_variant(&value, "DropFile") {
        NativeInterrupt::DropFile(DropFileArgs {
            x: js_f64(&payload, "x"),
            y: js_f64(&payload, "y"),
            name: js_string(&payload, "name"),
            mime_type: js_string(&payload, "mime_type"),
            size: js_u64(&payload, "size"),
        })
    } else if let Some(payload) = js_variant(&value, "Screenshot") {
        NativeInterrupt::Screenshot(parse_image_load_interrupt(&payload))
    } else {
        panic!("unknown native interrupt")
    }
}

mod js_value_serde {
    use super::*;
    use pax_message::serde::ser::{
        self, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
        SerializeTupleStruct, SerializeTupleVariant,
    };
    use pax_message::serde::Serialize;
    use std::fmt;

    #[derive(Debug)]
    pub struct JsSerializeError;

    impl ser::Error for JsSerializeError {
        fn custom<T: fmt::Display>(_msg: T) -> Self {
            JsSerializeError
        }
    }

    impl fmt::Display for JsSerializeError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("failed to serialize value to JsValue")
        }
    }

    impl std::error::Error for JsSerializeError {}

    type Result<T> = std::result::Result<T, JsSerializeError>;

    pub fn to_value<T: Serialize + ?Sized>(value: &T) -> JsValue {
        value.serialize(&JsSerializer).unwrap_or(JsValue::NULL)
    }

    struct JsSerializer;

    fn set_field(target: &Object, key: &str, value: JsValue) -> Result<()> {
        Reflect::set(target.as_ref(), &JsValue::from_str(key), &value)
            .map(|_| ())
            .map_err(|_| JsSerializeError)
    }

    fn variant_object(variant: &'static str, value: JsValue) -> Result<JsValue> {
        let object = Object::new();
        set_field(&object, variant, value)?;
        Ok(object.into())
    }

    pub struct JsArraySerializer {
        array: Array,
    }

    impl JsArraySerializer {
        fn new() -> Self {
            Self {
                array: Array::new(),
            }
        }

        fn push<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
            self.array.push(&value.serialize(&JsSerializer)?);
            Ok(())
        }

        fn into_js(self) -> JsValue {
            self.array.into()
        }
    }

    impl SerializeSeq for JsArraySerializer {
        type Ok = JsValue;
        type Error = JsSerializeError;

        fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
            self.push(value)
        }

        fn end(self) -> Result<JsValue> {
            Ok(self.into_js())
        }
    }

    impl SerializeTuple for JsArraySerializer {
        type Ok = JsValue;
        type Error = JsSerializeError;

        fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
            self.push(value)
        }

        fn end(self) -> Result<JsValue> {
            Ok(self.into_js())
        }
    }

    impl SerializeTupleStruct for JsArraySerializer {
        type Ok = JsValue;
        type Error = JsSerializeError;

        fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
            self.push(value)
        }

        fn end(self) -> Result<JsValue> {
            Ok(self.into_js())
        }
    }

    pub struct JsTupleVariantSerializer {
        variant: &'static str,
        inner: JsArraySerializer,
    }

    impl SerializeTupleVariant for JsTupleVariantSerializer {
        type Ok = JsValue;
        type Error = JsSerializeError;

        fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
            self.inner.push(value)
        }

        fn end(self) -> Result<JsValue> {
            variant_object(self.variant, self.inner.into_js())
        }
    }

    pub struct JsObjectSerializer {
        object: Object,
    }

    impl JsObjectSerializer {
        fn new() -> Self {
            Self {
                object: Object::new(),
            }
        }
    }

    impl SerializeStruct for JsObjectSerializer {
        type Ok = JsValue;
        type Error = JsSerializeError;

        fn serialize_field<T: ?Sized + Serialize>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<()> {
            set_field(&self.object, key, value.serialize(&JsSerializer)?)
        }

        fn end(self) -> Result<JsValue> {
            Ok(self.object.into())
        }
    }

    pub struct JsStructVariantSerializer {
        variant: &'static str,
        inner: JsObjectSerializer,
    }

    impl SerializeStructVariant for JsStructVariantSerializer {
        type Ok = JsValue;
        type Error = JsSerializeError;

        fn serialize_field<T: ?Sized + Serialize>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<()> {
            self.inner.serialize_field(key, value)
        }

        fn end(self) -> Result<JsValue> {
            variant_object(self.variant, self.inner.end()?)
        }
    }

    pub struct JsMapSerializer {
        object: Object,
        next_key: Option<String>,
    }

    impl JsMapSerializer {
        fn new() -> Self {
            Self {
                object: Object::new(),
                next_key: None,
            }
        }
    }

    impl SerializeMap for JsMapSerializer {
        type Ok = JsValue;
        type Error = JsSerializeError;

        fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<()> {
            self.next_key = key.serialize(&JsSerializer)?.as_string();
            if self.next_key.is_some() {
                Ok(())
            } else {
                Err(JsSerializeError)
            }
        }

        fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
            let key = self.next_key.take().ok_or(JsSerializeError)?;
            set_field(&self.object, &key, value.serialize(&JsSerializer)?)
        }

        fn end(self) -> Result<JsValue> {
            Ok(self.object.into())
        }
    }

    macro_rules! serialize_number {
        ($($name:ident($ty:ty);)*) => {
            $(
                fn $name(self, value: $ty) -> Result<JsValue> {
                    Ok(JsValue::from_f64(value as f64))
                }
            )*
        };
    }

    impl ser::Serializer for &JsSerializer {
        type Ok = JsValue;
        type Error = JsSerializeError;
        type SerializeSeq = JsArraySerializer;
        type SerializeTuple = JsArraySerializer;
        type SerializeTupleStruct = JsArraySerializer;
        type SerializeTupleVariant = JsTupleVariantSerializer;
        type SerializeMap = JsMapSerializer;
        type SerializeStruct = JsObjectSerializer;
        type SerializeStructVariant = JsStructVariantSerializer;

        fn serialize_bool(self, value: bool) -> Result<JsValue> {
            Ok(JsValue::from_bool(value))
        }

        serialize_number! {
            serialize_i8(i8);
            serialize_i16(i16);
            serialize_i32(i32);
            serialize_i64(i64);
            serialize_i128(i128);
            serialize_u8(u8);
            serialize_u16(u16);
            serialize_u32(u32);
            serialize_u64(u64);
            serialize_u128(u128);
            serialize_f32(f32);
            serialize_f64(f64);
        }

        fn serialize_char(self, value: char) -> Result<JsValue> {
            Ok(JsValue::from_str(&value.to_string()))
        }

        fn serialize_str(self, value: &str) -> Result<JsValue> {
            Ok(JsValue::from_str(value))
        }

        fn serialize_bytes(self, value: &[u8]) -> Result<JsValue> {
            let array = Array::new();
            for byte in value {
                array.push(&JsValue::from_f64(*byte as f64));
            }
            Ok(array.into())
        }

        fn serialize_none(self) -> Result<JsValue> {
            Ok(JsValue::NULL)
        }

        fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<JsValue> {
            value.serialize(self)
        }

        fn serialize_unit(self) -> Result<JsValue> {
            Ok(JsValue::NULL)
        }

        fn serialize_unit_struct(self, _name: &'static str) -> Result<JsValue> {
            Ok(JsValue::NULL)
        }

        fn serialize_unit_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
        ) -> Result<JsValue> {
            Ok(JsValue::from_str(variant))
        }

        fn serialize_newtype_struct<T: ?Sized + Serialize>(
            self,
            _name: &'static str,
            value: &T,
        ) -> Result<JsValue> {
            value.serialize(self)
        }

        fn serialize_newtype_variant<T: ?Sized + Serialize>(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            value: &T,
        ) -> Result<JsValue> {
            variant_object(variant, value.serialize(self)?)
        }

        fn serialize_seq(self, _len: Option<usize>) -> Result<JsArraySerializer> {
            Ok(JsArraySerializer::new())
        }

        fn serialize_tuple(self, _len: usize) -> Result<JsArraySerializer> {
            Ok(JsArraySerializer::new())
        }

        fn serialize_tuple_struct(
            self,
            _name: &'static str,
            _len: usize,
        ) -> Result<JsArraySerializer> {
            Ok(JsArraySerializer::new())
        }

        fn serialize_tuple_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            _len: usize,
        ) -> Result<JsTupleVariantSerializer> {
            Ok(JsTupleVariantSerializer {
                variant,
                inner: JsArraySerializer::new(),
            })
        }

        fn serialize_map(self, _len: Option<usize>) -> Result<JsMapSerializer> {
            Ok(JsMapSerializer::new())
        }

        fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<JsObjectSerializer> {
            Ok(JsObjectSerializer::new())
        }

        fn serialize_struct_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            _len: usize,
        ) -> Result<JsStructVariantSerializer> {
            Ok(JsStructVariantSerializer {
                variant,
                inner: JsObjectSerializer::new(),
            })
        }
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
