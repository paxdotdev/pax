use crate::RelationData;
use pax_kit::*;

#[pax]
#[inlined(
    <Button anchor=0% width=100% height=40px label={relation.title}
        color=TRANSPARENT hover_color=rgb(232, 241, 243) corner_radius=3
        outline={paint: TRANSPARENT, width: 0px}
        style={font: "Arial", font_size: 14px, fill: rgb(65, 102, 129),
            align_horizontal: TextAlignHorizontal::Left, align_vertical: TextAlignVertical::Center}
        @button_click=self.follow/>
    <Text anchor=0% x=8px y=39px width={100% - 16px} height=24px text={relation.description}
        _raycastable=false style={font: "Arial", font_size: 12px, fill: rgb(103, 118, 119)}/>
)]
pub struct ConnectionLink {
    pub relation: Property<RelationData>,
    pub selected: Property<String>,
}
impl ConnectionLink {
    pub fn follow(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.selected.set(self.relation.get().target);
    }
}
