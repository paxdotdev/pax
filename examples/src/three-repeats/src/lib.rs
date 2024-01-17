#![allow(unused_imports)]

use pax_lang::*;
use pax_lang::api::*;
use pax_std::primitives::*;
use pax_std::types::*;
use pax_std::types::text::*;
use pax_std::components::*;
use pax_std::components::Stacker;

#[derive(Pax)]
#[main]
#[custom(Default)]
#[file("lib.pax")]
pub struct ThreeRepeats {
    pub some_data: Property<Vec<CustomStruct>>,
}

impl Default for ThreeRepeats {
    fn default() -> Self {
        Self {
            some_data: Box::new(PropertyLiteral::new(vec![
                CustomStruct{
                    x: 250
                },
                CustomStruct{
                    x: 300
                },
                CustomStruct{
                    x: 450
                },
                CustomStruct{
                    x: 550
                },
                CustomStruct{
                    x: 850
                }
            ]))
        }
    }
}

#[derive(Pax)]
#[custom(Imports)]
pub struct CustomStruct {
    pub x: isize,
}