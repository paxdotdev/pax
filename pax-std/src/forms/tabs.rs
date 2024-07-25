use crate::*;
use pax_engine::api::*;
use pax_engine::*;
use std::cmp::Ordering;
use std::iter;

#[pax]
#[inlined(
    <Group x=50% height=30px width={100%-4px} @click=on_click>
    	for (name, i) in self.names_filled {
    		<Group x={(100.0*i/(self.slot_count - 1))%} width={(100.0/self.slot_count)%}>
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
    pub names: Property<Vec<String>>,
    pub selected: Property<usize>,
    pub color: Property<Color>,

    // private
    pub slot_count: Property<usize>,
    pub names_filled: Property<Vec<String>>,
}

impl Tabs {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let slot_count = ctx.slot_children_count.clone();
        let deps = [slot_count.untyped()];
        self.slot_count
            .replace_with(Property::computed(move || slot_count.get(), &deps));
        let slot_count = ctx.slot_children_count.clone();
        let names = self.names.clone();
        let deps = [slot_count.untyped(), names.untyped()];
        self.names_filled.replace_with(Property::computed(
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

    pub fn on_click(&mut self, ctx: &NodeContext, event: Event<Click>) {
        let bounds = ctx.bounds_self.get();
        let parts = self.slot_count.get();
        let x = event.mouse.x;
        let id = (x * parts as f64 / bounds.0) as usize;
        self.selected.set(id);
    }
}
