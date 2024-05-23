#![allow(unused, unused_imports, non_snake_case, unused_parens)]
use pax_manifest::*;
use pax_runtime::api::*;
use pax_runtime::*;
use pax_manifest::deserializer::{from_pax_try_coerce};
use std::cell::Ref;
use pax_runtime::api::properties::UntypedProperty;
use pax_manifest::ControlFlowRepeatPredicateDefinition::ElemIdIndexId;
use pax_manifest::ControlFlowRepeatPredicateDefinition::ElemId;
use pax_runtime_api::pax_value::PaxValue;
use pax_runtime_api::pax_value::PaxAny;
use pax_runtime_api::pax_value::ToFromPaxAny;
use pax_runtime_api::{borrow, borrow_mut};
use pax_runtime::api::pax_value::ToFromPaxValue;

const INITAL_MANIFEST: &str = include_str!("../initial-manifest.json");

// generate imports, pointing to userland cartridge `pub mod pax_reexports`

use ui_components::pax_reexports::pax_engine::api::Numeric;

use usize;

use ui_components::pax_reexports::pax_component_library::resizable::Section;

use ui_components::pax_reexports::pax_engine::api::Rotation;

use ui_components::pax_reexports::pax_std::types::text::TextStyle;

use ui_components::pax_reexports::std::option::Option;

use ui_components::pax_reexports::pax_engine::api::ColorChannel;

use ui_components::pax_reexports::pax_std::primitives::BlankComponent;

use ui_components::pax_reexports::pax_engine::api::Stroke;

use ui_components::pax_reexports::pax_std::types::text::TextAlignVertical;

use ui_components::pax_reexports::pax_engine::api::Color;

use ui_components::pax_reexports::std::string::String;

use ui_components::pax_reexports::pax_std::types::text::WebFont;

use ui_components::pax_reexports::pax_std::types::text::FontStyle;

use ui_components::pax_reexports::pax_std::primitives::Group;

use bool;

use ui_components::pax_reexports::pax_component_library::tabs::Tabs;

use ui_components::pax_reexports::std::vec::Vec;

use ui_components::pax_reexports::pax_component_library::resizable::ResizableDirection;

use ui_components::pax_reexports::pax_component_library::dropdown::PaxDropdown;

use ui_components::pax_reexports::pax_std::primitives::Dropdown;

use ui_components::pax_reexports::pax_std::types::text::FontWeight;

use ui_components::pax_reexports::pax_engine::api::Fill;

use ui_components::pax_reexports::pax_std::primitives::Text;

use ui_components::pax_reexports::pax_std::types::text::SystemFont;

use ui_components::pax_reexports::pax_component_library::resizable::Resizable;

use u32;

use ui_components::pax_reexports::pax_std::types::RectangleCornerRadii;

use ui_components::pax_reexports::pax_std::types::text::Font;

use ui_components::pax_reexports::pax_engine::api::Transform2D;

use ui_components::pax_reexports::pax_std::primitives::Rectangle;

use ui_components::pax_reexports::pax_engine::api::Size;

use ui_components::pax_reexports::pax_std::types::text::LocalFont;

use ui_components::pax_reexports::Example;

use ui_components::pax_reexports::pax_std::types::text::TextAlignHorizontal;

use std::any::Any;

use pax_runtime::api::{use_RefCell};

use std::collections::HashMap;

use std::collections::VecDeque;

use std::ops::Deref;

use std::rc::Rc;

use pax_runtime::RepeatItem;

use pax_runtime::RepeatProperties;

use pax_runtime::ConditionalProperties;

use pax_runtime::SlotProperties;

use pax_runtime::api::Property;

use pax_runtime::api::CommonProperties;

use pax_runtime::api::Color::*;

use pax_runtime::ComponentInstance;

use pax_runtime::InstanceNodePtr;

use pax_runtime::InstanceNodePtrList;

use pax_runtime::ExpressionContext;

use pax_runtime::PaxEngine;

use pax_runtime::InstanceNode;

use pax_runtime::HandlerRegistry;

use pax_runtime::InstantiationArgs;

use pax_runtime::ConditionalInstance;

use pax_runtime::SlotInstance;

use pax_runtime::properties::RuntimePropertiesStackFrame;

use pax_runtime::repeat::RepeatInstance;

use piet_common::RenderContext;


use_RefCell!();

pub fn instantiate_expression_table() -> HashMap<usize, Box<dyn Fn(ExpressionContext) -> PaxAny>> {
    let mut vtable: HashMap<usize, Box<dyn Fn(ExpressionContext) -> PaxAny>> = HashMap::new();

    
    // sections
    
        
    
    vtable.insert(0, Box::new(|ec: ExpressionContext| -> PaxAny {
        
            let sections =
            {
                let properties = if let Some(sf) = ec.stack_frame.resolve_symbol("sections") {
                    Rc::clone(&sf)
                } else {
                    panic!("sections didn't have an 0th stackframe");
                };
                let mut borrowed = &mut *borrow_mut!(*properties);
                

                    if let Ok(p) = <ui_components::pax_reexports::pax_component_library::resizable::Resizable>::ref_from_pax_any(&*borrowed) {

                        
                            //binding cloneable property
                            p.sections.get().clone()
                        
                    } else {unreachable!()}
                
            };
            

        

        
            
            
                let sections = sections.iter().map(|t|{
                    let converted_cell: Rc<RefCell<PaxAny>> = Rc::new(RefCell::new(t.clone().to_pax_any()));
                    converted_cell
                }).collect::<Vec<Rc<RefCell<PaxAny>>>>();
            
        

        let ___ret = (sections).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    
    // s.x
    
        
    
    vtable.insert(1, Box::new(|ec: ExpressionContext| -> PaxAny {
        
            let sPERIx =
            {
                let properties = if let Some(sf) = ec.stack_frame.resolve_symbol("s") {
                    Rc::clone(&sf)
                } else {
                    panic!("sPERIx didn't have an 0th stackframe");
                };
                let mut borrowed = &mut *borrow_mut!(*properties);
                
                    // binding repeat elem
                    if let Ok(unwrapped_repeat_item) = RepeatItem::ref_from_pax_any(&*borrowed) {
                        let i = unwrapped_repeat_item.i.get();
                        let elem = Rc::clone(&unwrapped_repeat_item.elem.get().unwrap());

                        
                            //iterable complex type
                            let mut elem_borrowed = &mut *borrow_mut!(elem);
                            if let Ok(dc) = <ui_components::pax_reexports::pax_component_library::resizable::Section>::mut_from_pax_any(elem_borrowed) {
                                dc.clone()
                            } else {unreachable!()}
                        
                    } else {panic!()} // Failed to downcast


                
            }.x.clone();
            

        

        

        let ___ret = (sPERIx).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    
    // s.y
    
        
    
    vtable.insert(2, Box::new(|ec: ExpressionContext| -> PaxAny {
        
            let sPERIy =
            {
                let properties = if let Some(sf) = ec.stack_frame.resolve_symbol("s") {
                    Rc::clone(&sf)
                } else {
                    panic!("sPERIy didn't have an 0th stackframe");
                };
                let mut borrowed = &mut *borrow_mut!(*properties);
                
                    // binding repeat elem
                    if let Ok(unwrapped_repeat_item) = RepeatItem::ref_from_pax_any(&*borrowed) {
                        let i = unwrapped_repeat_item.i.get();
                        let elem = Rc::clone(&unwrapped_repeat_item.elem.get().unwrap());

                        
                            //iterable complex type
                            let mut elem_borrowed = &mut *borrow_mut!(elem);
                            if let Ok(dc) = <ui_components::pax_reexports::pax_component_library::resizable::Section>::mut_from_pax_any(elem_borrowed) {
                                dc.clone()
                            } else {unreachable!()}
                        
                    } else {panic!()} // Failed to downcast


                
            }.y.clone();
            

        

        

        let ___ret = (sPERIy).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    
    // s.width
    
        
    
    vtable.insert(3, Box::new(|ec: ExpressionContext| -> PaxAny {
        
            let sPERIwidth =
            {
                let properties = if let Some(sf) = ec.stack_frame.resolve_symbol("s") {
                    Rc::clone(&sf)
                } else {
                    panic!("sPERIwidth didn't have an 0th stackframe");
                };
                let mut borrowed = &mut *borrow_mut!(*properties);
                
                    // binding repeat elem
                    if let Ok(unwrapped_repeat_item) = RepeatItem::ref_from_pax_any(&*borrowed) {
                        let i = unwrapped_repeat_item.i.get();
                        let elem = Rc::clone(&unwrapped_repeat_item.elem.get().unwrap());

                        
                            //iterable complex type
                            let mut elem_borrowed = &mut *borrow_mut!(elem);
                            if let Ok(dc) = <ui_components::pax_reexports::pax_component_library::resizable::Section>::mut_from_pax_any(elem_borrowed) {
                                dc.clone()
                            } else {unreachable!()}
                        
                    } else {panic!()} // Failed to downcast


                
            }.width.clone();
            

        

        

        let ___ret = (sPERIwidth).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    
    // s.height
    
        
    
    vtable.insert(4, Box::new(|ec: ExpressionContext| -> PaxAny {
        
            let sPERIheight =
            {
                let properties = if let Some(sf) = ec.stack_frame.resolve_symbol("s") {
                    Rc::clone(&sf)
                } else {
                    panic!("sPERIheight didn't have an 0th stackframe");
                };
                let mut borrowed = &mut *borrow_mut!(*properties);
                
                    // binding repeat elem
                    if let Ok(unwrapped_repeat_item) = RepeatItem::ref_from_pax_any(&*borrowed) {
                        let i = unwrapped_repeat_item.i.get();
                        let elem = Rc::clone(&unwrapped_repeat_item.elem.get().unwrap());

                        
                            //iterable complex type
                            let mut elem_borrowed = &mut *borrow_mut!(elem);
                            if let Ok(dc) = <ui_components::pax_reexports::pax_component_library::resizable::Section>::mut_from_pax_any(elem_borrowed) {
                                dc.clone()
                            } else {unreachable!()}
                        
                    } else {panic!()} // Failed to downcast


                
            }.height.clone();
            

        

        

        let ___ret = (sPERIheight).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    
    // (s.i)
    
        
    
    vtable.insert(5, Box::new(|ec: ExpressionContext| -> PaxAny {
        
            let sPERIi =
            {
                let properties = if let Some(sf) = ec.stack_frame.resolve_symbol("s") {
                    Rc::clone(&sf)
                } else {
                    panic!("sPERIi didn't have an 0th stackframe");
                };
                let mut borrowed = &mut *borrow_mut!(*properties);
                
                    // binding repeat elem
                    if let Ok(unwrapped_repeat_item) = RepeatItem::ref_from_pax_any(&*borrowed) {
                        let i = unwrapped_repeat_item.i.get();
                        let elem = Rc::clone(&unwrapped_repeat_item.elem.get().unwrap());

                        
                            //iterable complex type
                            let mut elem_borrowed = &mut *borrow_mut!(elem);
                            if let Ok(dc) = <ui_components::pax_reexports::pax_component_library::resizable::Section>::mut_from_pax_any(elem_borrowed) {
                                dc.clone()
                            } else {unreachable!()}
                        
                    } else {panic!()} // Failed to downcast


                
            }.i.clone();
            
                let sPERIi = Numeric::from( sPERIi );
            

        

        

        let ___ret = (sPERIi).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    
    // options
    
        
    
    vtable.insert(6, Box::new(|ec: ExpressionContext| -> PaxAny {
        
            let options =
            {
                let properties = if let Some(sf) = ec.stack_frame.resolve_symbol("options") {
                    Rc::clone(&sf)
                } else {
                    panic!("options didn't have an 0th stackframe");
                };
                let mut borrowed = &mut *borrow_mut!(*properties);
                

                    if let Ok(p) = <ui_components::pax_reexports::pax_component_library::dropdown::PaxDropdown>::ref_from_pax_any(&*borrowed) {

                        
                            //binding cloneable property
                            p.options.get().clone()
                        
                    } else {unreachable!()}
                
            };
            

        

        

        let ___ret = (options).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    
    // selected_id
    
        
    
    vtable.insert(7, Box::new(|ec: ExpressionContext| -> PaxAny {
        
            let selected_id =
            {
                let properties = if let Some(sf) = ec.stack_frame.resolve_symbol("selected_id") {
                    Rc::clone(&sf)
                } else {
                    panic!("selected_id didn't have an 0th stackframe");
                };
                let mut borrowed = &mut *borrow_mut!(*properties);
                

                    if let Ok(p) = <ui_components::pax_reexports::pax_component_library::dropdown::PaxDropdown>::ref_from_pax_any(&*borrowed) {

                        
                            //binding simple numeric property
                            Numeric::from(p.selected_id.get())
                        
                    } else {unreachable!()}
                
            };
            
                let selected_id = Numeric::from( selected_id );
            

        

        

        let ___ret = (selected_id).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    
    // text_style
    
        
    
    vtable.insert(8, Box::new(|ec: ExpressionContext| -> PaxAny {
        
            let text_style =
            {
                let properties = if let Some(sf) = ec.stack_frame.resolve_symbol("text_style") {
                    Rc::clone(&sf)
                } else {
                    panic!("text_style didn't have an 0th stackframe");
                };
                let mut borrowed = &mut *borrow_mut!(*properties);
                

                    if let Ok(p) = <ui_components::pax_reexports::pax_component_library::dropdown::PaxDropdown>::ref_from_pax_any(&*borrowed) {

                        
                            //binding cloneable property
                            p.text_style.get().clone()
                        
                    } else {unreachable!()}
                
            };
            

        

        

        let ___ret = (text_style).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    
    // background
    
        
    
    vtable.insert(9, Box::new(|ec: ExpressionContext| -> PaxAny {
        
            let background =
            {
                let properties = if let Some(sf) = ec.stack_frame.resolve_symbol("background") {
                    Rc::clone(&sf)
                } else {
                    panic!("background didn't have an 0th stackframe");
                };
                let mut borrowed = &mut *borrow_mut!(*properties);
                

                    if let Ok(p) = <ui_components::pax_reexports::pax_component_library::dropdown::PaxDropdown>::ref_from_pax_any(&*borrowed) {

                        
                            //binding cloneable property
                            p.background.get().clone()
                        
                    } else {unreachable!()}
                
            };
            

        

        

        let ___ret = (background).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    
    // stroke
    
        
    
    vtable.insert(10, Box::new(|ec: ExpressionContext| -> PaxAny {
        
            let stroke =
            {
                let properties = if let Some(sf) = ec.stack_frame.resolve_symbol("stroke") {
                    Rc::clone(&sf)
                } else {
                    panic!("stroke didn't have an 0th stackframe");
                };
                let mut borrowed = &mut *borrow_mut!(*properties);
                

                    if let Ok(p) = <ui_components::pax_reexports::pax_component_library::dropdown::PaxDropdown>::ref_from_pax_any(&*borrowed) {

                        
                            //binding cloneable property
                            p.stroke.get().clone()
                        
                    } else {unreachable!()}
                
            };
            

        

        

        let ___ret = (stroke).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    
    // [350px,(100%-350px)]
    
        
    
    vtable.insert(11, Box::new(|ec: ExpressionContext| -> PaxAny {
        

        

        let ___ret = vec![Size::Pixels(350.into()).to_pax_any(),(Percent(100.into()).to_pax_any()-Size::Pixels(350.into()).to_pax_any())];

        ___ret.to_pax_any()
    }));
    
        
    
    
    // ["tab1","tab2","tab3"]
    
        
    
    vtable.insert(12, Box::new(|ec: ExpressionContext| -> PaxAny {
        

        

        let ___ret = vec![("tab 1").to_string().to_pax_any(),("tab 2").to_string().to_pax_any(),("tab 3").to_string().to_pax_any()];

        ___ret.to_pax_any()
    }));
    
        
    
    
    // [30%,60%]
    
        
    
    vtable.insert(13, Box::new(|ec: ExpressionContext| -> PaxAny {
        

        

        let ___ret = vec![Percent(30.into()).to_pax_any(),Percent(60.into()).to_pax_any()];

        ___ret.to_pax_any()
    }));
    
        
    
    
    // [30%,60%]
    
        
    
    vtable.insert(14, Box::new(|ec: ExpressionContext| -> PaxAny {
        

        

        let ___ret = vec![Percent(30.into()).to_pax_any(),Percent(60.into()).to_pax_any()];

        ___ret.to_pax_any()
    }));
    
        
    
    
    // [40%,50%]
    
        
    
    vtable.insert(15, Box::new(|ec: ExpressionContext| -> PaxAny {
        

        

        let ___ret = vec![Percent(40.into()).to_pax_any(),Percent(50.into()).to_pax_any()];

        ___ret.to_pax_any()
    }));
    
        
    
    
    // ["hello","goodbye","red","greeen"]
    
        
    
    vtable.insert(16, Box::new(|ec: ExpressionContext| -> PaxAny {
        

        

        let ___ret = vec![("hello").to_string().to_pax_any(),("goodbye").to_string().to_pax_any(),("red").to_string().to_pax_any(),("greeen").to_string().to_pax_any()];

        ___ret.to_pax_any()
    }));
    
        
    
    
    // self.selected
    
        
    
    vtable.insert(17, Box::new(|ec: ExpressionContext| -> PaxAny {
        
            let selected =
            {
                let properties = if let Some(sf) = ec.stack_frame.resolve_symbol("selected") {
                    Rc::clone(&sf)
                } else {
                    panic!("selected didn't have an 0th stackframe");
                };
                let mut borrowed = &mut *borrow_mut!(*properties);
                

                    if let Ok(p) = <ui_components::pax_reexports::Example>::ref_from_pax_any(&*borrowed) {

                        
                            //binding simple numeric property
                            Numeric::from(p.selected.get())
                        
                    } else {unreachable!()}
                
            };
            
                let selected = Numeric::from( selected );
            

        

        

        let ___ret = (selected).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    
    // 100%-4px
    
        
    
    vtable.insert(18, Box::new(|ec: ExpressionContext| -> PaxAny {
        

        

        let ___ret = (Percent(100.into()).to_pax_any()-Size::Pixels(4.into()).to_pax_any());

        ___ret.to_pax_any()
    }));
    
        
    
    
    // names_filled
    
        
    
    vtable.insert(19, Box::new(|ec: ExpressionContext| -> PaxAny {
        
            let names_filled =
            {
                let properties = if let Some(sf) = ec.stack_frame.resolve_symbol("names_filled") {
                    Rc::clone(&sf)
                } else {
                    panic!("names_filled didn't have an 0th stackframe");
                };
                let mut borrowed = &mut *borrow_mut!(*properties);
                

                    if let Ok(p) = <ui_components::pax_reexports::pax_component_library::tabs::Tabs>::ref_from_pax_any(&*borrowed) {

                        
                            //binding cloneable property
                            p.names_filled.get().clone()
                        
                    } else {unreachable!()}
                
            };
            

        

        
            
            
                let names_filled = names_filled.iter().map(|t|{
                    let converted_cell: Rc<RefCell<PaxAny>> = Rc::new(RefCell::new(t.clone().to_pax_any()));
                    converted_cell
                }).collect::<Vec<Rc<RefCell<PaxAny>>>>();
            
        

        let ___ret = (names_filled).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    
    // (100.0*i/(self.slot_count-1))%
    
        
    
    vtable.insert(20, Box::new(|ec: ExpressionContext| -> PaxAny {
        
            let i =
            {
                let properties = if let Some(sf) = ec.stack_frame.resolve_symbol("i") {
                    Rc::clone(&sf)
                } else {
                    panic!("i didn't have an 0th stackframe");
                };
                let mut borrowed = &mut *borrow_mut!(*properties);
                
                    //binding repeat i
                    let mut unwrapped_repeat_item = if let Ok(val) = RepeatItem::mut_from_pax_any(&mut *borrowed) {val} else {panic!()}; // Failed to downcast
                    let i = unwrapped_repeat_item.i.get();
                    Numeric::from(i)
                
            };
            
                let i = Numeric::from( i );
            

        
            let slot_count =
            {
                let properties = if let Some(sf) = ec.stack_frame.resolve_symbol("slot_count") {
                    Rc::clone(&sf)
                } else {
                    panic!("slot_count didn't have an 1th stackframe");
                };
                let mut borrowed = &mut *borrow_mut!(*properties);
                

                    if let Ok(p) = <ui_components::pax_reexports::pax_component_library::tabs::Tabs>::ref_from_pax_any(&*borrowed) {

                        
                            //binding simple numeric property
                            Numeric::from(p.slot_count.get())
                        
                    } else {unreachable!()}
                
            };
            
                let slot_count = Numeric::from( slot_count );
            

        

        

        let ___ret = Percent((((100.0).to_pax_any()*(i).to_pax_any())/((slot_count).to_pax_any()-(1).to_pax_any())).try_coerce().unwrap()).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    
    // (100.0/self.slot_count)%
    
        
    
    vtable.insert(21, Box::new(|ec: ExpressionContext| -> PaxAny {
        
            let slot_count =
            {
                let properties = if let Some(sf) = ec.stack_frame.resolve_symbol("slot_count") {
                    Rc::clone(&sf)
                } else {
                    panic!("slot_count didn't have an 1th stackframe");
                };
                let mut borrowed = &mut *borrow_mut!(*properties);
                

                    if let Ok(p) = <ui_components::pax_reexports::pax_component_library::tabs::Tabs>::ref_from_pax_any(&*borrowed) {

                        
                            //binding simple numeric property
                            Numeric::from(p.slot_count.get())
                        
                    } else {unreachable!()}
                
            };
            
                let slot_count = Numeric::from( slot_count );
            

        

        

        let ___ret = Percent(((100.0).to_pax_any()/(slot_count).to_pax_any()).try_coerce().unwrap()).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    
    // 100%-4px
    
        
    
    vtable.insert(22, Box::new(|ec: ExpressionContext| -> PaxAny {
        

        

        let ___ret = (Percent(100.into()).to_pax_any()-Size::Pixels(4.into()).to_pax_any());

        ___ret.to_pax_any()
    }));
    
        
    
    
    // 100%-2px
    
        
    
    vtable.insert(23, Box::new(|ec: ExpressionContext| -> PaxAny {
        

        

        let ___ret = (Percent(100.into()).to_pax_any()-Size::Pixels(2.into()).to_pax_any());

        ___ret.to_pax_any()
    }));
    
        
    
    
    // rgba(255,255,255,30*(i==self.selected))
    
        
    
    vtable.insert(24, Box::new(|ec: ExpressionContext| -> PaxAny {
        
            let i =
            {
                let properties = if let Some(sf) = ec.stack_frame.resolve_symbol("i") {
                    Rc::clone(&sf)
                } else {
                    panic!("i didn't have an 0th stackframe");
                };
                let mut borrowed = &mut *borrow_mut!(*properties);
                
                    //binding repeat i
                    let mut unwrapped_repeat_item = if let Ok(val) = RepeatItem::mut_from_pax_any(&mut *borrowed) {val} else {panic!()}; // Failed to downcast
                    let i = unwrapped_repeat_item.i.get();
                    Numeric::from(i)
                
            };
            
                let i = Numeric::from( i );
            

        
            let selected =
            {
                let properties = if let Some(sf) = ec.stack_frame.resolve_symbol("selected") {
                    Rc::clone(&sf)
                } else {
                    panic!("selected didn't have an 1th stackframe");
                };
                let mut borrowed = &mut *borrow_mut!(*properties);
                

                    if let Ok(p) = <ui_components::pax_reexports::pax_component_library::tabs::Tabs>::ref_from_pax_any(&*borrowed) {

                        
                            //binding simple numeric property
                            Numeric::from(p.selected.get())
                        
                    } else {unreachable!()}
                
            };
            
                let selected = Numeric::from( selected );
            

        

        

        let ___ret = Color::rgba((255).to_pax_any().try_coerce().unwrap(),(255).to_pax_any().try_coerce().unwrap(),(255).to_pax_any().try_coerce().unwrap(),((30).to_pax_any()*PaxAny::Builtin(PaxValue::Bool((i ).to_pax_any()==(selected).to_pax_any()))).try_coerce().unwrap(),).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    
    // RectangleCornerRadii::radii(10.0,10.0,0.0,0.0)
    
        
    
    vtable.insert(25, Box::new(|ec: ExpressionContext| -> PaxAny {
        

        

        let ___ret = (RectangleCornerRadii::radii(((10.0).to_pax_any()).try_coerce().unwrap(),((10.0).to_pax_any()).try_coerce().unwrap(),((0.0).to_pax_any()).try_coerce().unwrap(),((0.0).to_pax_any()).try_coerce().unwrap(),)).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    
    // name
    
        
    
    vtable.insert(26, Box::new(|ec: ExpressionContext| -> PaxAny {
        
            let name =
            {
                let properties = if let Some(sf) = ec.stack_frame.resolve_symbol("name") {
                    Rc::clone(&sf)
                } else {
                    panic!("name didn't have an 0th stackframe");
                };
                let mut borrowed = &mut *borrow_mut!(*properties);
                
                    // binding repeat elem
                    if let Ok(unwrapped_repeat_item) = RepeatItem::ref_from_pax_any(&*borrowed) {
                        let i = unwrapped_repeat_item.i.get();
                        let elem = Rc::clone(&unwrapped_repeat_item.elem.get().unwrap());

                        
                            //string as `elem`

                            let elem_borrowed = borrow!(elem);
                            if let Ok(unwrapped) = <ui_components::pax_reexports::std::string::String>::ref_from_pax_any(&*elem_borrowed) {
                                unwrapped.clone()
                            } else {
                                panic!();//Failed to unpack string from PaxAny
                            }

                        
                    } else {panic!()} // Failed to downcast


                
            };
            

        

        

        let ___ret = (name ).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    
    // Font::system("Arial",FontStyle::Normal,FontWeight::Bold)
    
        
    
    vtable.insert(27, Box::new(|ec: ExpressionContext| -> PaxAny {
        

        

        let ___ret = (Font::system((("Arial").to_string().to_pax_any()).try_coerce().unwrap(),((FontStyle::Normal).to_pax_any()).try_coerce().unwrap(),((FontWeight::Bold).to_pax_any()).try_coerce().unwrap(),)).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    
    // 100%-4px
    
        
    
    vtable.insert(28, Box::new(|ec: ExpressionContext| -> PaxAny {
        

        

        let ___ret = (Percent(100.into()).to_pax_any()-Size::Pixels(4.into()).to_pax_any());

        ___ret.to_pax_any()
    }));
    
        
    
    
    // 100%-2px
    
        
    
    vtable.insert(29, Box::new(|ec: ExpressionContext| -> PaxAny {
        

        

        let ___ret = (Percent(100.into()).to_pax_any()-Size::Pixels(2.into()).to_pax_any());

        ___ret.to_pax_any()
    }));
    
        
    
    
    // self.color
    
        
    
    vtable.insert(30, Box::new(|ec: ExpressionContext| -> PaxAny {
        
            let color =
            {
                let properties = if let Some(sf) = ec.stack_frame.resolve_symbol("color") {
                    Rc::clone(&sf)
                } else {
                    panic!("color didn't have an 1th stackframe");
                };
                let mut borrowed = &mut *borrow_mut!(*properties);
                

                    if let Ok(p) = <ui_components::pax_reexports::pax_component_library::tabs::Tabs>::ref_from_pax_any(&*borrowed) {

                        
                            //binding cloneable property
                            p.color.get().clone()
                        
                    } else {unreachable!()}
                
            };
            

        

        

        let ___ret = (color).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    
    // RectangleCornerRadii::radii(10.0,10.0,0.0,0.0)
    
        
    
    vtable.insert(31, Box::new(|ec: ExpressionContext| -> PaxAny {
        

        

        let ___ret = (RectangleCornerRadii::radii(((10.0).to_pax_any()).try_coerce().unwrap(),((10.0).to_pax_any()).try_coerce().unwrap(),((0.0).to_pax_any()).try_coerce().unwrap(),((0.0).to_pax_any()).try_coerce().unwrap(),)).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    
    // 100%-30px
    
        
    
    vtable.insert(32, Box::new(|ec: ExpressionContext| -> PaxAny {
        

        

        let ___ret = (Percent(100.into()).to_pax_any()-Size::Pixels(30.into()).to_pax_any());

        ___ret.to_pax_any()
    }));
    
        
    
    
    // (self.selected)
    
        
    
    vtable.insert(33, Box::new(|ec: ExpressionContext| -> PaxAny {
        
            let selected =
            {
                let properties = if let Some(sf) = ec.stack_frame.resolve_symbol("selected") {
                    Rc::clone(&sf)
                } else {
                    panic!("selected didn't have an 0th stackframe");
                };
                let mut borrowed = &mut *borrow_mut!(*properties);
                

                    if let Ok(p) = <ui_components::pax_reexports::pax_component_library::tabs::Tabs>::ref_from_pax_any(&*borrowed) {

                        
                            //binding simple numeric property
                            Numeric::from(p.selected.get())
                        
                    } else {unreachable!()}
                
            };
            
                let selected = Numeric::from( selected );
            

        

        

        let ___ret = (selected).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    
    // 100%-30px
    
        
    
    vtable.insert(34, Box::new(|ec: ExpressionContext| -> PaxAny {
        

        

        let ___ret = (Percent(100.into()).to_pax_any()-Size::Pixels(30.into()).to_pax_any());

        ___ret.to_pax_any()
    }));
    
        
    
    
    // self.color
    
        
    
    vtable.insert(35, Box::new(|ec: ExpressionContext| -> PaxAny {
        
            let color =
            {
                let properties = if let Some(sf) = ec.stack_frame.resolve_symbol("color") {
                    Rc::clone(&sf)
                } else {
                    panic!("color didn't have an 0th stackframe");
                };
                let mut borrowed = &mut *borrow_mut!(*properties);
                

                    if let Ok(p) = <ui_components::pax_reexports::pax_component_library::tabs::Tabs>::ref_from_pax_any(&*borrowed) {

                        
                            //binding cloneable property
                            p.color.get().clone()
                        
                    } else {unreachable!()}
                
            };
            

        

        

        let ___ret = (color).to_pax_any();

        ___ret.to_pax_any()
    }));
    
        
    
    

    vtable
}

pub trait ComponentFactory {

    /// Returns the default CommonProperties factory
    fn build_default_common_properties(&self) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<CommonProperties>>>{
        Box::new(|_,_| Rc::new(RefCell::new(CommonProperties::default())))    
    }

    /// Returns the default properties factory for this component
    fn build_default_properties(&self) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>>;
    
    /// Returns the CommonProperties factory based on the defined properties 
    fn build_inline_common_properties(&self, defined_properties: HashMap<String,ValueDefinition>) ->Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<CommonProperties>>> {
        Box::new(move |stack_frame , table | Rc::new(RefCell::new({
            let mut cp = CommonProperties::default();
            for (key, value) in &defined_properties {
                match key.as_str() {
                    
                    "id" => {
                        let resolved_property: Property<Option<String>> = match value.clone() {
                            ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<String>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                            {
                                if let Some(info) = info {
                                    let mut dependents = vec![];
                                    for dependency in &info.dependencies {
                                        if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                            dependents.push(p);
                                        } else {
                                            panic!("Failed to resolve symbol {}", dependency);
                                        }
                                    }
                                    let cloned_stack = stack_frame.clone();
                                    let cloned_table = table.clone();
                                    Property::computed_with_name(move || {
                                        let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                        let coerced = new_value_wrapped.try_coerce::<String>().unwrap();
                                        Some(coerced)
                                    }, &dependents, &token.raw_value)
                                } else {
                                    unreachable!("No info for expression")
                                }
                            },
                            _ => unreachable!("Invalid value definition for id")
                        };
                        cp.id = resolved_property;
                    },
                    
                    "x" => {
                        let resolved_property: Property<Option<pax_engine::api::Size>> = match value.clone() {
                            ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<pax_engine::api::Size>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                            {
                                if let Some(info) = info {
                                    let mut dependents = vec![];
                                    for dependency in &info.dependencies {
                                        if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                            dependents.push(p);
                                        } else {
                                            panic!("Failed to resolve symbol {}", dependency);
                                        }
                                    }
                                    let cloned_stack = stack_frame.clone();
                                    let cloned_table = table.clone();
                                    Property::computed_with_name(move || {
                                        let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                        let coerced = new_value_wrapped.try_coerce::<pax_engine::api::Size>().unwrap();
                                        Some(coerced)
                                    }, &dependents, &token.raw_value)
                                } else {
                                    unreachable!("No info for expression")
                                }
                            },
                            _ => unreachable!("Invalid value definition for x")
                        };
                        cp.x = resolved_property;
                    },
                    
                    "y" => {
                        let resolved_property: Property<Option<pax_engine::api::Size>> = match value.clone() {
                            ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<pax_engine::api::Size>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                            {
                                if let Some(info) = info {
                                    let mut dependents = vec![];
                                    for dependency in &info.dependencies {
                                        if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                            dependents.push(p);
                                        } else {
                                            panic!("Failed to resolve symbol {}", dependency);
                                        }
                                    }
                                    let cloned_stack = stack_frame.clone();
                                    let cloned_table = table.clone();
                                    Property::computed_with_name(move || {
                                        let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                        let coerced = new_value_wrapped.try_coerce::<pax_engine::api::Size>().unwrap();
                                        Some(coerced)
                                    }, &dependents, &token.raw_value)
                                } else {
                                    unreachable!("No info for expression")
                                }
                            },
                            _ => unreachable!("Invalid value definition for y")
                        };
                        cp.y = resolved_property;
                    },
                    
                    "scale_x" => {
                        let resolved_property: Property<Option<pax_engine::api::Size>> = match value.clone() {
                            ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<pax_engine::api::Size>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                            {
                                if let Some(info) = info {
                                    let mut dependents = vec![];
                                    for dependency in &info.dependencies {
                                        if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                            dependents.push(p);
                                        } else {
                                            panic!("Failed to resolve symbol {}", dependency);
                                        }
                                    }
                                    let cloned_stack = stack_frame.clone();
                                    let cloned_table = table.clone();
                                    Property::computed_with_name(move || {
                                        let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                        let coerced = new_value_wrapped.try_coerce::<pax_engine::api::Size>().unwrap();
                                        Some(coerced)
                                    }, &dependents, &token.raw_value)
                                } else {
                                    unreachable!("No info for expression")
                                }
                            },
                            _ => unreachable!("Invalid value definition for scale_x")
                        };
                        cp.scale_x = resolved_property;
                    },
                    
                    "scale_y" => {
                        let resolved_property: Property<Option<pax_engine::api::Size>> = match value.clone() {
                            ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<pax_engine::api::Size>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                            {
                                if let Some(info) = info {
                                    let mut dependents = vec![];
                                    for dependency in &info.dependencies {
                                        if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                            dependents.push(p);
                                        } else {
                                            panic!("Failed to resolve symbol {}", dependency);
                                        }
                                    }
                                    let cloned_stack = stack_frame.clone();
                                    let cloned_table = table.clone();
                                    Property::computed_with_name(move || {
                                        let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                        let coerced = new_value_wrapped.try_coerce::<pax_engine::api::Size>().unwrap();
                                        Some(coerced)
                                    }, &dependents, &token.raw_value)
                                } else {
                                    unreachable!("No info for expression")
                                }
                            },
                            _ => unreachable!("Invalid value definition for scale_y")
                        };
                        cp.scale_y = resolved_property;
                    },
                    
                    "skew_x" => {
                        let resolved_property: Property<Option<f64>> = match value.clone() {
                            ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<f64>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                            {
                                if let Some(info) = info {
                                    let mut dependents = vec![];
                                    for dependency in &info.dependencies {
                                        if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                            dependents.push(p);
                                        } else {
                                            panic!("Failed to resolve symbol {}", dependency);
                                        }
                                    }
                                    let cloned_stack = stack_frame.clone();
                                    let cloned_table = table.clone();
                                    Property::computed_with_name(move || {
                                        let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                        let coerced = new_value_wrapped.try_coerce::<f64>().unwrap();
                                        Some(coerced)
                                    }, &dependents, &token.raw_value)
                                } else {
                                    unreachable!("No info for expression")
                                }
                            },
                            _ => unreachable!("Invalid value definition for skew_x")
                        };
                        cp.skew_x = resolved_property;
                    },
                    
                    "skew_y" => {
                        let resolved_property: Property<Option<f64>> = match value.clone() {
                            ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<f64>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                            {
                                if let Some(info) = info {
                                    let mut dependents = vec![];
                                    for dependency in &info.dependencies {
                                        if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                            dependents.push(p);
                                        } else {
                                            panic!("Failed to resolve symbol {}", dependency);
                                        }
                                    }
                                    let cloned_stack = stack_frame.clone();
                                    let cloned_table = table.clone();
                                    Property::computed_with_name(move || {
                                        let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                        let coerced = new_value_wrapped.try_coerce::<f64>().unwrap();
                                        Some(coerced)
                                    }, &dependents, &token.raw_value)
                                } else {
                                    unreachable!("No info for expression")
                                }
                            },
                            _ => unreachable!("Invalid value definition for skew_y")
                        };
                        cp.skew_y = resolved_property;
                    },
                    
                    "anchor_x" => {
                        let resolved_property: Property<Option<pax_engine::api::Size>> = match value.clone() {
                            ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<pax_engine::api::Size>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                            {
                                if let Some(info) = info {
                                    let mut dependents = vec![];
                                    for dependency in &info.dependencies {
                                        if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                            dependents.push(p);
                                        } else {
                                            panic!("Failed to resolve symbol {}", dependency);
                                        }
                                    }
                                    let cloned_stack = stack_frame.clone();
                                    let cloned_table = table.clone();
                                    Property::computed_with_name(move || {
                                        let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                        let coerced = new_value_wrapped.try_coerce::<pax_engine::api::Size>().unwrap();
                                        Some(coerced)
                                    }, &dependents, &token.raw_value)
                                } else {
                                    unreachable!("No info for expression")
                                }
                            },
                            _ => unreachable!("Invalid value definition for anchor_x")
                        };
                        cp.anchor_x = resolved_property;
                    },
                    
                    "anchor_y" => {
                        let resolved_property: Property<Option<pax_engine::api::Size>> = match value.clone() {
                            ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<pax_engine::api::Size>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                            {
                                if let Some(info) = info {
                                    let mut dependents = vec![];
                                    for dependency in &info.dependencies {
                                        if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                            dependents.push(p);
                                        } else {
                                            panic!("Failed to resolve symbol {}", dependency);
                                        }
                                    }
                                    let cloned_stack = stack_frame.clone();
                                    let cloned_table = table.clone();
                                    Property::computed_with_name(move || {
                                        let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                        let coerced = new_value_wrapped.try_coerce::<pax_engine::api::Size>().unwrap();
                                        Some(coerced)
                                    }, &dependents, &token.raw_value)
                                } else {
                                    unreachable!("No info for expression")
                                }
                            },
                            _ => unreachable!("Invalid value definition for anchor_y")
                        };
                        cp.anchor_y = resolved_property;
                    },
                    
                    "rotate" => {
                        let resolved_property: Property<Option<pax_engine::api::Rotation>> = match value.clone() {
                            ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<pax_engine::api::Rotation>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                            {
                                if let Some(info) = info {
                                    let mut dependents = vec![];
                                    for dependency in &info.dependencies {
                                        if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                            dependents.push(p);
                                        } else {
                                            panic!("Failed to resolve symbol {}", dependency);
                                        }
                                    }
                                    let cloned_stack = stack_frame.clone();
                                    let cloned_table = table.clone();
                                    Property::computed_with_name(move || {
                                        let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                        let coerced = new_value_wrapped.try_coerce::<pax_engine::api::Rotation>().unwrap();
                                        Some(coerced)
                                    }, &dependents, &token.raw_value)
                                } else {
                                    unreachable!("No info for expression")
                                }
                            },
                            _ => unreachable!("Invalid value definition for rotate")
                        };
                        cp.rotate = resolved_property;
                    },
                    
                    "transform" => {
                        let resolved_property: Property<Option<pax_engine::api::Transform2D>> = match value.clone() {
                            ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<pax_engine::api::Transform2D>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                            {
                                if let Some(info) = info {
                                    let mut dependents = vec![];
                                    for dependency in &info.dependencies {
                                        if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                            dependents.push(p);
                                        } else {
                                            panic!("Failed to resolve symbol {}", dependency);
                                        }
                                    }
                                    let cloned_stack = stack_frame.clone();
                                    let cloned_table = table.clone();
                                    Property::computed_with_name(move || {
                                        let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                        let coerced = new_value_wrapped.try_coerce::<pax_engine::api::Transform2D>().unwrap();
                                        Some(coerced)
                                    }, &dependents, &token.raw_value)
                                } else {
                                    unreachable!("No info for expression")
                                }
                            },
                            _ => unreachable!("Invalid value definition for transform")
                        };
                        cp.transform = resolved_property;
                    },
                    
                    "width" => {
                        let resolved_property: Property<Option<pax_engine::api::Size>> = match value.clone() {
                            ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<pax_engine::api::Size>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                            {
                                if let Some(info) = info {
                                    let mut dependents = vec![];
                                    for dependency in &info.dependencies {
                                        if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                            dependents.push(p);
                                        } else {
                                            panic!("Failed to resolve symbol {}", dependency);
                                        }
                                    }
                                    let cloned_stack = stack_frame.clone();
                                    let cloned_table = table.clone();
                                    Property::computed_with_name(move || {
                                        let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                        let coerced = new_value_wrapped.try_coerce::<pax_engine::api::Size>().unwrap();
                                        Some(coerced)
                                    }, &dependents, &token.raw_value)
                                } else {
                                    unreachable!("No info for expression")
                                }
                            },
                            _ => unreachable!("Invalid value definition for width")
                        };
                        cp.width = resolved_property;
                    },
                    
                    "height" => {
                        let resolved_property: Property<Option<pax_engine::api::Size>> = match value.clone() {
                            ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<pax_engine::api::Size>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                            {
                                if let Some(info) = info {
                                    let mut dependents = vec![];
                                    for dependency in &info.dependencies {
                                        if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                            dependents.push(p);
                                        } else {
                                            panic!("Failed to resolve symbol {}", dependency);
                                        }
                                    }
                                    let cloned_stack = stack_frame.clone();
                                    let cloned_table = table.clone();
                                    Property::computed_with_name(move || {
                                        let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                        let coerced = new_value_wrapped.try_coerce::<pax_engine::api::Size>().unwrap();
                                        Some(coerced)
                                    }, &dependents, &token.raw_value)
                                } else {
                                    unreachable!("No info for expression")
                                }
                            },
                            _ => unreachable!("Invalid value definition for height")
                        };
                        cp.height = resolved_property;
                    },
                    
                    _ => {}
                }
            }

            cp.clone()
        })))
    }

    /// Returns the properties factory based on the defined properties
    fn build_inline_properties(&self, defined_properties: HashMap<String,ValueDefinition>) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>>;
    
    /// Returns the requested closure for the handler registry based on the defined handlers for this component
    /// The argument type is extrapolated based on how the handler was used in the initial compiled template
    fn build_handler(&self, fn_name: &str) -> fn(Rc<RefCell<PaxAny>>, &NodeContext, Option::<PaxAny>);
    
    /// Returns the handler registry based on the defined handlers for this component
    fn build_component_handlers(&self, handlers: Vec<(String, Vec<String>)>) -> Rc<RefCell<HandlerRegistry>>;

    // Takes a hander registry and adds the given inline handlers to it
    fn add_inline_handlers(&self, handlers: Vec<(String, String)>, registry: Rc<RefCell<HandlerRegistry>>) -> Rc<RefCell<HandlerRegistry>>;
   
    // Calls the instantion function for the component
    fn build_component(&self, args: InstantiationArgs) -> Rc<dyn InstanceNode>;

    // Returns the property scope for the component
    fn get_properties_scope_factory(&self) -> Box<dyn Fn(Rc<RefCell<PaxAny>>) -> HashMap<String, UntypedProperty>> {
        Box::new(|_| HashMap::new())
    }
}


struct ResizableFactory{}

impl ComponentFactory for ResizableFactory {

    fn build_default_properties(&self) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>> {
        Box::new(|_,_| Rc::new(RefCell::new(Resizable::default().to_pax_any())))
    }

    fn build_inline_properties(&self, defined_properties: HashMap<String,ValueDefinition>) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>> {
        Box::new(move |stack_frame , table | Rc::new(RefCell::new(
            {
        let mut properties = Resizable::default();
        
            if let Some(vd) = defined_properties.get("dividers") {
                properties.dividers.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<std::vec::Vec<ui_components::pax_reexports::pax_engine::api::Size>>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<std::vec::Vec<ui_components::pax_reexports::pax_engine::api::Size>>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(stdCOCOvecCOCOVecLABRui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOSizeRABRTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for dividers")
                    });
            }
        
            if let Some(vd) = defined_properties.get("direction") {
                properties.direction.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_component_library::resizable::ResizableDirection>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_component_library::resizable::ResizableDirection>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_component_libraryCOCOresizableCOCOResizableDirectionTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for direction")
                    });
            }
        
            if let Some(vd) = defined_properties.get("sections") {
                properties.sections.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<std::vec::Vec<ui_components::pax_reexports::pax_component_library::resizable::Section>>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<std::vec::Vec<ui_components::pax_reexports::pax_component_library::resizable::Section>>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(stdCOCOvecCOCOVecLABRui_componentsCOCOpax_reexportsCOCOpax_component_libraryCOCOresizableCOCOSectionRABRTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for sections")
                    });
            }
        
            if let Some(vd) = defined_properties.get("index_moving") {
                properties.index_moving.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<std::option::Option<ui_components::pax_reexports::usize>>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<std::option::Option<ui_components::pax_reexports::usize>>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(stdCOCOoptionCOCOOptionLABRui_componentsCOCOpax_reexportsCOCOusizeRABRTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for index_moving")
                    });
            }
        
        properties.to_pax_any()
        })))
    }

    fn build_handler(&self,fn_name: &str) -> fn(Rc<RefCell<PaxAny>>, &NodeContext, Option::<PaxAny>) {
        match fn_name {
            
            "on_mount" => {
                |properties, ctx, args|{
                    let properties = &mut *borrow_mut!(properties.as_ref());
                    if let Ok(mut properties) = <Resizable>::mut_from_pax_any(properties) {
                        // downcast args to handler.type
                        
                            if let None = args {
                                Resizable::on_mount(properties,ctx);
                            } else {
                                panic!("Unexpected args present");
                            }
                        
                        
                    } else {panic!("Failed to downcast properties to Resizable")};
                }
            },
            
            "on_mouse_down" => {
                |properties, ctx, args|{
                    let properties = &mut *borrow_mut!(properties.as_ref());
                    if let Ok(mut properties) = <Resizable>::mut_from_pax_any(properties) {
                        // downcast args to handler.type
                        
                            if let Some(args) = args {
                                if let Ok(args) = <Event<MouseDown>>::ref_from_pax_any(&args) {
                                    Resizable::on_mouse_down(properties,ctx, args.clone());
                                } else {panic!("Failed to downcast args to Event<MouseDown>")};
                            } else {
                                panic!("No Event<MouseDown> present");
                            }
                        
                        
                    } else {panic!("Failed to downcast properties to Resizable")};
                }
            },
            
            "on_mouse_move" => {
                |properties, ctx, args|{
                    let properties = &mut *borrow_mut!(properties.as_ref());
                    if let Ok(mut properties) = <Resizable>::mut_from_pax_any(properties) {
                        // downcast args to handler.type
                        
                            if let Some(args) = args {
                                if let Ok(args) = <Event<MouseMove>>::ref_from_pax_any(&args) {
                                    Resizable::on_mouse_move(properties,ctx, args.clone());
                                } else {panic!("Failed to downcast args to Event<MouseMove>")};
                            } else {
                                panic!("No Event<MouseMove> present");
                            }
                        
                        
                    } else {panic!("Failed to downcast properties to Resizable")};
                }
            },
            
            "on_mouse_up" => {
                |properties, ctx, args|{
                    let properties = &mut *borrow_mut!(properties.as_ref());
                    if let Ok(mut properties) = <Resizable>::mut_from_pax_any(properties) {
                        // downcast args to handler.type
                        
                            if let Some(args) = args {
                                if let Ok(args) = <Event<MouseUp>>::ref_from_pax_any(&args) {
                                    Resizable::on_mouse_up(properties,ctx, args.clone());
                                } else {panic!("Failed to downcast args to Event<MouseUp>")};
                            } else {
                                panic!("No Event<MouseUp> present");
                            }
                        
                        
                    } else {panic!("Failed to downcast properties to Resizable")};
                }
            },
            
            _ => panic!("Unknown handler name {}", fn_name)
        }
    }

    fn build_component_handlers(&self, handlers: Vec<(String, Vec<String>)>) -> Rc<RefCell<HandlerRegistry>> {
        let mut handler_registry = HandlerRegistry::default();
        for (event, functions) in &handlers {
            handler_registry.handlers.insert(event.clone(), functions.iter().map(|fn_name| {
                Handler::new_component_handler(self.build_handler(&fn_name))
            }).collect());
        } 
        Rc::new(RefCell::new(handler_registry))
    }

    fn add_inline_handlers(&self, handlers: Vec<(String, String)>, handler_registry: Rc<RefCell<HandlerRegistry>>) -> Rc<RefCell<HandlerRegistry>> {
        {
            let mut handler_registry_mut = borrow_mut!(handler_registry);
            for (event, fn_name) in &handlers {
                let handler_vec = handler_registry_mut.handlers.entry(event.clone()).or_insert(Vec::new());
                handler_vec.push(Handler::new_inline_handler(self.build_handler(&fn_name)));
            } 
        }   
        handler_registry
    }

    fn build_component(&self, args: InstantiationArgs) -> Rc<dyn InstanceNode> {
        
        ComponentInstance::instantiate(args)
            
    }

    fn get_properties_scope_factory(&self) -> Box<dyn Fn(Rc<RefCell<PaxAny>>) -> HashMap<String, UntypedProperty>>  {
        Box::new(|props| {
            let properties = &mut *borrow_mut!(props.as_ref());
            if let Ok(properties) = <Resizable>::mut_from_pax_any(properties) {
                let mut scope = HashMap::new();
                
                    scope.insert("dividers".to_string(), properties.dividers.untyped());
                
                    scope.insert("direction".to_string(), properties.direction.untyped());
                
                    scope.insert("sections".to_string(), properties.sections.untyped());
                
                    scope.insert("index_moving".to_string(), properties.index_moving.untyped());
                
                scope
            } else {
                panic!("Failed to downcast properties to Resizable");
            }
        })
    }

}
struct BlankComponentFactory{}

impl ComponentFactory for BlankComponentFactory {

    fn build_default_properties(&self) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>> {
        Box::new(|_,_| Rc::new(RefCell::new(BlankComponent::default().to_pax_any())))
    }

    fn build_inline_properties(&self, defined_properties: HashMap<String,ValueDefinition>) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>> {
        Box::new(move |stack_frame , table | Rc::new(RefCell::new(
            {
        let mut properties = BlankComponent::default();
        
        properties.to_pax_any()
        })))
    }

    fn build_handler(&self,fn_name: &str) -> fn(Rc<RefCell<PaxAny>>, &NodeContext, Option::<PaxAny>) {
        match fn_name {
            
            _ => panic!("Unknown handler name {}", fn_name)
        }
    }

    fn build_component_handlers(&self, handlers: Vec<(String, Vec<String>)>) -> Rc<RefCell<HandlerRegistry>> {
        let mut handler_registry = HandlerRegistry::default();
        for (event, functions) in &handlers {
            handler_registry.handlers.insert(event.clone(), functions.iter().map(|fn_name| {
                Handler::new_component_handler(self.build_handler(&fn_name))
            }).collect());
        } 
        Rc::new(RefCell::new(handler_registry))
    }

    fn add_inline_handlers(&self, handlers: Vec<(String, String)>, handler_registry: Rc<RefCell<HandlerRegistry>>) -> Rc<RefCell<HandlerRegistry>> {
        {
            let mut handler_registry_mut = borrow_mut!(handler_registry);
            for (event, fn_name) in &handlers {
                let handler_vec = handler_registry_mut.handlers.entry(event.clone()).or_insert(Vec::new());
                handler_vec.push(Handler::new_inline_handler(self.build_handler(&fn_name)));
            } 
        }   
        handler_registry
    }

    fn build_component(&self, args: InstantiationArgs) -> Rc<dyn InstanceNode> {
        
        ComponentInstance::instantiate(args)
            
    }

    fn get_properties_scope_factory(&self) -> Box<dyn Fn(Rc<RefCell<PaxAny>>) -> HashMap<String, UntypedProperty>>  {
        Box::new(|props| {
            let properties = &mut *borrow_mut!(props.as_ref());
            if let Ok(properties) = <BlankComponent>::mut_from_pax_any(properties) {
                let mut scope = HashMap::new();
                
                scope
            } else {
                panic!("Failed to downcast properties to BlankComponent");
            }
        })
    }

}
struct TextFactory{}

impl ComponentFactory for TextFactory {

    fn build_default_properties(&self) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>> {
        Box::new(|_,_| Rc::new(RefCell::new(Text::default().to_pax_any())))
    }

    fn build_inline_properties(&self, defined_properties: HashMap<String,ValueDefinition>) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>> {
        Box::new(move |stack_frame , table | Rc::new(RefCell::new(
            {
        let mut properties = Text::default();
        
            if let Some(vd) = defined_properties.get("editable") {
                properties.editable.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<bool>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<bool>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(boolTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for editable")
                    });
            }
        
            if let Some(vd) = defined_properties.get("text") {
                properties.text.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<ui_components::pax_reexports::std::string::String>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::std::string::String>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOstdCOCOstringCOCOStringTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for text")
                    });
            }
        
            if let Some(vd) = defined_properties.get("style") {
                properties.style.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_std::types::text::TextStyle>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_std::types::text::TextStyle>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOTextStyleTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for style")
                    });
            }
        
            if let Some(vd) = defined_properties.get("style_link") {
                properties.style_link.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_std::types::text::TextStyle>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_std::types::text::TextStyle>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOTextStyleTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for style_link")
                    });
            }
        
        properties.to_pax_any()
        })))
    }

    fn build_handler(&self,fn_name: &str) -> fn(Rc<RefCell<PaxAny>>, &NodeContext, Option::<PaxAny>) {
        match fn_name {
            
            _ => panic!("Unknown handler name {}", fn_name)
        }
    }

    fn build_component_handlers(&self, handlers: Vec<(String, Vec<String>)>) -> Rc<RefCell<HandlerRegistry>> {
        let mut handler_registry = HandlerRegistry::default();
        for (event, functions) in &handlers {
            handler_registry.handlers.insert(event.clone(), functions.iter().map(|fn_name| {
                Handler::new_component_handler(self.build_handler(&fn_name))
            }).collect());
        } 
        Rc::new(RefCell::new(handler_registry))
    }

    fn add_inline_handlers(&self, handlers: Vec<(String, String)>, handler_registry: Rc<RefCell<HandlerRegistry>>) -> Rc<RefCell<HandlerRegistry>> {
        {
            let mut handler_registry_mut = borrow_mut!(handler_registry);
            for (event, fn_name) in &handlers {
                let handler_vec = handler_registry_mut.handlers.entry(event.clone()).or_insert(Vec::new());
                handler_vec.push(Handler::new_inline_handler(self.build_handler(&fn_name)));
            } 
        }   
        handler_registry
    }

    fn build_component(&self, args: InstantiationArgs) -> Rc<dyn InstanceNode> {
        
        pax_std_primitives::text::TextInstance::instantiate(args)
            
    }

    fn get_properties_scope_factory(&self) -> Box<dyn Fn(Rc<RefCell<PaxAny>>) -> HashMap<String, UntypedProperty>>  {
        Box::new(|props| {
            let properties = &mut *borrow_mut!(props.as_ref());
            if let Ok(properties) = <Text>::mut_from_pax_any(properties) {
                let mut scope = HashMap::new();
                
                    scope.insert("editable".to_string(), properties.editable.untyped());
                
                    scope.insert("text".to_string(), properties.text.untyped());
                
                    scope.insert("style".to_string(), properties.style.untyped());
                
                    scope.insert("style_link".to_string(), properties.style_link.untyped());
                
                scope
            } else {
                panic!("Failed to downcast properties to Text");
            }
        })
    }

}
struct PaxDropdownFactory{}

impl ComponentFactory for PaxDropdownFactory {

    fn build_default_properties(&self) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>> {
        Box::new(|_,_| Rc::new(RefCell::new(PaxDropdown::default().to_pax_any())))
    }

    fn build_inline_properties(&self, defined_properties: HashMap<String,ValueDefinition>) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>> {
        Box::new(move |stack_frame , table | Rc::new(RefCell::new(
            {
        let mut properties = PaxDropdown::default();
        
            if let Some(vd) = defined_properties.get("options") {
                properties.options.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<std::vec::Vec<ui_components::pax_reexports::std::string::String>>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<std::vec::Vec<ui_components::pax_reexports::std::string::String>>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(stdCOCOvecCOCOVecLABRui_componentsCOCOpax_reexportsCOCOstdCOCOstringCOCOStringRABRTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for options")
                    });
            }
        
            if let Some(vd) = defined_properties.get("selected_id") {
                properties.selected_id.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<u32>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<u32>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(u32TypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for selected_id")
                    });
            }
        
            if let Some(vd) = defined_properties.get("text_style") {
                properties.text_style.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_std::types::text::TextStyle>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_std::types::text::TextStyle>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOTextStyleTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for text_style")
                    });
            }
        
            if let Some(vd) = defined_properties.get("background") {
                properties.background.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Color>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_engine::api::Color>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOColorTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for background")
                    });
            }
        
            if let Some(vd) = defined_properties.get("stroke") {
                properties.stroke.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Stroke>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_engine::api::Stroke>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOStrokeTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for stroke")
                    });
            }
        
        properties.to_pax_any()
        })))
    }

    fn build_handler(&self,fn_name: &str) -> fn(Rc<RefCell<PaxAny>>, &NodeContext, Option::<PaxAny>) {
        match fn_name {
            
            _ => panic!("Unknown handler name {}", fn_name)
        }
    }

    fn build_component_handlers(&self, handlers: Vec<(String, Vec<String>)>) -> Rc<RefCell<HandlerRegistry>> {
        let mut handler_registry = HandlerRegistry::default();
        for (event, functions) in &handlers {
            handler_registry.handlers.insert(event.clone(), functions.iter().map(|fn_name| {
                Handler::new_component_handler(self.build_handler(&fn_name))
            }).collect());
        } 
        Rc::new(RefCell::new(handler_registry))
    }

    fn add_inline_handlers(&self, handlers: Vec<(String, String)>, handler_registry: Rc<RefCell<HandlerRegistry>>) -> Rc<RefCell<HandlerRegistry>> {
        {
            let mut handler_registry_mut = borrow_mut!(handler_registry);
            for (event, fn_name) in &handlers {
                let handler_vec = handler_registry_mut.handlers.entry(event.clone()).or_insert(Vec::new());
                handler_vec.push(Handler::new_inline_handler(self.build_handler(&fn_name)));
            } 
        }   
        handler_registry
    }

    fn build_component(&self, args: InstantiationArgs) -> Rc<dyn InstanceNode> {
        
        ComponentInstance::instantiate(args)
            
    }

    fn get_properties_scope_factory(&self) -> Box<dyn Fn(Rc<RefCell<PaxAny>>) -> HashMap<String, UntypedProperty>>  {
        Box::new(|props| {
            let properties = &mut *borrow_mut!(props.as_ref());
            if let Ok(properties) = <PaxDropdown>::mut_from_pax_any(properties) {
                let mut scope = HashMap::new();
                
                    scope.insert("options".to_string(), properties.options.untyped());
                
                    scope.insert("selected_id".to_string(), properties.selected_id.untyped());
                
                    scope.insert("text_style".to_string(), properties.text_style.untyped());
                
                    scope.insert("background".to_string(), properties.background.untyped());
                
                    scope.insert("stroke".to_string(), properties.stroke.untyped());
                
                scope
            } else {
                panic!("Failed to downcast properties to PaxDropdown");
            }
        })
    }

}
struct ExampleFactory{}

impl ComponentFactory for ExampleFactory {

    fn build_default_properties(&self) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>> {
        Box::new(|_,_| Rc::new(RefCell::new(Example::default().to_pax_any())))
    }

    fn build_inline_properties(&self, defined_properties: HashMap<String,ValueDefinition>) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>> {
        Box::new(move |stack_frame , table | Rc::new(RefCell::new(
            {
        let mut properties = Example::default();
        
            if let Some(vd) = defined_properties.get("selected") {
                properties.selected.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<u32>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<u32>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(u32TypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for selected")
                    });
            }
        
        properties.to_pax_any()
        })))
    }

    fn build_handler(&self,fn_name: &str) -> fn(Rc<RefCell<PaxAny>>, &NodeContext, Option::<PaxAny>) {
        match fn_name {
            
            "on_click" => {
                |properties, ctx, args|{
                    let properties = &mut *borrow_mut!(properties.as_ref());
                    if let Ok(mut properties) = <Example>::mut_from_pax_any(properties) {
                        // downcast args to handler.type
                        
                            if let Some(args) = args {
                                if let Ok(args) = <Event<Click>>::ref_from_pax_any(&args) {
                                    Example::on_click(properties,ctx, args.clone());
                                } else {panic!("Failed to downcast args to Event<Click>")};
                            } else {
                                panic!("No Event<Click> present");
                            }
                        
                        
                    } else {panic!("Failed to downcast properties to Example")};
                }
            },
            
            _ => panic!("Unknown handler name {}", fn_name)
        }
    }

    fn build_component_handlers(&self, handlers: Vec<(String, Vec<String>)>) -> Rc<RefCell<HandlerRegistry>> {
        let mut handler_registry = HandlerRegistry::default();
        for (event, functions) in &handlers {
            handler_registry.handlers.insert(event.clone(), functions.iter().map(|fn_name| {
                Handler::new_component_handler(self.build_handler(&fn_name))
            }).collect());
        } 
        Rc::new(RefCell::new(handler_registry))
    }

    fn add_inline_handlers(&self, handlers: Vec<(String, String)>, handler_registry: Rc<RefCell<HandlerRegistry>>) -> Rc<RefCell<HandlerRegistry>> {
        {
            let mut handler_registry_mut = borrow_mut!(handler_registry);
            for (event, fn_name) in &handlers {
                let handler_vec = handler_registry_mut.handlers.entry(event.clone()).or_insert(Vec::new());
                handler_vec.push(Handler::new_inline_handler(self.build_handler(&fn_name)));
            } 
        }   
        handler_registry
    }

    fn build_component(&self, args: InstantiationArgs) -> Rc<dyn InstanceNode> {
        
        ComponentInstance::instantiate(args)
            
    }

    fn get_properties_scope_factory(&self) -> Box<dyn Fn(Rc<RefCell<PaxAny>>) -> HashMap<String, UntypedProperty>>  {
        Box::new(|props| {
            let properties = &mut *borrow_mut!(props.as_ref());
            if let Ok(properties) = <Example>::mut_from_pax_any(properties) {
                let mut scope = HashMap::new();
                
                    scope.insert("selected".to_string(), properties.selected.untyped());
                
                scope
            } else {
                panic!("Failed to downcast properties to Example");
            }
        })
    }

}
struct DropdownFactory{}

impl ComponentFactory for DropdownFactory {

    fn build_default_properties(&self) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>> {
        Box::new(|_,_| Rc::new(RefCell::new(Dropdown::default().to_pax_any())))
    }

    fn build_inline_properties(&self, defined_properties: HashMap<String,ValueDefinition>) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>> {
        Box::new(move |stack_frame , table | Rc::new(RefCell::new(
            {
        let mut properties = Dropdown::default();
        
            if let Some(vd) = defined_properties.get("options") {
                properties.options.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<std::vec::Vec<ui_components::pax_reexports::std::string::String>>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<std::vec::Vec<ui_components::pax_reexports::std::string::String>>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(stdCOCOvecCOCOVecLABRui_componentsCOCOpax_reexportsCOCOstdCOCOstringCOCOStringRABRTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for options")
                    });
            }
        
            if let Some(vd) = defined_properties.get("selected_id") {
                properties.selected_id.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<u32>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<u32>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(u32TypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for selected_id")
                    });
            }
        
            if let Some(vd) = defined_properties.get("style") {
                properties.style.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_std::types::text::TextStyle>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_std::types::text::TextStyle>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOTextStyleTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for style")
                    });
            }
        
            if let Some(vd) = defined_properties.get("background") {
                properties.background.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Color>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_engine::api::Color>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOColorTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for background")
                    });
            }
        
            if let Some(vd) = defined_properties.get("stroke") {
                properties.stroke.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Stroke>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_engine::api::Stroke>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOStrokeTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for stroke")
                    });
            }
        
        properties.to_pax_any()
        })))
    }

    fn build_handler(&self,fn_name: &str) -> fn(Rc<RefCell<PaxAny>>, &NodeContext, Option::<PaxAny>) {
        match fn_name {
            
            _ => panic!("Unknown handler name {}", fn_name)
        }
    }

    fn build_component_handlers(&self, handlers: Vec<(String, Vec<String>)>) -> Rc<RefCell<HandlerRegistry>> {
        let mut handler_registry = HandlerRegistry::default();
        for (event, functions) in &handlers {
            handler_registry.handlers.insert(event.clone(), functions.iter().map(|fn_name| {
                Handler::new_component_handler(self.build_handler(&fn_name))
            }).collect());
        } 
        Rc::new(RefCell::new(handler_registry))
    }

    fn add_inline_handlers(&self, handlers: Vec<(String, String)>, handler_registry: Rc<RefCell<HandlerRegistry>>) -> Rc<RefCell<HandlerRegistry>> {
        {
            let mut handler_registry_mut = borrow_mut!(handler_registry);
            for (event, fn_name) in &handlers {
                let handler_vec = handler_registry_mut.handlers.entry(event.clone()).or_insert(Vec::new());
                handler_vec.push(Handler::new_inline_handler(self.build_handler(&fn_name)));
            } 
        }   
        handler_registry
    }

    fn build_component(&self, args: InstantiationArgs) -> Rc<dyn InstanceNode> {
        
        pax_std_primitives::dropdown::DropdownInstance::instantiate(args)
            
    }

    fn get_properties_scope_factory(&self) -> Box<dyn Fn(Rc<RefCell<PaxAny>>) -> HashMap<String, UntypedProperty>>  {
        Box::new(|props| {
            let properties = &mut *borrow_mut!(props.as_ref());
            if let Ok(properties) = <Dropdown>::mut_from_pax_any(properties) {
                let mut scope = HashMap::new();
                
                    scope.insert("options".to_string(), properties.options.untyped());
                
                    scope.insert("selected_id".to_string(), properties.selected_id.untyped());
                
                    scope.insert("style".to_string(), properties.style.untyped());
                
                    scope.insert("background".to_string(), properties.background.untyped());
                
                    scope.insert("stroke".to_string(), properties.stroke.untyped());
                
                scope
            } else {
                panic!("Failed to downcast properties to Dropdown");
            }
        })
    }

}
struct RectangleFactory{}

impl ComponentFactory for RectangleFactory {

    fn build_default_properties(&self) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>> {
        Box::new(|_,_| Rc::new(RefCell::new(Rectangle::default().to_pax_any())))
    }

    fn build_inline_properties(&self, defined_properties: HashMap<String,ValueDefinition>) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>> {
        Box::new(move |stack_frame , table | Rc::new(RefCell::new(
            {
        let mut properties = Rectangle::default();
        
            if let Some(vd) = defined_properties.get("stroke") {
                properties.stroke.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Stroke>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_engine::api::Stroke>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOStrokeTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for stroke")
                    });
            }
        
            if let Some(vd) = defined_properties.get("fill") {
                properties.fill.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Fill>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_engine::api::Fill>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOFillTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for fill")
                    });
            }
        
            if let Some(vd) = defined_properties.get("corner_radii") {
                properties.corner_radii.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_std::types::RectangleCornerRadii>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_std::types::RectangleCornerRadii>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCORectangleCornerRadiiTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for corner_radii")
                    });
            }
        
        properties.to_pax_any()
        })))
    }

    fn build_handler(&self,fn_name: &str) -> fn(Rc<RefCell<PaxAny>>, &NodeContext, Option::<PaxAny>) {
        match fn_name {
            
            _ => panic!("Unknown handler name {}", fn_name)
        }
    }

    fn build_component_handlers(&self, handlers: Vec<(String, Vec<String>)>) -> Rc<RefCell<HandlerRegistry>> {
        let mut handler_registry = HandlerRegistry::default();
        for (event, functions) in &handlers {
            handler_registry.handlers.insert(event.clone(), functions.iter().map(|fn_name| {
                Handler::new_component_handler(self.build_handler(&fn_name))
            }).collect());
        } 
        Rc::new(RefCell::new(handler_registry))
    }

    fn add_inline_handlers(&self, handlers: Vec<(String, String)>, handler_registry: Rc<RefCell<HandlerRegistry>>) -> Rc<RefCell<HandlerRegistry>> {
        {
            let mut handler_registry_mut = borrow_mut!(handler_registry);
            for (event, fn_name) in &handlers {
                let handler_vec = handler_registry_mut.handlers.entry(event.clone()).or_insert(Vec::new());
                handler_vec.push(Handler::new_inline_handler(self.build_handler(&fn_name)));
            } 
        }   
        handler_registry
    }

    fn build_component(&self, args: InstantiationArgs) -> Rc<dyn InstanceNode> {
        
        pax_std_primitives::rectangle::RectangleInstance::instantiate(args)
            
    }

    fn get_properties_scope_factory(&self) -> Box<dyn Fn(Rc<RefCell<PaxAny>>) -> HashMap<String, UntypedProperty>>  {
        Box::new(|props| {
            let properties = &mut *borrow_mut!(props.as_ref());
            if let Ok(properties) = <Rectangle>::mut_from_pax_any(properties) {
                let mut scope = HashMap::new();
                
                    scope.insert("stroke".to_string(), properties.stroke.untyped());
                
                    scope.insert("fill".to_string(), properties.fill.untyped());
                
                    scope.insert("corner_radii".to_string(), properties.corner_radii.untyped());
                
                scope
            } else {
                panic!("Failed to downcast properties to Rectangle");
            }
        })
    }

}
struct GroupFactory{}

impl ComponentFactory for GroupFactory {

    fn build_default_properties(&self) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>> {
        Box::new(|_,_| Rc::new(RefCell::new(Group::default().to_pax_any())))
    }

    fn build_inline_properties(&self, defined_properties: HashMap<String,ValueDefinition>) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>> {
        Box::new(move |stack_frame , table | Rc::new(RefCell::new(
            {
        let mut properties = Group::default();
        
        properties.to_pax_any()
        })))
    }

    fn build_handler(&self,fn_name: &str) -> fn(Rc<RefCell<PaxAny>>, &NodeContext, Option::<PaxAny>) {
        match fn_name {
            
            _ => panic!("Unknown handler name {}", fn_name)
        }
    }

    fn build_component_handlers(&self, handlers: Vec<(String, Vec<String>)>) -> Rc<RefCell<HandlerRegistry>> {
        let mut handler_registry = HandlerRegistry::default();
        for (event, functions) in &handlers {
            handler_registry.handlers.insert(event.clone(), functions.iter().map(|fn_name| {
                Handler::new_component_handler(self.build_handler(&fn_name))
            }).collect());
        } 
        Rc::new(RefCell::new(handler_registry))
    }

    fn add_inline_handlers(&self, handlers: Vec<(String, String)>, handler_registry: Rc<RefCell<HandlerRegistry>>) -> Rc<RefCell<HandlerRegistry>> {
        {
            let mut handler_registry_mut = borrow_mut!(handler_registry);
            for (event, fn_name) in &handlers {
                let handler_vec = handler_registry_mut.handlers.entry(event.clone()).or_insert(Vec::new());
                handler_vec.push(Handler::new_inline_handler(self.build_handler(&fn_name)));
            } 
        }   
        handler_registry
    }

    fn build_component(&self, args: InstantiationArgs) -> Rc<dyn InstanceNode> {
        
        pax_std_primitives::group::GroupInstance::instantiate(args)
            
    }

    fn get_properties_scope_factory(&self) -> Box<dyn Fn(Rc<RefCell<PaxAny>>) -> HashMap<String, UntypedProperty>>  {
        Box::new(|props| {
            let properties = &mut *borrow_mut!(props.as_ref());
            if let Ok(properties) = <Group>::mut_from_pax_any(properties) {
                let mut scope = HashMap::new();
                
                scope
            } else {
                panic!("Failed to downcast properties to Group");
            }
        })
    }

}
struct TabsFactory{}

impl ComponentFactory for TabsFactory {

    fn build_default_properties(&self) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>> {
        Box::new(|_,_| Rc::new(RefCell::new(Tabs::default().to_pax_any())))
    }

    fn build_inline_properties(&self, defined_properties: HashMap<String,ValueDefinition>) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>> {
        Box::new(move |stack_frame , table | Rc::new(RefCell::new(
            {
        let mut properties = Tabs::default();
        
            if let Some(vd) = defined_properties.get("names") {
                properties.names.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<std::vec::Vec<ui_components::pax_reexports::std::string::String>>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<std::vec::Vec<ui_components::pax_reexports::std::string::String>>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(stdCOCOvecCOCOVecLABRui_componentsCOCOpax_reexportsCOCOstdCOCOstringCOCOStringRABRTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for names")
                    });
            }
        
            if let Some(vd) = defined_properties.get("selected") {
                properties.selected.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<usize>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<usize>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(usizeTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for selected")
                    });
            }
        
            if let Some(vd) = defined_properties.get("color") {
                properties.color.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Color>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_engine::api::Color>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOColorTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for color")
                    });
            }
        
            if let Some(vd) = defined_properties.get("slot_count") {
                properties.slot_count.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<usize>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<usize>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(usizeTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for slot_count")
                    });
            }
        
            if let Some(vd) = defined_properties.get("names_filled") {
                properties.names_filled.replace_with(
                    match vd.clone() {
                        ValueDefinition::LiteralValue(lv) => {
                                let val = from_pax_try_coerce::<std::vec::Vec<ui_components::pax_reexports::std::string::String>>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                            Property::new_with_name(val, &lv.raw_value)
                        },
                        ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token,info) =>
                        {
                            if let Some(info) = info {
                                let mut dependents = vec![];
                                for dependency in &info.dependencies {
                                    if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                        dependents.push(p);
                                    } else {
                                        panic!("Failed to resolve symbol {}", dependency);
                                    }
                                }
                                let cloned_stack = stack_frame.clone();
                                let cloned_table = table.clone();
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                    let coerced = new_value_wrapped.try_coerce::<std::vec::Vec<ui_components::pax_reexports::std::string::String>>().unwrap();
                                    coerced
                                }, &dependents, &token.raw_value)
                            } else {
                                unreachable!("No info for expression")
                            }
                        },
                        ValueDefinition::Block(block) => {
                            Property::new_with_name(stdCOCOvecCOCOVecLABRui_componentsCOCOpax_reexportsCOCOstdCOCOstringCOCOStringRABRTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                        }
                        _ => unreachable!("Invalid value definition for names_filled")
                    });
            }
        
        properties.to_pax_any()
        })))
    }

    fn build_handler(&self,fn_name: &str) -> fn(Rc<RefCell<PaxAny>>, &NodeContext, Option::<PaxAny>) {
        match fn_name {
            
            "on_mount" => {
                |properties, ctx, args|{
                    let properties = &mut *borrow_mut!(properties.as_ref());
                    if let Ok(mut properties) = <Tabs>::mut_from_pax_any(properties) {
                        // downcast args to handler.type
                        
                            if let None = args {
                                Tabs::on_mount(properties,ctx);
                            } else {
                                panic!("Unexpected args present");
                            }
                        
                        
                    } else {panic!("Failed to downcast properties to Tabs")};
                }
            },
            
            "on_click" => {
                |properties, ctx, args|{
                    let properties = &mut *borrow_mut!(properties.as_ref());
                    if let Ok(mut properties) = <Tabs>::mut_from_pax_any(properties) {
                        // downcast args to handler.type
                        
                            if let Some(args) = args {
                                if let Ok(args) = <Event<Click>>::ref_from_pax_any(&args) {
                                    Tabs::on_click(properties,ctx, args.clone());
                                } else {panic!("Failed to downcast args to Event<Click>")};
                            } else {
                                panic!("No Event<Click> present");
                            }
                        
                        
                    } else {panic!("Failed to downcast properties to Tabs")};
                }
            },
            
            _ => panic!("Unknown handler name {}", fn_name)
        }
    }

    fn build_component_handlers(&self, handlers: Vec<(String, Vec<String>)>) -> Rc<RefCell<HandlerRegistry>> {
        let mut handler_registry = HandlerRegistry::default();
        for (event, functions) in &handlers {
            handler_registry.handlers.insert(event.clone(), functions.iter().map(|fn_name| {
                Handler::new_component_handler(self.build_handler(&fn_name))
            }).collect());
        } 
        Rc::new(RefCell::new(handler_registry))
    }

    fn add_inline_handlers(&self, handlers: Vec<(String, String)>, handler_registry: Rc<RefCell<HandlerRegistry>>) -> Rc<RefCell<HandlerRegistry>> {
        {
            let mut handler_registry_mut = borrow_mut!(handler_registry);
            for (event, fn_name) in &handlers {
                let handler_vec = handler_registry_mut.handlers.entry(event.clone()).or_insert(Vec::new());
                handler_vec.push(Handler::new_inline_handler(self.build_handler(&fn_name)));
            } 
        }   
        handler_registry
    }

    fn build_component(&self, args: InstantiationArgs) -> Rc<dyn InstanceNode> {
        
        ComponentInstance::instantiate(args)
            
    }

    fn get_properties_scope_factory(&self) -> Box<dyn Fn(Rc<RefCell<PaxAny>>) -> HashMap<String, UntypedProperty>>  {
        Box::new(|props| {
            let properties = &mut *borrow_mut!(props.as_ref());
            if let Ok(properties) = <Tabs>::mut_from_pax_any(properties) {
                let mut scope = HashMap::new();
                
                    scope.insert("names".to_string(), properties.names.untyped());
                
                    scope.insert("selected".to_string(), properties.selected.untyped());
                
                    scope.insert("color".to_string(), properties.color.untyped());
                
                    scope.insert("slot_count".to_string(), properties.slot_count.untyped());
                
                    scope.insert("names_filled".to_string(), properties.names_filled.untyped());
                
                scope
            } else {
                panic!("Failed to downcast properties to Tabs");
            }
        })
    }

}

trait TypeFactory {
    type Output: Default + Clone;
    
    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output;
}

#[allow(non_camel_case_types)]
struct stdCOCOoptionCOCOOptionLABRui_componentsCOCOpax_reexportsCOCOusizeRABRTypeFactory{}

impl TypeFactory for stdCOCOoptionCOCOOptionLABRui_componentsCOCOpax_reexportsCOCOusizeRABRTypeFactory {

    type Output=std::option::Option<ui_components::pax_reexports::usize>;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: std::option::Option<ui_components::pax_reexports::usize> = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
#[allow(non_camel_case_types)]
struct StringTypeFactory{}

impl TypeFactory for StringTypeFactory {

    type Output=String;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: String = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
#[allow(non_camel_case_types)]
struct boolTypeFactory{}

impl TypeFactory for boolTypeFactory {

    type Output=bool;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: bool = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
#[allow(non_camel_case_types)]
struct f64TypeFactory{}

impl TypeFactory for f64TypeFactory {

    type Output=f64;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: f64 = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
#[allow(non_camel_case_types)]
struct i128TypeFactory{}

impl TypeFactory for i128TypeFactory {

    type Output=i128;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: i128 = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
#[allow(non_camel_case_types)]
struct i16TypeFactory{}

impl TypeFactory for i16TypeFactory {

    type Output=i16;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: i16 = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
#[allow(non_camel_case_types)]
struct i32TypeFactory{}

impl TypeFactory for i32TypeFactory {

    type Output=i32;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: i32 = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
#[allow(non_camel_case_types)]
struct i64TypeFactory{}

impl TypeFactory for i64TypeFactory {

    type Output=i64;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: i64 = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
#[allow(non_camel_case_types)]
struct i8TypeFactory{}

impl TypeFactory for i8TypeFactory {

    type Output=i8;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: i8 = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
#[allow(non_camel_case_types)]
struct isizeTypeFactory{}

impl TypeFactory for isizeTypeFactory {

    type Output=isize;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: isize = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
#[allow(non_camel_case_types)]
struct u128TypeFactory{}

impl TypeFactory for u128TypeFactory {

    type Output=u128;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: u128 = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
#[allow(non_camel_case_types)]
struct u16TypeFactory{}

impl TypeFactory for u16TypeFactory {

    type Output=u16;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: u16 = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
#[allow(non_camel_case_types)]
struct u32TypeFactory{}

impl TypeFactory for u32TypeFactory {

    type Output=u32;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: u32 = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
#[allow(non_camel_case_types)]
struct u64TypeFactory{}

impl TypeFactory for u64TypeFactory {

    type Output=u64;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: u64 = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
#[allow(non_camel_case_types)]
struct u8TypeFactory{}

impl TypeFactory for u8TypeFactory {

    type Output=u8;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: u8 = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
#[allow(non_camel_case_types)]
struct usizeTypeFactory{}

impl TypeFactory for usizeTypeFactory {

    type Output=usize;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: usize = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOprimitivesCOCOBlankComponentTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOprimitivesCOCOBlankComponentTypeFactory {

    type Output=ui_components::pax_reexports::pax_std::primitives::BlankComponent;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_std::primitives::BlankComponent = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOColorTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOColorTypeFactory {

    type Output=ui_components::pax_reexports::pax_engine::api::Color;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_engine::api::Color = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOprimitivesCOCODropdownTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOprimitivesCOCODropdownTypeFactory {

    type Output=ui_components::pax_reexports::pax_std::primitives::Dropdown;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_std::primitives::Dropdown = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    "options" => {
                        
                            properties.options = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<std::vec::Vec<ui_components::pax_reexports::std::string::String>>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<std::vec::Vec<ui_components::pax_reexports::std::string::String>>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(stdCOCOvecCOCOVecLABRui_componentsCOCOpax_reexportsCOCOstdCOCOstringCOCOStringRABRTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for options")
                                };
                            
                        
                    },
                    
                    "selected_id" => {
                        
                            properties.selected_id = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<u32>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<u32>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(u32TypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for selected_id")
                                };
                            
                        
                    },
                    
                    "style" => {
                        
                            properties.style = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_std::types::text::TextStyle>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_std::types::text::TextStyle>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOTextStyleTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for style")
                                };
                            
                        
                    },
                    
                    "background" => {
                        
                            properties.background = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Color>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_engine::api::Color>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOColorTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for background")
                                };
                            
                        
                    },
                    
                    "stroke" => {
                        
                            properties.stroke = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Stroke>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_engine::api::Stroke>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOStrokeTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for stroke")
                                };
                            
                        
                    },
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOExampleTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOExampleTypeFactory {

    type Output=ui_components::pax_reexports::Example;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::Example = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    "selected" => {
                        
                            properties.selected = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<u32>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<u32>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(u32TypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for selected")
                                };
                            
                        
                    },
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOFillTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOFillTypeFactory {

    type Output=ui_components::pax_reexports::pax_engine::api::Fill;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_engine::api::Fill = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOFontTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOFontTypeFactory {

    type Output=ui_components::pax_reexports::pax_std::types::text::Font;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_std::types::text::Font = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    "System" => {
                        
                    },
                    
                    "Web" => {
                        
                    },
                    
                    "Local" => {
                        
                    },
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOFontStyleTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOFontStyleTypeFactory {

    type Output=ui_components::pax_reexports::pax_std::types::text::FontStyle;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_std::types::text::FontStyle = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOFontWeightTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOFontWeightTypeFactory {

    type Output=ui_components::pax_reexports::pax_std::types::text::FontWeight;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_std::types::text::FontWeight = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOprimitivesCOCOGroupTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOprimitivesCOCOGroupTypeFactory {

    type Output=ui_components::pax_reexports::pax_std::primitives::Group;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_std::primitives::Group = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOLocalFontTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOLocalFontTypeFactory {

    type Output=ui_components::pax_reexports::pax_std::types::text::LocalFont;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_std::types::text::LocalFont = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    "family" => {
                        
                            properties.family = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        from_pax_try_coerce::<ui_components::pax_reexports::std::string::String>(&lv.raw_value).unwrap()
                                    },
                                    ValueDefinition::Block(block) => {
                                        ui_componentsCOCOpax_reexportsCOCOstdCOCOstringCOCOStringTypeFactory{}.build_type(args, stack_frame.clone(), table.clone())
                                    }
                                    _ => unreachable!("Invalid value definition for family")
                                };
                            
                        
                    },
                    
                    "path" => {
                        
                            properties.path = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        from_pax_try_coerce::<ui_components::pax_reexports::std::string::String>(&lv.raw_value).unwrap()
                                    },
                                    ValueDefinition::Block(block) => {
                                        ui_componentsCOCOpax_reexportsCOCOstdCOCOstringCOCOStringTypeFactory{}.build_type(args, stack_frame.clone(), table.clone())
                                    }
                                    _ => unreachable!("Invalid value definition for path")
                                };
                            
                        
                    },
                    
                    "style" => {
                        
                            properties.style = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        from_pax_try_coerce::<ui_components::pax_reexports::pax_std::types::text::FontStyle>(&lv.raw_value).unwrap()
                                    },
                                    ValueDefinition::Block(block) => {
                                        ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOFontStyleTypeFactory{}.build_type(args, stack_frame.clone(), table.clone())
                                    }
                                    _ => unreachable!("Invalid value definition for style")
                                };
                            
                        
                    },
                    
                    "weight" => {
                        
                            properties.weight = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        from_pax_try_coerce::<ui_components::pax_reexports::pax_std::types::text::FontWeight>(&lv.raw_value).unwrap()
                                    },
                                    ValueDefinition::Block(block) => {
                                        ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOFontWeightTypeFactory{}.build_type(args, stack_frame.clone(), table.clone())
                                    }
                                    _ => unreachable!("Invalid value definition for weight")
                                };
                            
                        
                    },
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCONumericTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCONumericTypeFactory {

    type Output=ui_components::pax_reexports::pax_engine::api::Numeric;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_engine::api::Numeric = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_component_libraryCOCOdropdownCOCOPaxDropdownTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_component_libraryCOCOdropdownCOCOPaxDropdownTypeFactory {

    type Output=ui_components::pax_reexports::pax_component_library::dropdown::PaxDropdown;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_component_library::dropdown::PaxDropdown = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    "options" => {
                        
                            properties.options = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<std::vec::Vec<ui_components::pax_reexports::std::string::String>>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<std::vec::Vec<ui_components::pax_reexports::std::string::String>>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(stdCOCOvecCOCOVecLABRui_componentsCOCOpax_reexportsCOCOstdCOCOstringCOCOStringRABRTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for options")
                                };
                            
                        
                    },
                    
                    "selected_id" => {
                        
                            properties.selected_id = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<u32>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<u32>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(u32TypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for selected_id")
                                };
                            
                        
                    },
                    
                    "text_style" => {
                        
                            properties.text_style = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_std::types::text::TextStyle>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_std::types::text::TextStyle>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOTextStyleTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for text_style")
                                };
                            
                        
                    },
                    
                    "background" => {
                        
                            properties.background = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Color>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_engine::api::Color>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOColorTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for background")
                                };
                            
                        
                    },
                    
                    "stroke" => {
                        
                            properties.stroke = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Stroke>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_engine::api::Stroke>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOStrokeTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for stroke")
                                };
                            
                        
                    },
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOprimitivesCOCORectangleTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOprimitivesCOCORectangleTypeFactory {

    type Output=ui_components::pax_reexports::pax_std::primitives::Rectangle;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_std::primitives::Rectangle = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    "stroke" => {
                        
                            properties.stroke = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Stroke>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_engine::api::Stroke>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOStrokeTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for stroke")
                                };
                            
                        
                    },
                    
                    "fill" => {
                        
                            properties.fill = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Fill>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_engine::api::Fill>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOFillTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for fill")
                                };
                            
                        
                    },
                    
                    "corner_radii" => {
                        
                            properties.corner_radii = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_std::types::RectangleCornerRadii>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_std::types::RectangleCornerRadii>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCORectangleCornerRadiiTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for corner_radii")
                                };
                            
                        
                    },
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCORectangleCornerRadiiTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCORectangleCornerRadiiTypeFactory {

    type Output=ui_components::pax_reexports::pax_std::types::RectangleCornerRadii;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_std::types::RectangleCornerRadii = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    "top_left" => {
                        
                            properties.top_left = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Numeric>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_engine::api::Numeric>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCONumericTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for top_left")
                                };
                            
                        
                    },
                    
                    "top_right" => {
                        
                            properties.top_right = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Numeric>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_engine::api::Numeric>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCONumericTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for top_right")
                                };
                            
                        
                    },
                    
                    "bottom_right" => {
                        
                            properties.bottom_right = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Numeric>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_engine::api::Numeric>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCONumericTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for bottom_right")
                                };
                            
                        
                    },
                    
                    "bottom_left" => {
                        
                            properties.bottom_left = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Numeric>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_engine::api::Numeric>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCONumericTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for bottom_left")
                                };
                            
                        
                    },
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_component_libraryCOCOresizableCOCOResizableTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_component_libraryCOCOresizableCOCOResizableTypeFactory {

    type Output=ui_components::pax_reexports::pax_component_library::resizable::Resizable;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_component_library::resizable::Resizable = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    "dividers" => {
                        
                            properties.dividers = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<std::vec::Vec<ui_components::pax_reexports::pax_engine::api::Size>>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<std::vec::Vec<ui_components::pax_reexports::pax_engine::api::Size>>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(stdCOCOvecCOCOVecLABRui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOSizeRABRTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for dividers")
                                };
                            
                        
                    },
                    
                    "direction" => {
                        
                            properties.direction = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_component_library::resizable::ResizableDirection>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_component_library::resizable::ResizableDirection>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_component_libraryCOCOresizableCOCOResizableDirectionTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for direction")
                                };
                            
                        
                    },
                    
                    "sections" => {
                        
                            properties.sections = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<std::vec::Vec<ui_components::pax_reexports::pax_component_library::resizable::Section>>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<std::vec::Vec<ui_components::pax_reexports::pax_component_library::resizable::Section>>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(stdCOCOvecCOCOVecLABRui_componentsCOCOpax_reexportsCOCOpax_component_libraryCOCOresizableCOCOSectionRABRTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for sections")
                                };
                            
                        
                    },
                    
                    "index_moving" => {
                        
                            properties.index_moving = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<std::option::Option<ui_components::pax_reexports::usize>>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<std::option::Option<ui_components::pax_reexports::usize>>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(stdCOCOoptionCOCOOptionLABRui_componentsCOCOpax_reexportsCOCOusizeRABRTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for index_moving")
                                };
                            
                        
                    },
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_component_libraryCOCOresizableCOCOResizableDirectionTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_component_libraryCOCOresizableCOCOResizableDirectionTypeFactory {

    type Output=ui_components::pax_reexports::pax_component_library::resizable::ResizableDirection;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_component_library::resizable::ResizableDirection = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_component_libraryCOCOresizableCOCOSectionTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_component_libraryCOCOresizableCOCOSectionTypeFactory {

    type Output=ui_components::pax_reexports::pax_component_library::resizable::Section;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_component_library::resizable::Section = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    "x" => {
                        
                            properties.x = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Size>(&lv.raw_value).unwrap()
                                    },
                                    ValueDefinition::Block(block) => {
                                        ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOSizeTypeFactory{}.build_type(args, stack_frame.clone(), table.clone())
                                    }
                                    _ => unreachable!("Invalid value definition for x")
                                };
                            
                        
                    },
                    
                    "y" => {
                        
                            properties.y = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Size>(&lv.raw_value).unwrap()
                                    },
                                    ValueDefinition::Block(block) => {
                                        ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOSizeTypeFactory{}.build_type(args, stack_frame.clone(), table.clone())
                                    }
                                    _ => unreachable!("Invalid value definition for y")
                                };
                            
                        
                    },
                    
                    "width" => {
                        
                            properties.width = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Size>(&lv.raw_value).unwrap()
                                    },
                                    ValueDefinition::Block(block) => {
                                        ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOSizeTypeFactory{}.build_type(args, stack_frame.clone(), table.clone())
                                    }
                                    _ => unreachable!("Invalid value definition for width")
                                };
                            
                        
                    },
                    
                    "height" => {
                        
                            properties.height = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Size>(&lv.raw_value).unwrap()
                                    },
                                    ValueDefinition::Block(block) => {
                                        ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOSizeTypeFactory{}.build_type(args, stack_frame.clone(), table.clone())
                                    }
                                    _ => unreachable!("Invalid value definition for height")
                                };
                            
                        
                    },
                    
                    "i" => {
                        
                            properties.i = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        from_pax_try_coerce::<usize>(&lv.raw_value).unwrap()
                                    },
                                    ValueDefinition::Block(block) => {
                                        usizeTypeFactory{}.build_type(args, stack_frame.clone(), table.clone())
                                    }
                                    _ => unreachable!("Invalid value definition for i")
                                };
                            
                        
                    },
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOSizeTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOSizeTypeFactory {

    type Output=ui_components::pax_reexports::pax_engine::api::Size;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_engine::api::Size = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOstdCOCOstringCOCOStringTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOstdCOCOstringCOCOStringTypeFactory {

    type Output=ui_components::pax_reexports::std::string::String;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::std::string::String = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOStrokeTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOStrokeTypeFactory {

    type Output=ui_components::pax_reexports::pax_engine::api::Stroke;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_engine::api::Stroke = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    "color" => {
                        
                            properties.color = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Color>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_engine::api::Color>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOColorTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for color")
                                };
                            
                        
                    },
                    
                    "width" => {
                        
                            properties.width = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Size>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_engine::api::Size>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOSizeTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for width")
                                };
                            
                        
                    },
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOSystemFontTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOSystemFontTypeFactory {

    type Output=ui_components::pax_reexports::pax_std::types::text::SystemFont;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_std::types::text::SystemFont = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    "family" => {
                        
                            properties.family = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        from_pax_try_coerce::<ui_components::pax_reexports::std::string::String>(&lv.raw_value).unwrap()
                                    },
                                    ValueDefinition::Block(block) => {
                                        ui_componentsCOCOpax_reexportsCOCOstdCOCOstringCOCOStringTypeFactory{}.build_type(args, stack_frame.clone(), table.clone())
                                    }
                                    _ => unreachable!("Invalid value definition for family")
                                };
                            
                        
                    },
                    
                    "style" => {
                        
                            properties.style = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        from_pax_try_coerce::<ui_components::pax_reexports::pax_std::types::text::FontStyle>(&lv.raw_value).unwrap()
                                    },
                                    ValueDefinition::Block(block) => {
                                        ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOFontStyleTypeFactory{}.build_type(args, stack_frame.clone(), table.clone())
                                    }
                                    _ => unreachable!("Invalid value definition for style")
                                };
                            
                        
                    },
                    
                    "weight" => {
                        
                            properties.weight = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        from_pax_try_coerce::<ui_components::pax_reexports::pax_std::types::text::FontWeight>(&lv.raw_value).unwrap()
                                    },
                                    ValueDefinition::Block(block) => {
                                        ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOFontWeightTypeFactory{}.build_type(args, stack_frame.clone(), table.clone())
                                    }
                                    _ => unreachable!("Invalid value definition for weight")
                                };
                            
                        
                    },
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_component_libraryCOCOtabsCOCOTabsTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_component_libraryCOCOtabsCOCOTabsTypeFactory {

    type Output=ui_components::pax_reexports::pax_component_library::tabs::Tabs;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_component_library::tabs::Tabs = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    "names" => {
                        
                            properties.names = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<std::vec::Vec<ui_components::pax_reexports::std::string::String>>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<std::vec::Vec<ui_components::pax_reexports::std::string::String>>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(stdCOCOvecCOCOVecLABRui_componentsCOCOpax_reexportsCOCOstdCOCOstringCOCOStringRABRTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for names")
                                };
                            
                        
                    },
                    
                    "selected" => {
                        
                            properties.selected = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<usize>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<usize>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(usizeTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for selected")
                                };
                            
                        
                    },
                    
                    "color" => {
                        
                            properties.color = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Color>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_engine::api::Color>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOColorTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for color")
                                };
                            
                        
                    },
                    
                    "slot_count" => {
                        
                            properties.slot_count = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<usize>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<usize>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(usizeTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for slot_count")
                                };
                            
                        
                    },
                    
                    "names_filled" => {
                        
                            properties.names_filled = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<std::vec::Vec<ui_components::pax_reexports::std::string::String>>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<std::vec::Vec<ui_components::pax_reexports::std::string::String>>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(stdCOCOvecCOCOVecLABRui_componentsCOCOpax_reexportsCOCOstdCOCOstringCOCOStringRABRTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for names_filled")
                                };
                            
                        
                    },
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOprimitivesCOCOTextTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOprimitivesCOCOTextTypeFactory {

    type Output=ui_components::pax_reexports::pax_std::primitives::Text;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_std::primitives::Text = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    "editable" => {
                        
                            properties.editable = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<bool>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<bool>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(boolTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for editable")
                                };
                            
                        
                    },
                    
                    "text" => {
                        
                            properties.text = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::std::string::String>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::std::string::String>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOstdCOCOstringCOCOStringTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for text")
                                };
                            
                        
                    },
                    
                    "style" => {
                        
                            properties.style = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_std::types::text::TextStyle>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_std::types::text::TextStyle>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOTextStyleTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for style")
                                };
                            
                        
                    },
                    
                    "style_link" => {
                        
                            properties.style_link = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_std::types::text::TextStyle>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_std::types::text::TextStyle>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOTextStyleTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for style_link")
                                };
                            
                        
                    },
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOTextAlignHorizontalTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOTextAlignHorizontalTypeFactory {

    type Output=ui_components::pax_reexports::pax_std::types::text::TextAlignHorizontal;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_std::types::text::TextAlignHorizontal = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOTextAlignVerticalTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOTextAlignVerticalTypeFactory {

    type Output=ui_components::pax_reexports::pax_std::types::text::TextAlignVertical;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_std::types::text::TextAlignVertical = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOTextStyleTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOTextStyleTypeFactory {

    type Output=ui_components::pax_reexports::pax_std::types::text::TextStyle;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_std::types::text::TextStyle = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    "font" => {
                        
                            properties.font = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_std::types::text::Font>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_std::types::text::Font>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOFontTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for font")
                                };
                            
                        
                    },
                    
                    "font_size" => {
                        
                            properties.font_size = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Size>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_engine::api::Size>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOSizeTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for font_size")
                                };
                            
                        
                    },
                    
                    "fill" => {
                        
                            properties.fill = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_engine::api::Color>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_engine::api::Color>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOColorTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for fill")
                                };
                            
                        
                    },
                    
                    "underline" => {
                        
                            properties.underline = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<bool>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<bool>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(boolTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for underline")
                                };
                            
                        
                    },
                    
                    "align_multiline" => {
                        
                            properties.align_multiline = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_std::types::text::TextAlignHorizontal>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_std::types::text::TextAlignHorizontal>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOTextAlignHorizontalTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for align_multiline")
                                };
                            
                        
                    },
                    
                    "align_vertical" => {
                        
                            properties.align_vertical = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_std::types::text::TextAlignVertical>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_std::types::text::TextAlignVertical>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOTextAlignVerticalTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for align_vertical")
                                };
                            
                        
                    },
                    
                    "align_horizontal" => {
                        
                            properties.align_horizontal = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let val = from_pax_try_coerce::<ui_components::pax_reexports::pax_std::types::text::TextAlignHorizontal>(&lv.raw_value).unwrap();
                                        Property::new_with_name(val, &lv.raw_value)
                                    },
                                    ValueDefinition::Expression(token, info) | ValueDefinition::Identifier(token, info ) =>
                                    {
                                        if let Some(info) = info {
                                            let mut dependents = vec![];
                                            for dependency in &info.dependencies {
                                                if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                    dependents.push(p);
                                                } else {
                                                    panic!("Failed to resolve symbol {}", dependency);
                                                }
                                            }
                                            let cloned_stack = stack_frame.clone();
                                            let cloned_table = table.clone();
                                            let cloned_info = info.clone();
                                            Property::computed_with_name(move || {
                                                let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, cloned_info.vtable_id);
                                                let coerced = new_value_wrapped.try_coerce::<ui_components::pax_reexports::pax_std::types::text::TextAlignHorizontal>().unwrap();
                                                coerced
                                            }, &dependents, &token.raw_value)
                                        } else {
                                            unreachable!("No info for expression")
                                        }
                                    },
                                    ValueDefinition::Block(block) => {
                                        Property::new_with_name(ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOTextAlignHorizontalTypeFactory{}.build_type(&block, stack_frame.clone(), table.clone()), "block")
                                    }
                                    _ => unreachable!("Invalid value definition for align_horizontal")
                                };
                            
                        
                    },
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOWebFontTypeFactory{}

impl TypeFactory for ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOWebFontTypeFactory {

    type Output=ui_components::pax_reexports::pax_std::types::text::WebFont;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: ui_components::pax_reexports::pax_std::types::text::WebFont = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    "family" => {
                        
                            properties.family = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        from_pax_try_coerce::<ui_components::pax_reexports::std::string::String>(&lv.raw_value).unwrap()
                                    },
                                    ValueDefinition::Block(block) => {
                                        ui_componentsCOCOpax_reexportsCOCOstdCOCOstringCOCOStringTypeFactory{}.build_type(args, stack_frame.clone(), table.clone())
                                    }
                                    _ => unreachable!("Invalid value definition for family")
                                };
                            
                        
                    },
                    
                    "url" => {
                        
                            properties.url = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        from_pax_try_coerce::<ui_components::pax_reexports::std::string::String>(&lv.raw_value).unwrap()
                                    },
                                    ValueDefinition::Block(block) => {
                                        ui_componentsCOCOpax_reexportsCOCOstdCOCOstringCOCOStringTypeFactory{}.build_type(args, stack_frame.clone(), table.clone())
                                    }
                                    _ => unreachable!("Invalid value definition for url")
                                };
                            
                        
                    },
                    
                    "style" => {
                        
                            properties.style = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        from_pax_try_coerce::<ui_components::pax_reexports::pax_std::types::text::FontStyle>(&lv.raw_value).unwrap()
                                    },
                                    ValueDefinition::Block(block) => {
                                        ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOFontStyleTypeFactory{}.build_type(args, stack_frame.clone(), table.clone())
                                    }
                                    _ => unreachable!("Invalid value definition for style")
                                };
                            
                        
                    },
                    
                    "weight" => {
                        
                            properties.weight = 
                            
                                match vd {
                                    ValueDefinition::LiteralValue(lv) => {
                                        from_pax_try_coerce::<ui_components::pax_reexports::pax_std::types::text::FontWeight>(&lv.raw_value).unwrap()
                                    },
                                    ValueDefinition::Block(block) => {
                                        ui_componentsCOCOpax_reexportsCOCOpax_stdCOCOtypesCOCOtextCOCOFontWeightTypeFactory{}.build_type(args, stack_frame.clone(), table.clone())
                                    }
                                    _ => unreachable!("Invalid value definition for weight")
                                };
                            
                        
                    },
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
        
#[allow(non_camel_case_types)]
struct stdCOCOvecCOCOVecLABRui_componentsCOCOpax_reexportsCOCOpax_component_libraryCOCOresizableCOCOSectionRABRTypeFactory{}

impl TypeFactory for stdCOCOvecCOCOVecLABRui_componentsCOCOpax_reexportsCOCOpax_component_libraryCOCOresizableCOCOSectionRABRTypeFactory {

    type Output=std::vec::Vec<ui_components::pax_reexports::pax_component_library::resizable::Section>;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: std::vec::Vec<ui_components::pax_reexports::pax_component_library::resizable::Section> = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
#[allow(non_camel_case_types)]
struct stdCOCOvecCOCOVecLABRui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOSizeRABRTypeFactory{}

impl TypeFactory for stdCOCOvecCOCOVecLABRui_componentsCOCOpax_reexportsCOCOpax_engineCOCOapiCOCOSizeRABRTypeFactory {

    type Output=std::vec::Vec<ui_components::pax_reexports::pax_engine::api::Size>;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: std::vec::Vec<ui_components::pax_reexports::pax_engine::api::Size> = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        
#[allow(non_camel_case_types)]
struct stdCOCOvecCOCOVecLABRui_componentsCOCOpax_reexportsCOCOstdCOCOstringCOCOStringRABRTypeFactory{}

impl TypeFactory for stdCOCOvecCOCOVecLABRui_componentsCOCOpax_reexportsCOCOstdCOCOstringCOCOStringRABRTypeFactory {

    type Output=std::vec::Vec<ui_components::pax_reexports::std::string::String>;

    fn build_type(&self, args: &LiteralBlockDefinition, stack_frame: Rc<RuntimePropertiesStackFrame>, table: Rc<ExpressionTable>) -> Self::Output {
        let mut properties: std::vec::Vec<ui_components::pax_reexports::std::string::String> = Default::default();
        for setting in &args.elements {
            if let SettingElement::Setting(k, vd) = setting {
                match k.raw_value.as_str() {
                    
                    _ => panic!("Unknown property name {}", k.raw_value)
                }
            
            }
        }
        properties
    }
}
        

pub struct DefinitionToInstanceTraverser {
    #[cfg(not(feature = "designtime"))]
    manifest: PaxManifest,
    #[cfg(feature = "designtime")]
    designtime_manager: Rc<RefCell<pax_designtime::DesigntimeManager>>,
}

impl DefinitionToInstanceTraverser {

    #[cfg(not(feature = "designtime"))]
    pub fn new() -> Self {
        let manifest: PaxManifest = serde_json::from_str(INITAL_MANIFEST).expect("Failed to deserialize initial manifest");
        Self {
            manifest,
        }
    }

    #[cfg(not(feature = "designtime"))]
    pub fn get_manifest(&self) ->  &PaxManifest {
        &self.manifest
    }

    #[cfg(feature = "designtime")]
    pub fn new() -> Self {
        let manifest: PaxManifest = serde_json::from_str(INITAL_MANIFEST).expect("Failed to deserialize initial manifest");
        let designtime_manager = Rc::new(RefCell::new(pax_designtime::DesigntimeManager::new(manifest)));
        Self {
            designtime_manager,
        }
    }

    #[cfg(feature = "designtime")]
    pub fn get_designtime_manager(&self) -> Rc<RefCell<pax_designtime::DesigntimeManager>> {
        self.designtime_manager.clone()
    }

    #[cfg(feature = "designtime")]
    pub fn get_manifest(&self) ->  Ref<PaxManifest> {
        Ref::map(borrow!(self.designtime_manager), |manager| {
            manager.get_manifest()
        })
    }

    pub fn get_main_component(&mut self) -> Rc<ComponentInstance> {
        let main_component_type_id = {
            let manifest = self.get_manifest();
            manifest.main_component_type_id.clone()
        };
        let args = self.build_component_args(&main_component_type_id);
        let main_component = ComponentInstance::instantiate(args);
        main_component
    }

    pub fn get_component(&mut self, type_id: &TypeId) -> Rc<dyn InstanceNode> {
        let factory = Self::get_component_factory(type_id).expect("Failed to get component factory");
        let args = self.build_component_args(type_id);
        factory.build_component(args)
    }

    pub fn get_component_factory(type_id: &TypeId) -> Option<Box<dyn ComponentFactory>> {
        if type_id.is_blank_component() {
            return Some(Box::new(BlankComponentFactory{}) as Box<dyn ComponentFactory>);
        }

        match type_id.get_unique_identifier().as_str() {
            
            "ui_components::pax_reexports::pax_component_library::resizable::Resizable" => {
                        Some(Box::new(ResizableFactory{}) as Box<dyn ComponentFactory>)
                },
            
            "ui_components::pax_reexports::pax_std::primitives::BlankComponent" => {
                        Some(Box::new(BlankComponentFactory{}) as Box<dyn ComponentFactory>)
                },
            
            "ui_components::pax_reexports::pax_std::primitives::Text" => {
                        Some(Box::new(TextFactory{}) as Box<dyn ComponentFactory>)
                },
            
            "ui_components::pax_reexports::pax_component_library::dropdown::PaxDropdown" => {
                        Some(Box::new(PaxDropdownFactory{}) as Box<dyn ComponentFactory>)
                },
            
            "ui_components::pax_reexports::Example" => {
                        Some(Box::new(ExampleFactory{}) as Box<dyn ComponentFactory>)
                },
            
            "ui_components::pax_reexports::pax_std::primitives::Dropdown" => {
                        Some(Box::new(DropdownFactory{}) as Box<dyn ComponentFactory>)
                },
            
            "ui_components::pax_reexports::pax_std::primitives::Rectangle" => {
                        Some(Box::new(RectangleFactory{}) as Box<dyn ComponentFactory>)
                },
            
            "ui_components::pax_reexports::pax_std::primitives::Group" => {
                        Some(Box::new(GroupFactory{}) as Box<dyn ComponentFactory>)
                },
            
            "ui_components::pax_reexports::pax_component_library::tabs::Tabs" => {
                        Some(Box::new(TabsFactory{}) as Box<dyn ComponentFactory>)
                },
            
            _ => None
        }
    }

    pub fn build_component_args(&self, type_id: &TypeId) -> InstantiationArgs {
        let manifest = self.get_manifest();
        let property_names = manifest.get_all_property_names(type_id);
        if let None = manifest.components.get(type_id) {
            panic!("Components with type_id {} not found in manifest", type_id);
        }
        let component = manifest.components.get(type_id).unwrap();
        let factory = Self::get_component_factory(&type_id).expect(&format!("No component factory for type: {}", type_id));
        let prototypical_common_properties_factory = factory.build_default_common_properties();
        let prototypical_properties_factory = factory.build_default_properties();

        // pull handlers for this component
        let handlers = manifest.get_component_handlers(type_id);
        let handler_registry = Some(factory.build_component_handlers(handlers));

        let mut component_template = None;
        if let Some(template) = &component.template {

            let root = template.get_root();
            let mut instances = Vec::new();
            for node_id in root {
                let node = template.get_node(&node_id).unwrap();
                match node.type_id.get_pax_type(){
                    PaxType::If | PaxType::Slot | PaxType::Repeat => {
                        instances.push(self.build_control_flow(type_id, &node_id));
                    },
                    PaxType::Comment => continue,
                    _ => {  
                        instances.push(self.build_template_node(type_id, &node_id));
                    }
                }
            }
            component_template = Some(RefCell::new(instances));
        }

        InstantiationArgs {
            prototypical_common_properties_factory,
            prototypical_properties_factory,
            handler_registry,
            component_template,
            children: None,
            template_node_identifier: None,
            properties_scope_factory: Some(factory.get_properties_scope_factory()),
        }
    }

    pub fn build_control_flow(&self, containing_component_type_id: &TypeId, node_id: &TemplateNodeId) -> Rc<dyn InstanceNode> {

        let manifest = self.get_manifest();
        let prototypical_common_properties_factory = Box::new(|_,_| Rc::new(RefCell::new(CommonProperties::default())));

        let containing_component = manifest.components.get(containing_component_type_id).unwrap();
        let containing_template = containing_component.template.as_ref().unwrap();
        let tnd = containing_template.get_node(node_id).unwrap();
        let unique_identifier = UniqueTemplateNodeIdentifier::build(containing_component_type_id.clone(), node_id.clone());

        let children = self.build_children(containing_component_type_id, &node_id);
        match tnd.type_id.get_pax_type(){
            PaxType::If => {
                let expression_info = tnd
                    .control_flow_settings
                    .as_ref()
                    .unwrap()
                    .condition_expression_info
                    .as_ref()
                    .unwrap();
                let vtable_id = expression_info.vtable_id;
                let dep_symbols = expression_info.dependencies.clone();
                let prototypical_properties_factory : Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>> = Box::new(move |stack_frame, table| Rc::new(RefCell::new( {
                        let mut properties = ConditionalProperties::default();
                        let cloned_table = table.clone();
                        let cloned_stack = stack_frame.clone();

                        let mut dependencies = Vec::new();
                        for dependency in &dep_symbols {
                            if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                dependencies.push(p);
                            } else {
                                panic!("Failed to resolve symbol {}", dependency);
                            }
                        }

                        properties.boolean_expression =  Property::computed_with_name(move || {
                            let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, vtable_id);
                            let coerced = new_value_wrapped.try_coerce::<bool>().map_err(|e| format!("expr with vtable_id {} failed: {}", vtable_id, e)).unwrap();
                            coerced
                        }, &dependencies, "conditional (if) expr");
                        properties.to_pax_any()
                    })));
                ConditionalInstance::instantiate(InstantiationArgs {
                    prototypical_common_properties_factory,
                    prototypical_properties_factory,
                    handler_registry: None,
                    component_template: None,
                    children: Some(RefCell::new(children)),
                    template_node_identifier: Some(unique_identifier),
                    properties_scope_factory: None,
                })
            },
            PaxType::Slot => {
                let expression_info = tnd
                    .control_flow_settings
                    .as_ref()
                    .unwrap()
                    .slot_index_expression_info
                    .as_ref()
                    .unwrap();
                
                let vtable_id = expression_info.vtable_id;
                let dep_symbols = expression_info.dependencies.clone();

                let prototypical_properties_factory : Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>>  = Box::new(move |stack_frame, table| Rc::new(RefCell::new( {
                        let mut properties = SlotProperties::default();
                        let cloned_table = table.clone();
                        let cloned_stack = stack_frame.clone();

                        let mut dependencies = Vec::new();
                        for dependency in &dep_symbols {
                            if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                dependencies.push(p);
                            } else {
                                panic!("Failed to resolve symbol {}", dependency);
                            }
                        }
                        properties.index = Property::computed_with_name(move || {
                            let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, vtable_id);
                            let coerced = new_value_wrapped.try_coerce::<Numeric>().unwrap();
                            coerced
                        }, &dependencies, "slot index");
                        properties.to_pax_any()
                    })));
                SlotInstance::instantiate(InstantiationArgs {
                    prototypical_common_properties_factory,
                    prototypical_properties_factory,
                    handler_registry: None,
                    component_template: None,
                    children: Some(RefCell::new(children)),
                    template_node_identifier: Some(unique_identifier),
                    properties_scope_factory: None,
                })
            },
            PaxType::Repeat => {
                let rsd = tnd
                    .control_flow_settings
                    .as_ref()
                    .unwrap()
                    .repeat_source_definition
                    .clone()
                    .unwrap();
                let rpd = tnd
                    .control_flow_settings
                    .as_ref()
                    .unwrap()
                    .repeat_predicate_definition
                    .clone()
                    .unwrap();
                let expression_info = rsd.expression_info.as_ref().unwrap();
                let vtable_id = expression_info.vtable_id.clone();
                let dep_symbols = expression_info.dependencies.clone();
                let prototypical_properties_factory : Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>> = Box::new(move |stack_frame,table| Rc::new(RefCell::new( {
                        let mut properties = RepeatProperties::default();

                        let mut dependencies = Vec::new();
                        for dependency in &dep_symbols {
                            if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                dependencies.push(p);
                            } else {
                                panic!("Failed to resolve symbol {}", dependency);
                            }
                        }

                        properties.source_expression_vec = 
                            if let Some(t) = &rsd.symbolic_binding {
                                let cloned_table = table.clone();
                                let cloned_stack = stack_frame.clone();
                                Some(
                                    Property::computed_with_name(move || {
                                        let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, vtable_id);
                                        let coerced = new_value_wrapped.try_coerce::<Vec<Rc<RefCell<PaxAny>>>>().unwrap();
                                        coerced
                                    }, &dependencies, "repeat source vec")
                                    ) 
                            } else {
                                None
                            };
                            
                        properties.source_expression_range =
                            if let Some(t) = &rsd.range_expression_paxel {
                                let cloned_table = table.clone();
                                let cloned_stack = stack_frame.clone();
                                Some(
                                    Property::computed_with_name(move || {
                                        let new_value_wrapped: PaxAny = cloned_table.compute_vtable_value(&cloned_stack, vtable_id);
                                        let coerced = new_value_wrapped.try_coerce::<std::ops::Range::<isize>>().unwrap();
                                        coerced
                                    }, &dependencies, "repeat source range")
                                )
                            } else {
                                None
                            };

                        let (elem, index) = match &rpd {
                            ElemId(token) => {
                                (Some(token.raw_value.clone()), None)
                            },
                            ElemIdIndexId(t1,t2) => {
                                (Some(t1.raw_value.clone()), Some(t2.raw_value.clone()))
                            }
                        };
                        properties.iterator_i_symbol = index;
                        properties.iterator_elem_symbol = elem;
                        properties.to_pax_any()
                    })));
                RepeatInstance::instantiate(InstantiationArgs {
                    prototypical_common_properties_factory,
                    prototypical_properties_factory,
                    handler_registry: None,
                    component_template: None,
                    children: Some(RefCell::new(children)),
                    template_node_identifier: Some(unique_identifier),
                    properties_scope_factory: None
                })
            },
            _ => {  
                unreachable!("Unexpected control flow type {}", tnd.type_id)
            }
        
        }

    }

    fn build_children(&self, containing_component_type_id: &TypeId, node_id: &TemplateNodeId) -> Vec<Rc<dyn InstanceNode>> {
        let manifest = self.get_manifest();
        let containing_component = manifest.components.get(containing_component_type_id).unwrap();
        let containing_template = containing_component.template.as_ref().unwrap();
        let children = containing_template.get_children(node_id);

        let mut children_instances = Vec::new();
        for child_id in &children.unwrap_or_default() {
            let child = containing_template.get_node(&child_id).unwrap();
            match child.type_id.get_pax_type() {
                PaxType::If | PaxType::Slot | PaxType::Repeat  => {
                    children_instances.push(self.build_control_flow(containing_component_type_id, &child_id));
                },
                PaxType::Comment => continue,
                _ => {  
                    children_instances.push(self.build_template_node(containing_component_type_id, child_id));
                }
            }
        }
        children_instances
    }

    pub fn build_template_node(&self, containing_component_type_id: &TypeId, node_id: &TemplateNodeId) -> Rc<dyn InstanceNode> {
        let manifest = self.get_manifest();

        let containing_component = manifest.components.get(containing_component_type_id).unwrap();
        let containing_template = containing_component.template.as_ref().unwrap();
        let node = containing_template.get_node(node_id).unwrap();
        let containing_component_factory = Self::get_component_factory(containing_component_type_id).unwrap();

        let mut args = self.build_component_args(&node.type_id);
        let node_component_factory = Self::get_component_factory(&node.type_id).unwrap();

        // update handlers from tnd
        let handlers_from_tnd = manifest.get_inline_event_handlers(node);
        let updated_registry = if let Some(registry) = args.handler_registry {
            containing_component_factory.add_inline_handlers(handlers_from_tnd, registry)    
        } else {
            containing_component_factory.add_inline_handlers(handlers_from_tnd, Rc::new(RefCell::new(HandlerRegistry::default())))       
        };

        args.handler_registry = Some(updated_registry);

        // update properties from tnd 
        let inline_properties = manifest.get_inline_properties(containing_component_type_id, node);
        let updated_properties = node_component_factory.build_inline_properties(inline_properties.clone());
        args.prototypical_properties_factory = updated_properties;

        // update common properties from tnd
        let updated_common_properties = node_component_factory.build_inline_common_properties(inline_properties);
        args.prototypical_common_properties_factory = updated_common_properties;

       
        args.children = Some(RefCell::new(self.build_children(containing_component_type_id, node_id)));
        args.template_node_identifier = Some(UniqueTemplateNodeIdentifier::build(containing_component_type_id.clone(), node_id.clone()));

        node_component_factory.build_component(args)
    }


    pub fn get_template_node_by_id(&self, id: &str) -> Option<Rc<dyn InstanceNode>> {
        let manifest = self.get_manifest();
        let main_component_type_id = manifest.main_component_type_id.clone();
        let main_component = manifest.components.get(&main_component_type_id).unwrap();
        let template = main_component.template.as_ref().unwrap();
        for node_id in template.get_ids() {
            if let Some(found) = self.recurse_get_template_node_by_id(id, &main_component_type_id, node_id) {
                return Some(self.build_template_node(&found.0, &found.1))
            }
        }
        None
    }

    fn check_for_id_in_template_node(&self, id: &str, tnd: &TemplateNodeDefinition) -> bool {
        if let Some(settings) = &tnd.settings {
            for setting in settings {
                if let SettingElement::Setting(token, value) = setting {
                    if &token.raw_value == "id" {
                        if let ValueDefinition::LiteralValue(lv) = value {
                            if lv.raw_value == id {
                                return true;
                            }
                        }
                    
                    }
                }
            }
        }
        false
    }

    fn recurse_get_template_node_by_id<'a>(&'a self, id: &str, containing_component_type_id: &'a TypeId, node_id: &'a TemplateNodeId) -> Option<(TypeId, TemplateNodeId)>{
        let manifest = self.get_manifest();
        let containing_component = manifest.components.get(containing_component_type_id).unwrap();
        let containing_template = containing_component.template.as_ref().unwrap();
        let tnd = containing_template.get_node(node_id).unwrap();

        if self.check_for_id_in_template_node(id, tnd) {
            return Some((containing_component_type_id.clone(), node_id.clone()));
        }

        if let Some(component) = &manifest.components.get(&tnd.type_id){
            if let Some(template) = &component.template {
                for node_id in template.get_ids() {
                    if let Some(found) = self.recurse_get_template_node_by_id(id, &tnd.type_id, node_id) {
                        return Some(found.clone());
                    }
                }
            }
        }
        None
    }
}
