use pax_engine::{
    api::{NodeContext, Size, Store},
    pax, Property,
};

use super::PathElement;

pub struct PathContext {
    pub elements: Property<Vec<PathElement>>,
}

impl Store for PathContext {}

#[pax]
#[inlined( @settings { @mount: on_mount @pre_render: pre_render })]
pub struct PathPoint {
    pub id: Property<usize>,
    pub x: Property<Size>,
    pub y: Property<Size>,
    pub on_change: Property<bool>,
}

impl PathPoint {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let path_elems = ctx
            .peek_local_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path point can only exist in <Path> tag");

        let x = self.x.clone();
        let y = self.y.clone();
        let id = self.id.clone();
        let deps = [x.untyped(), y.untyped(), id.untyped()];
        self.on_change.replace_with(Property::computed(
            move || {
                path_elems.update(|elems| {
                    let id = id.get();
                    while elems.len() < id + 1 {
                        elems.push(PathElement::Close)
                    }
                    elems[id] = PathElement::point(x.get(), y.get());
                    log::debug!("elems are: {:?}", elems);
                });
                false
            },
            &deps,
        ));
    }

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        // trigger dirty prop to fire closure
        self.on_change.get();
    }
}

#[pax]
#[inlined( @settings { @mount: on_mount @pre_render: pre_render })]
pub struct PathLine {
    pub id: Property<usize>,
    pub on_change: Property<bool>,
}

impl PathLine {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let path_elems = ctx
            .peek_local_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path line can only exist in <Path> tag");

        let id = self.id.clone();
        let deps = [id.untyped()];
        self.on_change.replace_with(Property::computed(
            move || {
                path_elems.update(|elems| {
                    let id = id.get();
                    while elems.len() < id + 1 {
                        elems.push(PathElement::Close)
                    }
                    elems[id] = PathElement::line();
                    log::debug!("elems are: {:?}", elems);
                });
                false
            },
            &deps,
        ));
    }

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        // trigger dirty prop to fire closure
        self.on_change.get();
    }
}
