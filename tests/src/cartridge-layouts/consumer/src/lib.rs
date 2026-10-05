use pax_kit::*;
#[allow(unused_imports)]
use workspace_ui::ui::App;

pax_kit::runnable! { root: Shell }

#[pax]
#[inlined(<App width=100% height=100%/>)]
pub struct Shell {}
