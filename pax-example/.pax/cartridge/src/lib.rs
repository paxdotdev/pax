
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::ops::Deref;
use std::rc::Rc;
use pax_core::{ComponentInstance, RenderNodePtr, PropertyExpression, RenderNodePtrList, RenderTreeContext, ExpressionContext, PaxEngine, RenderNode, InstanceRegistry, HandlerRegistry, InstantiationArgs, ConditionalInstance, SlotInstance, StackFrame};
use pax_core::pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use pax_core::repeat::{RepeatInstance};
use piet_common::RenderContext;

use pax_runtime_api::{ArgsCoproduct, Size, SizePixels, PropertyInstance, PropertyLiteral, Size2D, Transform2D};

//generate dependencies, pointing to userland cartridge (same logic as in PropertiesCoproduct)
use pax_example::pax_types::{HelloWorld};
use pax_example::pax_types::pax_std::primitives::{Rectangle, Group, Text};
use pax_example::pax_types::pax_std::types::{Color, Font, Stroke, StackerCellProperties, StackerDirection};
use pax_example::pax_types::pax_std::components::Stacker;

//dependency paths below come from pax_primitive macro, where these crate+module paths are passed as parameters:
use pax_std_primitives::{RectangleInstance, GroupInstance, ScrollerInstance, FrameInstance, TextInstance};

const JABBERWOCKY : &str = r#"’Twas brillig, and the slithy toves
Did gyre and gimble in the wabe:
All mimsy were the borogoves,
And the mome raths outgrabe.

“Beware the Jabberwock, my son!
The jaws that bite, the claws that catch!
Beware the Jubjub bird, and shun
The frumious Bandersnatch!”

He took his vorpal sword in hand;
Long time the manxome foe he sought—
So rested he by the Tumtum tree
And stood awhile in thought.

And, as in uffish thought he stood,
The Jabberwock, with eyes of flame,
Came whiffling through the tulgey wood,
And burbled as it came!

One, two! One, two! And through and through
The vorpal blade went snicker-snack!
He left it dead, and with its head
He went galumphing back.

“And hast thou slain the Jabberwock?
Come to my arms, my beamish boy!
O frabjous day! Callooh! Callay!”
He chortled in his joy.

’Twas brillig, and the slithy toves
Did gyre and gimble in the wabe:
All mimsy were the borogoves,
And the mome raths outgrabe.
"#;

pub fn instantiate_expression_table<R: 'static + RenderContext>() -> HashMap<u64, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct>> {
    let mut vtable: HashMap<u64, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct>> = HashMap::new();

    //Note: this is probably the source of most (all?) of instance churn
    //this expression handles re-packing `data_list` for
    //`@for (elem, i) in computed_layout_spec {`
    vtable.insert(0, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {

        //note this unwrapping is nested inside the `if let`, rather than flatted into a single assignment.
        //This is necessary for the non-clonable `Vec` in this case, and might need/want to be applied to codegen template (that is: nesting instead of implicit cloning, e.g. of primitive types)
        #[allow(non_snake_case)]
        if let PropertiesCoproduct::Stacker(p) = &*(*(*ec.stack_frame).borrow().get_properties()).borrow() {
            let computed_layout_spec = p.computed_layout_spec.get();
            return TypesCoproduct::Vec_Rc_PropertiesCoproduct___(computed_layout_spec.iter().enumerate().map(|(i,e)|{
                let cloned = Rc::clone(e);

                //TODO: there should be a way to pull off this re-wrapping without cloning the data structure (below).
                let rewrapped = PropertiesCoproduct::StackerCellProperties((*cloned).clone());
                Rc::new(rewrapped)
            }).collect());
        } else { unreachable!("{}",0) };

    }));

    vtable.insert(1, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        let (datum, i) = if let PropertiesCoproduct::RepeatItem(datum, i) = &*(*(*ec.stack_frame).borrow().get_properties()).borrow() {

            (Rc::clone(datum), *i)
        } else { unreachable!("{}",1) };

        let datum_cast = if let PropertiesCoproduct::StackerCellProperties(d)= &*datum {d} else {unreachable!("{}",1)};

        return TypesCoproduct::Transform2D(
            Transform2D::translate(datum_cast.x_px, datum_cast.y_px)
        )
    }));

    //Frame size x
    vtable.insert(2, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        let (datum, i) = if let PropertiesCoproduct::RepeatItem(datum, i) = &*(*(*ec.stack_frame).borrow().get_properties()).borrow() {

            (Rc::clone(datum), *i)
        } else { unreachable!("{}",2) };

        let datum_cast = if let PropertiesCoproduct::StackerCellProperties(d)= &*datum {d} else {unreachable!("{}","epsilon")};

        return TypesCoproduct::Size(
            Size::Pixels(datum_cast.width_px)
        )
    }));

    //Frame size y
    vtable.insert(3, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        let (datum, i) = if let PropertiesCoproduct::RepeatItem(datum, i) = &*(*(*ec.stack_frame).borrow().get_properties()).borrow() {

            (Rc::clone(datum), *i)
        } else { unreachable!("{}",3) };

        let datum_cast = if let PropertiesCoproduct::StackerCellProperties(d)= &*datum {d} else {unreachable!("{}",123)};

        return TypesCoproduct::Size(
            Size::Pixels(datum_cast.height_px)
        )
    }));

    //Frame index
    vtable.insert(4, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        let (datum, i) = if let PropertiesCoproduct::RepeatItem(datum, i) = &*(*(*ec.stack_frame).borrow().get_properties()).borrow() {

            (Rc::clone(datum), *i)
        } else { unreachable!("{}",4) };

        return TypesCoproduct::usize(
            i
        );
    }));

    //Propeller rectangle
    vtable.insert(5, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        #[allow(non_snake_case)]

        const STACK_FRAME_OFFSET : isize = 2;
        let SCOPED_STACK_FRAME = (*ec.stack_frame).borrow().nth_descendant(STACK_FRAME_OFFSET); //just gen `ec.stack_frame` if offset == 0

        let properties = SCOPED_STACK_FRAME.deref().borrow().get_properties();
        let properties = &*(*properties).borrow();

        let current_rotation = if let PropertiesCoproduct::HelloWorld(p) = properties {
            *p.current_rotation.get() as f64
        } else { unreachable!("{}",5) };

        TypesCoproduct::Transform2D(
            Transform2D::anchor(Size::Percent(50.0), Size::Percent(50.0))
                * Transform2D::align(Size::Percent(50.0), Size::Percent(50.0))
                * Transform2D::rotate(current_rotation)
        )
    }));

    //Text content
    vtable.insert(6, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        let (datum, i) = if let PropertiesCoproduct::RepeatItem(datum, i) = &*(*(*ec.stack_frame).borrow().get_properties()).borrow() {
            (Rc::clone(datum), *i)
        } else { unreachable!("{}",6) };

        return TypesCoproduct::String(
            format!("{}", i)
        );
    }));

    vtable.insert(7, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        let (datum, i) = if let PropertiesCoproduct::RepeatItem(datum, i) = &*(*(*ec.stack_frame).borrow().get_properties()).borrow() {
            (Rc::clone(datum), *i)
        } else { unreachable!("{}",7) };

        return TypesCoproduct::Color(
            Color::rgba(0.2 * (i as f64), 0.0, 0.75, 1.0)
        );
    }));

    vtable.insert(8, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        let (datum, i) = if let PropertiesCoproduct::RepeatItem(datum, i) = &*(*(*ec.stack_frame).borrow().get_properties()).borrow() {
            (Rc::clone(datum), *i)
        } else { unreachable!("{}",8) };
        return TypesCoproduct::Transform2D(
            Transform2D::anchor(Size::Percent(0.0), Size::Percent(i as f64 * 14.286)) *
                Transform2D::align(Size::Percent(0.0), Size::Percent(i as f64 * 14.286))
        );
    }));

    // {Color::rgba(100%, (100 - (i * 12.5))%, (i * 12.5)%, 100%)}
    vtable.insert(9, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        let (datum, i) = if let PropertiesCoproduct::RepeatItem(datum, i) = &*(*(*ec.stack_frame).borrow().get_properties()).borrow() {
            (Rc::clone(datum), *i)
        } else { unreachable!("{}",9) };

        return TypesCoproduct::Color(
            Color::rgba(1.0, 1.0 - (i as f64 * 0.125), i as f64 * 0.125, 1.0)
        );
    }));

    // {(20 + (i * 5))px}
    vtable.insert(10, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        let (datum, i) = if let PropertiesCoproduct::RepeatItem(datum, i) = &*(*(*ec.stack_frame).borrow().get_properties()).borrow() {
            (Rc::clone(datum), *i)
        } else { unreachable!("{}",10) };

        return TypesCoproduct::SizePixels(
            SizePixels(20.0 + (i as f64 * 5.0))
        );
    }));

    vtable
}

pub fn instantiate_component_stacker<R: 'static + RenderContext>(instance_registry: Rc<RefCell<InstanceRegistry<R>>>, mut args: InstantiationArgs<R>) -> Rc<RefCell<ComponentInstance<R>>>  {
    args.component_template = Some(Rc::new(RefCell::new(
        vec![
            RepeatInstance::instantiate(InstantiationArgs {
                properties: PropertiesCoproduct::None,
                handler_registry: None,
                instance_registry: Rc::clone(&instance_registry),
                transform: Transform2D::default_wrapped(),
                size: None,
                component_template: None,
                children: Some(Rc::new(RefCell::new(vec![
                    FrameInstance::instantiate(InstantiationArgs{
                        properties: PropertiesCoproduct::None,
                        handler_registry: None,
                        instance_registry: Rc::clone(&instance_registry),
                        transform: Rc::new(RefCell::new(PropertyExpression::new(1))),
                        size: Some([
                            Box::new(PropertyExpression::new(2)),
                            Box::new(PropertyExpression::new(3)),
                        ]),
                        children: Some(Rc::new(RefCell::new(vec![
                            SlotInstance::instantiate(InstantiationArgs {
                                properties: PropertiesCoproduct::None,
                                handler_registry: None,
                                instance_registry: Rc::clone(&instance_registry),
                                transform: Transform2D::default_wrapped(),
                                size: Some([PropertyLiteral::new(Size::Percent(100.0)).into(),PropertyLiteral::new(Size::Percent(100.0)).into()]),
                                children: None,
                                component_template: None,
                                scroller_args: None,
                                slot_index: Some(Box::new(PropertyExpression::new(4))),
                                repeat_data_list: None,
                                conditional_boolean_expression: None,
                                compute_properties_fn: None
                            }),
                        ]))),
                        component_template: None,
                        scroller_args: None,
                        slot_index: None,
                        repeat_data_list: None,
                        conditional_boolean_expression: None,
                        compute_properties_fn: None
                    }),
                ]))),
                slot_index: None,
                repeat_data_list: Some(Box::new(PropertyExpression::new(0))),
                conditional_boolean_expression: None,
                compute_properties_fn: None,
                scroller_args: None
            }),
        ]
    )));

    args.handler_registry = Some(Rc::new(RefCell::new(
        HandlerRegistry {
            click_handlers: vec![],
            will_render_handlers: vec![
                |properties,args|{
                    let properties = &mut *properties.as_ref().borrow_mut();
                    let properties = if let PropertiesCoproduct::Stacker(p) = properties {p} else {unreachable!("{}",123)};
                    Stacker::handle_will_render(properties, args);
                }
            ],
        }
    )));

    args.compute_properties_fn = Some(Box::new(|properties, rtc|{
        let properties = &mut *properties.as_ref().borrow_mut();
        let properties = if let PropertiesCoproduct::Stacker(p) = properties {p} else {unreachable!("{}",123)};

        // if let Some(new_value) = rtc.get_eased_value(properties.direction._get_transition_manager()) {
        //     properties.direction.set(new_value);
        // }else
        if let Some(new_value) = rtc.compute_vtable_value(properties.direction._get_vtable_id()) {
            let new_value = if let TypesCoproduct::StackerDirection(v) = new_value { v } else { unreachable!("{}",123) };
            properties.direction.set(new_value);
        }

        if let Some(new_value) = rtc.compute_vtable_value(properties.cells._get_vtable_id()) {
            let new_value = if let TypesCoproduct::usize(v) = new_value { v } else { unreachable!("{}",123) };
            properties.cells.set(new_value);
        }

        if let Some(new_value) = rtc.compute_vtable_value(properties.gutter_width._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Size(v) = new_value { v } else { unreachable!("{}",123) };
            properties.gutter_width.set(new_value);
        }

        if let Some(new_value) = rtc.compute_vtable_value(properties.overrides_cell_size._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Vec_LPAREN_usize_COMMA_Size_RPAREN(v) = new_value { v } else { unreachable!("{}",123) };
            properties.overrides_cell_size.set(new_value);
        }

        if let Some(new_value) = rtc.compute_vtable_value(properties.overrides_gutter_size._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Vec_LPAREN_usize_COMMA_Size_RPAREN(v) = new_value { v } else { unreachable!("{}",123) };
            properties.overrides_gutter_size.set(new_value);
        }

    }));

    ComponentInstance::instantiate(args)
}

pub fn instantiate_root_component<R: 'static + RenderContext>(instance_registry: Rc<RefCell<InstanceRegistry<R>>>) -> Rc<RefCell<ComponentInstance<R>>> {
    //Root
    ComponentInstance::instantiate(
        InstantiationArgs{
            properties: PropertiesCoproduct::HelloWorld(HelloWorld {
                //these values are code-genned by pax-compiler.  If not provided, pax-compiler
                //can inject Default::default.  If the rust compiler throws an error,
                //that is the author's responsibility.
                num_clicks: Default::default(),
                current_rotation: Default::default(),
            }),
            handler_registry: Some(Rc::new(RefCell::new(HandlerRegistry {
                click_handlers: vec![],
                will_render_handlers: vec![
                    |properties,args|{
                        let properties = &mut *properties.as_ref().borrow_mut();
                        let properties = if let PropertiesCoproduct::HelloWorld(p) = properties {p} else {unreachable!("{}",123)};
                        HelloWorld::handle_will_render(properties, args);
                    }
                ]
            }))),
            instance_registry: Rc::clone(&instance_registry),
            transform: Transform2D::default_wrapped(),
            size: None,
            children: None,
            component_template: Some(Rc::new(RefCell::new(vec![
                //Horizontal stacker
                instantiate_component_stacker(
                    Rc::clone(&instance_registry),
                    InstantiationArgs {
                        properties: PropertiesCoproduct::Stacker(Stacker {
                            computed_layout_spec: Default::default(),
                            direction: Default::default(),
                            cells: Box::new(PropertyLiteral::new(10)),
                            gutter_width: Box::new(PropertyLiteral::new(Size::Pixels(5.0))),
                            overrides_cell_size: Default::default(),
                            overrides_gutter_size: Default::default(),
                        }),
                        handler_registry: None,
                        instance_registry: Rc::clone(&instance_registry),
                        transform: Rc::new(RefCell::new(PropertyLiteral::new(Transform2D::rotate(0.0)))),
                        size: Some([Box::new(PropertyLiteral::new(Size::Percent(100.0))), Box::new(PropertyLiteral::new(Size::Percent(100.0)))]),
                        children: Some(Rc::new(RefCell::new(vec![
                            //Vertical stacker
                            instantiate_component_stacker(
                                Rc::clone(&instance_registry),
                                InstantiationArgs {
                                    properties: PropertiesCoproduct::Stacker(Stacker {
                                        computed_layout_spec: Default::default(),
                                        direction: Box::new(PropertyLiteral::new(StackerDirection::Vertical)),
                                        cells: Box::new(PropertyLiteral::new(5)),
                                        gutter_width: Box::new(PropertyLiteral::new(Size::Pixels(5.0))),
                                        overrides_cell_size: Default::default(),
                                        overrides_gutter_size: Default::default(),
                                    }),
                                    handler_registry: None,
                                    instance_registry: Rc::clone(&instance_registry),
                                    transform: Rc::new(RefCell::new(PropertyLiteral::new(Transform2D::rotate(0.0)))),
                                    size: Some([Box::new(PropertyLiteral::new(Size::Percent(100.0))), Box::new(PropertyLiteral::new(Size::Percent(100.0)))]),
                                    children: Some(Rc::new(RefCell::new(vec![
                                        RepeatInstance::instantiate(InstantiationArgs {
                                            properties: PropertiesCoproduct::None,
                                            handler_registry: None,
                                            instance_registry: Rc::clone(&instance_registry),
                                            transform: Transform2D::default_wrapped(),
                                            size: None,
                                            children: Some(Rc::new(RefCell::new( vec![
                                                GroupInstance::instantiate(InstantiationArgs {
                                                    properties: PropertiesCoproduct::Group(Group{}),
                                                    handler_registry: None,
                                                    instance_registry: Rc::clone(&instance_registry),
                                                    transform: Transform2D::default_wrapped(),
                                                    size: Some([PropertyLiteral::new(Size::Percent(100.0)).into(),PropertyLiteral::new(Size::Percent(100.0)).into()]),
                                                    children: Some(Rc::new(RefCell::new(vec![
                                                        TextInstance::instantiate(InstantiationArgs {
                                                            properties: PropertiesCoproduct::Text( Text {
                                                                content: Box::new(PropertyLiteral::new("Hello".to_string()) ),
                                                                fill: Box::new(PropertyLiteral::new(Color::rgba(1.0,1.0,1.0,1.0))),
                                                                font: Default::default(),
                                                            }),
                                                            handler_registry: None,
                                                            instance_registry: Rc::clone(&instance_registry),
                                                            transform: Transform2D::default_wrapped(),
                                                            size: Some([PropertyLiteral::new(Size::Percent(100.0)).into(),PropertyLiteral::new(Size::Percent(100.0)).into()]),
                                                            children: None,
                                                            component_template: None,
                                                            scroller_args: None,
                                                            slot_index: None,
                                                            repeat_data_list: None,
                                                            conditional_boolean_expression: None,
                                                            compute_properties_fn: None
                                                        }),
                                                        RectangleInstance::instantiate(InstantiationArgs{
                                                            properties: PropertiesCoproduct::Rectangle(Rectangle {
                                                                stroke: pax_example::pax_types::pax_std::types::Stroke{
                                                                    color: Box::new(PropertyLiteral::new(Color::rgba(0.0,0.0,0.0,0.0))),
                                                                    width: Box::new(PropertyLiteral::new(SizePixels(0.0))),
                                                                },
                                                                fill: Box::new(PropertyExpression::new(7))
                                                            }),
                                                            handler_registry: None,
                                                            instance_registry: Rc::clone(&instance_registry),
                                                            transform: Transform2D::default_wrapped(),
                                                            size: Some([PropertyLiteral::new(Size::Percent(100.0)).into(),PropertyLiteral::new(Size::Percent(100.0)).into()]),
                                                            children: None,
                                                            component_template: None,
                                                            scroller_args: None,
                                                            slot_index: None,
                                                            repeat_data_list: None,
                                                            conditional_boolean_expression: None,
                                                            compute_properties_fn: None
                                                        }),
                                                    ]))),
                                                    component_template: None,
                                                    scroller_args: None,
                                                    slot_index: None,
                                                    repeat_data_list: None,
                                                    conditional_boolean_expression: None,
                                                    compute_properties_fn: None
                                                })
                                            ]))),
                                            component_template: None,
                                            scroller_args: None,
                                            slot_index: None,
                                            repeat_data_list: Some(Box::new(PropertyLiteral::new((0..8).into_iter().map(|i|{
                                                Rc::new(PropertiesCoproduct::isize(i))
                                            }).collect()))),
                                            conditional_boolean_expression: None,
                                            compute_properties_fn: None
                                        }),

                                    ]))),
                                    component_template: None,
                                    scroller_args: None,
                                    slot_index: None,
                                    repeat_data_list: None,
                                    conditional_boolean_expression: None,
                                    compute_properties_fn: None,
                                }
                            ),
                            RepeatInstance::instantiate(InstantiationArgs {
                                properties: PropertiesCoproduct::None,
                                handler_registry: None,
                                instance_registry: Rc::clone(&instance_registry),
                                transform: Transform2D::default_wrapped(),
                                size: None,
                                children: Some(Rc::new(RefCell::new( vec![
                                    GroupInstance::instantiate(InstantiationArgs {
                                        properties: PropertiesCoproduct::Group(Group{}),
                                        handler_registry: None,
                                        instance_registry: Rc::clone(&instance_registry),
                                        transform: Transform2D::default_wrapped(),
                                        size: Some([PropertyLiteral::new(Size::Percent(100.0)).into(),PropertyLiteral::new(Size::Percent(100.0)).into()]),
                                        children: Some(Rc::new(RefCell::new(vec![
                                            TextInstance::instantiate(InstantiationArgs {
                                                properties: PropertiesCoproduct::Text( Text {
                                                    content: Box::new(PropertyExpression::new(6) ),
                                                    fill: Box::new(PropertyLiteral::new(Color::rgba(0.0,0.0,0.0,1.0))),
                                                    font: Font {
                                                        family: Box::new(PropertyLiteral::new("Real Head Pro".to_string())),
                                                        variant: Box::new(PropertyLiteral::new("Light".to_string())),
                                                        size: Box::new(PropertyExpression::new(10)),
                                                    },
                                                }),
                                                handler_registry: None,
                                                instance_registry: Rc::clone(&instance_registry),
                                                transform: Rc::new(RefCell::new(PropertyExpression::new(8))),
                                                size: Some([PropertyLiteral::new(Size::Percent(100.0)).into(),PropertyLiteral::new(Size::Pixels(55.0)).into()]),
                                                children: None,
                                                component_template: None,
                                                scroller_args: None,
                                                slot_index: None,
                                                repeat_data_list: None,
                                                conditional_boolean_expression: None,
                                                compute_properties_fn: None
                                            }),
                                            RectangleInstance::instantiate(InstantiationArgs{
                                                properties: PropertiesCoproduct::Rectangle(Rectangle {
                                                    stroke: pax_example::pax_types::pax_std::types::Stroke{
                                                        color: Box::new(PropertyLiteral::new(Color::rgba(0.0,0.0,0.0,0.0))),
                                                        width: Box::new(PropertyLiteral::new(SizePixels(0.0))),
                                                    },
                                                    fill: Box::new(PropertyExpression::new(9)),
                                                }),
                                                handler_registry: None,
                                                instance_registry: Rc::clone(&instance_registry),
                                                transform: Transform2D::default_wrapped(),
                                                size: Some([PropertyLiteral::new(Size::Percent(100.0)).into(),PropertyLiteral::new(Size::Percent(100.0)).into()]),
                                                children: None,
                                                component_template: None,
                                                scroller_args: None,
                                                slot_index: None,
                                                repeat_data_list: None,
                                                conditional_boolean_expression: None,
                                                compute_properties_fn: None
                                            }),
                                        ]))),
                                        component_template: None,
                                        scroller_args: None,
                                        slot_index: None,
                                        repeat_data_list: None,
                                        conditional_boolean_expression: None,
                                        compute_properties_fn: None
                                    })
                                ]))),
                                component_template: None,
                                scroller_args: None,
                                slot_index: None,
                                repeat_data_list: Some(Box::new(PropertyLiteral::new((0..8).into_iter().map(|i|{
                                    Rc::new(PropertiesCoproduct::isize(i))
                                }).collect()))),
                                conditional_boolean_expression: None,
                                compute_properties_fn: None
                            }),
                            GroupInstance::instantiate(InstantiationArgs {
                                properties: PropertiesCoproduct::Group(Group {}),
                                handler_registry: Some(Rc::new(RefCell::new(
                                    HandlerRegistry {
                                        click_handlers: vec![
                                            |stack_frame, args|{
                                                const STACK_FRAME_OFFSET : isize = 2;
                                                let SCOPED_STACK_FRAME = (*stack_frame).borrow().nth_descendant(STACK_FRAME_OFFSET); //just gen `ec.stack_frame` if offset == 0

                                                let properties = SCOPED_STACK_FRAME.deref().borrow().get_properties();
                                                let properties = &mut *(*properties).borrow_mut();
                                                let properties = if let PropertiesCoproduct::HelloWorld(p) = properties {p} else {unreachable!("{}",123)};
                                                HelloWorld::handle_click(properties, args);
                                            }
                                        ],
                                        will_render_handlers: vec![],
                                    }
                                ))),
                                instance_registry: Rc::clone(&instance_registry),
                                transform: Rc::new(RefCell::new(PropertyExpression::new(5))),
                                size: Some([PropertyLiteral::new(Size::Percent(100.0)).into(),PropertyLiteral::new(Size::Percent(100.0)).into()]),
                                children: Some(Rc::new(RefCell::new(vec![
                                    TextInstance::instantiate(InstantiationArgs {
                                        properties: PropertiesCoproduct::Text( Text {
                                            content: Box::new(PropertyLiteral::new(JABBERWOCKY.to_string()) ),
                                            fill: Box::new(PropertyLiteral::new(Color::rgba(0.0,0.0,0.0,1.0))),
                                            font: Default::default(),
                                        }),
                                        handler_registry: None,
                                        instance_registry: Rc::clone(&instance_registry),
                                        transform: Transform2D::default_wrapped(),
                                        size: Some([PropertyLiteral::new(Size::Percent(100.0)).into(),PropertyLiteral::new(Size::Percent(100.0)).into()]),
                                        children: None,
                                        component_template: None,
                                        scroller_args: None,
                                        slot_index: None,
                                        repeat_data_list: None,
                                        conditional_boolean_expression: None,
                                        compute_properties_fn: None
                                    }),
                                    RectangleInstance::instantiate(InstantiationArgs{
                                        properties: PropertiesCoproduct::Rectangle(Rectangle {
                                            stroke: pax_example::pax_types::pax_std::types::Stroke{
                                                color: Box::new(PropertyLiteral::new(Color::rgba(0.0,0.0,0.0,0.0))),
                                                width: Box::new(PropertyLiteral::new(SizePixels(0.0))),
                                            },
                                            fill: Box::new(PropertyLiteral::new(Color::rgba(1.0, 1.0, 0.0, 1.0)))
                                        }),
                                        handler_registry: None,
                                        instance_registry: Rc::clone(&instance_registry),
                                        transform: Transform2D::default_wrapped(),
                                        size: Some([PropertyLiteral::new(Size::Percent(100.0)).into(),PropertyLiteral::new(Size::Percent(100.0)).into()]),
                                        children: None,
                                        component_template: None,
                                        scroller_args: None,
                                        slot_index: None,
                                        repeat_data_list: None,
                                        conditional_boolean_expression: None,
                                        compute_properties_fn: None
                                    }),
                                ]))),
                                component_template: None,
                                scroller_args: None,
                                slot_index: None,
                                repeat_data_list: None,
                                conditional_boolean_expression: None,
                                compute_properties_fn: None,
                            }),
                        ]))),
                        component_template: None,
                        scroller_args: None,
                        slot_index: None,
                        repeat_data_list: None,
                        conditional_boolean_expression: None,
                        compute_properties_fn: None,
                    }
                ),
            ]))),
            scroller_args: None,
            slot_index: None,
            repeat_data_list: None,
            conditional_boolean_expression: None,
            compute_properties_fn: Some(Box::new(|properties, rtc|{
                let properties = &mut *properties.as_ref().borrow_mut();
                let properties = if let PropertiesCoproduct::HelloWorld(p) = properties {p} else {unreachable!("{}",123)};

                if let Some(new_value) = rtc.compute_eased_value(properties.current_rotation._get_transition_manager()) {
                    properties.current_rotation.set(new_value);
                }else if let Some(new_current_rotation) = rtc.compute_vtable_value(properties.current_rotation._get_vtable_id()) {
                    let new_value = if let TypesCoproduct::f64(v) = new_current_rotation { v } else { unreachable!("{}",123) };
                    properties.current_rotation.set(new_value);
                }

                if let Some(new_num_clicks) = rtc.compute_vtable_value(properties.num_clicks._get_vtable_id()) {
                    let new_value = if let TypesCoproduct::i64(v) = new_num_clicks { v } else { unreachable!("{}",123) };
                    properties.num_clicks.set(new_value);
                }
            }))
        }
    )
}