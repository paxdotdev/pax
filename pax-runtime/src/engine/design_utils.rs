use std::rc::Rc;

// For the designer
// What should be exposed to the user?
use crate::{math::Transform2, ExpandedNode};

pub struct Node {
    inner: Rc<ExpandedNode>,
}

impl Node {
    fn screen_transform(&self) -> Transform2 {
        todo!()
    }
}
