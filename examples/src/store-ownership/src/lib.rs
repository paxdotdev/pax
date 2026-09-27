use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
#[custom(Default)]
pub struct StoreOwnership {
    pub show_left: Property<bool>,
    pub keys: Property<Vec<usize>>,
    pub combo_left: Property<Option<usize>>,
    pub combo_right: Property<Option<usize>>,
    pub sources: Property<Vec<ExampleSource>>,
}

impl Default for StoreOwnership {
    fn default() -> Self {
        Self {
            show_left: Property::new(true),
            keys: Property::new(vec![10, 20, 30]),
            combo_left: Property::new(None),
            combo_right: Property::new(None),
            sources: Property::new(vec![
                ExampleSource {
                    label: "Pax".into(),
                    language: "pax".into(),
                    code: "<Text text=\"Independent\"/>".into(),
                },
                ExampleSource {
                    label: "Rust".into(),
                    language: "rust".into(),
                    code: "// Each host owns its selection".into(),
                },
            ]),
        }
    }
}

impl StoreOwnership {
    pub fn toggle_left(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.show_left.set(!self.show_left.get());
    }
    pub fn reverse(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.keys.update(|keys| keys.reverse());
    }
    pub fn toggle_item(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.keys.update(|keys| {
            if let Some(index) = keys.iter().position(|key| *key == 20) {
                keys.remove(index);
            } else {
                keys.push(20);
            }
        });
    }
}

pub struct CounterStore {
    value: Property<usize>,
}
impl Store for CounterStore {}

#[pax]
#[file("provider.pax")]
pub struct CounterProvider {
    pub label: Property<String>,
    pub count: Property<usize>,
}

impl CounterProvider {
    pub fn mount(&mut self, ctx: &NodeContext) {
        ctx.provide_store(CounterStore {
            value: self.count.clone(),
        })
        .expect("counter provider is mounting");
    }
}

#[pax]
#[file("consumer.pax")]
pub struct CounterConsumer {
    pub label: Property<String>,
    pub observed: Property<usize>,
}

impl CounterConsumer {
    pub fn mount(&mut self, ctx: &NodeContext) {
        let value = ctx
            .with_store(|store: &mut CounterStore| store.value.clone())
            .expect("counter consumer needs a provider");
        let deps = [value.untyped()];
        self.observed
            .replace_with(Property::computed(move || value.get(), &deps));
    }
    pub fn increment(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        let value = ctx
            .with_store(|store: &mut CounterStore| store.value.clone())
            .expect("counter provider remains mounted");
        value.update(|value| *value += 1);
    }
}
