use crate::PaxText;
use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;
use std::cmp::Ordering;
use std::iter;

#[pax]
#[inlined(
    for cell in self.cells {
        <Group anchor_x=0% anchor_y=0% x={cell.x} y={cell.y} width={cell.w} height={cell.h}>
            <PaxText x=50% width={100%-10px} height=100% text={cell.def.text} align={cell.def.text_align} color={cell.text_color}/>
            <Rectangle x=50% y=50% width={100%-1px} height={100%-1px} fill={cell.background}/>
        </Group>
    }
    <Rectangle fill={self.borders}/>

    @settings {
        @mount: on_mount,
    }
)]
#[custom(Default)]
pub struct Table {
    pub headers: Property<Vec<Cell>>,
    pub header_color: Property<Color>,
    pub header_text_color: Property<Color>,
    pub rows: Property<Vec<Vec<Cell>>>,
    pub row_colors: Property<Vec<Color>>,
    pub row_text_color: Property<Color>,

    pub sections: Property<Vec<Size>>,
    pub borders: Property<Color>,

    // private
    pub cells: Property<Vec<CellData>>,
}

impl Default for Table {
    fn default() -> Self {
        Self {
            headers: Property::new(vec![
                Cell {
                    text: "Name".to_string(),
                    text_align: TextAlignHorizontal::Left,
                },
                Cell {
                    text: "Type".to_string(),
                    text_align: TextAlignHorizontal::Left,
                },
                Cell {
                    text: "Description".to_string(),
                    text_align: TextAlignHorizontal::Left,
                },
            ]),
            header_color: Property::new(Color::rgb(30.into(), 30.into(), 30.into())),
            header_text_color: Property::new(Color::WHITE),
            rows: Property::new(vec![
                vec![
                    Cell {
                        text: "headers".to_string(),
                        text_align: TextAlignHorizontal::Left,
                    },
                    Cell {
                        text: "List of Cells".to_string(),
                        text_align: TextAlignHorizontal::Left,
                    },
                    Cell {
                        text: "the headers of this table".to_string(),
                        text_align: TextAlignHorizontal::Left,
                    },
                ],
                vec![
                    Cell {
                        text: "header_color".to_string(),
                        text_align: TextAlignHorizontal::Left,
                    },
                    Cell {
                        text: "Color".to_string(),
                        text_align: TextAlignHorizontal::Left,
                    },
                    Cell {
                        text: "color of headers".to_string(),
                        text_align: TextAlignHorizontal::Left,
                    },
                ],
            ]),
            row_colors: Property::new(vec![
                Color::rgb(120.into(), 120.into(), 120.into()),
                Color::rgb(70.into(), 70.into(), 70.into()),
            ]),
            row_text_color: Property::new(Color::WHITE),
            sections: Property::new(vec![Size::Percent(20.into()), Size::Percent(40.into())]),
            cells: Default::default(),
            borders: Property::new(Color::rgb(200.into(), 200.into(), 200.into())),
        }
    }
}

#[pax]
pub struct Cell {
    pub text: String,
    pub text_align: TextAlignHorizontal,
}

#[pax]
pub struct CellData {
    pub def: Cell,
    pub background: Color,
    pub text_color: Color,
    pub x: Size,
    pub y: Size,
    pub w: Size,
    pub h: Size,
}

impl Table {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        let headers = self.headers.clone();
        let sections = self.sections.clone();
        let header_color = self.header_color.clone();
        let header_text_color = self.header_text_color.clone();
        let row_colors = self.row_colors.clone();
        let row_text_color = self.row_text_color.clone();
        let rows = self.rows.clone();
        let deps = [
            headers.untyped(),
            header_color.untyped(),
            header_text_color.untyped(),
            sections.untyped(),
            rows.untyped(),
            row_colors.untyped(),
            row_text_color.untyped(),
        ];

        self.cells.replace_with(Property::computed(
            move || {
                let mut cell_data = Vec::new();
                let headers = headers.get();
                let header_color = header_color.get();
                let header_text_color = header_text_color.get();
                let sections = sections.get();
                let rows = rows.get();
                let row_colors = row_colors.get();
                let row_text_color = row_text_color.get();

                let mut positions = Vec::new();
                positions.push(Size::ZERO());
                positions.extend(sections);
                positions.push(Size::default());
                for (i, w) in positions.windows(2).enumerate() {
                    let (pos, width) = (w[0].clone(), w[1] - w[0]);
                    cell_data.push(CellData {
                        def: headers[i].clone(),
                        background: header_color.clone(),
                        text_color: header_text_color.clone(),
                        x: pos,
                        w: width,
                        y: Size::Pixels(0.0.into()),
                        h: Size::Pixels(30.into()),
                    });
                    for (j, cell) in rows.iter().map(|v| v[i].clone()).enumerate() {
                        cell_data.push(CellData {
                            def: cell,
                            background: row_colors[j % row_colors.len()].clone(),
                            text_color: row_text_color.clone(),
                            x: pos,
                            w: width,
                            y: Size::Pixels(((j + 1) * 30).into()),
                            h: Size::Pixels(30.into()),
                        })
                    }
                }
                cell_data
            },
            &deps,
        ));
    }
}
