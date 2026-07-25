use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    env,
    rc::Rc,
    time::{Duration, Instant},
};

use actix_web::{web::Data, App};
use pax_compiler::design_server::{web_socket, AppState};
use pax_manifest::{
    ComponentDefinition, ComponentTemplate, LiteralBlockDefinition, PaxManifest,
    SettingsBlockElement, TemplateNodeDefinition, Token, TypeId,
};

pub fn get_test_server(manifest: PaxManifest) -> actix_test::TestServer {
    actix_test::start(move || {
        App::new()
            .app_data(Data::new(
                AppState::new(
                    Default::default(),
                    Default::default(),
                    manifest.clone(),
                    None,
                    None,
                    pax_compiler::HotReloadMode::All,
                    None,
                    None,
                )
                .unwrap(),
            ))
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
        selector_info: Default::default(),
        raw_comment_string: None,
    });

    // .add puts the node on the top of the template by default in line with designing
    template.add(TemplateNodeDefinition {
        type_id: TypeId::build_comment(),
        control_flow_settings: None,
        settings: None,
        selector_info: Default::default(),
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
                    Token::new_without_location("click".to_string()),
                    vec![Token::new_without_location("handler_action".to_string())],
                ),
            ]),
            timelines: vec![],
            route_branch: None,
        },
    );

    PaxManifest {
        components,
        main_component_type_id: component_type_id,
        type_table: HashMap::new(),
        assets_dirs: vec![],
        engine_import_path: "".to_string(),
    }
}

#[actix_web::test]
async fn designtime_integration_test() {
    const EXPECTED_PAX: &str = "// Hello world
<SpecialComponent />

@settings {
    @click: handler_action
    #existing_selector {
\x20\x20\x20\x20
    }
}";

    let component_type_id = TypeId::build_singleton("Component1", Some("Component1"));
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let path = current_dir.join("tests/data/designtime_integration_test.pax");
    let path_str = path.to_str().expect("Path is not a valid UTF-8 string");

    let manifest: PaxManifest = create_basic_manifest(path_str.to_owned());
    let _srv = get_test_server(manifest.clone());
    let socket_addr = _srv.addr();
    let url = format!("ws://{}", socket_addr);
    let mut designtime = pax_designtime::DesigntimeManager::new_with_local_addr(manifest, &url);
    let screenshot_map = Rc::new(RefCell::new(HashMap::new()));
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        designtime.handle_recv(screenshot_map.clone()).unwrap();
        match designtime.send_component_update(&component_type_id) {
            Ok(()) => break,
            Err(error) if Instant::now() < deadline => {
                let _ = error;
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("timed out waiting for the design-server socket: {error}"),
        }
    }

    std::thread::sleep(Duration::from_secs(1));

    let output = std::fs::read_to_string(path_str).expect("Failed to read output file");
    assert_eq!(output, EXPECTED_PAX);
    std::fs::write(path_str, b"FILE HAS NOT BEEN UPDATED BY DESIGNTIME")
        .expect("couldn't reset file");
}
