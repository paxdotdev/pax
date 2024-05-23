pub mod button;
pub mod checkbox;
pub mod dropdown;
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
    if !old_state.as_ref().is_some_and(|v| v == &new_value) {
        *patch = Some(new_value.clone());
        *old_state = Some(new_value);
        true
    } else {
        false
    }
}
