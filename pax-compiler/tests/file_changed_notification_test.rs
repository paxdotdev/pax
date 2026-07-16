use actix_web::web::Data;
use actix_web::App;
use awc::Client;
use futures_util::{SinkExt, StreamExt};
use pax_compiler::design_server::{setup_file_watcher, web_socket, AppState};
use pax_compiler::HotReloadMode;
use pax_designtime::messages::{AgentMessage, LoadManifestRequest};
use pax_manifest::{PaxManifest, TypeId};
use rmp_serde::from_slice;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tempfile::tempdir;

pub fn get_test_server(state: Data<AppState>) -> actix_test::TestServer {
    actix_test::start(move || App::new().app_data(state.clone()).service(web_socket))
}

fn simulate_file_change(dir: &str) {
    let file_path = Path::new(dir).join("test.txt");
    let mut file = File::create(file_path).unwrap();
    writeln!(file, "Hello world").unwrap();
}

#[actix_web::test]
async fn test_file_changed_notification() {
    // Create temp directory
    let dir = tempdir().unwrap();
    let dir_path = dir.path().to_str().unwrap();

    // setup file watcher
    let state = Data::new(
        AppState::new(
            Default::default(),
            Default::default(),
            PaxManifest {
                components: BTreeMap::new(),
                main_component_type_id: TypeId::build_singleton("Test", Some("Test")),
                type_table: HashMap::new(),
                assets_dirs: vec![],
                engine_import_path: String::new(),
            },
            None,
            None,
            HotReloadMode::All,
            None,
            None,
        )
        .unwrap(),
    );
    let _watcher =
        setup_file_watcher(state.clone(), dir_path).expect("Failed to setup file watcher");

    // Start test server
    let srv = get_test_server(state.clone());

    // Connect to WebSocket
    let client = Client::new();
    let (_resp, mut connection) = client.ws(srv.url("/ws")).connect().await.unwrap();
    connection
        .send(awc::ws::Message::Binary(
            rmp_serde::to_vec(&AgentMessage::LoadManifestRequest(LoadManifestRequest {
                active_revision: None,
            }))
            .unwrap()
            .into(),
        ))
        .await
        .unwrap();
    let Some(Ok(awc::ws::Frame::Binary(bin_data))) = connection.next().await else {
        panic!("No manifest response received from server");
    };
    assert!(matches!(
        from_slice::<AgentMessage>(&bin_data).unwrap(),
        AgentMessage::LoadManifestResponse(_)
    ));
    actix_web::rt::time::sleep(std::time::Duration::from_millis(1_050)).await;

    // Simulate file change
    simulate_file_change(dir_path);

    // Wait for WebSocket to receive message
    if let Some(Ok(awc::ws::Frame::Binary(bin_data))) = connection.next().await {
        let notification: AgentMessage = from_slice(&bin_data).unwrap();

        // Assert that the received message is a ProjectFileChangedNotification
        if let AgentMessage::ProjectFileChangedNotification(_) = notification {
            println!("Received ProjectFileChangedNotification");
        } else {
            panic!("Unexpected message type received");
        }
    } else {
        panic!("No message received from server");
    }

    // Close the WebSocket connection
    connection.close().await.expect("Failed to close WebSocket");

    // Cleanup test directory
    dir.close().unwrap();
}
