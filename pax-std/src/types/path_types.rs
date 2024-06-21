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
#[inlined( @settings { @mount: on_mount @pre_render: pre_render @unmount: on_unmount })]
pub struct PathPoint {
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
        let id = ctx.slot_index.clone();
        let deps = [x.get_id(), y.get_id(), id.get_id()];
        self.on_change.replace_with(Property::expression(
            move || {
                path_elems.update(|elems| {
                    let id = id.get().unwrap();
                    while elems.len() < id + 1 {
                        elems.push(PathElement::Close)
                    }
                    elems[id] = PathElement::point(x.get(), y.get());
                });
                false
            },
            &deps, "PathPoint.on_change"
        ));
    }

    pub fn on_unmount(&mut self, ctx: &NodeContext) {
        let path_elems = ctx
            .peek_local_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path point can only exist in <Path> tag");
        let id = ctx.slot_index.get().unwrap();
        path_elems.update(|elems| {
            if id < elems.len() {
                elems.remove(id);
            }
        });
    }

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        // trigger dirty prop to fire closure
        self.on_change.get();
    }
}

#[pax]
#[inlined( @settings { @mount: on_mount @pre_render: pre_render @unmount: on_unmount })]
pub struct PathLine {
    pub on_change: Property<bool>,
}

impl PathLine {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let path_elems = ctx
            .peek_local_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path line can only exist in <Path> tag");

        let id = ctx.slot_index.clone();
        let deps = [id.get_id()];
        self.on_change.replace_with(Property::expression(
            move || {
                path_elems.update(|elems| {
                    let id = id.get().unwrap();
                    while elems.len() < id + 1 {
                        elems.push(PathElement::Close)
                    }
                    elems[id] = PathElement::line();
                });
                false
            },
            &deps, "PathLine.on_change"
        ));
    }

    pub fn on_unmount(&mut self, ctx: &NodeContext) {
        let path_elems = ctx
            .peek_local_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path point can only exist in <Path> tag");
        let id = ctx.slot_index.get().unwrap();
        path_elems.update(|elems| {
            if id < elems.len() {
                elems.remove(id);
            }
        });
    }
    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        // trigger dirty prop to fire closure
        self.on_change.get();
    }
}

#[pax]
#[inlined( @settings { @mount: on_mount @pre_render: pre_render @unmount: on_unmount })]
pub struct PathClose {
    pub on_change: Property<bool>,
}

impl PathClose {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let path_elems = ctx
            .peek_local_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path line can only exist in <Path> tag");

        let id = ctx.slot_index.clone();
        let deps = [id.get_id()];
        self.on_change.replace_with(Property::expression(
            move || {
                path_elems.update(|elems| {
                    let id = id.get().unwrap();
                    while elems.len() < id + 1 {
                        elems.push(PathElement::Close)
                    }
                    elems[id] = PathElement::close();
                });
                false
            },
            &deps, "PathClose.on_change"
        ));
    }
    pub fn on_unmount(&mut self, ctx: &NodeContext) {
        let path_elems = ctx
            .peek_local_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path point can only exist in <Path> tag");
        let id = ctx.slot_index.clone();
        path_elems.update(|elems| {
            let id = id.get().unwrap();
            if id < elems.len() {
                elems.remove(id);
            }
        });
    }

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        // trigger dirty prop to fire closure
        self.on_change.get();
    }
}

#[pax]
#[inlined( @settings { @mount: on_mount @pre_render: pre_render @unmount: on_unmount })]
pub struct PathCurve {
    pub x: Property<Size>,
    pub y: Property<Size>,
    pub on_change: Property<bool>,
}

impl PathCurve {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let path_elems = ctx
            .peek_local_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path point can only exist in <Path> tag");

        let x = self.x.clone();
        let y = self.y.clone();
        let id = ctx.slot_index.clone();
        let deps = [x.get_id(), y.get_id(), id.get_id()];
        self.on_change.replace_with(Property::expression(
            move || {
                path_elems.update(|elems| {
                    let id = id.get().unwrap();
                    while elems.len() < id + 1 {
                        elems.push(PathElement::Close)
                    }
                    elems[id] = PathElement::curve(x.get(), y.get());
                });
                false
            },
            &deps, "PathCurve.on_change"
        ));
    }

    pub fn on_unmount(&mut self, ctx: &NodeContext) {
        let path_elems = ctx
            .peek_local_store(|path_ctx: &mut PathContext| path_ctx.elements.clone())
            .expect("path point can only exist in <Path> tag");
        let id = ctx.slot_index.get().unwrap();
        path_elems.update(|elems| {
            if id < elems.len() {
                elems.remove(id);
            }
        });
    }

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        // trigger dirty prop to fire closure
        self.on_change.get();
    }
}
