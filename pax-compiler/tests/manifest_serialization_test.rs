use std::{
    collections::{BTreeMap, HashMap},
    time::{Duration, Instant},
};

use actix_web::{web::Data, App};
use awc::Client;
use futures_util::SinkExt as _;
use pax_compiler::design_server::{web_socket, AppState};
use pax_designtime::messages::{AgentMessage, ManifestSerializationRequest};
use pax_manifest::pax_runtime_api::PaxValue;
use pax_manifest::{
    ComponentDefinition, ComponentTemplate, LiteralBlockDefinition, PaxManifest,
    SettingsBlockElement, TemplateNodeDefinition, TimelineBlockElement, TimelineDefinition,
    TimelineKeyframe, TimelineMarker, TimelineSelectorBlockDefinition, TimelineSelectorElement,
    TimelineTrackDefinition, TimelineTrackElement, Token, TypeId, ValueDefinition,
};
use rmp_serde::to_vec;
use tempfile::tempdir;

const EXPECTED_PAX: &str = "// Hello world
<SpecialComponent />

@settings {
    @existing_handler: handler_action
    #existing_selector {
    
    }
}

@timeline existing_timeline {
    playhead: self.phase,
    frames: 120,
    #existing_selector {
        progress: {
            0: 0, Linear,
            100%: 1,
        }
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
                    Token::new_without_location("existing_handler".to_string()),
                    vec![Token::new_without_location("handler_action".to_string())],
                ),
            ]),
            timelines: vec![TimelineDefinition {
                name: Some(Token::new_without_location("existing_timeline".to_string())),
                playhead: Some(ValueDefinition::Identifier(
                    pax_manifest::PaxIdentifier::new("self.phase"),
                )),
                frames: Some(120),
                repeat: true,
                elements: vec![TimelineBlockElement::SelectorBlock(
                    Token::new_without_location("#existing_selector".to_string()),
                    TimelineSelectorBlockDefinition {
                        elements: vec![TimelineSelectorElement::Track(
                            Token::new_without_location("progress".to_string()),
                            TimelineTrackDefinition {
                                elements: vec![
                                    TimelineTrackElement::Keyframe(TimelineKeyframe {
                                        marker: TimelineMarker::Frame(0),
                                        value: ValueDefinition::LiteralValue(PaxValue::Numeric(
                                            0.into(),
                                        )),
                                        easing: Some(Token::new_without_location(
                                            "Linear".to_string(),
                                        )),
                                    }),
                                    TimelineTrackElement::Keyframe(TimelineKeyframe {
                                        marker: TimelineMarker::Percent(100.0),
                                        value: ValueDefinition::LiteralValue(PaxValue::Numeric(
                                            1.into(),
                                        )),
                                        easing: None,
                                    }),
                                ],
                                playhead: None,
                                frames: None,
                                repeat: None,
                                starting_value: None,
                                use_local_property_scope: false,
                            },
                        )],
                    },
                )],
            }],
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
async fn test_manifest_serialization_request() {
    let dir = tempdir().expect("failed to create temp dir");
    let path = dir.path().join("manifest_serialization_test.pax");
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

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Ok(output) = std::fs::read_to_string(path_str) {
            if output == EXPECTED_PAX {
                break;
            }
        }

        if Instant::now() >= deadline {
            let output = std::fs::read_to_string(path_str).unwrap_or_default();
            panic!("timed out waiting for manifest serialization output.\nactual:\n{output}");
        }

        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // Close the WebSocket connection
    connection.close().await.expect("Failed to close WebSocket");

    dir.close().expect("failed to clean up temp dir");
}
