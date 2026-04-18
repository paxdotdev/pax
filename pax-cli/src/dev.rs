use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use clap::{App, Arg, ArgMatches, SubCommand};
use color_eyre::eyre::{eyre, Report, Result};
use pax_compiler::dev_session::{
    self, list_registered_sessions, project_dev_dir, read_project_active_session,
    remove_registered_session, session_request_dir, session_response_dir, DevInspectTreeRequest,
    DevInspectTreeResponse, DevLogsRequest, DevLogsResponse, DevLookRequest, DevLookResponse,
    DevRayCastRequest, DevRayCastResponse, DevReplaceNodeRequest, DevReplaceNodeResponse,
    DevSelectorQueryRequest, DevSelectorQueryResponse, DevSession,
};
use pax_manifest::PaxManifest;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub fn command() -> App<'static, 'static> {
    SubCommand::with_name("dev")
        .about("Developer tooling for running designtime Pax sessions")
        .subcommand(list_command())
        .subcommand(status_command())
        .subcommand(look_command())
        .subcommand(logs_command())
        .subcommand(ray_cast_command())
        .subcommand(selector_command())
        .subcommand(inspect_command())
        .subcommand(touch_command())
}

fn list_command() -> App<'static, 'static> {
    SubCommand::with_name("list")
        .about("List live Pax dev sessions discoverable on this machine")
        .arg(
            Arg::with_name("json")
                .long("json")
                .takes_value(false)
                .help("Print JSON instead of a table"),
        )
}

fn touch_command() -> App<'static, 'static> {
    SubCommand::with_name("touch")
        .about("Apply mutating changes to a running Pax dev session")
        .subcommand(
            SubCommand::with_name("apply-component-source")
                .about("Replace a component's .pax source and let designtime hot-reload it")
                .arg(arg_path())
                .arg(arg_session())
                .arg(
                    Arg::with_name("component")
                        .long("component")
                        .takes_value(true)
                        .required(true)
                        .help("PascalCase component name to replace"),
                )
                .arg(
                    Arg::with_name("source")
                        .long("source")
                        .takes_value(true)
                        .conflicts_with("source-file")
                        .help("Inline component source"),
                )
                .arg(
                    Arg::with_name("source-file")
                        .long("source-file")
                        .takes_value(true)
                        .conflicts_with("source")
                        .help("Path to a file containing the replacement source, or - for stdin"),
                ),
        )
        .subcommand(
            SubCommand::with_name("replace-node")
                .about("Replace a single live template node with a Pax subtemplate string")
                .arg(arg_path())
                .arg(arg_session())
                .arg(
                    Arg::with_name("component")
                        .long("component")
                        .takes_value(true)
                        .required(true)
                        .help("Containing component type id from `pax dev inspect tree`, for example `crate::Example`"),
                )
                .arg(
                    Arg::with_name("template-node-id")
                        .long("template-node-id")
                        .takes_value(true)
                        .required(true)
                        .help("Template node id from `pax dev inspect tree`"),
                )
                .arg(
                    Arg::with_name("source")
                        .long("source")
                        .takes_value(true)
                        .conflicts_with("source-file")
                        .help("Inline Pax subtemplate string; empty string deletes the node"),
                )
                .arg(
                    Arg::with_name("source-file")
                        .long("source-file")
                        .takes_value(true)
                        .conflicts_with("source")
                        .help("Path to a file containing the replacement subtemplate, or - for stdin"),
                )
                .arg(
                    Arg::with_name("timeout-ms")
                        .long("timeout-ms")
                        .takes_value(true)
                        .help("How long to wait for the running app to fulfill the request"),
                ),
        )
}

fn status_command() -> App<'static, 'static> {
    SubCommand::with_name("status")
        .about("Print the selected Pax dev session")
        .arg(arg_path())
        .arg(arg_session())
}

fn look_command() -> App<'static, 'static> {
    SubCommand::with_name("look")
        .about("Observe a running app by capturing screenshots over time")
        .arg(arg_path())
        .arg(arg_session())
        .arg(
            Arg::with_name("scale")
                .long("scale")
                .takes_value(true)
                .default_value("1.0")
                .help("Output scale multiplier for captures"),
        )
        .arg(
            Arg::with_name("period-ms")
                .long("period-ms")
                .takes_value(true)
                .default_value("0")
                .help("Milliseconds between screenshot samples; set with --duration-ms to watch over time"),
        )
        .arg(
            Arg::with_name("duration-ms")
                .long("duration-ms")
                .takes_value(true)
                .default_value("0")
                .help("Total observation window in milliseconds; 0 captures a single frame"),
        )
        .arg(
            Arg::with_name("format")
                .long("format")
                .takes_value(true)
                .default_value("png")
                .help("Capture format: png or jpeg"),
        )
        .arg(
            Arg::with_name("quality")
                .long("quality")
                .takes_value(true)
                .help("JPEG quality between 0.0 and 1.0"),
        )
        .arg(
            Arg::with_name("output-dir")
                .long("output-dir")
                .takes_value(true)
                .help("Directory where capture files should be written"),
        )
        .arg(
            Arg::with_name("timeout-ms")
                .long("timeout-ms")
                .takes_value(true)
                .help("How long to wait for the running app to fulfill the request"),
        )
}

fn logs_command() -> App<'static, 'static> {
    SubCommand::with_name("logs")
        .about("Print recent logs captured from a running web Pax dev session")
        .arg(arg_path())
        .arg(arg_session())
        .arg(
            Arg::with_name("follow")
                .long("follow")
                .takes_value(false)
                .help("Poll for new log entries until interrupted"),
        )
        .arg(
            Arg::with_name("limit")
                .long("limit")
                .takes_value(true)
                .default_value("200")
                .help("Maximum number of log entries to fetch per poll"),
        )
        .arg(
            Arg::with_name("json")
                .long("json")
                .takes_value(false)
                .help("Print raw JSON instead of human-readable lines"),
        )
        .arg(
            Arg::with_name("poll-ms")
                .long("poll-ms")
                .takes_value(true)
                .default_value("500")
                .help("Milliseconds between polls when --follow is set"),
        )
        .arg(
            Arg::with_name("timeout-ms")
                .long("timeout-ms")
                .takes_value(true)
                .help("How long to wait for each running app response"),
        )
}

fn ray_cast_command() -> App<'static, 'static> {
    SubCommand::with_name("ray-cast")
        .about("Return the z-sorted stack of expanded nodes beneath a window-space point")
        .arg(arg_path())
        .arg(arg_session())
        .arg(
            Arg::with_name("x")
                .long("x")
                .takes_value(true)
                .required(true)
                .help("Window-space x coordinate in px"),
        )
        .arg(
            Arg::with_name("y")
                .long("y")
                .takes_value(true)
                .required(true)
                .help("Window-space y coordinate in px"),
        )
        .arg(
            Arg::with_name("hit-invisible")
                .long("hit-invisible")
                .takes_value(false)
                .help("Include nodes that are normally invisible to ray-casting"),
        )
        .arg(
            Arg::with_name("timeout-ms")
                .long("timeout-ms")
                .takes_value(true)
                .help("How long to wait for the running app to fulfill the request"),
        )
}

fn selector_command() -> App<'static, 'static> {
    SubCommand::with_name("selector")
        .about("Query the live expanded tree by selector")
        .arg(arg_path())
        .arg(arg_session())
        .arg(
            Arg::with_name("selector")
                .required(true)
                .help("Selector string: #id, .class, or a type name like Ellipse"),
        )
        .arg(
            Arg::with_name("timeout-ms")
                .long("timeout-ms")
                .takes_value(true)
                .help("How long to wait for the running app to fulfill the request"),
        )
}

fn inspect_command() -> App<'static, 'static> {
    SubCommand::with_name("inspect")
        .about("Inspect a running Pax dev session")
        .subcommand(
            SubCommand::with_name("tree")
                .about("Print the current expanded userland tree as machine-friendly JSON")
                .arg(arg_path())
                .arg(arg_session())
                .arg(
                    Arg::with_name("max-depth")
                        .long("max-depth")
                        .takes_value(true)
                        .help("Maximum child depth to include; 0 prints only the root node"),
                )
                .arg(
                    Arg::with_name("timeout-ms")
                        .long("timeout-ms")
                        .takes_value(true)
                        .help("How long to wait for the running app to fulfill the request"),
                ),
        )
}

pub fn handle(
    args: &ArgMatches<'_>,
    process_child_ids: Arc<Mutex<Vec<u64>>>,
) -> Result<(), Report> {
    match args.subcommand() {
        ("list", Some(sub_args)) => handle_list(sub_args),
        ("status", Some(sub_args)) => handle_status(sub_args),
        ("look", Some(sub_args)) => handle_look(sub_args),
        ("logs", Some(sub_args)) => handle_logs(sub_args),
        ("ray-cast", Some(sub_args)) => handle_ray_cast(sub_args),
        ("selector", Some(sub_args)) => handle_selector(sub_args),
        ("inspect", Some(sub_args)) => handle_inspect(sub_args),
        ("touch", Some(sub_args)) => handle_touch(sub_args, process_child_ids),
        _ => Err(eyre!("unknown dev subcommand")),
    }
}

fn handle_list(args: &ArgMatches<'_>) -> Result<(), Report> {
    let sessions = live_registered_sessions()?;
    if args.is_present("json") {
        return print_json(&sessions);
    }

    let mut rows = vec![vec![
        "SESSION ID".to_string(),
        "PLATFORM".to_string(),
        "LOCATION".to_string(),
        "PROJECT".to_string(),
        "WS URL".to_string(),
    ]];
    for session in sessions {
        rows.push(vec![
            session.session_id,
            session.platform,
            session.location.unwrap_or_else(|| "-".to_string()),
            session
                .project_root
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "-".to_string()),
            session
                .design_server_addr
                .unwrap_or_else(|| "-".to_string()),
        ]);
    }

    print_table(rows)
}

fn handle_status(args: &ArgMatches<'_>) -> Result<(), Report> {
    let session = resolve_session(args)?;
    print_json(&session)
}

fn handle_look(args: &ArgMatches<'_>) -> Result<(), Report> {
    let session = resolve_session(args)?;
    let request_id = format!("look-{}", dev_session::now_ms());

    let scale = parse_f64(args, "scale")?;
    if scale <= 0.0 {
        return Err(eyre!("--scale must be greater than 0"));
    }

    let format = args.value_of("format").unwrap().to_lowercase();
    if format != "png" && format != "jpeg" {
        return Err(eyre!("--format must be either png or jpeg"));
    }

    let period_ms = parse_u64(args, "period-ms")?;
    let duration_ms = parse_u64(args, "duration-ms")?;
    match (period_ms, duration_ms) {
        (0, 0) => {}
        (0, _) => {
            return Err(eyre!(
                "--period-ms must be greater than 0 when --duration-ms is set"
            ));
        }
        (_, 0) => {
            return Err(eyre!(
                "--duration-ms must be greater than 0 when --period-ms is set"
            ));
        }
        _ => {}
    }

    let quality = args
        .value_of("quality")
        .map(|_| parse_f64(args, "quality"))
        .transpose()?;

    let session_dir = session.session_dir.as_ref().ok_or_else(|| {
        eyre!(
            "session {} does not expose a local session directory",
            session.session_id
        )
    })?;
    let output_dir = args
        .value_of("output-dir")
        .map(PathBuf::from)
        .map(|path| absolutize(&std::env::current_dir().unwrap_or_default(), path))
        .unwrap_or_else(|| session_dir.join("captures").join(&request_id));
    fs::create_dir_all(&output_dir)?;

    let request = DevLookRequest {
        request_id: request_id.clone(),
        kind: "look".to_string(),
        output_dir,
        scale,
        period_ms,
        duration_ms,
        format,
        quality,
    };

    write_request(&session, &request_id, &request)?;
    let timeout_ms = args
        .value_of("timeout-ms")
        .map(|_| parse_u64(args, "timeout-ms"))
        .transpose()?
        .unwrap_or_else(|| duration_ms.saturating_add(period_ms).saturating_add(5_000));
    let response: DevLookResponse =
        wait_for_response(&session, &request_id, Duration::from_millis(timeout_ms))?;
    if response.status != "ok" {
        return Err(eyre!(
            "{}",
            response
                .error
                .unwrap_or_else(|| "look request failed".to_string())
        ));
    }

    print_json(&response)
}

fn handle_logs(args: &ArgMatches<'_>) -> Result<(), Report> {
    let session = resolve_session(args)?;
    if session.platform != "web" {
        return Err(eyre!(
            "pax dev logs is currently supported for web dev sessions only"
        ));
    }

    let follow = args.is_present("follow");
    let as_json = args.is_present("json");
    let limit = parse_usize(args, "limit")?;
    if limit == 0 {
        return Err(eyre!("--limit must be greater than 0"));
    }
    let poll_ms = parse_u64(args, "poll-ms")?;
    let timeout = Duration::from_millis(
        args.value_of("timeout-ms")
            .map(|_| parse_u64(args, "timeout-ms"))
            .transpose()?
            .unwrap_or(5_000),
    );

    let mut since_seq = None;
    loop {
        let response = request_logs(&session, since_seq, limit, timeout)?;
        if response.status != "ok" {
            return Err(eyre!(
                "{}",
                response
                    .error
                    .unwrap_or_else(|| "logs request failed".to_string())
            ));
        }

        if as_json {
            print_json(&response)?;
        } else {
            print_log_lines(&response)?;
        }

        since_seq = Some(response.next_seq.saturating_sub(1));
        if !follow {
            break;
        }
        thread::sleep(Duration::from_millis(poll_ms));
    }

    Ok(())
}

fn handle_ray_cast(args: &ArgMatches<'_>) -> Result<(), Report> {
    let session = resolve_session(args)?;
    let request_id = format!("ray-cast-{}", dev_session::now_ms());
    let x = parse_f64(args, "x")?;
    let y = parse_f64(args, "y")?;
    let hit_invisible = args.is_present("hit-invisible");

    let request = DevRayCastRequest {
        request_id: request_id.clone(),
        kind: "ray-cast".to_string(),
        x,
        y,
        hit_invisible,
    };

    write_request(&session, &request_id, &request)?;
    let timeout_ms = args
        .value_of("timeout-ms")
        .map(|_| parse_u64(args, "timeout-ms"))
        .transpose()?
        .unwrap_or(5_000);
    let response: DevRayCastResponse =
        wait_for_response(&session, &request_id, Duration::from_millis(timeout_ms))?;
    if response.status != "ok" {
        return Err(eyre!(
            "{}",
            response
                .error
                .unwrap_or_else(|| "ray-cast request failed".to_string())
        ));
    }

    let nodes = parse_dev_nodes_json(
        response.nodes_json,
        "ray-cast response did not include nodes_json",
    )?;
    print_json(&serde_json::json!({
        "session_id": session.session_id,
        "x": response.x,
        "y": response.y,
        "hit_invisible": response.hit_invisible,
        "node_count": response.node_count,
        "nodes": nodes,
    }))
}

fn handle_selector(args: &ArgMatches<'_>) -> Result<(), Report> {
    let session = resolve_session(args)?;
    let request_id = format!("selector-query-{}", dev_session::now_ms());
    let selector = args.value_of("selector").unwrap().to_string();

    let request = DevSelectorQueryRequest {
        request_id: request_id.clone(),
        kind: "selector-query".to_string(),
        selector,
    };

    write_request(&session, &request_id, &request)?;
    let timeout_ms = args
        .value_of("timeout-ms")
        .map(|_| parse_u64(args, "timeout-ms"))
        .transpose()?
        .unwrap_or(5_000);
    let response: DevSelectorQueryResponse =
        wait_for_response(&session, &request_id, Duration::from_millis(timeout_ms))?;
    if response.status != "ok" {
        return Err(eyre!(
            "{}",
            response
                .error
                .unwrap_or_else(|| "selector request failed".to_string())
        ));
    }

    let nodes = parse_dev_nodes_json(
        response.nodes_json,
        "selector response did not include nodes_json",
    )?;
    print_json(&serde_json::json!({
        "session_id": session.session_id,
        "selector": response.selector,
        "node_count": response.node_count,
        "nodes": nodes,
    }))
}

fn handle_inspect(args: &ArgMatches<'_>) -> Result<(), Report> {
    match args.subcommand() {
        ("tree", Some(sub_args)) => handle_inspect_tree(sub_args),
        _ => Err(eyre!("unknown dev inspect subcommand")),
    }
}

fn handle_inspect_tree(args: &ArgMatches<'_>) -> Result<(), Report> {
    let session = resolve_session(args)?;
    let request_id = format!("inspect-tree-{}", dev_session::now_ms());
    let max_depth = args
        .value_of("max-depth")
        .map(|_| parse_usize(args, "max-depth"))
        .transpose()?;

    let request = DevInspectTreeRequest {
        request_id: request_id.clone(),
        kind: "inspect-tree".to_string(),
        max_depth,
    };

    write_request(&session, &request_id, &request)?;
    let timeout_ms = args
        .value_of("timeout-ms")
        .map(|_| parse_u64(args, "timeout-ms"))
        .transpose()?
        .unwrap_or(5_000);
    let response: DevInspectTreeResponse =
        wait_for_response(&session, &request_id, Duration::from_millis(timeout_ms))?;
    if response.status != "ok" {
        return Err(eyre!(
            "{}",
            response
                .error
                .unwrap_or_else(|| "inspect tree request failed".to_string())
        ));
    }

    let tree_json = response
        .tree_json
        .ok_or_else(|| eyre!("inspect tree response did not include tree_json"))?;
    let tree: serde_json::Value = serde_json::from_str(&tree_json)?;
    print_json(&serde_json::json!({
        "session_id": session.session_id,
        "node_count": response.node_count,
        "tree": tree,
    }))
}

fn handle_touch(
    args: &ArgMatches<'_>,
    process_child_ids: Arc<Mutex<Vec<u64>>>,
) -> Result<(), Report> {
    match args.subcommand() {
        ("apply-component-source", Some(sub_args)) => {
            handle_apply_component_source(sub_args, process_child_ids)
        }
        ("replace-node", Some(sub_args)) => handle_replace_node(sub_args),
        _ => Err(eyre!("unknown dev touch subcommand")),
    }
}

fn handle_apply_component_source(
    args: &ArgMatches<'_>,
    process_child_ids: Arc<Mutex<Vec<u64>>>,
) -> Result<(), Report> {
    let session = resolve_session(args)?;
    let project_root = session_project_root(&session, args)?;
    let component_name = args.value_of("component").unwrap();
    let source = read_component_source(args)?;

    pax_language::parse_pax_str(pax_language::Rule::pax_component_definition, &source)
        .map_err(|err| eyre!("replacement source failed to parse: {err}"))?;

    let manifests = parse_manifests(&project_root, process_child_ids)?;
    let mut matches = vec![];
    for manifest in manifests {
        for component in manifest.components.values() {
            if component.type_id.get_pascal_identifier().as_deref() == Some(component_name) {
                if let Some(path) = component
                    .template
                    .as_ref()
                    .and_then(|template| template.get_file_path())
                {
                    matches.push(PathBuf::from(path));
                }
            }
        }
    }

    let Some(component_path) = (match matches.len() {
        0 => None,
        1 => matches.into_iter().next(),
        _ => {
            return Err(eyre!(
                "component {} matched multiple files; rename the component or make the target more specific",
                component_name
            ))
        }
    }) else {
        return Err(eyre!("could not find component {}", component_name));
    };

    atomic_write_string(&component_path, &source)?;
    print_json(&serde_json::json!({
        "status": "ok",
        "session_id": session.session_id,
        "component": component_name,
        "path": component_path,
    }))
}

fn handle_replace_node(args: &ArgMatches<'_>) -> Result<(), Report> {
    let session = resolve_session(args)?;
    let request_id = format!("replace-node-{}", dev_session::now_ms());
    let component_type_id = args.value_of("component").unwrap().to_string();
    let template_node_id = parse_usize(args, "template-node-id")?;
    let subtemplate = read_component_source(args)?;

    if !subtemplate.trim().is_empty() {
        pax_language::parse_pax_str(pax_language::Rule::pax_component_definition, &subtemplate)
            .map_err(|err| eyre!("replacement subtemplate failed to parse: {err}"))?;
    }

    let request = DevReplaceNodeRequest {
        request_id: request_id.clone(),
        kind: "replace-node".to_string(),
        component_type_id,
        template_node_id,
        subtemplate,
    };

    write_request(&session, &request_id, &request)?;
    let timeout_ms = args
        .value_of("timeout-ms")
        .map(|_| parse_u64(args, "timeout-ms"))
        .transpose()?
        .unwrap_or(10_000);
    let response: DevReplaceNodeResponse =
        wait_for_response(&session, &request_id, Duration::from_millis(timeout_ms))?;
    if response.status != "ok" {
        return Err(eyre!(
            "{}",
            response
                .error
                .unwrap_or_else(|| "replace-node request failed".to_string())
        ));
    }

    print_json(&response)
}

fn request_logs(
    session: &DevSession,
    since_seq: Option<u64>,
    limit: usize,
    timeout: Duration,
) -> Result<DevLogsResponse, Report> {
    let request_id = format!("logs-{}", dev_session::now_ms());
    let request = DevLogsRequest {
        request_id: request_id.clone(),
        kind: "logs".to_string(),
        since_seq,
        limit: Some(limit),
    };

    write_request(session, &request_id, &request)?;
    wait_for_response_and_cleanup(session, &request_id, timeout)
}

fn parse_manifests(
    project_root: &PathBuf,
    process_child_ids: Arc<Mutex<Vec<u64>>>,
) -> Result<Vec<PaxManifest>, Report> {
    let output = pax_compiler::run_parser_binary(project_root, process_child_ids, true, false);
    std::io::stderr().write_all(output.stderr.as_slice())?;
    if !output.status.success() {
        return Err(eyre!(
            "failed to parse the Pax project before applying source"
        ));
    }
    let stdout = String::from_utf8(output.stdout)?;
    Ok(serde_json::from_str(&stdout)?)
}

fn read_component_source(args: &ArgMatches<'_>) -> Result<String, Report> {
    if let Some(source) = args.value_of("source") {
        return Ok(source.to_string());
    }

    let source_file = args
        .value_of("source-file")
        .ok_or_else(|| eyre!("either --source or --source-file is required"))?;
    if source_file == "-" {
        let mut stdin = String::new();
        std::io::stdin().read_to_string(&mut stdin)?;
        return Ok(stdin);
    }

    Ok(fs::read_to_string(source_file)?)
}

fn live_registered_sessions() -> Result<Vec<DevSession>, Report> {
    let now = dev_session::now_ms();
    let mut live = vec![];
    for session in list_registered_sessions()? {
        if session.is_stale(now) {
            let _ = remove_registered_session(&session.session_id);
            continue;
        }
        live.push(session);
    }
    Ok(live)
}

fn resolve_session(args: &ArgMatches<'_>) -> Result<DevSession, Report> {
    let live_sessions = live_registered_sessions()?;
    if let Some(session_id) = args.value_of("session") {
        return live_sessions
            .into_iter()
            .find(|session| session.session_id == session_id)
            .ok_or_else(|| eyre!("no live dev session with id {}", session_id));
    }

    if let Some(project_session) = resolve_project_session(args, &live_sessions)? {
        return Ok(project_session);
    }

    match live_sessions.len() {
        0 => Err(eyre!(
            "no live dev sessions found; run `pax run` with designtime enabled first"
        )),
        1 => Ok(live_sessions.into_iter().next().unwrap()),
        _ => Err(eyre!(
            "multiple live dev sessions found; run `pax dev list` and pass --session <id>"
        )),
    }
}

fn resolve_project_session(
    args: &ArgMatches<'_>,
    live_sessions: &[DevSession],
) -> Result<Option<DevSession>, Report> {
    let project_root = project_root(args)?;
    let pax_dir = project_root.join(".pax");
    if !project_dev_dir(&pax_dir).exists() {
        return Ok(None);
    }
    let Some(active) = read_project_active_session(&pax_dir)? else {
        return Ok(None);
    };
    Ok(live_sessions
        .iter()
        .find(|session| session.session_id == active.session_id)
        .cloned())
}

fn session_project_root(session: &DevSession, args: &ArgMatches<'_>) -> Result<PathBuf, Report> {
    if let Some(project_root) = &session.project_root {
        return Ok(project_root.clone());
    }
    project_root(args)
}

fn write_request<T: Serialize>(
    session: &DevSession,
    request_id: &str,
    request: &T,
) -> Result<(), Report> {
    let request_dir = session_request_dir(session)?;
    fs::create_dir_all(&request_dir)?;
    let request_path = request_dir.join(format!("{request_id}.json"));
    atomic_write_bytes(&request_path, &serde_json::to_vec_pretty(request)?)?;
    Ok(())
}

fn wait_for_response<T: DeserializeOwned>(
    session: &DevSession,
    request_id: &str,
    timeout: Duration,
) -> Result<T, Report> {
    let response_path = session_response_dir(session)?.join(format!("{request_id}.json"));
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if response_path.exists() {
            let response: T = serde_json::from_slice(&fs::read(&response_path)?)?;
            return Ok(response);
        }
        thread::sleep(Duration::from_millis(50));
    }

    Err(eyre!(
        "timed out waiting for the running app to fulfill dev request {}",
        request_id
    ))
}

fn wait_for_response_and_cleanup<T: DeserializeOwned>(
    session: &DevSession,
    request_id: &str,
    timeout: Duration,
) -> Result<T, Report> {
    let response_path = session_response_dir(session)?.join(format!("{request_id}.json"));
    let response = wait_for_response(session, request_id, timeout)?;
    let _ = fs::remove_file(response_path);
    Ok(response)
}

fn project_root(args: &ArgMatches<'_>) -> Result<PathBuf, Report> {
    let path = PathBuf::from(args.value_of("path").unwrap_or("."));
    let joined = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(path)
    };
    Ok(joined)
}

fn arg_path() -> Arg<'static, 'static> {
    Arg::with_name("path")
        .short("p")
        .long("path")
        .takes_value(true)
        .default_value(".")
        .help("Project path used to resolve the active local session when --session is omitted")
}

fn arg_session() -> Arg<'static, 'static> {
    Arg::with_name("session")
        .short("s")
        .long("session")
        .takes_value(true)
        .help("Explicit dev session id to target; overrides project-based resolution")
}

fn parse_u64(args: &ArgMatches<'_>, name: &str) -> Result<u64, Report> {
    Ok(args
        .value_of(name)
        .unwrap()
        .parse::<u64>()
        .map_err(|_| eyre!("{} must be an unsigned integer", name))?)
}

fn parse_usize(args: &ArgMatches<'_>, name: &str) -> Result<usize, Report> {
    Ok(args
        .value_of(name)
        .unwrap()
        .parse::<usize>()
        .map_err(|_| eyre!("{} must be an unsigned integer", name))?)
}

fn parse_f64(args: &ArgMatches<'_>, name: &str) -> Result<f64, Report> {
    Ok(args
        .value_of(name)
        .unwrap()
        .parse::<f64>()
        .map_err(|_| eyre!("{} must be a number", name))?)
}

fn absolutize(base: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}

fn atomic_write_string(path: &Path, contents: &str) -> Result<(), Report> {
    atomic_write_bytes(path, contents.as_bytes())
}

fn atomic_write_bytes(path: &Path, contents: &[u8]) -> Result<(), Report> {
    let tmp_path = path.with_extension("tmp");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&tmp_path, contents)?;
    fs::rename(tmp_path, path)?;
    Ok(())
}

fn print_json<T: Serialize>(value: &T) -> Result<(), Report> {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    lock.write_all(b"\n")?;
    Ok(())
}

fn print_log_lines(response: &DevLogsResponse) -> Result<(), Report> {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    for entry in &response.entries {
        writeln!(
            lock,
            "[{}] {} {}",
            entry.seq,
            entry.level.to_uppercase(),
            entry.message
        )?;
    }
    Ok(())
}

fn parse_dev_nodes_json(
    nodes_json: Option<String>,
    missing_message: &str,
) -> Result<serde_json::Value, Report> {
    let nodes_json = nodes_json.ok_or_else(|| eyre!(missing_message.to_string()))?;
    Ok(serde_json::from_str(&nodes_json)?)
}

fn print_table(rows: Vec<Vec<String>>) -> Result<(), Report> {
    if rows.is_empty() {
        return Ok(());
    }

    let column_count = rows.iter().map(|row| row.len()).max().unwrap_or(0);
    let mut widths = vec![0usize; column_count];
    for row in &rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(cell.len());
        }
    }

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    for (row_index, row) in rows.iter().enumerate() {
        for column in 0..column_count {
            let cell = row.get(column).cloned().unwrap_or_default();
            let width = widths[column];
            if column + 1 == column_count {
                write!(lock, "{cell}")?;
            } else {
                write!(lock, "{cell:width$}  ", width = width)?;
            }
        }
        lock.write_all(b"\n")?;
        if row_index == 0 {
            for (column, width) in widths.iter().enumerate() {
                let divider = "-".repeat(*width);
                if column + 1 == widths.len() {
                    write!(lock, "{divider}")?;
                } else {
                    write!(lock, "{divider}  ")?;
                }
            }
            lock.write_all(b"\n")?;
        }
    }
    Ok(())
}
