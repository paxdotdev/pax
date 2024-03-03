use actix_web::web::Data;
use actix_web::App;
use awc::Client;
use futures_util::{SinkExt, StreamExt};
use pax_design_server::{setup_file_watcher, web_socket, AppState};
use pax_designtime::messages::AgentMessage;
use rmp_serde::from_slice;
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
    let state = Data::new(AppState::new());
    let _watcher =
        setup_file_watcher(state.clone(), dir_path).expect("Failed to setup file watcher");

    // Start test server
    let srv = get_test_server(state.clone());

    // Connect to WebSocket
    let client = Client::new();
    let (_resp, mut connection) = client.ws(srv.url("/ws")).connect().await.unwrap();

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
