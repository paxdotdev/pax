use pax_engine::{api::cursor::CursorStyle, *};

#[pax]
#[engine_import_path("pax_engine")]
#[derive(PartialEq, Copy)]
pub struct DesignerCursor {
    pub cursor_type: DesignerCursorType,
    pub rotation_degrees: f64,
}

impl DesignerCursor {
    pub fn to_cursor_style(&self) -> CursorStyle {
        match self.cursor_type {
            DesignerCursorType::Rotation => {
                let svg = format!(
                    r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 256 256">
                        <g transform="rotate({} 128 128) scale(0.6 0.6)">
                            <path d="m78.46,245.01v-68.68c0-7.44,6.06-13.5,13.5-13.5s13.5,6.06,13.5,13.5v20.8c25.73-11.69,42.26-37.27,42.26-65.61s-16.54-53.93-42.26-65.62v21.65c0,7.44-6.06,13.5-13.5,13.5s-13.5-6.06-13.5-13.5V18.86h68.68c7.44,0,13.5,6.06,13.5,13.5s-6.06,13.5-13.5,13.5h-21.72c30.56,17.69,49.3,49.98,49.3,85.65,0,18.76-5.28,37.05-15.27,52.88-8.86,14.04-21.06,25.56-35.51,33.62h23.2c7.44,0,13.5,6.06,13.5,13.5s-6.06,13.5-13.5,13.5h-68.68Z" stroke-width="0"/>
                            <path d="m147.14,20.36c6.63,0,12,5.37,12,12s-5.37,12-12,12h-27.65c32.82,16.5,53.73,50.07,53.73,87.15s-21.63,71.82-55.44,88h29.36c6.63,0,12,5.37,12,12s-5.37,12-12,12h-67.18v-67.18c0-6.63,5.37-12,12-12s12,5.37,12,12v23.13c.21-.11.42-.22.64-.31,27.11-11.61,44.62-38.16,44.62-67.63s-17.52-56.03-44.62-67.63c-.22-.09-.43-.21-.64-.31v23.98c0,6.63-5.37,12-12,12s-12-5.37-12-12V20.36h67.18m0-3h-70.18v70.18c0,8.27,6.73,15,15,15s15-6.73,15-15v-19.28c23.98,11.86,39.26,36.27,39.26,63.24s-15.29,51.38-39.26,63.24v-18.43c0-8.27-6.73-15-15-15s-15,6.73-15,15v70.18h70.18c8.27,0,15-6.73,15-15s-6.73-15-15-15h-17.74c12.61-7.96,23.3-18.63,31.31-31.32,10.14-16.07,15.51-34.63,15.51-53.68s-5.19-36.99-15-52.87c-7.78-12.58-18.2-23.24-30.51-31.28h16.43c8.27,0,15-6.73,15-15s-6.73-15-15-15h0Z" fill="#fff" stroke-width="0"/>
                        </g>
                    </svg>"##,
                    self.rotation_degrees
                );
                let encoded_svg = encode_svg(&svg);
                CursorStyle::Url(format!("data:image/svg+xml,{}", encoded_svg), 16, 16)
            }
            DesignerCursorType::Resize => {
                let svg = format!(
                    r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 256 256">
                        <g transform="rotate({} 128 128) scale(0.6 0.6)">
                            <path d="m78.03,203.59c-5.26-5.26-5.26-13.83,0-19.09,2.55-2.55,5.94-3.95,9.55-3.95s7,1.4,9.55,3.95l15.97,15.97V58.09l-15.97,15.97c-2.55,2.55-5.94,3.95-9.55,3.95-3.61,0-7-1.4-9.55-3.95-5.26-5.26-5.26-13.83,0-19.09L126.59,6.41l48.57,48.57c5.26,5.26,5.26,13.83,0,19.09-2.55,2.55-5.94,3.95-9.55,3.95s-7-1.4-9.55-3.95l-15.97-15.97v142.38l15.97-15.97c2.55-2.55,5.94-3.95,9.55-3.95s7,1.4,9.55,3.95c5.26,5.26,5.26,13.83,0,19.09l-48.57,48.57-48.57-48.57Z" stroke-width="0"/>
                            <path d="m126.59,8.53l47.5,47.5c4.69,4.69,4.69,12.28,0,16.97-2.34,2.34-5.41,3.51-8.49,3.51s-6.14-1.17-8.49-3.51l-18.53-18.53v149.62l18.53-18.53c2.34-2.34,5.41-3.51,8.49-3.51s6.14,1.17,8.49,3.51c4.69,4.69,4.69,12.28,0,16.97l-47.5,47.5-47.5-47.5c-4.69-4.69-4.69-12.28,0-16.97,2.34-2.34,5.41-3.51,8.49-3.51s6.14,1.17,8.49,3.51l18.53,18.53V54.47l-18.53,18.53c-2.34,2.34-5.41,3.51-8.49,3.51s-6.14-1.17-8.49-3.51c-4.69-4.69-4.69-12.28,0-16.97L126.59,8.53m0-4.24l-2.12,2.12-47.5,47.5c-2.83,2.83-4.39,6.6-4.39,10.61s1.56,7.77,4.39,10.61c2.83,2.83,6.6,4.39,10.61,4.39s7.77-1.56,10.61-4.39l13.41-13.41v135.13l-13.41-13.41c-2.83-2.83-6.6-4.39-10.61-4.39s-7.77,1.56-10.61,4.39c-2.83,2.83-4.39,6.6-4.39,10.61s1.56,7.77,4.39,10.61l47.5,47.5,2.12,2.12,2.12-2.12,47.5-47.5c2.83-2.83,4.39-6.6,4.39-10.61s-1.56-7.77-4.39-10.61c-2.83-2.83-6.6-4.39-10.61-4.39s-7.77,1.56-10.61,4.39l-13.41,13.41V61.72l13.41,13.41c2.83,2.83,6.6,4.39,10.61,4.39s7.77-1.56,10.61-4.39c2.83-2.83,4.39-6.6,4.39-10.61s-1.56-7.77-4.39-10.61L128.71,6.41l-2.12-2.12h0Z" fill="#fff" stroke-width="0"/>
                        </g>
                    </svg>"##,
                    self.rotation_degrees + 90.0
                );
                let encoded_svg = encode_svg(&svg);
                CursorStyle::Url(format!("data:image/svg+xml,{}", encoded_svg), 16, 16)
            }
            DesignerCursorType::Move => CursorStyle::Cell,
            DesignerCursorType::None => CursorStyle::Auto,
        }
    }
}

#[pax]
#[engine_import_path("pax_engine")]
#[derive(PartialEq, Copy)]
pub enum DesignerCursorType {
    Rotation,
    Resize,
    Move,
    #[default]
    None,
}

impl DesignerCursorType {}

fn encode_svg(svg: &str) -> String {
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    utf8_percent_encode(svg, NON_ALPHANUMERIC).to_string()
}
