use pax_kit::*;

#[pax]
#[inlined(
    <Button width=100% height=100% label={title} color=TRANSPARENT
        hover_color=rgb(232, 241, 243) corner_radius=3
        outline={paint: TRANSPARENT, width: 0px}
        style={font: "Arial", font_size: 14px, fill: rgb(65, 102, 129), underline: true,
            align_horizontal: TextAlignHorizontal::Left, align_vertical: TextAlignVertical::Center}
        @button_click=self.open/>
)]
pub struct DocLink {
    pub title: Property<String>,
    pub url: Property<String>,
}

impl DocLink {
    pub fn open(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        ctx.navigate_to(&self.url.get(), NavigationTarget::New);
    }
}
