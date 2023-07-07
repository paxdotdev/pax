use std::cell::RefCell;
use std::rc::Rc;

//Component types
#[repr(C)]
pub enum PropertiesCoproduct {
    /* entries generated via properties-coproduct-lib.tera */
    None,
    RepeatList(Vec<Rc<RefCell<PropertiesCoproduct>>>),
    RepeatItem(Rc<PropertiesCoproduct>, usize),
    isize(isize),
    stdCOCOopsCOCORangeLABRisizeRABR(std::ops::Range<isize>),

    
    crateCOCOExample(pax_example::pax_reexports::Example),
    
    crateCOCOcameraCOCOCamera(pax_example::pax_reexports::camera::Camera),
    
    crateCOCOcameraCOCOTypeExample(pax_example::pax_reexports::camera::TypeExample),
    
    crateCOCOfireworksCOCOFireworks(pax_example::pax_reexports::fireworks::Fireworks),
    
    crateCOCOgridsCOCOGrids(pax_example::pax_reexports::grids::Grids),
    
    crateCOCOgridsCOCORectDef(pax_example::pax_reexports::grids::RectDef),
    
    crateCOCOhello_rgbCOCOHelloRGB(pax_example::pax_reexports::hello_rgb::HelloRGB),
    
    crateCOCOwordsCOCOWords(pax_example::pax_reexports::words::Words),
    
    pax_stdCOCOprimitivesCOCOEllipse(pax_example::pax_reexports::pax_std::primitives::Ellipse),
    
    pax_stdCOCOprimitivesCOCOFrame(pax_example::pax_reexports::pax_std::primitives::Frame),
    
    pax_stdCOCOprimitivesCOCOGroup(pax_example::pax_reexports::pax_std::primitives::Group),
    
    pax_stdCOCOprimitivesCOCORectangle(pax_example::pax_reexports::pax_std::primitives::Rectangle),
    
    pax_stdCOCOprimitivesCOCOText(pax_example::pax_reexports::pax_std::primitives::Text),
    
    pax_stdCOCOstackerCOCOStacker(pax_example::pax_reexports::pax_std::stacker::Stacker),
    
    pax_stdCOCOtypesCOCOColor(pax_example::pax_reexports::pax_std::types::Color),
    
    pax_stdCOCOtypesCOCOColorVariant(pax_example::pax_reexports::pax_std::types::ColorVariant),
    
    pax_stdCOCOtypesCOCOFill(pax_example::pax_reexports::pax_std::types::Fill),
    
    pax_stdCOCOtypesCOCOLinearGradient(pax_example::pax_reexports::pax_std::types::LinearGradient),
    
    pax_stdCOCOtypesCOCORadialGradient(pax_example::pax_reexports::pax_std::types::RadialGradient),
    
    pax_stdCOCOtypesCOCORectangleCornerRadii(pax_example::pax_reexports::pax_std::types::RectangleCornerRadii),
    
    pax_stdCOCOtypesCOCOStackerCell(pax_example::pax_reexports::pax_std::types::StackerCell),
    
    pax_stdCOCOtypesCOCOStackerDirection(pax_example::pax_reexports::pax_std::types::StackerDirection),
    
    pax_stdCOCOtypesCOCOStroke(pax_example::pax_reexports::pax_std::types::Stroke),
    
    pax_stdCOCOtypesCOCOtextCOCOFont(pax_example::pax_reexports::pax_std::types::text::Font),
    
    pax_stdCOCOtypesCOCOtextCOCOFontStyle(pax_example::pax_reexports::pax_std::types::text::FontStyle),
    
    pax_stdCOCOtypesCOCOtextCOCOFontWeight(pax_example::pax_reexports::pax_std::types::text::FontWeight),
    
    pax_stdCOCOtypesCOCOtextCOCOLocalFont(pax_example::pax_reexports::pax_std::types::text::LocalFont),
    
    pax_stdCOCOtypesCOCOtextCOCOSystemFont(pax_example::pax_reexports::pax_std::types::text::SystemFont),
    
    pax_stdCOCOtypesCOCOtextCOCOTextAlignHorizontal(pax_example::pax_reexports::pax_std::types::text::TextAlignHorizontal),
    
    pax_stdCOCOtypesCOCOtextCOCOTextAlignVertical(pax_example::pax_reexports::pax_std::types::text::TextAlignVertical),
    
    pax_stdCOCOtypesCOCOtextCOCOTextStyle(pax_example::pax_reexports::pax_std::types::text::TextStyle),
    
    pax_stdCOCOtypesCOCOtextCOCOWebFont(pax_example::pax_reexports::pax_std::types::text::WebFont),
    
}

//Property types
#[repr(C)]
pub enum TypesCoproduct {
    
    Numeric(pax_runtime_api::Numeric),
    
    Size(pax_runtime_api::Size),
    
    Size2D(pax_runtime_api::Size2D),
    
    SizePixels(pax_runtime_api::SizePixels),
    
    String(String),
    
    Transform2D(pax_runtime_api::Transform2D),
    
    bool(bool),
    
    crateCOCOcameraCOCOTypeExample(pax_example::pax_reexports::camera::TypeExample),
    
    f64(f64),
    
    isize(isize),
    
    pax_langCOCOapiCOCONumeric(pax_example::pax_reexports::pax_lang::api::Numeric),
    
    pax_langCOCOapiCOCOSize(pax_example::pax_reexports::pax_lang::api::Size),
    
    pax_langCOCOapiCOCOSizePixels(pax_example::pax_reexports::pax_lang::api::SizePixels),
    
    pax_stdCOCOtypesCOCOColor(pax_example::pax_reexports::pax_std::types::Color),
    
    pax_stdCOCOtypesCOCOColorVariant(pax_example::pax_reexports::pax_std::types::ColorVariant),
    
    pax_stdCOCOtypesCOCOFill(pax_example::pax_reexports::pax_std::types::Fill),
    
    pax_stdCOCOtypesCOCOLinearGradient(pax_example::pax_reexports::pax_std::types::LinearGradient),
    
    pax_stdCOCOtypesCOCORadialGradient(pax_example::pax_reexports::pax_std::types::RadialGradient),
    
    pax_stdCOCOtypesCOCORectangleCornerRadii(pax_example::pax_reexports::pax_std::types::RectangleCornerRadii),
    
    pax_stdCOCOtypesCOCOStackerDirection(pax_example::pax_reexports::pax_std::types::StackerDirection),
    
    pax_stdCOCOtypesCOCOStroke(pax_example::pax_reexports::pax_std::types::Stroke),
    
    pax_stdCOCOtypesCOCOtextCOCOFont(pax_example::pax_reexports::pax_std::types::text::Font),
    
    pax_stdCOCOtypesCOCOtextCOCOFontStyle(pax_example::pax_reexports::pax_std::types::text::FontStyle),
    
    pax_stdCOCOtypesCOCOtextCOCOFontWeight(pax_example::pax_reexports::pax_std::types::text::FontWeight),
    
    pax_stdCOCOtypesCOCOtextCOCOLocalFont(pax_example::pax_reexports::pax_std::types::text::LocalFont),
    
    pax_stdCOCOtypesCOCOtextCOCOSystemFont(pax_example::pax_reexports::pax_std::types::text::SystemFont),
    
    pax_stdCOCOtypesCOCOtextCOCOTextAlignHorizontal(pax_example::pax_reexports::pax_std::types::text::TextAlignHorizontal),
    
    pax_stdCOCOtypesCOCOtextCOCOTextAlignVertical(pax_example::pax_reexports::pax_std::types::text::TextAlignVertical),
    
    pax_stdCOCOtypesCOCOtextCOCOTextStyle(pax_example::pax_reexports::pax_std::types::text::TextStyle),
    
    pax_stdCOCOtypesCOCOtextCOCOWebFont(pax_example::pax_reexports::pax_std::types::text::WebFont),
    
    stdCOCOopsCOCORangeLABRisizeRABR(std::ops::Range<isize>),
    
    stdCOCOoptionCOCOOptionLABRpax_stdCOCOtypesCOCOtextCOCOTextStyleRABR(pax_example::pax_reexports::std::option::Option<pax_example::pax_reexports::pax_std::types::text::TextStyle>),
    
    stdCOCOstringCOCOString(pax_example::pax_reexports::std::string::String),
    
    stdCOCOvecCOCOVecLABRcrateCOCOgridsCOCORectDefRABR(pax_example::pax_reexports::std::vec::Vec<pax_example::pax_reexports::grids::RectDef>),
    
    stdCOCOvecCOCOVecLABRpax_stdCOCOtypesCOCOStackerCellRABR(pax_example::pax_reexports::std::vec::Vec<pax_example::pax_reexports::pax_std::types::StackerCell>),
    
    stdCOCOvecCOCOVecLABRstdCOCOoptionCOCOOptionLABRpax_langCOCOapiCOCOSizeRABRRABR(pax_example::pax_reexports::std::vec::Vec<pax_example::pax_reexports::std::option::Option<pax_example::pax_reexports::pax_lang::api::Size>>),
    
    stdCOCOvecCOCOVecLABRstdCOCOrcCOCORcLABRPropertiesCoproductRABRRABR(std::vec::Vec<std::rc::Rc<PropertiesCoproduct>>),
    
    usize(pax_example::pax_reexports::usize),
    
}
