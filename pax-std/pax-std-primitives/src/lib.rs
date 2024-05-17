pub mod button;
pub mod checkbox;
pub mod ellipse;
pub mod frame;
pub mod group;
pub mod image;
pub mod path;
pub mod rectangle;
pub mod scrollbar;
pub mod text;
pub mod textbox;

fn patch_if_needed<T: PartialEq + Clone>(
    old_state: &mut Option<T>,
    patch: &mut Option<T>,
    new_value: T,
) -> bool {
    optional_patch_if_needed(old_state, patch, Some(new_value))
}

fn optional_patch_if_needed<T: PartialEq + Clone>(
    old_state: &mut Option<T>,
    patch: &mut Option<T>,
    new_value: Option<T>,
) -> bool {
    if old_state != &new_value {
        *patch = new_value.clone();
        *old_state = new_value;
        true
    } else {
        false
    }
}
