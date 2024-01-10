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
#[inlined(
<Text text={self.message} class=centered class=small id=text/>
<Rectangle class=centered class=small @click=self.increment fill={Fill::Solid(Color::hlc(ticks, 75.0, 150.0))} 
    corner_radii={RectangleCornerRadii::radii(10.0, 10.0, 10.0, 10.0)}/>

@settings {
    .centered {
        x: 50%
        y: 50%
        anchor_x: 50%
        anchor_y: 50%
    }
    .small {
        width: 120px
        height: 120px
    }
    #text {
        style: {
            font: {Font::system("Times New Roman", FontStyle::Normal, FontWeight::Bold)}
            font_size: 32px
            fill: {Color::rgba(1.0, 1.0, 1.0, 1.0)}
            align_vertical: TextAlignVertical::Center
            align_horizontal: TextAlignHorizontal::Center
            align_multiline: TextAlignHorizontal::Center
        }
    }
}

@handlers {
    mount: handle_mount,
    pre_render: handle_pre_render
}
)]
pub struct Example {
    pub ticks: Property<usize>,
    pub num_clicks: Property<usize>,
    pub message: Property<String>,
}

impl Example {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.message.set("Click me".to_string());
    }
    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        let old_ticks = self.ticks.get();
        self.ticks.set(old_ticks + 1);
    }

    pub fn increment(&mut self, ctx: &NodeContext, args: ArgsClick){
        let old_num_clicks = self.num_clicks.get();
        self.num_clicks.set(old_num_clicks + 1);
        self.message.set(format!("{} clicks", self.num_clicks.get()));
    }
}