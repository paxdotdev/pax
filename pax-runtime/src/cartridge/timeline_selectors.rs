use super::*;

#[derive(Default)]
pub(super) struct TimelineSelectors(BTreeMap<String, PropertyTimelines>);

#[derive(Default)]
struct PropertyTimelines {
    tracks: Vec<SelectorTrack>,
    definitions: RefCell<HashMap<SelectedTracks, RuntimeResolvedPropertyEntry>>,
}

struct SelectorTrack {
    selector: SelectorExpr,
    phase: Option<u64>,
    location: Option<LocationInfo>,
    track: TimelineTrackDefinition,
}

// Only authored tracks that contribute to the final property are part of its identity.
// Class values, playheads and per-instance bindings never enter this cache.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
struct SelectedTracks {
    ordinary: Option<usize>,
    enter: Option<usize>,
    exit: Option<usize>,
}

impl TimelineSelectors {
    pub(super) fn new(
        node: &TemplateNodeDefinition,
        timelines: &[TimelineDefinition],
        settings: Option<&PreparedRuntimeSettings>,
    ) -> Self {
        let mut properties = Self::default();
        let empty_base = BTreeMap::new();
        for timeline in timelines {
            let phase = timeline_transition_kind(timeline, settings);
            for element in &timeline.elements {
                let TimelineBlockElement::SelectorBlock(target, block) = element else {
                    continue;
                };
                // The id is fixed by this template. Classes are matched again on
                // every resolve, including the class snapshot for lifecycle tracks.
                let Ok(selector) = SelectorExpr::parse(&target.token_value) else {
                    continue;
                };
                match &selector {
                    SelectorExpr::Id(id)
                        if node
                            .selector_info
                            .id
                            .as_ref()
                            .is_some_and(|token| &token.token_value == id) => {}
                    SelectorExpr::Class(_) => {}
                    _ => continue,
                }
                for element in &block.elements {
                    let TimelineSelectorElement::Track(property, track) = element else {
                        continue;
                    };
                    let track = if phase.is_some() {
                        pax_manifest::PaxManifest::prepare_transition_track(
                            timeline,
                            track,
                            &empty_base,
                            &property.token_value,
                        )
                    } else {
                        pax_manifest::PaxManifest::prepare_timeline_track(timeline, track)
                    };
                    properties
                        .0
                        .entry(property.token_value.clone())
                        .or_default()
                        .tracks
                        .push(SelectorTrack {
                            selector: selector.clone(),
                            phase,
                            location: property.token_location.clone(),
                            track,
                        });
                }
            }
        }
        properties
    }

    pub(super) fn append(
        &self,
        columns: &mut RuntimeResolvedPropertyColumns,
        classes: &[String],
        transition_classes: Option<&[String]>,
    ) {
        for (name, property) in &self.0 {
            let mut selected = SelectedTracks::default();
            for (index, track) in property.tracks.iter().enumerate() {
                if let SelectorExpr::Class(class) = &track.selector {
                    let applicable = track.phase.and(transition_classes).unwrap_or(classes);
                    if !applicable.contains(class) {
                        continue;
                    }
                }
                match track.phase {
                    Some(TRANSITION_PHASE_ENTER) => selected.enter = Some(index),
                    Some(TRANSITION_PHASE_EXIT) => selected.exit = Some(index),
                    // A later ordinary timeline replaces both earlier lifecycle tracks.
                    _ => {
                        selected = SelectedTracks {
                            ordinary: Some(index),
                            ..Default::default()
                        }
                    }
                }
            }
            let Some(last) = selected.ordinary.max(selected.enter).max(selected.exit) else {
                continue;
            };
            let mut definitions = property.definitions.borrow_mut();
            let entry = definitions.entry(selected).or_insert_with(|| {
                let tracks = &property.tracks;
                let last = &tracks[last];
                let value = if selected.enter.is_some() || selected.exit.is_some() {
                    ValueDefinition::Transition(TransitionDefinition {
                        starting_value: selected
                            .ordinary
                            .map(|i| Box::new(ValueDefinition::Timeline(tracks[i].track.clone()))),
                        enter: selected.enter.map(|i| tracks[i].track.clone()),
                        exit: selected.exit.map(|i| tracks[i].track.clone()),
                    })
                } else {
                    ValueDefinition::Timeline(last.track.clone())
                };
                runtime_property_entry(
                    RuntimeSettingsSource::ComponentSettings,
                    Some(last.selector.clone()),
                    last.location.clone(),
                    None,
                    value,
                    None,
                    None,
                )
            });
            append_resolved_property_entry(columns, name, entry.clone());
        }
    }
}

fn timeline_transition_kind(
    timeline: &TimelineDefinition,
    settings: Option<&PreparedRuntimeSettings>,
) -> Option<u64> {
    let timeline_name = timeline.name.as_ref()?.token_value.as_str();
    settings?
        .0
        .transitions
        .iter()
        .find_map(|(name, phase)| (name == timeline_name).then_some(*phase))
}
