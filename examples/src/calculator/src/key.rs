use crate::CalculatorStore;
use pax_kit::*;

#[pax]
#[file("key.pax")]
pub struct CalculatorKey {
    pub label: Property<String>,
    pub action: Property<String>,
    pub tone: Property<usize>,
    pub pressed: Property<f64>,
    pub hovered: Property<bool>,
    pub surface: Property<Material>,
    pub unlit: Property<Material>,
}
impl CalculatorKey {
    pub fn mount(&mut self, _ctx: &NodeContext) {
        self.surface.set(Material::matte());
        self.unlit.set(Material::unlit());
    }
    pub fn activate(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        let _ = ctx
            .peek_local_store(|store: &mut CalculatorStore| store.model.action(&self.action.get()));
    }
    pub fn down(&mut self, _ctx: &NodeContext, _event: Event<MouseDown>) {
        self.press();
    }
    pub fn up(&mut self, _ctx: &NodeContext, _event: Event<MouseUp>) {
        self.release();
    }
    pub fn over(&mut self, _ctx: &NodeContext, _event: Event<MouseOver>) {
        self.hovered.set(true);
    }
    pub fn out(&mut self, _ctx: &NodeContext, _event: Event<MouseOut>) {
        self.hovered.set(false);
        self.release();
    }
    pub fn touch(&mut self, _ctx: &NodeContext, _event: Event<TouchStart>) {
        self.press();
    }
    pub fn touch_end(&mut self, _ctx: &NodeContext, _event: Event<TouchEnd>) {
        self.release();
    }
    pub fn cancel(&mut self, _ctx: &NodeContext, _event: Event<TouchCancel>) {
        self.release();
    }
    fn press(&mut self) {
        self.pressed
            .ease_to(2., Duration::Milliseconds(55.into()), EasingCurve::OutQuad);
    }
    fn release(&mut self) {
        self.pressed
            .ease_to(0., Duration::Milliseconds(120.into()), EasingCurve::OutQuad);
    }
}
