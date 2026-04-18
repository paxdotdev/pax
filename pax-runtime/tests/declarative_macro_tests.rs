#[derive(Default, Debug, PartialEq)]
struct Color {
    fill: String,
}

#[test]
fn default_color_is_empty() {
    assert_eq!(
        Color::default(),
        Color {
            fill: String::new()
        }
    );
}
