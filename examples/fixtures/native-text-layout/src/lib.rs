use pax_kit::*;

pax_kit::runnable! { root: crate::NativeTextLayout }

#[pax]
#[file("lib.pax")]
pub struct NativeTextLayout {
    pub expanded: Property<bool>,
    pub narrow: Property<bool>,
}

impl NativeTextLayout {
    pub fn record_layout(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        // Snapshot the engine's settled layout as well as the native UIA tree.
        // This works in baked Release without enabling development endpoints.
        record("pax-text-layout: record invoked".to_owned());
        let mut pending = ctx.get_node_interface().into_iter().collect::<Vec<_>>();
        let mut mounted = Vec::new();
        while let Some(node) = pending.pop() {
            pending.extend(node.children());
            mounted.push(node);
        }
        for name in [
            "intrinsic",
            "fixed",
            "paragraph",
            "stack",
            "stack_text",
            "follower",
            "frame",
            "frame_text",
            "repeat_text",
        ] {
            let mut nodes = mounted
                .iter()
                .filter(|node| node.has_id(name))
                .cloned()
                .collect::<Vec<_>>();
            nodes.sort_by_key(|node| node.engine_id().to_u32());
            if nodes.is_empty() {
                record(format!("pax-text-layout: missing name={name}"));
            }
            for (index, node) in nodes.into_iter().enumerate() {
                let name = if name == "repeat_text" {
                    format!("repeat_text_{index}")
                } else {
                    name.to_owned()
                };
                let layout = node.transform_and_bounds().get();
                let transform = layout.transform.coeffs();
                let measured = node.measured_size();
                record(format!(
                    "pax-text-layout: name={} id={} width={} height={} x={} y={} measured={:?} expanded={} narrow={}",
                    name, node.engine_id().to_u32(), layout.bounds.0, layout.bounds.1,
                    transform[4], transform[5], measured, self.expanded.get(), self.narrow.get()
                ));
            }
        }
    }

    pub fn toggle_content(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.expanded.set(!self.expanded.get());
    }

    pub fn toggle_width(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.narrow.set(!self.narrow.get());
    }
}

fn record(line: String) {
    eprintln!("{line}");
    // Windows GUI cartridges do not necessarily inherit the host's stderr.
    // An opt-in log keeps the same snapshot observable in packaged builds.
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(path) = std::env::var_os("PAX_TEXT_LAYOUT_LOG") {
        use std::io::Write;
        let mut log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("open PAX_TEXT_LAYOUT_LOG");
        writeln!(log, "{line}").expect("write PAX_TEXT_LAYOUT_LOG");
    }
}
