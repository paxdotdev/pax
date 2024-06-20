use actix_http::ws::Item;
use actix_web_actors::ws;
use bytes::BytesMut;

pub struct SocketMessageAccumulator {
    partial_binary: Vec<u8>,
}

impl SocketMessageAccumulator {
    pub fn new() -> Self {
        Self {
            partial_binary: Vec::new(),
        }
    }
    pub fn process(&mut self, message: ws::Message) -> Result<Option<Vec<u8>>, String> {
        match message {
            ws::Message::Continuation(item) => {
                match item {
                    Item::FirstText(_) => return Err("no text support".to_string()),
                    Item::FirstBinary(first) => self.partial_binary.extend_from_slice(&first),
                    Item::Continue(b) => self.partial_binary.extend_from_slice(&b),
                    Item::Last(b) => {
                        self.partial_binary.extend_from_slice(&b);
                        let data = std::mem::take(&mut self.partial_binary);
                        return Ok(Some(data));
                    }
                };
            }
            ws::Message::Text(_) => return Err("no text support".to_string()),
            ws::Message::Binary(binary) => return Ok(Some(binary.to_vec())),
            // TODO
            ws::Message::Ping(_) => (),
            ws::Message::Pong(_) => (),
            ws::Message::Close(_) => (),
            ws::Message::Nop => (),
        };
        Ok(None)
    }
}
