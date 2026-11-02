#![cfg(feature = "compiler")]

use pax_manifest::{binary, cartridge_generation::*, parsing::*, program_ir, rust_manifest, *};
use std::collections::{BTreeMap, HashMap};

fn manifest(body: &str) -> PaxManifest {
    parse_manifest(&format!(
        "<Group id=panel @in=enter @out=exit />
         @settings {{ @in: enter @out: exit }}
         @timeline enter {{ {body} }}
         @timeline exit {{ {body} }}"
    ))
}

fn parse_manifest(source: &str) -> PaxManifest {
    let id = TypeId::build_singleton("Example", Some("Example"));
    let (_, component) = assemble_component_definition(
        ParsingContext::default(),
        source,
        false,
        HashMap::from([(
            "Group".into(),
            TypeId::build_singleton("Group", Some("Group")),
        )]),
        HashMap::new(),
        None,
        "crate",
        id.clone(),
        "transition_duration.pax",
        file!(),
    );
    PaxManifest {
        components: BTreeMap::from([(id.clone(), component)]),
        main_component_type_id: id,
        type_table: Default::default(),
        assets_dirs: vec![],
        engine_import_path: "pax_engine".into(),
    }
}

fn assert_clocks(body: &str, frames: u64, millis: Option<u64>, playheads: &[&str]) {
    let rich = manifest(body);
    let decoded = binary::from_slice(&binary::to_vec(&rich).unwrap()).unwrap();
    let ir = program_ir::ProgramIR::from_manifest(&rich);
    let baked_ir =
        program_ir::binary::from_slice(&program_ir::binary::to_vec(&ir).unwrap()).unwrap();
    // ProgramIR retains the same runtime timeline payload, with source locations stripped.
    for (id, component) in &ir.components {
        assert_eq!(
            serde_json::to_value(&component.timelines).unwrap(),
            serde_json::to_value(&baked_ir.components[id].timelines).unwrap()
        );
    }
    syn::parse_str::<syn::Expr>(&rust_manifest::to_rust_expression(&rich)).unwrap();
    for manifest in [&rich, &decoded] {
        let id = &manifest.main_component_type_id;
        let component = &manifest.components[id];
        let panel = &component.template.as_ref().unwrap().get_root()[0];
        for config in [
            manifest.get_component_transition_config(id),
            manifest.get_template_node_transition_config(id, panel),
        ] {
            assert!(config.has_enter && config.has_exit, "{body}");
            assert_eq!(
                (config.enter_frame_count, config.enter_millis_count),
                (frames, millis),
                "enter: {body}"
            );
            assert_eq!(
                (config.exit_frame_count, config.exit_millis_count),
                (frames, millis),
                "exit: {body}"
            );
        }
        let timeline = &component.timelines[0];
        let prepared: Vec<_> = timeline
            .elements
            .iter()
            .flat_map(|element| {
                let TimelineBlockElement::SelectorBlock(_, block) = element else {
                    return vec![];
                };
                block
                    .elements
                    .iter()
                    .filter_map(|element| {
                        let TimelineSelectorElement::Track(property, track) = element else {
                            return None;
                        };
                        let track = PaxManifest::prepare_transition_track(
                            timeline,
                            track,
                            &BTreeMap::new(),
                            &property.token_value,
                        );
                        let ValueDefinition::Identifier(playhead) = *track.playhead.unwrap() else {
                            panic!("transition clock");
                        };
                        Some(playhead.name)
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        assert_eq!(prepared, playheads, "{body}");
    }
}

#[test]
fn literal_wall_time_transitions_do_not_require_nominal_frame_ticks() {
    for duration in ["1000ms", "1s"] {
        assert_clocks(
            &format!("duration: {duration}, self {{ opacity: {{ 0%: 1, 100%: 0 }} }}"),
            0,
            Some(1000),
            &[TRANSITION_PLAYHEAD_MILLIS_SYMBOL],
        );
    }
}

#[test]
fn inline_website_exit_uses_only_authored_milliseconds() {
    let manifest = parse_manifest(
        r#"
        <Group @out=@timeline {
            duration: 1000ms,
            opacity: { 0: 1, 100%: 0 },
        } />
    "#,
    );
    let id = &manifest.main_component_type_id;
    let panel = &manifest.components[id]
        .template
        .as_ref()
        .unwrap()
        .get_root()[0];
    let config = manifest.get_template_node_transition_config(id, panel);
    assert!(config.has_exit);
    assert_eq!(
        (config.exit_frame_count, config.exit_millis_count),
        (0, Some(1000))
    );
}

#[test]
fn frame_transitions_remain_tick_based() {
    for duration in ["18", "18f"] {
        assert_clocks(
            &format!("duration: {duration}, self {{ opacity: {{ 0%: 1, 100%: 0 }} }}"),
            18,
            None,
            &[TRANSITION_PLAYHEAD_SYMBOL],
        );
    }
}

#[test]
fn mixed_markers_use_the_tracks_clock_without_a_second_completion_gate() {
    assert_clocks(
        "duration: 1000ms, self { opacity: { 0: 1, 30f: 0.5, 100%: 0 } }",
        0,
        Some(1000),
        &[TRANSITION_PLAYHEAD_MILLIS_SYMBOL],
    );
    assert_clocks(
        "duration: 60f, self { opacity: { 0: 1, 1000ms: 0 } }",
        60,
        None,
        &[TRANSITION_PLAYHEAD_SYMBOL],
    );
}

#[test]
fn independent_frame_and_time_tracks_keep_both_completion_gates() {
    assert_clocks("self { opacity: { duration: 20f, 0%: 1, 100%: 0 }, x: { duration: 500ms, 0%: 0px, 100%: 20px } }",
        20, Some(500), &[TRANSITION_PLAYHEAD_SYMBOL, TRANSITION_PLAYHEAD_MILLIS_SYMBOL]);
}

#[test]
fn inferred_time_markers_select_the_time_playhead() {
    assert_clocks(
        "self { opacity: { 0: 1, 500ms: 0 } }",
        0,
        Some(500),
        &[TRANSITION_PLAYHEAD_MILLIS_SYMBOL],
    );
    assert_clocks(
        "self { opacity: { 0%: 1, 50ms: 0.5, 100%: 0 } }",
        0,
        Some(100),
        &[TRANSITION_PLAYHEAD_MILLIS_SYMBOL],
    );
}

#[test]
fn durationless_percent_tracks_keep_the_historical_frame_fallback() {
    assert_clocks(
        "self { opacity: { 0%: 1, 100%: 0 } }",
        100,
        None,
        &[TRANSITION_PLAYHEAD_SYMBOL],
    );
}

#[test]
fn explicit_duration_bounds_completion_even_when_a_marker_is_later() {
    assert_clocks(
        "duration: 1000ms, self { opacity: { 0%: 1, 1500ms: 0 } }",
        0,
        Some(1000),
        &[TRANSITION_PLAYHEAD_MILLIS_SYMBOL],
    );
    assert_clocks(
        "duration: 30f, self { opacity: { 0%: 1, 60f: 0 } }",
        30,
        None,
        &[TRANSITION_PLAYHEAD_SYMBOL],
    );
}

#[test]
fn dynamic_duration_preserves_existing_marker_gates_and_unresolved_fallbacks() {
    assert_clocks(
        "self { opacity: { duration: {self.duration}, 10f: 1, 100%: 0 } }",
        10,
        None,
        &[TRANSITION_PLAYHEAD_SYMBOL],
    );
    assert_clocks(
        "self { opacity: { duration: {self.duration}, 10f: 1, 20f: 0 } }",
        20,
        None,
        &[TRANSITION_PLAYHEAD_SYMBOL],
    );
    assert_clocks(
        "self { opacity: { duration: {self.duration}, 0%: 1, 500ms: 0 } }",
        30,
        Some(500),
        &[TRANSITION_PLAYHEAD_SYMBOL],
    );
    assert_clocks(
        "self { opacity: { duration: {self.duration}, 0%: 1, 100%: 0 } }",
        0,
        None,
        &[TRANSITION_PLAYHEAD_SYMBOL],
    );
    let manifest = manifest("self { opacity: { duration: {self.duration}, 10f: 1, 100%: 0 } }");
    let config = manifest.get_component_transition_config(&manifest.main_component_type_id);
    assert!(matches!(
        config.exit_dynamic_durations.as_slice(),
        [ValueDefinition::Expression(_)]
    ));
    assert!(matches!(
        config.enter_dynamic_durations.as_slice(),
        [ValueDefinition::Expression(_)]
    ));
}

#[test]
fn track_clock_overrides_replace_the_inherited_completion_clock() {
    assert_clocks(
        "duration: 1000ms, self { opacity: { duration: 20f, 0%: 1, 100%: 0 } }",
        20,
        None,
        &[TRANSITION_PLAYHEAD_SYMBOL],
    );
    assert_clocks(
        "duration: 60f, self { opacity: { duration: 500ms, 0%: 1, 100%: 0 } }",
        0,
        Some(500),
        &[TRANSITION_PLAYHEAD_MILLIS_SYMBOL],
    );
}

#[test]
fn trackless_transitions_preserve_the_explicit_duration_gate() {
    assert_clocks("duration: 1000ms, self {}", 0, Some(1000), &[]);
    assert_clocks("duration: 20f, self {}", 20, None, &[]);
}
