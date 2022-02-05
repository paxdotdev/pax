use std::cell::RefCell;
use std::rc::Rc;
use pax_core::{ComponentInstance, RenderNode, RenderNodePtrList};
use pax_core::pax_properties_coproduct::PropertiesCoproduct;

//generate dependencies, pointing to userland cartridge (same logic as in PropertiesCoproduct)
use pax_example::pax_types::Root;
use pax_example::pax_types::pax_std::primitives::{Group, Rectangle};
use pax_example::pax_types::pax_std::types::{Color, Stroke, Transform, Size};

//dependency paths below come from pax_primitive macro, where these crate+module paths are passed as parameters:
use pax_std_primitives::RectangleInstance;
use pax_std_primitives::GroupInstance;

pub fn instantiate_root() -> Rc<RefCell<ComponentInstance>> {
    RootInstance::instantiate(
        PropertiesCoproduct::Root(Root {
            num_clicks: 0,
            current_rotation: 0.0,
            deeper_struct: Default::default()
        }),
        Rc::new(RefCell::new(vec![
            GroupInstance::instantiate(
                PropertiesCoproduct::Group(Group {
                    transform: Transform::default()
                }),
                Rc::new(RefCell::new(vec![
                    RectangleInstance::instantiate(
                        PropertiesCoproduct::Rectangle(
                            Rectangle {
                                stroke: Stroke {},
                                fill: Color::hsla(10.0, 1.0, 1.0, 0.75),
                                size: [Size::Pixel(100.0), Size::Pixel(200.0)]
                            }
                        )
                    ),
                ]))
            ),
            // Rc::new(RefCell::new()),
            // Rc::new(RefCell::new()),
        ]))
    )
}

pub struct RootInstance {}
impl RootInstance {
    pub fn instantiate(properties: PropertiesCoproduct, children: RenderNodePtrList /*, adoptees*/) -> Rc<RefCell<ComponentInstance>> {
        Rc::new(RefCell::new(ComponentInstance {
            template: children,
            adoptees: Rc::new(RefCell::new(vec![])),
            transform: Rc::new(RefCell::new(Default::default())),
            properties: Rc::new(RefCell::new(properties)),
            timeline: None
        }))
    }
}

//Root => get_instance()


//Rectangle => get_instance()
//Group => get_instance()