#[allow(unused)]
use crate::*;
use pax_engine::api::*;
use pax_engine::*;

/// A component displaying a list of tabs, e.g. for tabbed navigation.
#[pax]
#[engine_import_path("pax_engine")]
#[inlined(
    <Group x=50% height=30px width={100%-4px} @click=on_click>
        for (name, i) in self._names_filled {
            <Group x={(100.0*i/(self._slot_count - 1))%} width={(100.0/self._slot_count)%}>
    			//highlight selected
    			<Rectangle x=50% y=100% width={100%-4px} height={100%-2px} fill={rgba(255, 255, 255, 30*(i == self.selected))}
    			    corner_radii={RectangleCornerRadii::radii(10.0,10.0,0.0,0.0)}
    			/>
    			<Text align={TextAlignHorizontal::Center} width=100% height=100% text={name}/>
    			<Rectangle x=50% y=100% width={100%-4px} height={100%-2px} fill={self.color}
    			    corner_radii={RectangleCornerRadii::radii(10.0,10.0,0.0,0.0)}
    			/>
    		</Group>
    	}
    </Group>

    <Group y=30px height={100% - 30px}>
    	slot(self.selected)
    </Group>
    <Rectangle y=30px height={100% - 30px} fill={self.color}/>

    @settings {
        @mount: on_mount
    }
)]
pub struct Tabs {
    /// A list of string labels for the tabs
    pub names: Property<Vec<String>>,
    /// The index of the currently selected tab
    pub selected: Property<usize>,
    /// The background color of the tabs
    pub color: Property<Color>,

    // Private slot count mirrored from `NodeContext`.
    pub _slot_count: Property<usize>,
    // Private tab labels after filling missing labels with placeholders.
    pub _names_filled: Property<Vec<String>>,
}

impl Tabs {
    // Mirrors slot count and derives fallback tab labels for the inline template.
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let slot_count = ctx.projected_children_count.clone();
        let deps = [slot_count.untyped()];
        self._slot_count
            .replace_with(Property::computed(move || slot_count.get(), &deps));
        let slot_count = ctx.projected_children_count.clone();
        let names = self.names.clone();
        let deps = [slot_count.untyped(), names.untyped()];
        self._names_filled.replace_with(Property::computed(
            move || {
                let names = names.get();
                let mut names_filled = vec![];
                for i in 0..slot_count.get() {
                    names_filled.push(
                        names
                            .get(i)
                            .map(|s| s.as_str())
                            .unwrap_or("[no name]")
                            .to_owned(),
                    );
                }
                names_filled
            },
            &deps,
        ));
    }

    // Selects the tab segment under the click.
    pub fn on_click(&mut self, ctx: &NodeContext, event: Event<Click>) {
        let bounds = ctx.bounds_self.get();
        let parts = self._slot_count.get();
        let x = event.mouse.x;
        let id = (x * parts as f64 / bounds.0) as usize;
        self.selected.set(id);
    }
}
