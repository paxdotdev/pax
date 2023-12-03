use super::NamespaceTrieNode;
use std::collections::HashMap;

fn test_serialize_to_reexports() {
    let input_vec = vec![
        "crate::Example",
        "crate::fireworks::Fireworks",
        "crate::grids::Grids",
        "crate::grids::RectDef",
        "crate::hello_rgb::HelloRGB",
        "f64",
        "pax_std::primitives::Ellipse",
        "pax_std::primitives::Group",
        "pax_std::primitives::Rectangle",
        "pax_std::types::Color",
        "pax_std::types::Stroke",
        "std::vec::Vec",
        "usize",
    ];

    let mut root_node = NamespaceTrieNode {
        node_string: None,
        children: HashMap::new(),
    };

    for namespace_string in input_vec {
        root_node.insert(&namespace_string);
    }

    let output = root_node.serialize_to_reexports();

    let expected_output = r#"pub mod pax_reexports {
pub use crate::Example;
pub mod fireworks {
    pub use crate::fireworks::Fireworks;
}
pub mod grids {
    pub use crate::grids::Grids;
    pub use crate::grids::RectDef;
}
pub mod hello_rgb {
    pub use crate::hello_rgb::HelloRGB;
}
pub use f64;
pub mod pax_std{
    pub mod primitives{
        pub use pax_std::primitives::Ellipse;
        pub use pax_std::primitives::Group;
        pub use pax_std::primitives::Rectangle;
    }
    pub mod types{
        pub use pax_std::types::Color;
        pub use pax_std::types::Stroke;
    }
}
pub mod std{
    pub mod vec{
        pub use std::vec::Vec;
    }
}
pub use usize;

}"#;

    assert_eq!(output, expected_output);
}
