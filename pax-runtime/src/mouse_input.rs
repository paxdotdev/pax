use crate::api::{math::Point2, Window};
use crate::{ExpandedNode, ExpandedNodeIdentifier, RuntimeContext};
use pax_runtime_api::MouseButton;
use std::rc::Rc;

type Target = Option<ExpandedNodeIdentifier>;

#[derive(Default)]
pub(crate) struct MouseInput {
    // An active sequence with no target stays captured to nothing after unmount.
    pressed: Vec<(MouseButton, Target)>,
    click: Option<(MouseButton, ExpandedNodeIdentifier)>,
}

impl MouseInput {
    fn down(&mut self, button: MouseButton, hit: Target) {
        self.pressed.retain(|(b, _)| *b != button);
        self.pressed.push((button, hit));
        self.click = None;
    }

    fn moving(&self, button: &MouseButton) -> Option<Target> {
        self.pressed
            .iter()
            .find(|(b, _)| b == button)
            .map(|(_, t)| *t)
    }

    fn up(&mut self, button: &MouseButton, hit: Target, cancelled: bool) -> Target {
        self.click = None;
        let index = self.pressed.iter().position(|(b, _)| b == button)?;
        let (_, target) = self.pressed.remove(index);
        if !cancelled && target == hit {
            self.click = target.map(|t| (button.clone(), t));
        }
        target
    }

    fn click(&mut self, button: &MouseButton, hit: Target) -> Target {
        let (released_button, target) = self.click.take()?;
        (released_button == *button && hit == Some(target)).then_some(target)
    }

    pub(crate) fn forget(&mut self, id: ExpandedNodeIdentifier) {
        for (_, target) in &mut self.pressed {
            if *target == Some(id) {
                *target = None;
            }
        }
        if self.click.as_ref().is_some_and(|(_, target)| *target == id) {
            self.click = None;
        }
    }
}

impl RuntimeContext {
    /// Start implicit mouse capture at the hit node, independently for each button.
    pub fn mouse_down_target(
        self: &Rc<Self>,
        button: MouseButton,
        ray: Point2<Window>,
    ) -> Option<Rc<ExpandedNode>> {
        let hit = self.get_topmost_element_beneath_ray(ray);
        self.mouse_input
            .borrow_mut()
            .down(button, hit.as_ref().map(|n| n.id));
        hit
    }

    /// Route moves to the press target until release; ordinary hover still hit-tests.
    pub fn mouse_move_target(
        self: &Rc<Self>,
        button: MouseButton,
        ray: Point2<Window>,
    ) -> Option<Rc<ExpandedNode>> {
        let hit = self.get_topmost_element_beneath_ray(ray);
        let capture = self.mouse_input.borrow().moving(&button);
        match capture {
            Some(target) => target.and_then(|id| self.get_expanded_node_by_eid(id)),
            None => hit,
        }
    }

    /// Release capture and qualify activation only when the physical release hits the press target.
    /// Cancellation releases capture without qualifying a click.
    pub fn mouse_up_target(
        self: &Rc<Self>,
        button: MouseButton,
        ray: Point2<Window>,
        cancelled: bool,
    ) -> Option<Rc<ExpandedNode>> {
        let hit = if cancelled {
            None
        } else {
            self.get_topmost_element_beneath_ray(ray).map(|n| n.id)
        };
        let target = self.mouse_input.borrow_mut().up(&button, hit, cancelled);
        target.and_then(|id| self.get_expanded_node_by_eid(id))
    }

    /// Consume a host click only after a matching press and release on the same live node.
    pub fn mouse_click_target(
        self: &Rc<Self>,
        button: MouseButton,
        ray: Point2<Window>,
    ) -> Option<Rc<ExpandedNode>> {
        let hit = self.get_topmost_element_beneath_ray(ray).map(|n| n.id);
        let target = self.mouse_input.borrow_mut().click(&button, hit);
        target.and_then(|id| self.get_expanded_node_by_eid(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const KNOB: Target = Some(ExpandedNodeIdentifier(1));
    const DISPLAY: Target = Some(ExpandedNodeIdentifier(2));

    #[test]
    fn drag_stays_on_origin_but_release_over_another_node_cannot_click() {
        let mut input = MouseInput::default();
        input.down(MouseButton::Left, KNOB);
        assert_eq!(input.moving(&MouseButton::Left), Some(KNOB));
        assert_eq!(input.up(&MouseButton::Left, DISPLAY, false), KNOB);
        assert_eq!(input.moving(&MouseButton::Left), None);
        assert_eq!(input.click(&MouseButton::Left, DISPLAY), None);
    }

    #[test]
    fn click_requires_a_matching_button_and_consumes_one_completed_sequence() {
        let mut input = MouseInput::default();
        assert_eq!(input.click(&MouseButton::Left, DISPLAY), None);
        input.down(MouseButton::Left, DISPLAY);
        assert_eq!(input.up(&MouseButton::Right, DISPLAY, false), None);
        assert_eq!(input.click(&MouseButton::Right, DISPLAY), None);
        assert_eq!(input.up(&MouseButton::Left, DISPLAY, false), DISPLAY);
        assert_eq!(input.click(&MouseButton::Left, DISPLAY), DISPLAY);
        assert_eq!(input.click(&MouseButton::Left, DISPLAY), None);
    }

    #[test]
    fn release_outside_or_cancelled_does_not_activate() {
        let mut input = MouseInput::default();
        for (hit, cancelled) in [(None, false), (KNOB, true)] {
            input.down(MouseButton::Left, KNOB);
            assert_eq!(input.up(&MouseButton::Left, hit, cancelled), KNOB);
            assert_eq!(input.click(&MouseButton::Left, KNOB), None);
        }
    }

    #[test]
    fn unmount_never_retargets_a_drag_or_activates_a_replacement() {
        let mut input = MouseInput::default();
        input.down(MouseButton::Left, KNOB);
        input.forget(KNOB.unwrap());
        assert_eq!(input.moving(&MouseButton::Left), Some(None));
        assert_eq!(input.up(&MouseButton::Left, DISPLAY, false), None);
        assert_eq!(input.click(&MouseButton::Left, DISPLAY), None);
        input.down(MouseButton::Left, KNOB);
        input.up(&MouseButton::Left, KNOB, false);
        input.forget(KNOB.unwrap());
        assert_eq!(input.click(&MouseButton::Left, KNOB), None);
    }

    #[test]
    fn concurrent_buttons_keep_independent_capture() {
        let mut input = MouseInput::default();
        input.down(MouseButton::Left, KNOB);
        input.down(MouseButton::Right, DISPLAY);
        input.up(&MouseButton::Right, DISPLAY, false);
        assert_eq!(input.moving(&MouseButton::Left), Some(KNOB));
        assert_eq!(input.moving(&MouseButton::Right), None);
    }
}
