use pax_kit::*;
use renamed_widgets::Widget;
#[pax]
#[inlined(@settings {})]
pub struct BlankComponent {}
#[cfg(any())]
mod missing;

#[pax]
#[inlined(<Widget/>)]
pub struct App {
    #[cfg(feature = "fancy")]
    pub app_fancy: Property<bool>,
    #[cfg(widget_config)]
    pub wrongly_inherited: Property<MissingType>,
    #[cfg(test)]
    pub test_only: Property<bool>,
    #[cfg(proof_flag)]
    pub explicit_flag: Property<bool>,
}

#[cfg(any())]
#[pax]
#[file("missing.pax")]
pub struct App {}

#[pax]
#[cfg_attr(all(), inlined(<App/><Widget/>))]
pub struct Shell {}

pax_kit::runnable! { root: crate::Shell, init: crate::init_shell }
pax_kit::runnable! { root: crate::App, init: crate::init_app }
pax_kit::runnable! { root: crate::App, init: crate::init_app }

#[cfg(test)]
mod tests {
    #[test]
    fn generated_descriptors_match_rust_configuration() {
        for manifest in [
            super::__pax_cartridge::init_manifest_for("crate::App"),
            super::__pax_cartridge::init_manifest_for("crate::Shell"),
        ] {
            let properties = |name: &str| {
                manifest
                    .type_table
                    .iter()
                    .find(|(id, _)| id.to_string() == name)
                    .unwrap()
                    .1
                    .property_definitions
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>()
            };
            let app = properties("crate::App");
            assert_eq!(app.contains(&"app_fancy"), cfg!(feature = "fancy"));
            assert!(app.contains(&"test_only"));
            assert!(!app.contains(&"wrongly_inherited"));
            let widget = properties("renamed_widgets::selected::Widget");
            assert_eq!(widget.contains(&"fancy"), cfg!(feature = "widget-fancy"));
            assert_eq!(widget.contains(&"plain"), !cfg!(feature = "widget-fancy"));
            assert_eq!(widget.contains(&"web"), cfg!(target_arch = "wasm32"));
            assert_eq!(widget.contains(&"native"), !cfg!(target_arch = "wasm32"));
            assert_eq!(widget.contains(&"debug"), cfg!(debug_assertions));
            assert!(!widget.contains(&"impossible"));
            assert!(!widget.contains(&"dependency_test_only"));
            let leaf = manifest
                .type_table
                .iter()
                .find(|(id, _)| id.to_string().ends_with("::Leaf"))
                .unwrap()
                .1;
            assert!(leaf
                .property_definitions
                .iter()
                .any(|p| p.name == "selected"));
            assert!(!leaf
                .property_definitions
                .iter()
                .any(|p| p.name == "impossible" || p.name == "dependency_test_only"));
            let component = manifest
                .components
                .iter()
                .find(|(id, _)| id.to_string() == "renamed_widgets::selected::Widget")
                .unwrap()
                .1;
            let settings = format!("{:?}", component.settings);
            assert!(settings.contains(if cfg!(feature = "widget-fancy") {
                "fancy"
            } else {
                "plain"
            }));
        }
    }
}

static APP_INITIALIZATIONS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static SHELL_INITIALIZATIONS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
fn init_app(_: &InitContext) -> AppResult<App> {
    APP_INITIALIZATIONS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(App::default())
}
fn init_shell(_: &InitContext) -> AppResult<Shell> {
    SHELL_INITIALIZATIONS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(Shell::default())
}

// Fixture-only observation across the same C boundary used by a native host.
#[no_mangle]
pub extern "C" fn cfg_initializer_count(shell: bool) -> usize {
    (if shell {
        &SHELL_INITIALIZATIONS
    } else {
        &APP_INITIALIZATIONS
    })
    .load(std::sync::atomic::Ordering::SeqCst)
}
