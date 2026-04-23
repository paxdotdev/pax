use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

use crate::{
    constants::{COMMON_PROPERTIES, COMMON_PROPERTIES_TYPE},
    PaxManifest, PropertyDefinition, SettingElement, SettingsBlockElement, TemplateNodeDefinition,
    TimelineBlockElement, TimelineDefinition, TimelineSelectorElement, TransitionDefinition,
    TypeId, ValueDefinition,
};

pub const TRANSITION_PHASE_IDLE: u64 = 0;
pub const TRANSITION_PHASE_ENTER: u64 = 1;
pub const TRANSITION_PHASE_EXIT: u64 = 2;
pub const TRANSITION_PLAYHEAD_SYMBOL: &str = "$transition_playhead";
pub const TRANSITION_PHASE_SYMBOL: &str = "$transition_phase";
pub const DEFAULT_OUT_TRANSITION_TIMEOUT_MS: u64 = 5_000;

#[derive(Clone, Default, Debug)]
pub struct ComponentTransitionBindingInfo {
    pub enter: Option<String>,
    pub exit: Option<String>,
}

#[derive(Clone, Default, Debug)]
pub struct ComponentTransitionConfig {
    pub has_enter: bool,
    pub has_exit: bool,
    pub exit_frame_count: u64,
    pub timeout_ms: u64,
}

#[derive(Serialize, Debug)]
/// Template context for generating a component's cartridge code.
pub struct ComponentInfo {
    pub type_id: TypeId,
    pub pascal_identifier: String,
    pub primitive_instance_import_path: Option<String>,
    pub properties: Vec<PropertyInfo>,
    pub handlers: Vec<HandlerInfo>,
}

#[derive(Serialize, Debug)]
/// Property entry passed into cartridge codegen templates.
pub struct PropertyInfo {
    pub name: String,
    pub property_type: PropertyDefinition,
}

#[derive(Serialize, Debug)]
/// Event handler entry passed into cartridge codegen templates.
pub struct HandlerInfo {
    pub name: String,
    pub args_type: Option<String>,
}

impl PaxManifest {
    pub fn get_component_handlers(&self, type_id: &TypeId) -> Vec<(String, Vec<String>)> {
        let mut handlers = Vec::new();
        if let Some(component) = self.components.get(type_id) {
            if let Some(settings) = &component.settings {
                for setting in settings {
                    if let SettingsBlockElement::Handler(key, values) = setting {
                        handlers.push((
                            key.token_value.clone(),
                            values
                                .iter()
                                .map(|v| self.clean_handler(v.token_value.clone()))
                                .collect(),
                        ));
                    }
                }
            }
        }
        handlers
    }

    pub fn event_to_args_map(&self) -> HashMap<String, Option<String>> {
        let mut map = HashMap::new();
        let mut add = |from: &str, to: &str| {
            map.insert(from.to_owned(), Some(to.to_owned()));
        };
        add("scroll", "Scroll");
        add("click_or_tap", "ClickOrTap");
        add("touch_start", "TouchStart");
        add("touch_move", "TouchMove");
        add("touch_end", "TouchEnd");
        add("key_down", "KeyDown");
        add("key_up", "KeyUp");
        add("key_press", "KeyPress");
        add("checkbox_change", "CheckboxChange");
        add("button_click", "ButtonClick");
        add("textbox_change", "TextboxChange");
        add("text_input", "TextInput");
        add("textbox_input", "TextboxInput");
        add("click", "Click");
        add("mouse_down", "MouseDown");
        add("mouse_up", "MouseUp");
        add("mouse_move", "MouseMove");
        add("mouse_over", "MouseOver");
        add("mouse_out", "MouseOut");
        add("double_click", "DoubleClick");
        add("context_menu", "ContextMenu");
        add("wheel", "Wheel");
        add("drop", "Drop");
        add("focused", "Focus");
        add("select_start", "SelectStart");

        map.insert("pre_render".to_string(), None);
        map.insert("mount".to_string(), None);
        map.insert("unmount".to_string(), None);
        map.insert("tick".to_string(), None);
        map
    }

    fn clean_handler(&self, handler: String) -> String {
        handler.replace("self.", "").replace("this.", "")
    }

    pub fn generate_codegen_component_info(&self) -> Vec<ComponentInfo> {
        let mut component_infos = Vec::new();
        let event_map = self.event_to_args_map();

        // get all the properties for this commonent type
        for (type_id, component) in &self.components {
            // No need to generate helpers on simple types
            if component.is_struct_only_component {
                continue;
            }

            let mut properties = Vec::new();
            for property in &self.type_table[type_id].property_definitions {
                properties.push(PropertyInfo {
                    name: property.name.clone(),
                    property_type: property.clone(),
                });
            }

            let mut handler_data = Vec::new();

            // pull all handlers from the component settings
            if let Some(settings) = &component.settings {
                for setting in settings {
                    if let SettingsBlockElement::Handler(key, values) = setting {
                        for value in values {
                            let event_args = event_map
                                .get(key.token_value.as_str())
                                .expect("custom handlers not supported in settings block")
                                .as_ref();
                            handler_data.push(HandlerInfo {
                                name: self.clean_handler(value.token_value.clone()),
                                args_type: event_args.map(|t| format!("Event<{}>", &t)),
                            });
                        }
                    }
                }
            }

            // pull all handlers from the template inline settings
            if let Some(template) = &component.template {
                for tnd in template.get_nodes() {
                    if let Some(settings) = &tnd.settings {
                        for setting in settings {
                            if let SettingElement::Setting(key, value) = setting {
                                if let ValueDefinition::EventBindingTarget(e) = value {
                                    let event_args = event_map
                                        .get(key.token_value.as_str())
                                        .and_then(|v| v.as_ref());
                                    handler_data.push(HandlerInfo {
                                        name: self.clean_handler(e.name.clone()),
                                        args_type: event_args.map(|t| format!("Event<{}>", &t)),
                                    });
                                }
                            }
                        }
                    }
                }
            }

            component_infos.push(ComponentInfo {
                type_id: component.type_id.clone(),
                pascal_identifier: component.type_id.get_pascal_identifier().unwrap(),
                primitive_instance_import_path: component.primitive_instance_import_path.clone(),
                properties,
                handlers: handler_data,
            });
        }
        component_infos
    }

    pub fn get_inline_properties(
        &self,
        containing_component_type_id: &TypeId,
        tnd: &TemplateNodeDefinition,
    ) -> BTreeMap<String, ValueDefinition> {
        let component = self.components.get(&containing_component_type_id).unwrap();
        let settings =
            Self::merge_inline_settings_with_settings_block(&tnd.settings, &component.settings);
        let mut map = BTreeMap::new();
        if let Some(settings) = &settings {
            for setting in settings {
                if let SettingElement::Setting(key, value) = setting {
                    match value {
                        ValueDefinition::LiteralValue(_)
                        | ValueDefinition::Block(_)
                        | ValueDefinition::Timeline(_)
                        | ValueDefinition::Transition(_)
                        | ValueDefinition::Expression(_)
                        | ValueDefinition::Identifier(_)
                        | ValueDefinition::DoubleBinding(_) => {
                            map.insert(key.token_value.clone(), value.clone());
                        }
                        ValueDefinition::EventBindingTarget(_) | ValueDefinition::Undefined => {}
                    }
                }
            }
        }
        Self::merge_inline_timelines_with_timeline_blocks(
            &mut map,
            &tnd.settings,
            &component.timelines,
            &component.settings,
        );
        map
    }

    pub fn get_inline_common_properties(
        &self,
        containing_component_type_id: &TypeId,
        tnd: &TemplateNodeDefinition,
    ) -> BTreeMap<String, ValueDefinition> {
        let component = self.components.get(containing_component_type_id).unwrap();
        let settings =
            Self::merge_inline_settings_with_settings_block(&tnd.settings, &component.settings);
        let mut map = BTreeMap::new();
        if let Some(settings) = &settings {
            for setting in settings {
                if let SettingElement::Setting(key, value) = setting {
                    match value {
                        ValueDefinition::LiteralValue(_)
                        | ValueDefinition::Block(_)
                        | ValueDefinition::Timeline(_)
                        | ValueDefinition::Transition(_)
                        | ValueDefinition::Expression(_)
                        | ValueDefinition::Identifier(_) => {
                            if CommonProperty::get_common_properties().contains(&key.token_value) {
                                map.insert(key.token_value.clone(), value.clone());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Self::merge_inline_timelines_with_timeline_blocks(
            &mut map,
            &tnd.settings,
            &component.timelines,
            &component.settings,
        );
        map.retain(|key, _| CommonProperty::get_common_properties().contains(key));
        map
    }

    pub fn get_inline_event_handlers(&self, tnd: &TemplateNodeDefinition) -> Vec<(String, String)> {
        let mut handlers = Vec::new();
        if let Some(settings) = &tnd.settings {
            for setting in settings {
                if let SettingElement::Setting(key, value) = setting {
                    match value {
                        ValueDefinition::EventBindingTarget(e) => {
                            if matches!(key.token_value.as_str(), "in" | "out") {
                                continue;
                            }
                            handlers.push((
                                key.token_value.clone(),
                                self.clean_handler(e.name.clone()),
                            ));
                        }
                        _ => {}
                    }
                }
            }
        }
        handlers
    }

    fn pull_matched_identifiers_from_inline(
        inline_settings: &Option<Vec<SettingElement>>,
        s: String,
    ) -> Vec<String> {
        let mut ret = Vec::new();
        if let Some(val) = inline_settings {
            let matched_settings = val.iter().filter(|e| match e {
                SettingElement::Setting(token, _) => token.token_value == s.as_str(),
                _ => false,
            });
            for e in matched_settings {
                if let SettingElement::Setting(_, value) = e {
                    match value {
                        ValueDefinition::Identifier(s) => ret.push(s.name.clone()),
                        _ => {}
                    };
                }
            }
        }
        ret
    }

    fn pull_settings_with_selector(
        settings: &Option<Vec<SettingsBlockElement>>,
        selector: String,
    ) -> Option<Vec<SettingElement>> {
        settings.as_ref().and_then(|val| {
            let mut merged_setting = Vec::new();
            for settings_value in val.iter() {
                match settings_value {
                    SettingsBlockElement::SelectorBlock(token, value) => {
                        if token.token_value == selector {
                            merged_setting.extend(value.elements.clone());
                        }
                    }
                    _ => {}
                }
            }
            (!merged_setting.is_empty()).then(|| merged_setting)
        })
    }

    fn has_inline_property(
        inline_settings: &Option<Vec<SettingElement>>,
        property_name: &str,
    ) -> bool {
        inline_settings
            .as_ref()
            .map(|settings| {
                settings.iter().any(|setting| match setting {
                    SettingElement::Setting(token, _) => token.token_value == property_name,
                    SettingElement::Comment(_) => false,
                })
            })
            .unwrap_or(false)
    }

    fn timeline_target_matches_node(target: &str, classes: &[String], ids: &[String]) -> bool {
        if let Some(class) = target.strip_prefix('.') {
            return classes.iter().any(|candidate| candidate == class);
        }
        if let Some(id) = target.strip_prefix('#') {
            return ids.iter().any(|candidate| candidate == id);
        }
        false
    }

    fn timeline_target_is_local(target: &str) -> bool {
        matches!(target, "self" | "this")
    }

    fn timeline_label(timeline_definition: &TimelineDefinition, ordinal: usize) -> String {
        timeline_definition
            .name
            .as_ref()
            .map(|token| token.token_value.clone())
            .unwrap_or_else(|| format!("#{}", ordinal + 1))
    }

    fn timeline_name(timeline_definition: &TimelineDefinition) -> Option<&str> {
        timeline_definition
            .name
            .as_ref()
            .map(|token| token.token_value.as_str())
    }

    fn transition_bindings_from_settings(
        settings: &Option<Vec<SettingsBlockElement>>,
    ) -> ComponentTransitionBindingInfo {
        let mut bindings = ComponentTransitionBindingInfo::default();
        if let Some(settings) = settings {
            for setting in settings {
                if let SettingsBlockElement::Transition(key, value) = setting {
                    match key.token_value.as_str() {
                        "in" => bindings.enter = Some(value.token_value.clone()),
                        "out" => bindings.exit = Some(value.token_value.clone()),
                        _ => {}
                    }
                }
            }
        }
        bindings
    }

    pub fn get_component_transition_bindings(
        &self,
        component_type_id: &TypeId,
    ) -> ComponentTransitionBindingInfo {
        self.components
            .get(component_type_id)
            .map(|component| Self::transition_bindings_from_settings(&component.settings))
            .unwrap_or_default()
    }

    fn timeline_track_frame_count(
        timeline_definition: &TimelineDefinition,
        track: &crate::TimelineTrackDefinition,
    ) -> u64 {
        let mut max_frame = track
            .frames
            .or(timeline_definition.frames)
            .unwrap_or_default();
        let mut uses_percent_markers = false;
        for keyframe in track.keyframes() {
            match keyframe.marker {
                crate::TimelineMarker::Frame(frame) => max_frame = max_frame.max(frame),
                crate::TimelineMarker::Percent(_) => uses_percent_markers = true,
            }
        }
        if uses_percent_markers {
            max_frame.max(track.frames.or(timeline_definition.frames).unwrap_or(100))
        } else {
            max_frame
        }
    }

    fn timeline_frame_count(timeline_definition: &TimelineDefinition) -> u64 {
        let mut max_frame = timeline_definition.frames.unwrap_or_default();
        for element in &timeline_definition.elements {
            if let TimelineBlockElement::SelectorBlock(_, block) = element {
                for selector_element in &block.elements {
                    if let TimelineSelectorElement::Track(_, track) = selector_element {
                        max_frame = max_frame
                            .max(Self::timeline_track_frame_count(timeline_definition, track));
                    }
                }
            }
        }
        max_frame
    }

    pub fn get_component_transition_config(
        &self,
        component_type_id: &TypeId,
    ) -> ComponentTransitionConfig {
        let Some(component) = self.components.get(component_type_id) else {
            return ComponentTransitionConfig::default();
        };
        let bindings = Self::transition_bindings_from_settings(&component.settings);
        let mut config = ComponentTransitionConfig {
            timeout_ms: DEFAULT_OUT_TRANSITION_TIMEOUT_MS,
            ..Default::default()
        };

        for timeline in &component.timelines {
            match Self::timeline_transition_kind(timeline, &bindings) {
                Some(TRANSITION_PHASE_ENTER) => {
                    config.has_enter = true;
                }
                Some(TRANSITION_PHASE_EXIT) => {
                    config.has_exit = true;
                    config.exit_frame_count = config
                        .exit_frame_count
                        .max(Self::timeline_frame_count(timeline));
                }
                _ => {}
            }
        }

        config
    }

    fn timeline_transition_kind(
        timeline_definition: &TimelineDefinition,
        bindings: &ComponentTransitionBindingInfo,
    ) -> Option<u64> {
        let name = Self::timeline_name(timeline_definition)?;
        if bindings.enter.as_deref() == Some(name) {
            return Some(TRANSITION_PHASE_ENTER);
        }
        if bindings.exit.as_deref() == Some(name) {
            return Some(TRANSITION_PHASE_EXIT);
        }
        None
    }

    fn prepare_timeline_track(
        timeline_definition: &TimelineDefinition,
        track: &crate::TimelineTrackDefinition,
    ) -> crate::TimelineTrackDefinition {
        let mut track = track.clone();
        if track.frames.is_none() {
            track.frames = timeline_definition.frames;
        }
        if track.repeat.is_none() {
            track.repeat = Some(timeline_definition.repeat);
        }
        if track.playhead.is_none() {
            track.playhead = timeline_definition.playhead.clone().map(Box::new);
        }
        track
    }

    fn prepare_transition_track(
        timeline_definition: &TimelineDefinition,
        track: &crate::TimelineTrackDefinition,
        base_map: &BTreeMap<String, ValueDefinition>,
        property_name: &str,
    ) -> crate::TimelineTrackDefinition {
        let mut track = track.clone();
        if track.frames.is_none() {
            track.frames = timeline_definition.frames;
        }
        track.repeat = Some(false);
        track.playhead = Some(Box::new(ValueDefinition::Identifier(
            crate::PaxIdentifier::new(TRANSITION_PLAYHEAD_SYMBOL),
        )));
        if track.starting_value.is_none() {
            track.starting_value = base_map.get(property_name).cloned().map(Box::new);
        }
        track
    }

    fn merge_timeline_track_into_map(
        map: &mut BTreeMap<String, ValueDefinition>,
        base_map: &BTreeMap<String, ValueDefinition>,
        timeline_definition: &TimelineDefinition,
        timeline_ordinal: usize,
        property_name: &str,
        track: &crate::TimelineTrackDefinition,
        use_local_property_scope: bool,
    ) {
        let mut track = Self::prepare_timeline_track(timeline_definition, track);
        if track.starting_value.is_none() {
            track.starting_value = base_map.get(property_name).cloned().map(Box::new);
        }
        track.use_local_property_scope = use_local_property_scope;

        if let Some(ValueDefinition::Timeline(existing_track)) = map.get(property_name) {
            let winner = Self::timeline_label(timeline_definition, timeline_ordinal);
            log::warn!(
                "timeline conflict on property '{}'; later timeline '{}' overrides an earlier timeline",
                property_name,
                winner,
            );
            if existing_track.use_local_property_scope && !track.use_local_property_scope {
                track.use_local_property_scope = false;
            }
        }

        map.insert(property_name.to_string(), ValueDefinition::Timeline(track));
    }

    fn merge_transition_track_into_map(
        map: &mut BTreeMap<String, ValueDefinition>,
        base_map: &BTreeMap<String, ValueDefinition>,
        timeline_definition: &TimelineDefinition,
        timeline_ordinal: usize,
        property_name: &str,
        track: &crate::TimelineTrackDefinition,
        transition_kind: u64,
    ) {
        let track =
            Self::prepare_transition_track(timeline_definition, track, base_map, property_name);
        let mut transition = match map.remove(property_name) {
            Some(ValueDefinition::Transition(existing)) => existing,
            Some(existing) => TransitionDefinition {
                starting_value: Some(Box::new(existing)),
                ..Default::default()
            },
            None => TransitionDefinition {
                starting_value: base_map.get(property_name).cloned().map(Box::new),
                ..Default::default()
            },
        };

        let slot = match transition_kind {
            TRANSITION_PHASE_ENTER => &mut transition.enter,
            TRANSITION_PHASE_EXIT => &mut transition.exit,
            _ => return,
        };
        if slot.is_some() {
            let winner = Self::timeline_label(timeline_definition, timeline_ordinal);
            log::warn!(
                "transition conflict on property '{}'; later transition timeline '{}' overrides an earlier transition track",
                property_name,
                winner,
            );
        }
        *slot = Some(track);

        map.insert(
            property_name.to_string(),
            ValueDefinition::Transition(transition),
        );
    }

    fn merge_inline_timelines_with_timeline_blocks(
        map: &mut BTreeMap<String, ValueDefinition>,
        inline_settings: &Option<Vec<SettingElement>>,
        timelines: &[TimelineDefinition],
        component_settings: &Option<Vec<SettingsBlockElement>>,
    ) {
        let base_map = map.clone();
        let transition_bindings = Self::transition_bindings_from_settings(component_settings);
        let ids = Self::pull_matched_identifiers_from_inline(inline_settings, "id".to_string());
        let classes =
            Self::pull_matched_identifiers_from_inline(inline_settings, "class".to_string());
        if ids.len() > 1 {
            panic!("Specified more than one id inline!");
        }

        for (timeline_ordinal, timeline_definition) in timelines.iter().enumerate() {
            for timeline_value in timeline_definition.elements.iter() {
                if let TimelineBlockElement::SelectorBlock(token, value) = timeline_value {
                    if !Self::timeline_target_matches_node(&token.token_value, &classes, &ids) {
                        continue;
                    }
                    let transition_kind =
                        Self::timeline_transition_kind(timeline_definition, &transition_bindings);

                    for element in value.elements.iter() {
                        if let TimelineSelectorElement::Track(property, track) = element {
                            if let Some(transition_kind) = transition_kind {
                                Self::merge_transition_track_into_map(
                                    map,
                                    &base_map,
                                    timeline_definition,
                                    timeline_ordinal,
                                    &property.token_value,
                                    track,
                                    transition_kind,
                                );
                                continue;
                            }
                            if Self::has_inline_property(inline_settings, &property.token_value) {
                                continue;
                            }
                            Self::merge_timeline_track_into_map(
                                map,
                                &base_map,
                                timeline_definition,
                                timeline_ordinal,
                                &property.token_value,
                                track,
                                false,
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn merge_component_self_timelines_with_properties(
        &self,
        component_type_id: &TypeId,
        map: &mut BTreeMap<String, ValueDefinition>,
    ) {
        let Some(component) = self.components.get(component_type_id) else {
            return;
        };

        let base_map = map.clone();
        let transition_bindings = Self::transition_bindings_from_settings(&component.settings);
        for (timeline_ordinal, timeline_definition) in component.timelines.iter().enumerate() {
            for timeline_value in timeline_definition.elements.iter() {
                if let TimelineBlockElement::SelectorBlock(token, value) = timeline_value {
                    if !Self::timeline_target_is_local(&token.token_value) {
                        continue;
                    }
                    let transition_kind =
                        Self::timeline_transition_kind(timeline_definition, &transition_bindings);

                    for element in value.elements.iter() {
                        if let TimelineSelectorElement::Track(property, track) = element {
                            if let Some(transition_kind) = transition_kind {
                                Self::merge_transition_track_into_map(
                                    map,
                                    &base_map,
                                    timeline_definition,
                                    timeline_ordinal,
                                    &property.token_value,
                                    track,
                                    transition_kind,
                                );
                                continue;
                            }
                            Self::merge_timeline_track_into_map(
                                map,
                                &base_map,
                                timeline_definition,
                                timeline_ordinal,
                                &property.token_value,
                                track,
                                true,
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn merge_inline_settings_with_settings_block(
        inline_settings: &Option<Vec<SettingElement>>,
        settings_block: &Option<Vec<SettingsBlockElement>>,
    ) -> Option<Vec<SettingElement>> {
        // collect id settings
        let ids = Self::pull_matched_identifiers_from_inline(&inline_settings, "id".to_string());

        let mut id_settings = Vec::new();
        if ids.len() == 1 {
            if let Some(settings) =
                Self::pull_settings_with_selector(&settings_block, format!("#{}", ids[0]))
            {
                id_settings.extend(settings.clone());
            }
        } else if ids.len() > 1 {
            panic!("Specified more than one id inline!");
        }

        // collect all class settings
        let classes =
            Self::pull_matched_identifiers_from_inline(&inline_settings, "class".to_string());

        let mut class_settings = Vec::new();
        for class in classes {
            if let Some(settings) =
                Self::pull_settings_with_selector(&settings_block, format!(".{}", class))
            {
                class_settings.extend(settings.clone());
            }
        }

        let mut map = BTreeMap::new();

        // Iterate in reverse order of priority (class, then id, then inline)
        for e in class_settings.into_iter() {
            if let SettingElement::Setting(key, _) = e.clone() {
                map.insert(key, e);
            }
        }

        for e in id_settings.into_iter() {
            if let SettingElement::Setting(key, _) = e.clone() {
                map.insert(key, e);
            }
        }

        let mut merged = Vec::new();
        if let Some(inline) = inline_settings.clone() {
            for e in inline.iter() {
                if let SettingElement::Setting(key, _) = e.clone() {
                    map.remove(&key);
                }
            }
            let unique_setting_block_settings: Vec<SettingElement> =
                map.values().cloned().collect();
            merged.extend(inline);
            merged.extend(unique_setting_block_settings);
        }

        if merged.len() > 0 {
            Some(merged)
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize)]
/// Common property metadata passed into cartridge codegen templates.
pub struct CommonProperty {
    name: String,
    property_type: TypeId,
    is_optional: bool,
}

impl CommonProperty {
    pub fn get_common_properties() -> Vec<String> {
        COMMON_PROPERTIES.iter().map(|e| e.to_string()).collect()
    }

    pub fn get_property_types() -> Vec<(String, String)> {
        COMMON_PROPERTIES_TYPE
            .iter()
            .map(|(c, t)| (c.to_string(), t.to_string()))
            .collect()
    }

    pub fn get_as_common_property() -> Vec<CommonProperty> {
        let mut common_properties = Vec::new();
        for (name, property_type) in CommonProperty::get_property_types() {
            common_properties.push(CommonProperty {
                name: name.clone(),
                property_type: TypeId::build_singleton(&property_type, None),
                is_optional: (name != "transform" && name != "width" && name != "height"),
            });
        }
        common_properties
    }
}
