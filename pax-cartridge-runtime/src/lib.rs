use std::cell::RefCell;
use std::rc::Rc;
use pax_core::{ComponentInstance, PropertyLiteral, RenderNode, RenderNodePtrList};
use pax_core::pax_properties_coproduct::PropertiesCoproduct;

use pax_runtime_api::{Property, PropertyLiteral, Transform};

//generate dependencies, pointing to userland cartridge (same logic as in PropertiesCoproduct)
use pax_example::pax_types::Root;
use pax_example::pax_types::pax_std::primitives::{Group, GroupProperties, Rectangle, RectangleProperties};
use pax_example::pax_types::pax_std::types::{Color, Stroke, Size};

//dependency paths below come from pax_primitive macro, where these crate+module paths are passed as parameters:
use pax_std_primitives::{RectangleInstance, GroupInstance };


pub fn instantiate_root() -> Rc<RefCell<ComponentInstance>> {
    RootInstance::instantiate(
        PropertiesCoproduct::Root(Root {
            num_clicks: 0,
            current_rotation: 0.0,
            deeper_struct: Default::default()
        }),
        Transform::default(),
        Rc::new(RefCell::new(vec![
            GroupInstance::instantiate(
                PropertiesCoproduct::Group(GroupProperties {}),
                Transform::default(),
                Rc::new(RefCell::new(vec![
                    RectangleInstance::instantiate(
                        PropertiesCoproduct::Rectangle(
                            RectangleProperties {
                                stroke: PropertyLiteral::new( Stroke {}),
                                fill: PropertyLiteral::new(Color::hlca(180.0, 20.0, 20.0, 20.0)),

                            }
                        ),
                        Transform::translate(100.0, 400.0),
                        [PropertyLiteral::new(Size::Pixel(100.0)), PropertyLiteral::new(Size::Pixel(200.0))]
                    ),
                    RectangleInstance::instantiate(
                        PropertiesCoproduct::Rectangle(
                            RectangleProperties {
                                stroke: PropertyLiteral::new( Stroke {}),
                                fill: PropertyLiteral::new(Color::hlca(180.0, 20.0, 20.0, 20.0)),
                                size: [PropertyLiteral::new(Size::Pixel(100.0)), PropertyLiteral::new(Size::Pixel(200.0))]
                            }
                        )
                    ),
                ])),
            ),
            // Rc::new(RefCell::new()),
            // Rc::new(RefCell::new()),
        ]))
    )
}

pub struct RootInstance {}
impl RootInstance {
    pub fn instantiate(properties: PropertiesCoproduct, transform: Transform, children: RenderNodePtrList /*, adoptees*/) -> Rc<RefCell<ComponentInstance>> {
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