use std::{
    collections::{HashMap, HashSet},
    env,
    time::Duration,
};

use actix_web::{web::Data, App};
use pax_design_server::{web_socket, AppState};
use pax_manifest::{
    ComponentDefinition, ComponentTemplate, LiteralBlockDefinition, PaxManifest,
    SettingsBlockElement, TemplateNodeDefinition, Token, TokenType, TypeId,
};

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
            .app_data(Data::new(AppState::new()))
            .service(web_socket)
    })
}

fn create_basic_manifest(source_path: String) -> PaxManifest {
    let mut components = HashMap::new();
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
                    Token::new_from_raw_value(
                        "#existing_selector".to_string(),
                        TokenType::Selector,
                    ),
                    LiteralBlockDefinition::new(vec![]),
                ),
                SettingsBlockElement::Handler(
                    Token::new_from_raw_value("@existing_handler".to_string(), TokenType::EventId),
                    vec![Token::new_from_raw_value(
                        "handler_action".to_string(),
                        TokenType::Handler,
                    )],
                ),
            ]),
        },
    );

    PaxManifest {
        components,
        main_component_type_id: component_type_id,
        expression_specs: None,
        type_table: HashMap::new(),
        import_paths: HashSet::new(),
    }
}

#[actix_web::test]
async fn designtime_integration_test() {
    let component_type_id = TypeId::build_singleton("Component1", Some("Component1"));
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let path = current_dir.join("tests/data/designtime_integration_test.pax");
    let path_str = path.to_str().expect("Path is not a valid UTF-8 string");

    let srv = get_test_server();

    let manifest: PaxManifest = create_basic_manifest(path_str.to_owned());
    let mut designer = pax_designtime::DesigntimeManager::new_with_addr(manifest, srv.addr());
    designer.send_component_update(&component_type_id).unwrap();

    std::thread::sleep(Duration::from_secs(1));

    let output = std::fs::read_to_string(path_str).expect("Failed to read output file");
    assert_eq!(output, EXPECTED_PAX);
    std::fs::write(path_str, b"FILE HAS NOT BEEN UPDATED BY DESIGNTIME")
        .expect("couldn't reset file");
}
