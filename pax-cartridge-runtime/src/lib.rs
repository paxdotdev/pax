use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use pax_core::{ComponentInstance, RenderNode, RenderNodePtrList, ComputableProperty, RenderTreeContext};
use pax_core::pax_properties_coproduct::PropertiesCoproduct;

use pax_runtime_api::{Property, PropertyLiteral, Transform};

//generate dependencies, pointing to userland cartridge (same logic as in PropertiesCoproduct)
use pax_example::pax_types::{Root, RootProperties};
use pax_example::pax_types::pax_std::primitives::{Group, GroupProperties, Rectangle, RectangleProperties};
use pax_example::pax_types::pax_std::types::{Color, Stroke, Size};

//dependency paths below come from pax_primitive macro, where these crate+module paths are passed as parameters:
use pax_std_primitives::{RectangleInstance, GroupInstance };




pub fn instantiate_root() -> Rc<RefCell<ComponentInstance>> {
    RootInstance::instantiate(
        PropertiesCoproduct::Root(RootProperties {
            num_clicks: Box::new(PropertyLiteral {value: 0}),
            current_rotation: Box::new(PropertyLiteral {value: 0.0}),
            deeper_struct: Box::new(PropertyLiteral {value: Default::default()})
        }),
        Transform::default(),
        Rc::new(RefCell::new(vec![
            GroupInstance::instantiate(
                PropertiesCoproduct::Group(GroupProperties {}),
                Box::new(PropertyLiteral {value: Transform::default()}),
                Rc::new(RefCell::new(vec![
                    RectangleInstance::instantiate(
                        PropertiesCoproduct::Rectangle(
                            RectangleProperties {
                                stroke: PropertyLiteral::new( Stroke {
                                    color: Color::rgba(1.0, 0.0, 0.0, 1.0),
                                    width: 5.0,
                                }),
                                fill: PropertyLiteral::new(Color::hlca(180.0, 20.0, 20.0, 20.0)),
                            }
                        ),
                        Box::new(PropertyLiteral {value: Transform::translate(100.0, 400.0)}),
                        [PropertyLiteral::new(Size::Pixel(100.0)), PropertyLiteral::new(Size::Pixel(200.0))]
                    ),
                    RectangleInstance::instantiate(
                        PropertiesCoproduct::Rectangle(
                            RectangleProperties {
                                stroke: PropertyLiteral::new( Stroke {
                                    color: Color::rgba(1.0, 1.0, 0.0, 1.0),
                                    width: 5.0,
                                }),
                                fill: PropertyLiteral::new(Color::rgba(0.0, 1.0, 0.0, 1.0)),
                            }
                        ),
                        Box::new(PropertyLiteral {value: Transform::rotate(std::f64::consts::FRAC_1_SQRT_2)}),
                        [PropertyLiteral::new(Size::Pixel(300.0)), PropertyLiteral::new(Size::Pixel(300.0))]
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
            transform: Rc::new(RefCell::new(Box::new(PropertyLiteral{ value: Default::default()}))),
            properties: Rc::new(RefCell::new(properties)),
            compute_properties_fn: Box::new(|mut properties: Rc<RefCell<PropertiesCoproduct>>, rtc: &mut RenderTreeContext|{

                let mut properties_unwrapped = &mut *properties.deref().borrow_mut();
                if let PropertiesCoproduct::Root(properties_cast) =  properties_unwrapped {
                    //Note: this is code-genned based on parsed knowledge of the properties
                    //      of `Root`
                    properties_cast.deeper_struct.compute_in_place(rtc);
                    properties_cast.current_rotation.compute_in_place(rtc);
                    properties_cast.num_clicks.compute_in_place(rtc);
                } else {unreachable!()}
            }),
            timeline: None
        }))
    }

}

//Root => get_instance()


//Rectangle => get_instance()
//Group => get_instance()