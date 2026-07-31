#![allow(dead_code)]

#[allow(unused)]
use crate::*;
use pax_engine::api::*;
use pax_engine::*;

/// A confirmation dialog, a modal dialog that asks the user to confirm an action with "Yes" or "No" buttons.
#[pax]
#[engine_import_path("pax_engine")]
#[inlined(
    if self.open {
        <Group x=50% y=50% width=300px height=170px>
            <Button width=80px height=30px anchor_y=100% anchor_x=100% y={100%-20px} x={100% -80px -20px -20px} label="Yes" @button_click=handle_yes/>
            <Button width=80px height=30px anchor_y=100% anchor_x=100% y={100% -20px} x={100% -20px} label="No" @button_click=handle_no/>
            <Text x=20px y=20px height=15px width=100px text=text/>
            <Rectangle fill=rgb(20, 20, 20)
            corner_radius=15.0/>
        </Group>
        <Rectangle fill=rgba(0, 0, 0, 70)/>
    }
)]
#[custom(Default)]
pub struct ConfirmationDialog {
    /// Prompt text shown in the dialog.
    pub text: Property<String>,
    /// Whether the dialog is visible.
    pub open: Property<bool>,
    // Private signal toggled when the user confirms.
    pub _signal: Property<bool>,
}

impl Default for ConfirmationDialog {
    fn default() -> Self {
        Self {
            text: Property::new("Are you sure?".to_owned()),
            open: Property::default(),
            _signal: Property::default(),
        }
    }
}

impl ConfirmationDialog {
    // Handles the affirmative button and emits the confirmation signal.
    pub fn handle_yes(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.open.set(false);
        self._signal.set(true);
    }

    // Handles the negative button by closing the dialog.
    pub fn handle_no(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.open.set(false);
    }
}
