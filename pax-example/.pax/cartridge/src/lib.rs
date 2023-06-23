
//Prelude: Rust
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::ops::Deref;
use std::rc::Rc;
//Prelude: Pax
use pax_runtime_api::{PropertyInstance, PropertyLiteral, Size2D, Transform2D};
use pax_core::{ComponentInstance, RenderNodePtr, PropertyExpression, RenderNodePtrList, RenderTreeContext, ExpressionContext, PaxEngine, RenderNode, InstanceRegistry, HandlerRegistry, InstantiationArgs, ConditionalInstance, SlotInstance, StackFrame};
use pax_core::pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use pax_core::repeat::{RepeatInstance};
use piet_common::RenderContext;

// generate imports, pointing to userland cartridge `pub mod pax_reexports`

use pax_example::pax_reexports::pax_std::stacker::Stacker;

use pax_example::pax_reexports::std::vec::Vec;

use pax_example::pax_reexports::hello_rgb::HelloRGB;

use pax_example::pax_reexports::pax_std::primitives::Group;

use pax_example::pax_reexports::pax::api::SizePixels;

use pax_example::pax_reexports::pax_std::types::Color;

use pax_example::pax_reexports::pax_std::types::ColorVariant;

use pax_example::pax_reexports::grids::Grids;

use pax_example::pax_reexports::pax_std::types::StackerCell;

use pax_example::pax_reexports::f64;

use pax_example::pax_reexports::pax_std::primitives::Frame;

use pax_example::pax_reexports::camera::Camera;

use pax_example::pax_reexports::std::option::Option;

use pax_example::pax_reexports::pax::api::Numeric;

use pax_example::pax_reexports::camera::TypeExample;

use pax_example::pax_reexports::grids::RectDef;

use pax_example::pax_reexports::Example;

use pax_example::pax_reexports::pax::api::Size;

use pax_example::pax_reexports::pax_std::types::StackerDirection;

use pax_example::pax_reexports::pax_std::primitives::Rectangle;

use pax_example::pax_reexports::pax_std::types::Stroke;

use pax_example::pax_reexports::fireworks::Fireworks;

use pax_example::pax_reexports::pax_std::primitives::Ellipse;

use pax_example::pax_reexports::usize;


//pull in entire const token stream here e.g. `const JABBERWOCKY : &str = r#"’Twas brillig, and the slithy toves `...


pub fn instantiate_expression_table<R: 'static + RenderContext>() -> HashMap<usize, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct>> {
    let mut vtable: HashMap<usize, Box<dyn Fn(ExpressionContext<R>) -> TypesCoproduct>> = HashMap::new();

    
    //0..60
    vtable.insert(0, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        

        #[allow(unused_parens)]
        TypesCoproduct::stdCOCOopsCOCORangeLABRisizeRABR(
            0 ..60
        )
    }));
    
    //Transform2D::anchor(50%,50%)*Transform2D::align(50%,50%)*Transform2D::rotate((i+2)*rotation+ticks/1000.0)*Transform2D::scale(0.75+(i*rotation),0.75+(i*rotation))*Transform2D::scale(1-((rotation/5)+i/1000.0),1-((rotation/5)+i/1000.0))
    vtable.insert(1, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let i =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();
                
                    //binding repeat elem
                    if let PropertiesCoproduct::RepeatItem(elem, i) = properties {
                        
                            //iterable numeric
                            
                            let unwrapped = if let PropertiesCoproduct::isize(i) = **elem {i} else {unreachable!()};
                            Numeric::from(unwrapped)
                        
                    } else {unreachable!()}
                
            };
            
                let i = Numeric::from( i );
            

        
            let rotation =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(1) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();
                
                    if let PropertiesCoproduct::crateCOCOfireworksCOCOFireworks(p) = properties {
                        
                            //binding simple numeric property
                            Numeric::from(p.rotation.get())
                        
                    } else {unreachable!()}
                
            };
            
                let rotation = Numeric::from( rotation );
            

        
            let ticks =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(1) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();
                
                    if let PropertiesCoproduct::crateCOCOfireworksCOCOFireworks(p) = properties {
                        
                            //binding simple numeric property
                            Numeric::from(p.ticks.get())
                        
                    } else {unreachable!()}
                
            };
            
                let ticks = Numeric::from( ticks );
            

        

        

        #[allow(unused_parens)]
        TypesCoproduct::Transform2D(
            ((((Transform2D::anchor((Size::Percent(50.into())),(Size::Percent(50.into())),)*(Transform2D::align((Size::Percent(50.into())),(Size::Percent(50.into())),)).into())*(Transform2D::rotate(((((i +Numeric::from(2))*(rotation ).into())+(ticks /Numeric::from(1000.0)))),)).into())*(Transform2D::scale(((Numeric::from(0.75)+(i *(rotation).into()))),((Numeric::from(0.75)+(i *(rotation).into()))),)).into())*(Transform2D::scale(((Numeric::from(1 )-((rotation /Numeric::from(5))+(i /Numeric::from(1000.0))))),((Numeric::from(1 )-((rotation /Numeric::from(5))+(i /Numeric::from(1000.0))))),)).into())
        )
    }));
    
    //Color::hlc(ticks+i*360.0/30.0,75.0,150.0)
    vtable.insert(2, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let i =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();
                
                    //binding repeat elem
                    if let PropertiesCoproduct::RepeatItem(elem, i) = properties {
                        
                            //iterable numeric
                            
                            let unwrapped = if let PropertiesCoproduct::isize(i) = **elem {i} else {unreachable!()};
                            Numeric::from(unwrapped)
                        
                    } else {unreachable!()}
                
            };
            
                let i = Numeric::from( i );
            

        
            let ticks =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(1) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();
                
                    if let PropertiesCoproduct::crateCOCOfireworksCOCOFireworks(p) = properties {
                        
                            //binding simple numeric property
                            Numeric::from(p.ticks.get())
                        
                    } else {unreachable!()}
                
            };
            
                let ticks = Numeric::from( ticks );
            

        

        

        #[allow(unused_parens)]
        TypesCoproduct::pax_stdCOCOtypesCOCOColor(
            Color::hlc(((ticks +((i *(Numeric::from(360.0 )).into())/Numeric::from(30.0)))),(Numeric::from(75.0)),(Numeric::from(150.0)),)
        )
    }));
    
    //0..10
    vtable.insert(3, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        

        #[allow(unused_parens)]
        TypesCoproduct::stdCOCOopsCOCORangeLABRisizeRABR(
            0 ..10
        )
    }));
    
    //Color::rgb(0.0,1.0/(10.0-i),0.5)
    vtable.insert(4, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let i =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();
                
                    //binding repeat elem
                    if let PropertiesCoproduct::RepeatItem(elem, i) = properties {
                        
                            //iterable numeric
                            
                            let unwrapped = if let PropertiesCoproduct::isize(i) = **elem {i} else {unreachable!()};
                            Numeric::from(unwrapped)
                        
                    } else {unreachable!()}
                
            };
            
                let i = Numeric::from( i );
            

        

        

        #[allow(unused_parens)]
        TypesCoproduct::pax_stdCOCOtypesCOCOColor(
            Color::rgb((Numeric::from(0.0)),((Numeric::from(1.0 )/(Numeric::from(10.0 )-i))),(Numeric::from(0.5)),)
        )
    }));
    
    //0..4
    vtable.insert(5, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        

        #[allow(unused_parens)]
        TypesCoproduct::stdCOCOopsCOCORangeLABRisizeRABR(
            0 ..4

        )
    }));
    
    //Color::rgb(1.0/(5.0-i),0.0,0.5)
    vtable.insert(6, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let i =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();
                
                    //binding repeat elem
                    if let PropertiesCoproduct::RepeatItem(elem, i) = properties {
                        
                            //iterable numeric
                            
                            let unwrapped = if let PropertiesCoproduct::isize(i) = **elem {i} else {unreachable!()};
                            Numeric::from(unwrapped)
                        
                    } else {unreachable!()}
                
            };
            
                let i = Numeric::from( i );
            

        

        

        #[allow(unused_parens)]
        TypesCoproduct::pax_stdCOCOtypesCOCOColor(
            Color::rgb(((Numeric::from(1.0 )/(Numeric::from(5.0 )-i))),(Numeric::from(0.0)),(Numeric::from(0.5)),)
        )
    }));
    
    //current_route==0
    vtable.insert(7, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let current_route =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();
                
                    if let PropertiesCoproduct::crateCOCOExample(p) = properties {
                        
                            //binding simple numeric property
                            Numeric::from(p.current_route.get())
                        
                    } else {unreachable!()}
                
            };
            
                let current_route = Numeric::from( current_route );
            

        

        

        #[allow(unused_parens)]
        TypesCoproduct::bool(
            (current_route ==Numeric::from(0 ))
        )
    }));
    
    //current_route==1
    vtable.insert(8, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let current_route =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();
                
                    if let PropertiesCoproduct::crateCOCOExample(p) = properties {
                        
                            //binding simple numeric property
                            Numeric::from(p.current_route.get())
                        
                    } else {unreachable!()}
                
            };
            
                let current_route = Numeric::from( current_route );
            

        

        

        #[allow(unused_parens)]
        TypesCoproduct::bool(
            (current_route ==Numeric::from(1 ))
        )
    }));
    
    //current_route==2
    vtable.insert(9, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let current_route =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();
                
                    if let PropertiesCoproduct::crateCOCOExample(p) = properties {
                        
                            //binding simple numeric property
                            Numeric::from(p.current_route.get())
                        
                    } else {unreachable!()}
                
            };
            
                let current_route = Numeric::from( current_route );
            

        

        

        #[allow(unused_parens)]
        TypesCoproduct::bool(
            (current_route
==Numeric::from(2 ))
        )
    }));
    
    //current_route==3
    vtable.insert(10, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let current_route =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();
                
                    if let PropertiesCoproduct::crateCOCOExample(p) = properties {
                        
                            //binding simple numeric property
                            Numeric::from(p.current_route.get())
                        
                    } else {unreachable!()}
                
            };
            
                let current_route = Numeric::from( current_route );
            

        

        

        #[allow(unused_parens)]
        TypesCoproduct::bool(
            (current_route ==Numeric::from(3 ))
        )
    }));
    
    //_cell_specs
    vtable.insert(11, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let _cell_specs =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();
                
                    if let PropertiesCoproduct::pax_stdCOCOstackerCOCOStacker(p) = properties {
                        
                            //binding cloneable property
                            p._cell_specs.get().clone()
                        
                    } else {unreachable!()}
                
            };
            

        

        
            
            
                let _cell_specs = _cell_specs.iter().map(|t|{
                    Rc::new(PropertiesCoproduct::pax_stdCOCOtypesCOCOStackerCell(t.clone()))
                }).collect::<Vec<Rc<PropertiesCoproduct>>>();
            
        

        #[allow(unused_parens)]
        TypesCoproduct::stdCOCOvecCOCOVecLABRstdCOCOrcCOCORcLABRPropertiesCoproductRABRRABR(
            _cell_specs
        )
    }));
    
    //Transform2D::translate(cell_spec.x_px,cell_spec.y_px)
    vtable.insert(12, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let cell_specPERIx_px =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();
                
                    //binding repeat elem
                    if let PropertiesCoproduct::RepeatItem(elem, i) = properties {
                        
                            //iterable complex type
                            if let PropertiesCoproduct::pax_stdCOCOtypesCOCOStackerCell(ict) = elem.deref() {
                                ict.clone()
                            } else {unreachable!()}
                        
                    } else {unreachable!()}
                
            }.x_px.clone();
            
                let cell_specPERIx_px = Numeric::from( cell_specPERIx_px );
            

        
            let cell_specPERIy_px =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();
                
                    //binding repeat elem
                    if let PropertiesCoproduct::RepeatItem(elem, i) = properties {
                        
                            //iterable complex type
                            if let PropertiesCoproduct::pax_stdCOCOtypesCOCOStackerCell(ict) = elem.deref() {
                                ict.clone()
                            } else {unreachable!()}
                        
                    } else {unreachable!()}
                
            }.y_px.clone();
            
                let cell_specPERIy_px = Numeric::from( cell_specPERIy_px );
            

        

        

        #[allow(unused_parens)]
        TypesCoproduct::Transform2D(
            Transform2D::translate((cell_specPERIx_px),(cell_specPERIy_px),)
        )
    }));
    
    //(cell_spec.width_px)px
    vtable.insert(13, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let cell_specPERIwidth_px =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();
                
                    //binding repeat elem
                    if let PropertiesCoproduct::RepeatItem(elem, i) = properties {
                        
                            //iterable complex type
                            if let PropertiesCoproduct::pax_stdCOCOtypesCOCOStackerCell(ict) = elem.deref() {
                                ict.clone()
                            } else {unreachable!()}
                        
                    } else {unreachable!()}
                
            }.width_px.clone();
            
                let cell_specPERIwidth_px = Numeric::from( cell_specPERIwidth_px );
            

        

        

        #[allow(unused_parens)]
        TypesCoproduct::Size(
            Size::Pixels(cell_specPERIwidth_px.into())
        )
    }));
    
    //(cell_spec.height_px)px
    vtable.insert(14, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let cell_specPERIheight_px =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();
                
                    //binding repeat elem
                    if let PropertiesCoproduct::RepeatItem(elem, i) = properties {
                        
                            //iterable complex type
                            if let PropertiesCoproduct::pax_stdCOCOtypesCOCOStackerCell(ict) = elem.deref() {
                                ict.clone()
                            } else {unreachable!()}
                        
                    } else {unreachable!()}
                
            }.height_px.clone();
            
                let cell_specPERIheight_px = Numeric::from( cell_specPERIheight_px );
            

        

        

        #[allow(unused_parens)]
        TypesCoproduct::Size(
            Size::Pixels(cell_specPERIheight_px.into())
        )
    }));
    
    //(i)
    vtable.insert(15, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let i =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();
                
                    //binding repeat i
                    if let PropertiesCoproduct::RepeatItem(_, i) = properties {
                        Numeric::from(*i)
                    } else {unreachable!()}
                
            };
            
                let i = Numeric::from( i );
            

        

        

        #[allow(unused_parens)]
        TypesCoproduct::Numeric(
            i
        )
    }));
    
    //Transform2D::scale(zoom,zoom)*Transform2D::translate(pan_x,pan_y)
    vtable.insert(16, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let pan_x =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();
                
                    if let PropertiesCoproduct::crateCOCOcameraCOCOCamera(p) = properties {
                        
                            //binding simple numeric property
                            Numeric::from(p.pan_x.get())
                        
                    } else {unreachable!()}
                
            };
            
                let pan_x = Numeric::from( pan_x );
            

        
            let pan_y =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();
                
                    if let PropertiesCoproduct::crateCOCOcameraCOCOCamera(p) = properties {
                        
                            //binding simple numeric property
                            Numeric::from(p.pan_y.get())
                        
                    } else {unreachable!()}
                
            };
            
                let pan_y = Numeric::from( pan_y );
            

        
            let zoom =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();
                
                    if let PropertiesCoproduct::crateCOCOcameraCOCOCamera(p) = properties {
                        
                            //binding simple numeric property
                            Numeric::from(p.zoom.get())
                        
                    } else {unreachable!()}
                
            };
            
                let zoom = Numeric::from( zoom );
            

        

        

        #[allow(unused_parens)]
        TypesCoproduct::Transform2D(
            (Transform2D::scale((zoom),(zoom),)*(Transform2D::translate((pan_x),(pan_y),)).into())
        )
    }));
    
    //Transform2D::translate(0,0)
    vtable.insert(17, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        

        #[allow(unused_parens)]
        TypesCoproduct::Transform2D(
            Transform2D::translate((Numeric::from(0)),(Numeric::from(0)),)
        )
    }));
    
    //Color::rgb(100.0,0,0)
    vtable.insert(18, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        

        #[allow(unused_parens)]
        TypesCoproduct::pax_stdCOCOtypesCOCOColor(
            Color::rgb((Numeric::from(100.0)),(Numeric::from(0)),(Numeric::from(0)),)
        )
    }));
    
    //Transform2D::translate(0,200)
    vtable.insert(19, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        

        #[allow(unused_parens)]
        TypesCoproduct::Transform2D(
            Transform2D::translate((Numeric::from(0)),(Numeric::from(200)),)
        )
    }));
    
    //Color::rgb(0,100.0,0)
    vtable.insert(20, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        

        #[allow(unused_parens)]
        TypesCoproduct::pax_stdCOCOtypesCOCOColor(
            Color::rgb((Numeric::from(0)),(Numeric::from(100.0)),(Numeric::from(0)),)
        )
    }));
    
    //Transform2D::translate(200,0)
    vtable.insert(21, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        

        #[allow(unused_parens)]
        TypesCoproduct::Transform2D(
            Transform2D::translate((Numeric::from(200)),(Numeric::from(0)),)
        )
    }));
    
    //Color::rgb(0,0,100.0)
    vtable.insert(22, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        

        #[allow(unused_parens)]
        TypesCoproduct::pax_stdCOCOtypesCOCOColor(
            Color::rgb((Numeric::from(0)),(Numeric::from(0)),(Numeric::from(100.0)),)
        )
    }));
    
    //Transform2D::translate(200,200)
    vtable.insert(23, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        

        #[allow(unused_parens)]
        TypesCoproduct::Transform2D(
            Transform2D::translate((Numeric::from(200)),(Numeric::from(200)),)
        )
    }));
    
    //Color::rgb(0,50.0,50.0)
    vtable.insert(24, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        

        #[allow(unused_parens)]
        TypesCoproduct::pax_stdCOCOtypesCOCOColor(
            Color::rgb((Numeric::from(0)),(Numeric::from(50.0)),(Numeric::from(50.0)),)
        )
    }));
    
    //Transform2D::align(50%,50%)*Transform2D::anchor(50%,50%)*Transform2D::rotate(rotation)
    vtable.insert(25, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        
            let rotation =
            {
                let properties = if let Some(sf) = (*ec.stack_frame).borrow().peek_nth(0) {
                    Rc::clone(&sf)
                } else {
                    Rc::clone(&ec.stack_frame)
                }.borrow().deref().get_properties();
                let properties = &*(*properties).borrow();
                
                    if let PropertiesCoproduct::crateCOCOhello_rgbCOCOHelloRGB(p) = properties {
                        
                            //binding simple numeric property
                            Numeric::from(p.rotation.get())
                        
                    } else {unreachable!()}
                
            };
            
                let rotation = Numeric::from( rotation );
            

        

        

        #[allow(unused_parens)]
        TypesCoproduct::Transform2D(
            ((Transform2D::align((Size::Percent(50.into())),(Size::Percent(50.into())),)*(Transform2D::anchor((Size::Percent(50.into())),(Size::Percent(50.into())),)).into())*(Transform2D::rotate((rotation),)).into())
        )
    }));
    
    //Color::rgb(0.4,0.5,0)
    vtable.insert(26, Box::new(|ec: ExpressionContext<R>| -> TypesCoproduct {
        

        

        #[allow(unused_parens)]
        TypesCoproduct::pax_stdCOCOtypesCOCOColor(
            Color::rgb((Numeric::from(0.4)),(Numeric::from(0.5)),(Numeric::from(0)),)
        )
    }));
    

    vtable
}

//Begin component factory literals

    
pub fn instantiate_crate_fireworks_Fireworks<R: 'static + RenderContext>(instance_registry: Rc<RefCell<InstanceRegistry<R>>>, mut args: InstantiationArgs<R>) -> Rc<RefCell<ComponentInstance<R>>>  {
    args.component_template = Some(Rc::new(RefCell::new(vec![

pax_std_primitives::group::GroupInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::pax_stdCOCOprimitivesCOCOGroup
    
        ( Group {
            
        })
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![|stack_frame, ctx, args|{
                         let properties = (*stack_frame).borrow().get_properties();
                         let properties = &mut *properties.as_ref().borrow_mut();
                         let properties = if let PropertiesCoproduct::crateCOCOfireworksCOCOFireworks(p) = properties {p} else {unreachable!()};
                         Fireworks::handle_scroll(properties,ctx,args);
                     },],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

RepeatInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::None
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

pax_std_primitives::rectangle::RectangleInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::pax_stdCOCOprimitivesCOCORectangle
    
        ( Rectangle {
            
                stroke: Box::new( PropertyLiteral::new(Default::default()) ),
            
                fill: Box::new( PropertyExpression::new(2) ),
            
        })
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyExpression::new(1))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Pixels(300.into()))),Box::new(PropertyLiteral::new(Size::Pixels(300.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: Some(Box::new(PropertyExpression::new(0))),
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
])));

    args.handler_registry = Some(Rc::new(RefCell::new(HandlerRegistry {
        click_handlers: vec![],
        will_render_handlers: vec![
                 |properties, ctx|{
                     let properties = &mut *properties.as_ref().borrow_mut();
                     let properties = if let PropertiesCoproduct::crateCOCOfireworksCOCOFireworks(p) = properties {p} else {unreachable!()};
                     Fireworks::handle_will_render(properties,ctx);
                 },
             ],
        did_mount_handlers: vec![],
        scroll_handlers: vec![],
    })));

    args.compute_properties_fn = Some(Box::new(|properties, rtc|{
        let properties = &mut *properties.as_ref().borrow_mut();
        let properties = if let PropertiesCoproduct::crateCOCOfireworksCOCOFireworks(p) = properties {p} else {unreachable!()};

        
            if let Some(new_value) = rtc.compute_eased_value(properties.rotation._get_transition_manager()) {
            properties.rotation.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.rotation._get_vtable_id()) {
            let new_value = if let TypesCoproduct::f64(v) = new_value { v } else { unreachable!() };
            properties.rotation.set(new_value);
            }
        
            if let Some(new_value) = rtc.compute_eased_value(properties.ticks._get_transition_manager()) {
            properties.ticks.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.ticks._get_vtable_id()) {
            let new_value = if let TypesCoproduct::usize(v) = new_value { v } else { unreachable!() };
            properties.ticks.set(new_value);
            }
        
    }));

    ComponentInstance::instantiate(args)
}




    
pub fn instantiate_crate_grids_Grids<R: 'static + RenderContext>(instance_registry: Rc<RefCell<InstanceRegistry<R>>>, mut args: InstantiationArgs<R>) -> Rc<RefCell<ComponentInstance<R>>>  {
    args.component_template = Some(Rc::new(RefCell::new(vec![

instantiate_pax_std_stacker_Stacker( Rc::clone(&instance_registry),
 InstantiationArgs {
    properties: PropertiesCoproduct::pax_stdCOCOstackerCOCOStacker
    
        ( Stacker {
            
                cells: Box::new( PropertyLiteral::new(Numeric::from(5 )) ),
            
                direction: Box::new( PropertyLiteral::new(Default::default()) ),
            
                _cell_specs: Box::new( PropertyLiteral::new(Default::default()) ),
            
                gutter_width: Box::new( PropertyLiteral::new(Size::Pixels(0.into())) ),
            
                sizes: Box::new( PropertyLiteral::new(Default::default()) ),
            
        })
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Percent(100.into()))),Box::new(PropertyLiteral::new(Size::Percent(100.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

instantiate_pax_std_stacker_Stacker( Rc::clone(&instance_registry),
 InstantiationArgs {
    properties: PropertiesCoproduct::pax_stdCOCOstackerCOCOStacker
    
        ( Stacker {
            
                cells: Box::new( PropertyLiteral::new(Numeric::from(10)) ),
            
                direction: Box::new( PropertyLiteral::new(StackerDirection
:: Vertical .try_into().unwrap()) ),
            
                _cell_specs: Box::new( PropertyLiteral::new(Default::default()) ),
            
                gutter_width: Box::new( PropertyLiteral::new(Size::Pixels(0.into())) ),
            
                sizes: Box::new( PropertyLiteral::new(Default::default()) ),
            
        })
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Percent(100.into()))),Box::new(PropertyLiteral::new(Size::Percent(100.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

RepeatInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::None
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

pax_std_primitives::rectangle::RectangleInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::pax_stdCOCOprimitivesCOCORectangle
    
        ( Rectangle {
            
                stroke: Box::new( PropertyLiteral::new(Default::default()) ),
            
                fill: Box::new( PropertyExpression::new(4) ),
            
        })
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Percent(100.into()))),Box::new(PropertyLiteral::new(Size::Percent(100.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: Some(Box::new(PropertyExpression::new(3))),
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
            

RepeatInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::None
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

pax_std_primitives::rectangle::RectangleInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::pax_stdCOCOprimitivesCOCORectangle
    
        ( Rectangle {
            
                stroke: Box::new( PropertyLiteral::new(Default::default()) ),
            
                fill: Box::new( PropertyExpression::new(6) ),
            
        })
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Percent(100.into()))),Box::new(PropertyLiteral::new(Size::Percent(100.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: Some(Box::new(PropertyExpression::new(5))),
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
])));

    args.handler_registry = Some(Rc::new(RefCell::new(HandlerRegistry {
        click_handlers: vec![],
        will_render_handlers: vec![],
        did_mount_handlers: vec![],
        scroll_handlers: vec![],
    })));

    args.compute_properties_fn = Some(Box::new(|properties, rtc|{
        let properties = &mut *properties.as_ref().borrow_mut();
        let properties = if let PropertiesCoproduct::crateCOCOgridsCOCOGrids(p) = properties {p} else {unreachable!()};

        
            if let Some(new_value) = rtc.compute_eased_value(properties.ticks._get_transition_manager()) {
            properties.ticks.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.ticks._get_vtable_id()) {
            let new_value = if let TypesCoproduct::usize(v) = new_value { v } else { unreachable!() };
            properties.ticks.set(new_value);
            }
        
            if let Some(new_value) = rtc.compute_eased_value(properties.rects._get_transition_manager()) {
            properties.rects.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.rects._get_vtable_id()) {
            let new_value = if let TypesCoproduct::stdCOCOvecCOCOVecLABRcrateCOCOgridsCOCORectDefRABR(v) = new_value { v } else { unreachable!() };
            properties.rects.set(new_value);
            }
        
    }));

    ComponentInstance::instantiate(args)
}




    
pub fn instantiate_main_component<R: 'static + RenderContext>(instance_registry: Rc<RefCell<InstanceRegistry<R>>>) -> Rc<RefCell<ComponentInstance<R>>> {
    ComponentInstance::instantiate( InstantiationArgs{
        properties: PropertiesCoproduct::crateCOCOExample( Example::default() ),
        handler_registry:  Some(Rc::new(RefCell::new(
             HandlerRegistry {
                 click_handlers: vec![],
                 will_render_handlers: vec![],
                 did_mount_handlers: vec![],
                 scroll_handlers: vec![],
             }
         ))),
        instance_registry: Rc::clone(&instance_registry),
        transform: Transform2D::default_wrapped(),
        size: None,
        children: None,
        component_template: Some(Rc::new(RefCell::new(vec![

pax_std_primitives::frame::FrameInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::pax_stdCOCOprimitivesCOCOFrame
    
        ( Frame {
            
        })
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![|stack_frame, ctx, args|{
                        let properties = (*stack_frame).borrow().get_properties();
                        let properties = &mut *properties.as_ref().borrow_mut();
                        let properties = if let PropertiesCoproduct::crateCOCOExample(p) = properties {p} else {unreachable!()};
                        Example::modulate(properties, ctx, args);
                    },],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Percent(100.into()))),Box::new(PropertyLiteral::new(Size::Percent(100.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

ConditionalInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::None
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

instantiate_crate_grids_Grids( Rc::clone(&instance_registry),
 InstantiationArgs {
    properties: PropertiesCoproduct::crateCOCOgridsCOCOGrids
    
        ( Grids {
            
                ticks: Box::new( PropertyLiteral::new(Default::default()) ),
            
                rects: Box::new( PropertyLiteral::new(Default::default()) ),
            
        })
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: Some(Box::new(PropertyExpression::new(7))),
    compute_properties_fn: None,
})
,
        
            

ConditionalInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::None
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

instantiate_crate_fireworks_Fireworks( Rc::clone(&instance_registry),
 InstantiationArgs {
    properties: PropertiesCoproduct::crateCOCOfireworksCOCOFireworks
    
        ( Fireworks {
            
                rotation: Box::new( PropertyLiteral::new(Default::default()) ),
            
                ticks: Box::new( PropertyLiteral::new(Default::default()) ),
            
        })
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: Some(Box::new(PropertyExpression::new(8))),
    compute_properties_fn: None,
})
,
        
            

ConditionalInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::None
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

instantiate_crate_hello_rgb_HelloRGB( Rc::clone(&instance_registry),
 InstantiationArgs {
    properties: PropertiesCoproduct::crateCOCOhello_rgbCOCOHelloRGB
    
        ( HelloRGB {
            
                rotation: Box::new( PropertyLiteral::new(Default::default()) ),
            
        })
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: Some(Box::new(PropertyExpression::new(9))),
    compute_properties_fn: None,
})
,
        
            

ConditionalInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::None
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

instantiate_crate_camera_Camera( Rc::clone(&instance_registry),
 InstantiationArgs {
    properties: PropertiesCoproduct::crateCOCOcameraCOCOCamera
    
        ( Camera {
            
                ticks: Box::new( PropertyLiteral::new(Default::default()) ),
            
                zoom: Box::new( PropertyLiteral::new(Default::default()) ),
            
                pan_x: Box::new( PropertyLiteral::new(Default::default()) ),
            
                pan_y: Box::new( PropertyLiteral::new(Default::default()) ),
            
                type_example: Box::new( PropertyLiteral::new(Default::default()) ),
            
        })
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: Some(Box::new(PropertyExpression::new(10))),
    compute_properties_fn: None,
})
,
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
]))),
        scroller_args: None,
        slot_index: None,
        repeat_source_expression_vec: None,
        repeat_source_expression_range: None,
        conditional_boolean_expression: None,
        compute_properties_fn: Some(Box::new(|properties, rtc|{
            let properties = &mut *properties.as_ref().borrow_mut();
            let properties = if let PropertiesCoproduct::crateCOCOExample(p) = properties {p} else {unreachable!()};

            
            if let Some(new_value) = rtc.compute_eased_value(properties.current_route._get_transition_manager()) {
            properties.current_route.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.current_route._get_vtable_id()) {
            let new_value = if let TypesCoproduct::usize(v) = new_value { v } else { unreachable!() };
            properties.current_route.set(new_value);
            }
            
        })),
    })
}





    
pub fn instantiate_pax_std_stacker_Stacker<R: 'static + RenderContext>(instance_registry: Rc<RefCell<InstanceRegistry<R>>>, mut args: InstantiationArgs<R>) -> Rc<RefCell<ComponentInstance<R>>>  {
    args.component_template = Some(Rc::new(RefCell::new(vec![

RepeatInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::None
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

pax_std_primitives::frame::FrameInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::pax_stdCOCOprimitivesCOCOFrame
    
        ( Frame {
            
        })
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyExpression::new(12))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyExpression::new(13)),Box::new(PropertyExpression::new(14))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

SlotInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::None
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: Some(Box::new(PropertyExpression::new(15))),
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: Some(Box::new(PropertyExpression::new(11))),
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
])));

    args.handler_registry = Some(Rc::new(RefCell::new(HandlerRegistry {
        click_handlers: vec![],
        will_render_handlers: vec![
                 |properties, ctx|{
                     let properties = &mut *properties.as_ref().borrow_mut();
                     let properties = if let PropertiesCoproduct::pax_stdCOCOstackerCOCOStacker(p) = properties {p} else {unreachable!()};
                     Stacker::handle_will_render(properties,ctx);
                 },
             ],
        did_mount_handlers: vec![],
        scroll_handlers: vec![],
    })));

    args.compute_properties_fn = Some(Box::new(|properties, rtc|{
        let properties = &mut *properties.as_ref().borrow_mut();
        let properties = if let PropertiesCoproduct::pax_stdCOCOstackerCOCOStacker(p) = properties {p} else {unreachable!()};

        
            if let Some(new_value) = rtc.compute_eased_value(properties.cells._get_transition_manager()) {
            properties.cells.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.cells._get_vtable_id()) {
            let new_value = if let TypesCoproduct::paxCOCOapiCOCONumeric(v) = new_value { v } else { unreachable!() };
            properties.cells.set(new_value);
            }
        
            if let Some(new_value) = rtc.compute_eased_value(properties.direction._get_transition_manager()) {
            properties.direction.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.direction._get_vtable_id()) {
            let new_value = if let TypesCoproduct::pax_stdCOCOtypesCOCOStackerDirection(v) = new_value { v } else { unreachable!() };
            properties.direction.set(new_value);
            }
        
            if let Some(new_value) = rtc.compute_eased_value(properties._cell_specs._get_transition_manager()) {
            properties._cell_specs.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties._cell_specs._get_vtable_id()) {
            let new_value = if let TypesCoproduct::stdCOCOvecCOCOVecLABRpax_stdCOCOtypesCOCOStackerCellRABR(v) = new_value { v } else { unreachable!() };
            properties._cell_specs.set(new_value);
            }
        
            if let Some(new_value) = rtc.compute_eased_value(properties.gutter_width._get_transition_manager()) {
            properties.gutter_width.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.gutter_width._get_vtable_id()) {
            let new_value = if let TypesCoproduct::paxCOCOapiCOCOSize(v) = new_value { v } else { unreachable!() };
            properties.gutter_width.set(new_value);
            }
        
            if let Some(new_value) = rtc.compute_eased_value(properties.sizes._get_transition_manager()) {
            properties.sizes.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.sizes._get_vtable_id()) {
            let new_value = if let TypesCoproduct::stdCOCOvecCOCOVecLABRstdCOCOoptionCOCOOptionLABRpaxCOCOapiCOCOSizeRABRRABR(v) = new_value { v } else { unreachable!() };
            properties.sizes.set(new_value);
            }
        
    }));

    ComponentInstance::instantiate(args)
}




    
pub fn instantiate_crate_camera_Camera<R: 'static + RenderContext>(instance_registry: Rc<RefCell<InstanceRegistry<R>>>, mut args: InstantiationArgs<R>) -> Rc<RefCell<ComponentInstance<R>>>  {
    args.component_template = Some(Rc::new(RefCell::new(vec![

pax_std_primitives::frame::FrameInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::pax_stdCOCOprimitivesCOCOFrame
    
        ( Frame {
            
        })
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![|stack_frame, ctx, args|{
                        let properties = (*stack_frame).borrow().get_properties();
                        let properties = &mut *properties.as_ref().borrow_mut();
                        let properties = if let PropertiesCoproduct::crateCOCOcameraCOCOCamera(p) = properties {p} else {unreachable!()};
                        Camera::handle_click(properties, ctx, args);
                    },],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyLiteral::new(Default::default()))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Percent(100.into()))),Box::new(PropertyLiteral::new(Size::Percent(100.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

pax_std_primitives::group::GroupInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::pax_stdCOCOprimitivesCOCOGroup
    
        ( Group {
            
        })
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyExpression::new(16))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Default::default())),Box::new(PropertyLiteral::new(Default::default()))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
            

pax_std_primitives::rectangle::RectangleInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::pax_stdCOCOprimitivesCOCORectangle
    
        ( Rectangle {
            
                stroke: Box::new( PropertyLiteral::new(Default::default()) ),
            
                fill: Box::new( PropertyExpression::new(18) ),
            
        })
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyExpression::new(17))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Pixels(100.into()))),Box::new(PropertyLiteral::new(Size::Pixels(100.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
            

pax_std_primitives::rectangle::RectangleInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::pax_stdCOCOprimitivesCOCORectangle
    
        ( Rectangle {
            
                stroke: Box::new( PropertyLiteral::new(Default::default()) ),
            
                fill: Box::new( PropertyExpression::new(20) ),
            
        })
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyExpression::new(19))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Pixels(100.into()))),Box::new(PropertyLiteral::new(Size::Pixels(100.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
            

pax_std_primitives::rectangle::RectangleInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::pax_stdCOCOprimitivesCOCORectangle
    
        ( Rectangle {
            
                stroke: Box::new( PropertyLiteral::new(Default::default()) ),
            
                fill: Box::new( PropertyExpression::new(22) ),
            
        })
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyExpression::new(21))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Pixels(100.into()))),Box::new(PropertyLiteral::new(Size::Pixels(100.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
            

pax_std_primitives::ellipse::EllipseInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::pax_stdCOCOprimitivesCOCOEllipse
    
        ( Ellipse {
            
                stroke: Box::new( PropertyLiteral::new(Default::default()) ),
            
                fill: Box::new( PropertyExpression::new(24) ),
            
        })
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyExpression::new(23))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Pixels(100.into()))),Box::new(PropertyLiteral::new(Size::Pixels(100.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
,
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
])));

    args.handler_registry = Some(Rc::new(RefCell::new(HandlerRegistry {
        click_handlers: vec![],
        will_render_handlers: vec![
                 |properties, ctx|{
                     let properties = &mut *properties.as_ref().borrow_mut();
                     let properties = if let PropertiesCoproduct::crateCOCOcameraCOCOCamera(p) = properties {p} else {unreachable!()};
                     Camera::handle_will_render(properties,ctx);
                 },
             ],
        did_mount_handlers: vec![
                  |properties, ctx|{
                      let properties = &mut *properties.as_ref().borrow_mut();
                      let properties = if let PropertiesCoproduct::crateCOCOcameraCOCOCamera(p) = properties {p} else {unreachable!()};
                      Camera::handle_did_mount(properties,ctx);
                  },
              ],
        scroll_handlers: vec![],
    })));

    args.compute_properties_fn = Some(Box::new(|properties, rtc|{
        let properties = &mut *properties.as_ref().borrow_mut();
        let properties = if let PropertiesCoproduct::crateCOCOcameraCOCOCamera(p) = properties {p} else {unreachable!()};

        
            if let Some(new_value) = rtc.compute_eased_value(properties.ticks._get_transition_manager()) {
            properties.ticks.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.ticks._get_vtable_id()) {
            let new_value = if let TypesCoproduct::usize(v) = new_value { v } else { unreachable!() };
            properties.ticks.set(new_value);
            }
        
            if let Some(new_value) = rtc.compute_eased_value(properties.zoom._get_transition_manager()) {
            properties.zoom.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.zoom._get_vtable_id()) {
            let new_value = if let TypesCoproduct::f64(v) = new_value { v } else { unreachable!() };
            properties.zoom.set(new_value);
            }
        
            if let Some(new_value) = rtc.compute_eased_value(properties.pan_x._get_transition_manager()) {
            properties.pan_x.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.pan_x._get_vtable_id()) {
            let new_value = if let TypesCoproduct::f64(v) = new_value { v } else { unreachable!() };
            properties.pan_x.set(new_value);
            }
        
            if let Some(new_value) = rtc.compute_eased_value(properties.pan_y._get_transition_manager()) {
            properties.pan_y.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.pan_y._get_vtable_id()) {
            let new_value = if let TypesCoproduct::f64(v) = new_value { v } else { unreachable!() };
            properties.pan_y.set(new_value);
            }
        
            if let Some(new_value) = rtc.compute_eased_value(properties.type_example._get_transition_manager()) {
            properties.type_example.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.type_example._get_vtable_id()) {
            let new_value = if let TypesCoproduct::crateCOCOcameraCOCOTypeExample(v) = new_value { v } else { unreachable!() };
            properties.type_example.set(new_value);
            }
        
    }));

    ComponentInstance::instantiate(args)
}




    
pub fn instantiate_crate_hello_rgb_HelloRGB<R: 'static + RenderContext>(instance_registry: Rc<RefCell<InstanceRegistry<R>>>, mut args: InstantiationArgs<R>) -> Rc<RefCell<ComponentInstance<R>>>  {
    args.component_template = Some(Rc::new(RefCell::new(vec![

pax_std_primitives::ellipse::EllipseInstance::instantiate(
 InstantiationArgs {
    properties: PropertiesCoproduct::pax_stdCOCOprimitivesCOCOEllipse
    
        ( Ellipse {
            
                stroke: Box::new( PropertyLiteral::new(Default::default()) ),
            
                fill: Box::new( PropertyExpression::new(26) ),
            
        })
    ,
    handler_registry:  Some(Rc::new(RefCell::new(
    HandlerRegistry {
         click_handlers: vec![|stack_frame, ctx, args|{
                        let properties = (*stack_frame).borrow().get_properties();
                        let properties = &mut *properties.as_ref().borrow_mut();
                        let properties = if let PropertiesCoproduct::crateCOCOhello_rgbCOCOHelloRGB(p) = properties {p} else {unreachable!()};
                        HelloRGB::handle_click(properties, ctx, args);
                    },],
         will_render_handlers: vec![],
         did_mount_handlers: vec![],
         scroll_handlers: vec![|stack_frame, ctx, args|{
                         let properties = (*stack_frame).borrow().get_properties();
                         let properties = &mut *properties.as_ref().borrow_mut();
                         let properties = if let PropertiesCoproduct::crateCOCOhello_rgbCOCOHelloRGB(p) = properties {p} else {unreachable!()};
                         HelloRGB::handle_scroll(properties,ctx,args);
                     },],
    }
    ))),
    instance_registry: Rc::clone(&instance_registry),
    transform: Rc::new(RefCell::new(PropertyExpression::new(25))),
    size: Some(Rc::new(RefCell::new(
        [Box::new(PropertyLiteral::new(Size::Percent(50.into()))),Box::new(PropertyLiteral::new(Size::Percent(50.into())))]
    ))),
    children: Some(Rc::new(RefCell::new(vec![
        
    ]))),
    component_template: None,
    scroller_args: None,
    slot_index: None,
    repeat_source_expression_vec: None,
    repeat_source_expression_range: None,
    conditional_boolean_expression: None,
    compute_properties_fn: None,
})
])));

    args.handler_registry = Some(Rc::new(RefCell::new(HandlerRegistry {
        click_handlers: vec![],
        will_render_handlers: vec![],
        did_mount_handlers: vec![],
        scroll_handlers: vec![],
    })));

    args.compute_properties_fn = Some(Box::new(|properties, rtc|{
        let properties = &mut *properties.as_ref().borrow_mut();
        let properties = if let PropertiesCoproduct::crateCOCOhello_rgbCOCOHelloRGB(p) = properties {p} else {unreachable!()};

        
            if let Some(new_value) = rtc.compute_eased_value(properties.rotation._get_transition_manager()) {
            properties.rotation.set(new_value);
            } else if let Some(new_value) = rtc.compute_vtable_value(properties.rotation._get_vtable_id()) {
            let new_value = if let TypesCoproduct::f64(v) = new_value { v } else { unreachable!() };
            properties.rotation.set(new_value);
            }
        
    }));

    ComponentInstance::instantiate(args)
}



