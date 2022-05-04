


pub mod compiletime {
    use bincode::{deserialize, serialize};
    use serde::{Deserialize, Serialize};

    #[cfg(test)]
    mod tests {
        #[test]
        fn it_works() {
            let result = 2 + 2;
            assert_eq!(result, 4);
        }
    }

    //definition container for an entire Pax cartridge
    #[derive(Serialize, Deserialize)]
    pub struct PaxManifest {
        pub components: Vec<ComponentDefinition>,
        pub root_component_id: String,
    }

    //these methods are exposed to encapsulate the serialization method/version (at time of writing: bincode 1.3.3)
//though that particular version isn't important, this prevents consuming libraries from having to
//coordinate versions/strategies for serialization
    impl PaxManifest {
        pub fn serialize(&self) -> Vec<u8> {
            serialize(&self).unwrap()
        }

        pub fn deserialize(bytes: &[u8]) -> Self {
            deserialize(bytes).unwrap()
        }
    }
//
// pub enum Action {
//     Create,
//     Read,
//     Update,
//     Delete,
//     Command,
// }
//
// #[allow(dead_code)]
// pub struct PaxMessage {
//     pub action: Action,
//     pub payload: Entity,
// }
//
// pub enum Entity {
//     ComponentDefinition(ComponentDefinition),
//     TemplateNodeDefinition(TemplateNodeDefinition),
//     CommandDefinitionTODO,
// }

    #[derive(Serialize, Deserialize)]
    pub struct ComponentDefinition {
        pub source_id: String,
        pub pascal_identifier: String,
        pub module_path: String,
        //optional not because it cannot exist, but because
        //there are times in this data structure's lifecycle when it
        //is not yet known
        pub root_template_node_id: Option<String>,
        pub template: Option<Vec<TemplateNodeDefinition>>,
        //can be hydrated as a tree via child_ids/parent_id
        pub settings: Option<Vec<SettingsSelectorBlockDefinition>>,
    }

    #[derive(Serialize, Deserialize)]
//Represents an entry within a component template, e.g. a <Rectangle> declaration inside a template
    pub struct TemplateNodeDefinition {
        pub id: String,
        pub component_id: String,
        pub inline_attributes: Option<Vec<(String, AttributeValueDefinition)>>,
        pub children_ids: Vec<String>,
    }

    #[derive(Serialize, Deserialize)]
    pub enum AttributeValueDefinition {
        String(String),
        Expression(String),
    }

    #[derive(Serialize, Deserialize)]
    pub struct SettingsSelectorBlockDefinition {
        pub selector: String,
        pub value_block: SettingsLiteralBlockDefinition,
        //TODO: think through this recursive data structure and de/serialization.
        //      might need to normalize it, keeping a tree of `SettingsLiteralBlockDefinition`s
        //      where nodes are flattened into a list.
        //     First: DO we need to normalize it?  Will something like Serde magically fix this?
        //     It's possible that it will.  Revisit only if we have trouble serializing this data.
    }

    #[derive(Serialize, Deserialize)]
    pub struct SettingsLiteralBlockDefinition {
        pub explicit_type_pascal_identifier: Option<String>,
        pub settings_key_value_pairs: Vec<(String, SettingsValueDefinition)>,
    }

    #[derive(Serialize, Deserialize)]
    pub enum SettingsValueDefinition {
        Literal(SettingsLiteralValue),
        Expression(String),
        Enum(String),
        Block(SettingsLiteralBlockDefinition),
    }

    #[derive(Serialize, Deserialize)]
    pub enum SettingsLiteralValue {
        LiteralNumberWithUnit(Number, Unit),
        LiteralNumber(Number),
        LiteralArray(Vec<SettingsLiteralValue>),
        String(String),
    }

    #[derive(Serialize, Deserialize)]
    pub enum Number {
        Float(f64),
        Int(isize)
    }

    #[derive(Serialize, Deserialize)]
    pub enum Unit {
        Pixels,
        Percent
    }

//
//
// pub enum SettingsValue {
//     Literal(String),
//     Block(SettingsValueBlock),
// }
//
// #[allow(dead_code)]
// pub struct SettingsDefinition {
//     id: String,
//     selector: String,
//     value: SettingsValueBlock,
// }
//
// #[allow(dead_code)]
// pub struct SettingsValueBlock {
//     pairs: Option<Vec<(String, SettingsValue)>>,
// }



}


pub mod runtime {

    use std::ffi::CString;

    #[repr(C)]
    pub enum Message {
        TextCreate(u64), //node instance ID
        TextUpdate(u64, TextPatch),
        TextDelete(u64),
        ClippingCreate(u64),
        ClippingUpdate(u64, ClippingPatch),
        ClippingDelete(u64),
        //TODO: form controls
        //TODO: scroll containers
        NativeEventClick(NativeArgsClick)
    }

    #[repr(C)]
    pub struct NativeArgsClick {
        x: f64,
        y: f64,
        //TODO: probably native element id (in case of native element click), offset
        //TODO: right/middle/left click
    }

    #[repr(C)]
    pub struct ClippingPatch {
        size: Option<[f64; 2]>, //rectangle, top-left @ (0,0)
        transform: Option<[f64; 6]>,
    }

    #[repr(C)]
    pub enum TextSize {
        Auto(),
        Pixels(f64),
    }

    #[derive(Default)]
    #[repr(C)]
    pub struct TextPatch {
        pub content: Option<CString>, //See `TextContentMessage` for a sketched-out approach to rich text
        transform: Option<[f64; 6]>,
        size: [Option<TextSize>; 2],
    }
    //
    // #[repr(C)]
    // pub struct TextContentMessage {
    //     /// C-friendly `Vec<CString>`, along with explicit length.
    //     /// In other renderers, these sorts of `spans` are sometimes referred to as `runs`
    //     spans: *mut CString, //
    //     spans_len: u64,
    //     commands: *mut TextCommand, //C-friendly `Vec<MessageTextPropertiesCommand>`, along with explicit length
    //     commands_len: u64,
    // }

    #[repr(C)]
    pub struct TextCommand {
        set_font: Option<CString>,
        set_weight: Option<CString>,
        set_fill_color: Option<CString>,
        set_stroke_color: Option<CString>,
        set_decoration: Option<CString>,
    }

}




