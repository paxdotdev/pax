use std::{
    collections::{BTreeMap, HashMap},
    env,
};

use actix_web::{web::Data, App};
use awc::Client;
use futures_util::SinkExt as _;
use pax_compiler::design_server::{web_socket, AppState};
use pax_designtime::messages::{AgentMessage, ManifestSerializationRequest};
use pax_manifest::{
    ComponentDefinition, ComponentTemplate, LiteralBlockDefinition, PaxManifest,
    SettingsBlockElement, TemplateNodeDefinition, Token, TypeId,
};
use rmp_serde::to_vec;

const EXPECTED_PAX: &str = "// Hello world
<SpecialComponent />

@settings {
    @existing_handler: handler_action,
    #existing_selector {
    
    }
}";

pub fn get_test_server() -> actix_test::TestServer {
    actix_test::start(|| {
        App::new()
            .app_data(Data::new(AppState::new_empty()))
            .service(web_socket)
    })
}

fn create_basic_manifest(source_path: String) -> PaxManifest {
    let mut components = BTreeMap::new();
    let component_type_id = TypeId::build_singleton("Component1", Some("Component1"));
    let special_component_type_id =
        TypeId::build_singleton("SpecialComponent", Some("SpecialComponent"));
    let mut template = ComponentTemplate::new(component_type_id.clone(), Some(source_path));

    template.add(TemplateNodeDefinition {
        type_id: special_component_type_id,
        control_flow_settings: None,
        settings: None,
        raw_comment_string: None,
    });

    // .add puts the node on the top of the template by default in line with designing
    template.add(TemplateNodeDefinition {
        type_id: TypeId::build_comment(),
        control_flow_settings: None,
        settings: None,
        raw_comment_string: Some("// Hello world\n".to_owned()),
    });

    components.insert(
        component_type_id.clone(),
        ComponentDefinition {
            type_id: component_type_id.clone(),
            is_main_component: false,
            is_primitive: false,
            is_struct_only_component: false,
            module_path: "module_path1".to_string(),
            primitive_instance_import_path: None,
            template: Some(template),
            settings: Some(vec![
                SettingsBlockElement::SelectorBlock(
                    Token::new_without_location("#existing_selector".to_string()),
                    LiteralBlockDefinition::new(vec![]),
                ),
                SettingsBlockElement::Handler(
                    Token::new_without_location("@existing_handler".to_string()),
                    vec![Token::new_without_location("handler_action".to_string())],
                ),
            ]),
        },
    );

    PaxManifest {
        components,
        main_component_type_id: component_type_id,
        type_table: HashMap::new(),
        assets_dirs: vec![],
        project_files: vec![],
        engine_import_path: "".to_string(),
    }
}

#[actix_web::test]
async fn test_manifest_serialization_request() {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    // Join the current directory with the relative path to the output file
    let path = current_dir.join("tests/data/manifest_serialization_test.pax");
    let path_str = path.to_str().expect("Path is not a valid UTF-8 string");

    let srv = get_test_server();

    let client = Client::new();
    let (_resp, mut connection) = client.ws(srv.url("/ws")).connect().await.unwrap();

    // Prepare a ManifestSerializationRequest
    let request = AgentMessage::ManifestSerializationRequest(ManifestSerializationRequest {
        manifest: rmp_serde::to_vec(&create_basic_manifest(path_str.to_string())).unwrap(),
    });

    let serialized_request = to_vec(&request).expect("Failed to serialize request");
    connection
        .send(awc::ws::Message::Binary(serialized_request.into()))
        .await
        .unwrap();

    // Check that the output file contains the expected PAX
    let output = std::fs::read_to_string(path_str).expect("Failed to read output file");
    assert_eq!(output, EXPECTED_PAX);

    // Close the WebSocket connection
    connection.close().await.expect("Failed to close WebSocket");
}
