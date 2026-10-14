//! Typed input routing shared by native chassis.
//!
//! Platform adapters decode their transport and retain ownership of images,
//! surfaces, and native widgets. This module routes their portable events to
//! the engine without depending on a window system or graphics backend.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use pax_message::{borrow, NativeInterrupt};
use pax_runtime::api::math::Point2;
use pax_runtime::api::{
    Accel, ButtonClick, CheckboxChange, Click, Event, Focus, Gyro, KeyDown, KeyPress, KeyUp,
    KeyboardEventArgs, ModifierKey, MouseButton, MouseCancel, MouseDown, MouseEventArgs, MouseMove,
    MouseUp, PhotoPickerChange, Scroll, SelectStart, SliderChange, SubmitTrigger, TextboxChange, TextboxInput,
    Touch, TouchCancel, TouchEnd, TouchMove, TouchStart, Wheel,
};
use pax_runtime::PaxEngine;

#[derive(Clone, Copy)]
struct SyntheticScrollGesture {
    touch_identifier: i64,
    target_id: pax_runtime::ExpandedNodeIdentifier,
}

/// UI-thread input state belonging to one native engine session.
///
/// Create a fresh dispatcher for each engine. A new mount must not inherit a
/// previous mount's scroll positions or active gesture, even on the same thread.
#[derive(Default)]
pub struct NativeInterruptDispatcher {
    synthetic_scroll_gesture: Cell<Option<SyntheticScrollGesture>>,
    scroller_positions: RefCell<HashMap<u32, (f64, f64)>>,
}

impl NativeInterruptDispatcher {
    /// Routes a decoded event to this session's engine.
    ///
    /// `Some(true)` means a keyboard handler prevented the native default;
    /// `Some(false)` means the event was handled without preventing it.
    /// `None` leaves platform-specific work to the adapter, including images,
    /// screenshots and presentation-surface updates. Pointer coordinates are
    /// window-space. Native text editors retain ownership of text composition.
    /// Call synchronously on the engine's UI thread, between ticks.
    pub fn dispatch(&self, engine: &PaxEngine, interrupt: &NativeInterrupt) -> Option<bool> {
        let globals = engine.runtime_context.globals();
        let mut prevent_default = false;

        match interrupt {
            NativeInterrupt::KeyDown(args) => {
                prevent_default = engine.global_dispatch_key_down(KeyDown {
                    keyboard: KeyboardEventArgs {
                        key: args.key.clone(),
                        modifiers: args.modifiers.iter().map(ModifierKey::from).collect(),
                        is_repeat: args.is_repeat,
                    },
                });
            }
            NativeInterrupt::KeyUp(args) => {
                prevent_default = engine.global_dispatch_key_up(KeyUp {
                    keyboard: KeyboardEventArgs {
                        key: args.key.clone(),
                        modifiers: args.modifiers.iter().map(ModifierKey::from).collect(),
                        is_repeat: args.is_repeat,
                    },
                });
            }
            NativeInterrupt::KeyPress(args) => {
                prevent_default = engine.global_dispatch_key_press(KeyPress {
                    keyboard: KeyboardEventArgs {
                        key: args.key.clone(),
                        modifiers: args.modifiers.iter().map(ModifierKey::from).collect(),
                        is_repeat: args.is_repeat,
                    },
                });
            }
            NativeInterrupt::Focus(_args) => {
                engine.global_dispatch_focus(Focus {});
            }
            NativeInterrupt::SelectStart(_args) => {
                engine.global_dispatch_select_start(SelectStart {});
            }
            NativeInterrupt::ChassisResizeRequestCollection(collection) => {
                for args in collection {
                    let node =
                        engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id));
                    if let Some(node) = node {
                        node.chassis_resize_request(args.width, args.height);
                    }
                }
            }
            NativeInterrupt::TextMeasurementResponse(args) => {
                if let Some(node) =
                    engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
                {
                    borrow!(node.instance_node).handle_native_interrupt(&node, interrupt);
                }
            }
            NativeInterrupt::Click(args) => {
                let modifiers = args.modifiers.iter().map(ModifierKey::from).collect();
                let args_click = Click {
                    mouse: MouseEventArgs {
                        x: args.x,
                        y: args.y,
                        button: MouseButton::from(args.button.clone()),
                        modifiers,
                    },
                };
                if let Some(topmost_node) = engine.runtime_context.mouse_click_target(
                    MouseButton::from(args.button.clone()),
                    Point2::new(args.x, args.y),
                ) {
                    topmost_node.dispatch_click(
                        Event::new(args_click),
                        &globals,
                        &engine.runtime_context,
                    );
                }
            }
            NativeInterrupt::MouseMove(args) => {
                if let Some(topmost_node) = engine.runtime_context.mouse_move_target(
                    MouseButton::from(args.button.clone()),
                    Point2::new(args.x, args.y),
                ) {
                    let args_mouse_move = MouseMove {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::from(args.button.clone()),
                            modifiers: args.modifiers.iter().map(ModifierKey::from).collect(),
                        },
                    };
                    topmost_node.dispatch_mouse_move(
                        Event::new(args_mouse_move),
                        &globals,
                        &engine.runtime_context,
                    );
                }
            }
            NativeInterrupt::MouseDown(args) => {
                if let Some(topmost_node) = engine.runtime_context.mouse_down_target(
                    MouseButton::from(args.button.clone()),
                    Point2::new(args.x, args.y),
                ) {
                    let args_mouse_down = MouseDown {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::from(args.button.clone()),
                            modifiers: args.modifiers.iter().map(ModifierKey::from).collect(),
                        },
                    };
                    topmost_node.dispatch_mouse_down(
                        Event::new(args_mouse_down),
                        &globals,
                        &engine.runtime_context,
                    );
                }
            }
            NativeInterrupt::MouseUp(args) => {
                if let Some(topmost_node) = engine.runtime_context.mouse_up_target(
                    MouseButton::from(args.button.clone()),
                    Point2::new(args.x, args.y),
                    false,
                ) {
                    let args_mouse_up = MouseUp {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::from(args.button.clone()),
                            modifiers: args.modifiers.iter().map(ModifierKey::from).collect(),
                        },
                    };
                    topmost_node.dispatch_mouse_up(
                        Event::new(args_mouse_up),
                        &globals,
                        &engine.runtime_context,
                    );
                }
            }
            NativeInterrupt::MouseCancel(args) => {
                if let Some(topmost_node) = engine.runtime_context.mouse_up_target(
                    MouseButton::from(args.button.clone()),
                    Point2::new(args.x, args.y),
                    true,
                ) {
                    let args_mouse_cancel = MouseCancel {
                        mouse: MouseEventArgs {
                            x: args.x,
                            y: args.y,
                            button: MouseButton::from(args.button.clone()),
                            modifiers: args.modifiers.iter().map(ModifierKey::from).collect(),
                        },
                    };
                    topmost_node.dispatch_mouse_cancel(
                        Event::new(args_mouse_cancel),
                        &globals,
                        &engine.runtime_context,
                    );
                }
            }
            NativeInterrupt::Tap(args) => {
                if let Some(topmost_node) = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y))
                {
                    topmost_node.dispatch_tap(
                        Event::new(Click {
                            mouse: MouseEventArgs {
                                x: args.x,
                                y: args.y,
                                button: MouseButton::Left,
                                modifiers: vec![],
                            },
                        }),
                        &globals,
                        &engine.runtime_context,
                    );
                }
            }
            NativeInterrupt::TouchStart(args) => {
                self.synthetic_scroll_gesture.set(None);
                if let Some(first_touch) = args.touches.first() {
                    let touches = args.touches.iter().map(Touch::from).collect();
                    if let Some(topmost_node) = engine
                        .runtime_context
                        .get_topmost_element_beneath_ray(Point2::new(first_touch.x, first_touch.y))
                    {
                        engine
                            .runtime_context
                            .capture_touch_target(first_touch.identifier, topmost_node.id);
                        if args.touches.len() == 1 && !node_is_in_scroller_subtree(&topmost_node) {
                            self.synthetic_scroll_gesture
                                .set(Some(SyntheticScrollGesture {
                                    touch_identifier: first_touch.identifier,
                                    target_id: topmost_node.id,
                                }));
                        }
                        topmost_node.dispatch_touch_start(
                            Event::new(TouchStart { touches }),
                            &globals,
                            &engine.runtime_context,
                        );
                    }
                }
            }
            NativeInterrupt::TouchMove(args) => {
                if let Some(first_touch) = args.touches.first() {
                    let touches = args.touches.iter().map(Touch::from).collect();
                    let target_node = engine
                        .runtime_context
                        .captured_touch_target(first_touch.identifier)
                        .or_else(|| {
                            engine
                                .runtime_context
                                .get_topmost_element_beneath_ray(Point2::new(
                                    first_touch.x,
                                    first_touch.y,
                                ))
                        });
                    if let Some(target_node) = target_node {
                        target_node.dispatch_touch_move(
                            Event::new(TouchMove { touches }),
                            &globals,
                            &engine.runtime_context,
                        );
                    }
                    if args.touches.len() == 1 {
                        let gesture = self.synthetic_scroll_gesture.get();
                        if let Some(gesture) = gesture {
                            if let Some(active_touch) = args
                                .touches
                                .iter()
                                .find(|touch| touch.identifier == gesture.touch_identifier)
                            {
                                if let Some(target_node) =
                                    engine.get_expanded_node(gesture.target_id)
                                {
                                    target_node.dispatch_scroll(
                                        Event::new(Scroll {
                                            delta_x: active_touch.delta_x,
                                            delta_y: active_touch.delta_y,
                                        }),
                                        &globals,
                                        &engine.runtime_context,
                                    );
                                } else {
                                    self.synthetic_scroll_gesture.set(None);
                                }
                            } else {
                                self.synthetic_scroll_gesture.set(None);
                            }
                        }
                    } else {
                        self.synthetic_scroll_gesture.set(None);
                    }
                } else {
                    self.synthetic_scroll_gesture.set(None);
                }
            }
            NativeInterrupt::TouchEnd(args) => {
                let gesture = self.synthetic_scroll_gesture.get();
                if let Some(gesture) = gesture {
                    if args
                        .touches
                        .iter()
                        .any(|touch| touch.identifier == gesture.touch_identifier)
                    {
                        self.synthetic_scroll_gesture.set(None);
                    }
                }
                if let Some(first_touch) = args.touches.first() {
                    let touches = args.touches.iter().map(Touch::from).collect();
                    let target_node = engine
                        .runtime_context
                        .release_touch_target(first_touch.identifier)
                        .or_else(|| {
                            engine
                                .runtime_context
                                .get_topmost_element_beneath_ray(Point2::new(
                                    first_touch.x,
                                    first_touch.y,
                                ))
                        });
                    if let Some(target_node) = target_node {
                        target_node.dispatch_touch_end(
                            Event::new(TouchEnd { touches }),
                            &globals,
                            &engine.runtime_context,
                        );
                    }
                }
            }
            NativeInterrupt::TouchCancel(args) => {
                let gesture = self.synthetic_scroll_gesture.get();
                if let Some(gesture) = gesture {
                    if args
                        .touches
                        .iter()
                        .any(|touch| touch.identifier == gesture.touch_identifier)
                    {
                        self.synthetic_scroll_gesture.set(None);
                    }
                }
                if let Some(first_touch) = args.touches.first() {
                    let touches = args.touches.iter().map(Touch::from).collect();
                    let target_node = engine
                        .runtime_context
                        .release_touch_target(first_touch.identifier)
                        .or_else(|| {
                            engine
                                .runtime_context
                                .get_topmost_element_beneath_ray(Point2::new(
                                    first_touch.x,
                                    first_touch.y,
                                ))
                        });
                    if let Some(target_node) = target_node {
                        target_node.dispatch_touch_cancel(
                            Event::new(TouchCancel { touches }),
                            &globals,
                            &engine.runtime_context,
                        );
                    }
                }
            }
            NativeInterrupt::FormRadioListChange(args) => {
                let node = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id));
                if let Some(node) = node {
                    borrow!(node.instance_node).handle_native_interrupt(&node, interrupt);
                }
            }
            NativeInterrupt::FormSliderChange(args) => {
                let node = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id));
                if let Some(node) = node {
                    borrow!(node.instance_node).handle_native_interrupt(&node, interrupt);
                    node.dispatch_slider_change(
                        Event::new(SliderChange { value: args.value }),
                        &globals,
                        &engine.runtime_context,
                    );
                }
            }
            NativeInterrupt::FormDropdownChange(args) => {
                let node = engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id));
                if let Some(node) = node {
                    borrow!(node.instance_node).handle_native_interrupt(&node, interrupt);
                }
            }
            NativeInterrupt::FormButtonClick(args) => {
                if let Some(node) =
                    engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
                {
                    node.dispatch_button_click(
                        Event::new(ButtonClick {}),
                        &globals,
                        &engine.runtime_context,
                    );
                }
            }
            NativeInterrupt::PhotoPicker(args) => {
                if let Some(node) =
                    engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
                {
                    node.dispatch_photo_picker_change(
                        Event::new(PhotoPickerChange::from(args)),
                        &globals,
                        &engine.runtime_context,
                    );
                }
            }
            NativeInterrupt::FormTextboxInput(args) => {
                if let Some(node) =
                    engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
                {
                    borrow!(node.instance_node).handle_native_interrupt(&node, interrupt);
                    node.dispatch_textbox_input(
                        Event::new(TextboxInput {
                            text: args.text.clone(),
                        }),
                        &globals,
                        &engine.runtime_context,
                    );
                }
            }
            NativeInterrupt::TextInput(args) => {
                if let Some(node) =
                    engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
                {
                    borrow!(node.instance_node).handle_native_interrupt(&node, interrupt);
                }
            }
            NativeInterrupt::FormTextboxSubmit(args) => {
                if let Some(node) =
                    engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
                {
                    borrow!(node.instance_node).handle_native_interrupt(&node, interrupt);
                    node.emit_submit(SubmitTrigger::Return, &engine.runtime_context);
                }
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
                    );
                }
            }
            NativeInterrupt::FormCheckboxToggle(args) => {
                if let Some(node) =
                    engine.get_expanded_node(pax_runtime::ExpandedNodeIdentifier(args.id))
                {
                    borrow!(node.instance_node).handle_native_interrupt(&node, interrupt);
                    node.dispatch_checkbox_change(
                        Event::new(CheckboxChange {
                            checked: args.state,
                        }),
                        &globals,
                        &engine.runtime_context,
                    );
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
                        .scroller_positions
                        .borrow_mut()
                        .insert(args.id, (presentation_scroll_x, presentation_scroll_y));
                    borrow!(node.instance_node).handle_native_interrupt(&node, interrupt);
                    if let Some((previous_x, previous_y)) = previous {
                        let delta_x = presentation_scroll_x - previous_x;
                        let delta_y = presentation_scroll_y - previous_y;
                        if delta_x.abs() > f64::EPSILON || delta_y.abs() > f64::EPSILON {
                            node.dispatch_scroll(
                                Event::new(Scroll { delta_x, delta_y }),
                                &globals,
                                &engine.runtime_context,
                            );
                        }
                    }
                } else {
                    self.scroller_positions.borrow_mut().remove(&args.id);
                }
            }
            NativeInterrupt::Wheel(args) => {
                if let Some(node) = engine
                    .runtime_context
                    .get_topmost_element_beneath_ray(Point2::new(args.x, args.y))
                {
                    prevent_default = node.dispatch_wheel(
                        Event::new(Wheel {
                            x: args.x,
                            y: args.y,
                            delta_x: args.delta_x,
                            delta_y: args.delta_y,
                            modifiers: args.modifiers.iter().map(ModifierKey::from).collect(),
                        }),
                        &globals,
                        &engine.runtime_context,
                    );
                    // Native scrollers report their actual movement separately. Match web's
                    // synthetic Scroll stream only for content outside those scrollers.
                    if !node_is_in_scroller_subtree(&node) {
                        prevent_default |= node.dispatch_scroll(
                            Event::new(Scroll {
                                delta_x: args.delta_x,
                                delta_y: args.delta_y,
                            }),
                            &globals,
                            &engine.runtime_context,
                        );
                    }
                }
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
                    );
                }
            }
            NativeInterrupt::SafeAreaInsets(insets) => {
                engine.runtime_context.set_safe_area_insets(*insets);
            }
            NativeInterrupt::RouteChange(args) => {
                globals.route_location.set(args.into());
            }
            NativeInterrupt::Gyro(args) => {
                let gyro = Gyro {
                    x: args.x,
                    y: args.y,
                    z: args.z,
                };
                globals.gyro.set_if_neq(gyro);
                engine.global_dispatch_gyro(gyro);
            }
            NativeInterrupt::Accel(args) => {
                let accel = Accel {
                    x: args.x,
                    y: args.y,
                    z: args.z,
                };
                globals.accel.set_if_neq(accel);
                engine.global_dispatch_accel(accel);
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
            }
            _ => return None,
        }
        Some(prevent_default)
    }

    /// Retires state when the host is told to delete a scroller.
    pub fn forget_scroller(&self, id: u32) {
        self.scroller_positions.borrow_mut().remove(&id);
    }
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
