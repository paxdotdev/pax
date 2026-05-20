use crate::{
    ComponentDefinition, ComponentTemplate, ControlFlowConditionalBranchDefinition,
    ControlFlowConditionalBranchKind, ControlFlowRepeatPredicateDefinition,
    ControlFlowRouteBranchDefinition, ControlFlowSettingsDefinition, ExpressionInfo,
    GradientDefinition, GradientElement, GradientShapeDefinition, GradientStopDefinition,
    LiteralBlockDefinition, LocationInfo, PaxManifest, PaxType, PropertyDefinition,
    PropertyDefinitionFlags, RouteBranchDescriptor, SettingElement, SettingsBlockElement,
    TemplateNodeDefinition, TemplateNodeId, TimelineBlockElement, TimelineDefinition,
    TimelineKeyframe, TimelineMarker, TimelineSelectorBlockDefinition, TimelineSelectorElement,
    TimelineTrackDefinition, TimelineTrackElement, TransitionDefinition, TypeDefinition, TypeId,
    ValueDefinition,
};
use pax_language::interpreter::{PaxAccessor, PaxExpression, PaxPrimary, PaxUnit};
use pax_runtime_api::{
    Color, ColorChannel, Duration, Numeric, PathElement, PaxValue, Percent, Rotation, Size,
};
use std::fmt::Write;

/// Serialize a manifest as a Rust expression that constructs the same manifest.
pub fn to_rust_expression(manifest: &PaxManifest) -> String {
    RustManifestWriter::new(&manifest.engine_import_path).pax_manifest(manifest)
}

struct RustManifestWriter {
    manifest_path: String,
    api_path: String,
    lang_path: String,
}

impl RustManifestWriter {
    fn new(engine_import_path: &str) -> Self {
        Self {
            manifest_path: format!("{engine_import_path}::pax_manifest"),
            api_path: format!("{engine_import_path}::api"),
            lang_path: format!("{engine_import_path}::pax_language::interpreter"),
        }
    }

    fn pax_manifest(&self, manifest: &PaxManifest) -> String {
        let components = manifest
            .components
            .iter()
            .map(|(type_id, definition)| {
                format!(
                    "({}, {})",
                    self.type_id(type_id),
                    self.component_definition(definition)
                )
            })
            .collect::<Vec<_>>();

        let mut type_table = manifest.type_table.iter().collect::<Vec<_>>();
        type_table.sort_by(|(lhs, _), (rhs, _)| lhs.cmp(rhs));
        let type_table = type_table
            .into_iter()
            .map(|(type_id, definition)| {
                format!(
                    "({}, {})",
                    self.type_id(type_id),
                    self.type_definition(definition)
                )
            })
            .collect::<Vec<_>>();

        format!(
            "{mp}::PaxManifest {{ components: {components}, main_component_type_id: {main_component_type_id}, type_table: {type_table}, assets_dirs: {assets_dirs}, engine_import_path: {engine_import_path} }}",
            mp = self.manifest_path,
            components = map_from_entries("std::collections::BTreeMap", components),
            main_component_type_id = self.type_id(&manifest.main_component_type_id),
            type_table = map_from_entries("std::collections::HashMap", type_table),
            assets_dirs = self.vec(&manifest.assets_dirs, |value| rust_string(value)),
            engine_import_path = rust_string(&manifest.engine_import_path),
        )
    }

    fn component_definition(&self, definition: &ComponentDefinition) -> String {
        format!(
            "{mp}::ComponentDefinition {{ type_id: {type_id}, is_main_component: {is_main_component}, is_primitive: {is_primitive}, is_struct_only_component: {is_struct_only_component}, module_path: {module_path}, primitive_instance_import_path: {primitive_instance_import_path}, template: {template}, settings: {settings}, timelines: {timelines}, route_branch: {route_branch} }}",
            mp = self.manifest_path,
            type_id = self.type_id(&definition.type_id),
            is_main_component = definition.is_main_component,
            is_primitive = definition.is_primitive,
            is_struct_only_component = definition.is_struct_only_component,
            module_path = rust_string(&definition.module_path),
            primitive_instance_import_path = self.option(&definition.primitive_instance_import_path, |value| rust_string(value)),
            template = self.option(&definition.template, |value| self.component_template(value)),
            settings = self.option(&definition.settings, |value| self.vec(value, |element| self.settings_block_element(element))),
            timelines = self.vec(&definition.timelines, |timeline| self.timeline_definition(timeline)),
            route_branch = self.option(&definition.route_branch, |descriptor| self.route_branch_descriptor(descriptor)),
        )
    }

    fn route_branch_descriptor(&self, descriptor: &RouteBranchDescriptor) -> String {
        format!(
            "{mp}::RouteBranchDescriptor {{ path_property: {path_property}, default_property: {default_property} }}",
            mp = self.manifest_path,
            path_property = rust_string(&descriptor.path_property),
            default_property = rust_string(&descriptor.default_property),
        )
    }

    fn component_template(&self, template: &ComponentTemplate) -> String {
        let mut children = template.children.iter().collect::<Vec<_>>();
        children.sort_by_key(|(id, _)| id.as_usize());
        let children = children
            .into_iter()
            .map(|(id, child_ids)| {
                format!(
                    "({}, {})",
                    self.template_node_id(id),
                    self.vec_deque(child_ids, |child_id| self.template_node_id(child_id))
                )
            })
            .collect::<Vec<_>>();

        let mut nodes = template.nodes.iter().collect::<Vec<_>>();
        nodes.sort_by_key(|(id, _)| id.as_usize());
        let nodes = nodes
            .into_iter()
            .map(|(id, node)| {
                format!(
                    "({}, {})",
                    self.template_node_id(id),
                    self.template_node_definition(node)
                )
            })
            .collect::<Vec<_>>();

        format!(
            "{mp}::ComponentTemplate::from_parts({containing_component}, {root}, {children}, {nodes}, {next_id}usize, {template_source_file_path})",
            mp = self.manifest_path,
            containing_component = self.type_id(&template.containing_component),
            root = self.vec_deque(&template.root, |id| self.template_node_id(id)),
            children = map_from_entries("std::collections::HashMap", children),
            nodes = map_from_entries("std::collections::HashMap", nodes),
            next_id = template.next_id,
            template_source_file_path = self.option(&template.template_source_file_path, |value| rust_string(value)),
        )
    }

    fn template_node_definition(&self, node: &TemplateNodeDefinition) -> String {
        format!(
            "{mp}::TemplateNodeDefinition {{ type_id: {type_id}, control_flow_settings: {control_flow_settings}, settings: {settings}, selector_info: {selector_info}, raw_comment_string: {raw_comment_string} }}",
            mp = self.manifest_path,
            type_id = self.type_id(&node.type_id),
            control_flow_settings = self.option(&node.control_flow_settings, |value| self.control_flow_settings_definition(value)),
            settings = self.option(&node.settings, |value| self.vec(value, |element| self.setting_element(element))),
            selector_info = self.template_node_selector_info(&node.selector_info),
            raw_comment_string = self.option(&node.raw_comment_string, |value| rust_string(value)),
        )
    }

    fn template_node_selector_info(&self, info: &crate::TemplateNodeSelectorInfo) -> String {
        format!(
            "{mp}::TemplateNodeSelectorInfo {{ source_location: {source_location}, id: {id}, classes: {classes} }}",
            mp = self.manifest_path,
            source_location = self.option(&info.source_location, |value| self.location_info(value)),
            id = self.option(&info.id, |value| self.token(value)),
            classes = self.vec(&info.classes, |token| self.token(token)),
        )
    }

    fn type_definition(&self, definition: &TypeDefinition) -> String {
        format!(
            "{mp}::TypeDefinition {{ type_id: {type_id}, inner_iterable_type_id: {inner_iterable_type_id}, property_definitions: {property_definitions} }}",
            mp = self.manifest_path,
            type_id = self.type_id(&definition.type_id),
            inner_iterable_type_id = self.option(&definition.inner_iterable_type_id, |value| self.type_id(value)),
            property_definitions = self.vec(&definition.property_definitions, |property| self.property_definition(property)),
        )
    }

    fn property_definition(&self, definition: &PropertyDefinition) -> String {
        format!(
            "{mp}::PropertyDefinition {{ name: {name}, flags: {flags}, type_id: {type_id} }}",
            mp = self.manifest_path,
            name = rust_string(&definition.name),
            flags = self.property_definition_flags(&definition.flags),
            type_id = self.type_id(&definition.type_id),
        )
    }

    fn property_definition_flags(&self, flags: &PropertyDefinitionFlags) -> String {
        format!(
            "{mp}::PropertyDefinitionFlags {{ is_binding_repeat_i: {is_binding_repeat_i}, is_binding_repeat_elem: {is_binding_repeat_elem}, is_repeat_source_range: {is_repeat_source_range}, is_repeat_source_iterable: {is_repeat_source_iterable}, is_property_wrapped: {is_property_wrapped}, is_enum: {is_enum} }}",
            mp = self.manifest_path,
            is_binding_repeat_i = flags.is_binding_repeat_i,
            is_binding_repeat_elem = flags.is_binding_repeat_elem,
            is_repeat_source_range = flags.is_repeat_source_range,
            is_repeat_source_iterable = flags.is_repeat_source_iterable,
            is_property_wrapped = flags.is_property_wrapped,
            is_enum = flags.is_enum,
        )
    }

    fn settings_block_element(&self, element: &SettingsBlockElement) -> String {
        match element {
            SettingsBlockElement::SelectorBlock(token, block) => format!(
                "{mp}::SettingsBlockElement::SelectorBlock({token}, {block})",
                mp = self.manifest_path,
                token = self.token(token),
                block = self.literal_block_definition(block),
            ),
            SettingsBlockElement::Handler(token, handlers) => format!(
                "{mp}::SettingsBlockElement::Handler({token}, {handlers})",
                mp = self.manifest_path,
                token = self.token(token),
                handlers = self.vec(handlers, |handler| self.token(handler)),
            ),
            SettingsBlockElement::Transition(token, value) => format!(
                "{mp}::SettingsBlockElement::Transition({token}, {value})",
                mp = self.manifest_path,
                token = self.token(token),
                value = self.token(value),
            ),
            SettingsBlockElement::Conditional(block) => format!(
                "{mp}::SettingsBlockElement::Conditional({mp}::SettingsConditionalBlock {{ branches: {branches} }})",
                mp = self.manifest_path,
                branches = self.vec(&block.branches, |branch| {
                    format!(
                        "{mp}::SettingsConditionalBranch {{ condition_expression: {condition_expression}, elements: {elements} }}",
                        mp = self.manifest_path,
                        condition_expression = self.option(&branch.condition_expression, |value| self.expression_info(value)),
                        elements = self.vec(&branch.elements, |element| self.settings_block_element(element)),
                    )
                }),
            ),
            SettingsBlockElement::Comment(comment) => format!(
                "{mp}::SettingsBlockElement::Comment({comment})",
                mp = self.manifest_path,
                comment = rust_string(comment),
            ),
        }
    }

    fn setting_element(&self, element: &SettingElement) -> String {
        match element {
            SettingElement::Setting(token, value) => format!(
                "{mp}::SettingElement::Setting({token}, {value})",
                mp = self.manifest_path,
                token = self.token(token),
                value = self.value_definition(value),
            ),
            SettingElement::Comment(comment) => format!(
                "{mp}::SettingElement::Comment({comment})",
                mp = self.manifest_path,
                comment = rust_string(comment),
            ),
        }
    }

    fn timeline_definition(&self, timeline: &TimelineDefinition) -> String {
        format!(
            "{mp}::TimelineDefinition {{ name: {name}, playhead: {playhead}, duration: {duration}, repeat: {repeat}, elements: {elements} }}",
            mp = self.manifest_path,
            name = self.option(&timeline.name, |value| self.token(value)),
            playhead = self.option(&timeline.playhead, |value| self.value_definition(value)),
            duration = self.option(&timeline.duration, |value| self.value_definition(value)),
            repeat = timeline.repeat,
            elements = self.vec(&timeline.elements, |element| self.timeline_block_element(element)),
        )
    }

    fn timeline_block_element(&self, element: &TimelineBlockElement) -> String {
        match element {
            TimelineBlockElement::SelectorBlock(token, block) => format!(
                "{mp}::TimelineBlockElement::SelectorBlock({token}, {block})",
                mp = self.manifest_path,
                token = self.token(token),
                block = self.timeline_selector_block_definition(block),
            ),
            TimelineBlockElement::Comment(comment) => format!(
                "{mp}::TimelineBlockElement::Comment({comment})",
                mp = self.manifest_path,
                comment = rust_string(comment),
            ),
        }
    }

    fn timeline_selector_block_definition(
        &self,
        block: &TimelineSelectorBlockDefinition,
    ) -> String {
        format!(
            "{mp}::TimelineSelectorBlockDefinition {{ elements: {elements} }}",
            mp = self.manifest_path,
            elements = self.vec(&block.elements, |element| self
                .timeline_selector_element(element)),
        )
    }

    fn timeline_selector_element(&self, element: &TimelineSelectorElement) -> String {
        match element {
            TimelineSelectorElement::Track(token, track) => format!(
                "{mp}::TimelineSelectorElement::Track({token}, {track})",
                mp = self.manifest_path,
                token = self.token(token),
                track = self.timeline_track_definition(track),
            ),
            TimelineSelectorElement::Comment(comment) => format!(
                "{mp}::TimelineSelectorElement::Comment({comment})",
                mp = self.manifest_path,
                comment = rust_string(comment),
            ),
        }
    }

    fn timeline_track_definition(&self, track: &TimelineTrackDefinition) -> String {
        format!(
            "{mp}::TimelineTrackDefinition {{ elements: {elements}, playhead: {playhead}, duration: {duration}, repeat: {repeat}, starting_value: {starting_value}, use_local_property_scope: {use_local_property_scope} }}",
            mp = self.manifest_path,
            elements = self.vec(&track.elements, |element| self.timeline_track_element(element)),
            playhead = self.option_box_value_definition(&track.playhead),
            duration = self.option_box_value_definition(&track.duration),
            repeat = self.option(&track.repeat, |value| value.to_string()),
            starting_value = self.option_box_value_definition(&track.starting_value),
            use_local_property_scope = track.use_local_property_scope,
        )
    }

    fn timeline_track_element(&self, element: &TimelineTrackElement) -> String {
        match element {
            TimelineTrackElement::Keyframe(keyframe) => format!(
                "{mp}::TimelineTrackElement::Keyframe({keyframe})",
                mp = self.manifest_path,
                keyframe = self.timeline_keyframe(keyframe),
            ),
            TimelineTrackElement::Comment(comment) => format!(
                "{mp}::TimelineTrackElement::Comment({comment})",
                mp = self.manifest_path,
                comment = rust_string(comment),
            ),
        }
    }

    fn timeline_keyframe(&self, keyframe: &TimelineKeyframe) -> String {
        format!(
            "{mp}::TimelineKeyframe {{ marker: {marker}, value: {value}, easing: {easing} }}",
            mp = self.manifest_path,
            marker = self.timeline_marker(&keyframe.marker),
            value = self.value_definition(&keyframe.value),
            easing = self.option(&keyframe.easing, |value| self.token(value)),
        )
    }

    fn timeline_marker(&self, marker: &TimelineMarker) -> String {
        match marker {
            TimelineMarker::Frame(frame) => format!(
                "{mp}::TimelineMarker::Frame({frame}u64)",
                mp = self.manifest_path
            ),
            TimelineMarker::Duration(duration) => format!(
                "{mp}::TimelineMarker::Duration({duration})",
                mp = self.manifest_path,
                duration = self.duration(duration),
            ),
            TimelineMarker::Percent(percent) => format!(
                "{mp}::TimelineMarker::Percent({percent})",
                mp = self.manifest_path,
                percent = f64_literal(*percent),
            ),
        }
    }

    fn value_definition(&self, value: &ValueDefinition) -> String {
        match value {
            ValueDefinition::Undefined => {
                format!("{mp}::ValueDefinition::Undefined", mp = self.manifest_path)
            }
            ValueDefinition::LiteralValue(value) => format!(
                "{mp}::ValueDefinition::LiteralValue({value})",
                mp = self.manifest_path,
                value = self.pax_value(value),
            ),
            ValueDefinition::Block(block) => format!(
                "{mp}::ValueDefinition::Block({block})",
                mp = self.manifest_path,
                block = self.literal_block_definition(block),
            ),
            ValueDefinition::Timeline(track) => format!(
                "{mp}::ValueDefinition::Timeline({track})",
                mp = self.manifest_path,
                track = self.timeline_track_definition(track),
            ),
            ValueDefinition::Gradient(gradient) => format!(
                "{mp}::ValueDefinition::Gradient({gradient})",
                mp = self.manifest_path,
                gradient = self.gradient_definition(gradient),
            ),
            ValueDefinition::Transition(transition) => format!(
                "{mp}::ValueDefinition::Transition({transition})",
                mp = self.manifest_path,
                transition = self.transition_definition(transition),
            ),
            ValueDefinition::Expression(info) => format!(
                "{mp}::ValueDefinition::Expression({info})",
                mp = self.manifest_path,
                info = self.expression_info(info),
            ),
            ValueDefinition::Identifier(identifier) => format!(
                "{mp}::ValueDefinition::Identifier({identifier})",
                mp = self.manifest_path,
                identifier = self.pax_identifier(identifier),
            ),
            ValueDefinition::DoubleBinding(identifier) => format!(
                "{mp}::ValueDefinition::DoubleBinding({identifier})",
                mp = self.manifest_path,
                identifier = self.pax_identifier(identifier),
            ),
            ValueDefinition::EventBindingTarget(identifier) => format!(
                "{mp}::ValueDefinition::EventBindingTarget({identifier})",
                mp = self.manifest_path,
                identifier = self.pax_identifier(identifier),
            ),
        }
    }

    fn literal_block_definition(&self, block: &LiteralBlockDefinition) -> String {
        format!(
            "{mp}::LiteralBlockDefinition {{ explicit_type_pascal_identifier: {explicit_type_pascal_identifier}, elements: {elements} }}",
            mp = self.manifest_path,
            explicit_type_pascal_identifier = self.option(&block.explicit_type_pascal_identifier, |value| self.token(value)),
            elements = self.vec(&block.elements, |element| self.setting_element(element)),
        )
    }

    fn gradient_definition(&self, gradient: &GradientDefinition) -> String {
        format!(
            "{mp}::GradientDefinition {{ shape: {shape}, elements: {elements} }}",
            mp = self.manifest_path,
            shape = self.gradient_shape_definition(&gradient.shape),
            elements = self.vec(&gradient.elements, |element| self.gradient_element(element)),
        )
    }

    fn gradient_shape_definition(&self, shape: &GradientShapeDefinition) -> String {
        match shape {
            GradientShapeDefinition::Linear { start, end } => format!(
                "{mp}::GradientShapeDefinition::Linear {{ start: {start}, end: {end} }}",
                mp = self.manifest_path,
                start = self.option_box_value_definition(start),
                end = self.option_box_value_definition(end),
            ),
            GradientShapeDefinition::Radial { start, end, radius } => format!(
                "{mp}::GradientShapeDefinition::Radial {{ start: Box::new({start}), end: Box::new({end}), radius: Box::new({radius}) }}",
                mp = self.manifest_path,
                start = self.value_definition(start),
                end = self.value_definition(end),
                radius = self.value_definition(radius),
            ),
        }
    }

    fn gradient_element(&self, element: &GradientElement) -> String {
        match element {
            GradientElement::Stop(stop) => format!(
                "{mp}::GradientElement::Stop({stop})",
                mp = self.manifest_path,
                stop = self.gradient_stop_definition(stop),
            ),
            GradientElement::Comment(comment) => format!(
                "{mp}::GradientElement::Comment({comment})",
                mp = self.manifest_path,
                comment = rust_string(comment),
            ),
        }
    }

    fn gradient_stop_definition(&self, stop: &GradientStopDefinition) -> String {
        format!(
            "{mp}::GradientStopDefinition {{ position: {position}, color: {color} }}",
            mp = self.manifest_path,
            position = self.size(&stop.position),
            color = self.value_definition(&stop.color),
        )
    }

    fn transition_definition(&self, transition: &TransitionDefinition) -> String {
        format!(
            "{mp}::TransitionDefinition {{ enter: {enter}, exit: {exit}, starting_value: {starting_value} }}",
            mp = self.manifest_path,
            enter = self.option(&transition.enter, |value| self.timeline_track_definition(value)),
            exit = self.option(&transition.exit, |value| self.timeline_track_definition(value)),
            starting_value = self.option_box_value_definition(&transition.starting_value),
        )
    }

    fn control_flow_settings_definition(&self, settings: &ControlFlowSettingsDefinition) -> String {
        format!(
            "{mp}::ControlFlowSettingsDefinition {{ condition_expression: {condition_expression}, slot_index_expression: {slot_index_expression}, repeat_predicate_definition: {repeat_predicate_definition}, repeat_source_expression: {repeat_source_expression}, repeat_key_expression: {repeat_key_expression}, conditional_branches: {conditional_branches}, route_branches: {route_branches} }}",
            mp = self.manifest_path,
            condition_expression = self.option(&settings.condition_expression, |value| self.expression_info(value)),
            slot_index_expression = self.option(&settings.slot_index_expression, |value| self.expression_info(value)),
            repeat_predicate_definition = self.option(&settings.repeat_predicate_definition, |value| self.control_flow_repeat_predicate_definition(value)),
            repeat_source_expression = self.option(&settings.repeat_source_expression, |value| self.expression_info(value)),
            repeat_key_expression = self.option(&settings.repeat_key_expression, |value| self.expression_info(value)),
            conditional_branches = self.vec(&settings.conditional_branches, |value| self.control_flow_conditional_branch_definition(value)),
            route_branches = self.vec(&settings.route_branches, |value| self.control_flow_route_branch_definition(value)),
        )
    }

    fn control_flow_conditional_branch_definition(
        &self,
        branch: &ControlFlowConditionalBranchDefinition,
    ) -> String {
        format!(
            "{mp}::ControlFlowConditionalBranchDefinition {{ branch_kind: {branch_kind}, condition_expression: {condition_expression}, child_ids: {child_ids} }}",
            mp = self.manifest_path,
            branch_kind = self.control_flow_conditional_branch_kind(&branch.branch_kind),
            condition_expression = self.option(&branch.condition_expression, |value| self.expression_info(value)),
            child_ids = self.vec(&branch.child_ids, |value| self.template_node_id(value)),
        )
    }

    fn control_flow_conditional_branch_kind(
        &self,
        branch_kind: &ControlFlowConditionalBranchKind,
    ) -> String {
        match branch_kind {
            ControlFlowConditionalBranchKind::If => format!(
                "{mp}::ControlFlowConditionalBranchKind::If",
                mp = self.manifest_path,
            ),
            ControlFlowConditionalBranchKind::ElseIf => format!(
                "{mp}::ControlFlowConditionalBranchKind::ElseIf",
                mp = self.manifest_path,
            ),
            ControlFlowConditionalBranchKind::Else => format!(
                "{mp}::ControlFlowConditionalBranchKind::Else",
                mp = self.manifest_path,
            ),
        }
    }

    fn control_flow_repeat_predicate_definition(
        &self,
        predicate: &ControlFlowRepeatPredicateDefinition,
    ) -> String {
        match predicate {
            ControlFlowRepeatPredicateDefinition::ElemId(elem) => format!(
                "{mp}::ControlFlowRepeatPredicateDefinition::ElemId({elem})",
                mp = self.manifest_path,
                elem = rust_string(elem),
            ),
            ControlFlowRepeatPredicateDefinition::ElemIdIndexId(elem, index) => format!(
                "{mp}::ControlFlowRepeatPredicateDefinition::ElemIdIndexId({elem}, {index})",
                mp = self.manifest_path,
                elem = rust_string(elem),
                index = rust_string(index),
            ),
        }
    }

    fn control_flow_route_branch_definition(
        &self,
        branch: &ControlFlowRouteBranchDefinition,
    ) -> String {
        format!(
            "{mp}::ControlFlowRouteBranchDefinition {{ path: {path}, default: {default}, child_ids: {child_ids} }}",
            mp = self.manifest_path,
            path = self.option(&branch.path, |value| rust_string(value)),
            default = branch.default,
            child_ids = self.vec(&branch.child_ids, |value| self.template_node_id(value)),
        )
    }

    fn expression_info(&self, info: &ExpressionInfo) -> String {
        format!(
            "{mp}::ExpressionInfo {{ expression: {expression}, dependencies: {dependencies} }}",
            mp = self.manifest_path,
            expression = self.pax_expression(&info.expression),
            dependencies = self.vec(&info.dependencies, |value| rust_string(value)),
        )
    }

    fn pax_expression(&self, expression: &PaxExpression) -> String {
        match expression {
            PaxExpression::Primary(primary) => format!(
                "{lp}::PaxExpression::Primary(Box::new({primary}))",
                lp = self.lang_path,
                primary = self.pax_primary(primary),
            ),
            PaxExpression::Prefix(prefix) => format!(
                "{lp}::PaxExpression::prefix({operator}, {rhs})",
                lp = self.lang_path,
                operator = rust_str(prefix.operator_name()),
                rhs = self.pax_expression(prefix.rhs()),
            ),
            PaxExpression::Infix(infix) => format!(
                "{lp}::PaxExpression::infix({lhs}, {operator}, {rhs})",
                lp = self.lang_path,
                lhs = self.pax_expression(infix.lhs()),
                operator = rust_str(infix.operator_name()),
                rhs = self.pax_expression(infix.rhs()),
            ),
            PaxExpression::Postfix(postfix) => format!(
                "{lp}::PaxExpression::postfix({lhs}, {operator})",
                lp = self.lang_path,
                lhs = self.pax_expression(postfix.lhs()),
                operator = rust_str(postfix.operator_name()),
            ),
            PaxExpression::Ternary(ternary) => format!(
                "{lp}::PaxExpression::ternary({condition}, {then_branch}, {else_branch})",
                lp = self.lang_path,
                condition = self.pax_expression(ternary.condition()),
                then_branch = self.pax_expression(ternary.then_branch()),
                else_branch = self.pax_expression(ternary.else_branch()),
            ),
            PaxExpression::NullCoalesce(null_coalesce) => format!(
                "{lp}::PaxExpression::null_coalesce({lhs}, {rhs})",
                lp = self.lang_path,
                lhs = self.pax_expression(null_coalesce.lhs()),
                rhs = self.pax_expression(null_coalesce.rhs()),
            ),
        }
    }

    fn pax_primary(&self, primary: &PaxPrimary) -> String {
        match primary {
            PaxPrimary::Literal(value) => format!(
                "{lp}::PaxPrimary::Literal({value})",
                lp = self.lang_path,
                value = self.pax_value(value),
            ),
            PaxPrimary::Grouped(expression, unit) => format!(
                "{lp}::PaxPrimary::Grouped(Box::new({expression}), {unit})",
                lp = self.lang_path,
                expression = self.pax_expression(expression),
                unit = self.option(unit, |value| self.pax_unit(value)),
            ),
            PaxPrimary::Identifier(identifier, accessors) => format!(
                "{lp}::PaxPrimary::Identifier({identifier}, {accessors})",
                lp = self.lang_path,
                identifier = self.pax_identifier(identifier),
                accessors = self.vec(accessors, |accessor| self.pax_accessor(accessor)),
            ),
            PaxPrimary::Object(entries) => format!(
                "{lp}::PaxPrimary::Object({entries})",
                lp = self.lang_path,
                entries = self.vec(entries, |(key, value)| format!(
                    "({}, {})",
                    rust_string(key),
                    self.pax_expression(value)
                )),
            ),
            PaxPrimary::FunctionOrEnum(scope, name, args) => format!(
                "{lp}::PaxPrimary::FunctionOrEnum({scope}, {name}, {args})",
                lp = self.lang_path,
                scope = rust_string(scope),
                name = rust_string(name),
                args = self.vec(args, |arg| self.pax_expression(arg)),
            ),
            PaxPrimary::Range(start, end) => format!(
                "{lp}::PaxPrimary::Range({start}, {end})",
                lp = self.lang_path,
                start = self.pax_expression(start),
                end = self.pax_expression(end),
            ),
            PaxPrimary::Tuple(elements) => format!(
                "{lp}::PaxPrimary::Tuple({elements})",
                lp = self.lang_path,
                elements = self.vec(elements, |element| self.pax_expression(element)),
            ),
            PaxPrimary::List(elements) => format!(
                "{lp}::PaxPrimary::List({elements})",
                lp = self.lang_path,
                elements = self.vec(elements, |element| self.pax_expression(element)),
            ),
        }
    }

    fn pax_unit(&self, unit: &PaxUnit) -> String {
        let variant = match unit {
            PaxUnit::Percent => "Percent",
            PaxUnit::Pixels => "Pixels",
            PaxUnit::Radians => "Radians",
            PaxUnit::Degrees => "Degrees",
            PaxUnit::Milliseconds => "Milliseconds",
            PaxUnit::Seconds => "Seconds",
            PaxUnit::Frames => "Frames",
        };
        format!("{lp}::PaxUnit::{variant}", lp = self.lang_path)
    }

    fn pax_accessor(&self, accessor: &PaxAccessor) -> String {
        match accessor {
            PaxAccessor::Tuple(index) => format!(
                "{lp}::PaxAccessor::Tuple({index}usize)",
                lp = self.lang_path
            ),
            PaxAccessor::List(expression) => format!(
                "{lp}::PaxAccessor::List({expression})",
                lp = self.lang_path,
                expression = self.pax_expression(expression),
            ),
            PaxAccessor::Struct(field) => format!(
                "{lp}::PaxAccessor::Struct({field})",
                lp = self.lang_path,
                field = rust_string(field),
            ),
        }
    }

    fn pax_identifier(&self, identifier: &crate::PaxIdentifier) -> String {
        format!(
            "{lp}::PaxIdentifier::new({name})",
            lp = self.lang_path,
            name = rust_str(&identifier.name),
        )
    }

    fn pax_value(&self, value: &PaxValue) -> String {
        match value {
            PaxValue::Bool(value) => format!("{api}::PaxValue::Bool({value})", api = self.api_path),
            PaxValue::Numeric(value) => format!(
                "{api}::PaxValue::Numeric({value})",
                api = self.api_path,
                value = self.numeric(value),
            ),
            PaxValue::String(value) => format!(
                "{api}::PaxValue::String({value})",
                api = self.api_path,
                value = rust_string(value),
            ),
            PaxValue::Size(value) => format!(
                "{api}::PaxValue::Size({value})",
                api = self.api_path,
                value = self.size(value),
            ),
            PaxValue::Percent(value) => format!(
                "{api}::PaxValue::Percent({value})",
                api = self.api_path,
                value = self.percent(value),
            ),
            PaxValue::Color(value) => format!(
                "{api}::PaxValue::Color(Box::new({value}))",
                api = self.api_path,
                value = self.color(value),
            ),
            PaxValue::Rotation(value) => format!(
                "{api}::PaxValue::Rotation({value})",
                api = self.api_path,
                value = self.rotation(value),
            ),
            PaxValue::Duration(value) => format!(
                "{api}::PaxValue::Duration({value})",
                api = self.api_path,
                value = self.duration(value),
            ),
            PaxValue::PathElement(value) => format!(
                "{api}::PaxValue::PathElement(Box::new({value}))",
                api = self.api_path,
                value = self.path_element(value),
            ),
            PaxValue::Option(value) => match value.as_ref() {
                Some(value) => format!(
                    "{api}::PaxValue::Option(Box::new(Some({value})))",
                    api = self.api_path,
                    value = self.pax_value(value),
                ),
                None => format!(
                    "{api}::PaxValue::Option(Box::new(None))",
                    api = self.api_path
                ),
            },
            PaxValue::Vec(values) => format!(
                "{api}::PaxValue::Vec({values})",
                api = self.api_path,
                values = self.vec(values, |value| self.pax_value(value)),
            ),
            PaxValue::Range(start, end) => format!(
                "{api}::PaxValue::Range(Box::new({start}), Box::new({end}))",
                api = self.api_path,
                start = self.pax_value(start),
                end = self.pax_value(end),
            ),
            PaxValue::Object(entries) => format!(
                "{api}::PaxValue::Object({entries})",
                api = self.api_path,
                entries = self.vec(entries, |(key, value)| format!(
                    "({}, {})",
                    rust_string(key),
                    self.pax_value(value)
                )),
            ),
            PaxValue::Enum(value) => {
                let (scope, variant, values) = value.as_ref();
                format!(
                    "{api}::PaxValue::Enum(Box::new(({scope}, {variant}, {values})))",
                    api = self.api_path,
                    scope = rust_string(scope),
                    variant = rust_string(variant),
                    values = self.vec(values, |value| self.pax_value(value)),
                )
            }
        }
    }

    fn numeric(&self, value: &Numeric) -> String {
        let (variant, literal) = match value {
            Numeric::I8(value) => ("I8", format!("{value}i8")),
            Numeric::I16(value) => ("I16", format!("{value}i16")),
            Numeric::I32(value) => ("I32", format!("{value}i32")),
            Numeric::I64(value) => ("I64", format!("{value}i64")),
            Numeric::U8(value) => ("U8", format!("{value}u8")),
            Numeric::U16(value) => ("U16", format!("{value}u16")),
            Numeric::U32(value) => ("U32", format!("{value}u32")),
            Numeric::U64(value) => ("U64", format!("{value}u64")),
            Numeric::F64(value) => ("F64", f64_literal(*value)),
            Numeric::F32(value) => ("F32", f32_literal(*value)),
            Numeric::ISize(value) => ("ISize", format!("{value}isize")),
            Numeric::USize(value) => ("USize", format!("{value}usize")),
        };
        format!("{api}::Numeric::{variant}({literal})", api = self.api_path)
    }

    fn size(&self, value: &Size) -> String {
        match value {
            Size::Pixels(value) => format!(
                "{api}::Size::Pixels({value})",
                api = self.api_path,
                value = self.numeric(value),
            ),
            Size::Percent(value) => format!(
                "{api}::Size::Percent({value})",
                api = self.api_path,
                value = self.numeric(value),
            ),
            Size::Combined(pixels, percent) => format!(
                "{api}::Size::Combined({pixels}, {percent})",
                api = self.api_path,
                pixels = self.numeric(pixels),
                percent = self.numeric(percent),
            ),
        }
    }

    fn percent(&self, value: &Percent) -> String {
        format!(
            "{api}::Percent({value})",
            api = self.api_path,
            value = self.numeric(&value.0),
        )
    }

    fn rotation(&self, value: &Rotation) -> String {
        match value {
            Rotation::Radians(value) => format!(
                "{api}::Rotation::Radians({value})",
                api = self.api_path,
                value = self.numeric(value),
            ),
            Rotation::Degrees(value) => format!(
                "{api}::Rotation::Degrees({value})",
                api = self.api_path,
                value = self.numeric(value),
            ),
            Rotation::Percent(value) => format!(
                "{api}::Rotation::Percent({value})",
                api = self.api_path,
                value = self.numeric(value),
            ),
        }
    }

    fn duration(&self, value: &Duration) -> String {
        match value {
            Duration::Frames(value) => format!(
                "{api}::Duration::Frames({value})",
                api = self.api_path,
                value = self.numeric(value),
            ),
            Duration::Milliseconds(value) => format!(
                "{api}::Duration::Milliseconds({value})",
                api = self.api_path,
                value = self.numeric(value),
            ),
            Duration::Seconds(value) => format!(
                "{api}::Duration::Seconds({value})",
                api = self.api_path,
                value = self.numeric(value),
            ),
        }
    }

    fn color_channel(&self, value: &ColorChannel) -> String {
        match value {
            ColorChannel::Rotation(value) => format!(
                "{api}::ColorChannel::Rotation({value})",
                api = self.api_path,
                value = self.rotation(value),
            ),
            ColorChannel::Integer(value) => format!(
                "{api}::ColorChannel::Integer({value}u8)",
                api = self.api_path
            ),
            ColorChannel::Percent(value) => format!(
                "{api}::ColorChannel::Percent({value})",
                api = self.api_path,
                value = self.numeric(value),
            ),
        }
    }

    fn color(&self, value: &Color) -> String {
        match value {
            Color::rgb(r, g, b) => format!(
                "{api}::Color::rgb({r}, {g}, {b})",
                api = self.api_path,
                r = self.color_channel(r),
                g = self.color_channel(g),
                b = self.color_channel(b),
            ),
            Color::rgba(r, g, b, a) => format!(
                "{api}::Color::rgba({r}, {g}, {b}, {a})",
                api = self.api_path,
                r = self.color_channel(r),
                g = self.color_channel(g),
                b = self.color_channel(b),
                a = self.color_channel(a),
            ),
            Color::hsl(h, s, l) => format!(
                "{api}::Color::hsl({h}, {s}, {l})",
                api = self.api_path,
                h = self.rotation(h),
                s = self.color_channel(s),
                l = self.color_channel(l),
            ),
            Color::hsla(h, s, l, a) => format!(
                "{api}::Color::hsla({h}, {s}, {l}, {a})",
                api = self.api_path,
                h = self.rotation(h),
                s = self.color_channel(s),
                l = self.color_channel(l),
                a = self.color_channel(a),
            ),
            Color::SLATE => self.color_const("SLATE"),
            Color::GRAY => self.color_const("GRAY"),
            Color::ZINC => self.color_const("ZINC"),
            Color::NEUTRAL => self.color_const("NEUTRAL"),
            Color::STONE => self.color_const("STONE"),
            Color::RED => self.color_const("RED"),
            Color::ORANGE => self.color_const("ORANGE"),
            Color::AMBER => self.color_const("AMBER"),
            Color::YELLOW => self.color_const("YELLOW"),
            Color::LIME => self.color_const("LIME"),
            Color::GREEN => self.color_const("GREEN"),
            Color::EMERALD => self.color_const("EMERALD"),
            Color::TEAL => self.color_const("TEAL"),
            Color::CYAN => self.color_const("CYAN"),
            Color::SKY => self.color_const("SKY"),
            Color::BLUE => self.color_const("BLUE"),
            Color::INDIGO => self.color_const("INDIGO"),
            Color::VIOLET => self.color_const("VIOLET"),
            Color::PURPLE => self.color_const("PURPLE"),
            Color::FUCHSIA => self.color_const("FUCHSIA"),
            Color::PINK => self.color_const("PINK"),
            Color::ROSE => self.color_const("ROSE"),
            Color::BLACK => self.color_const("BLACK"),
            Color::WHITE => self.color_const("WHITE"),
            Color::TRANSPARENT => self.color_const("TRANSPARENT"),
            Color::NONE => self.color_const("NONE"),
        }
    }

    fn color_const(&self, variant: &str) -> String {
        format!("{api}::Color::{variant}", api = self.api_path)
    }

    fn path_element(&self, value: &PathElement) -> String {
        match value {
            PathElement::Empty => format!("{api}::PathElement::Empty", api = self.api_path),
            PathElement::Point(x, y) => format!(
                "{api}::PathElement::Point({x}, {y})",
                api = self.api_path,
                x = self.size(x),
                y = self.size(y),
            ),
            PathElement::Line => format!("{api}::PathElement::Line", api = self.api_path),
            PathElement::Quadratic(x, y) => format!(
                "{api}::PathElement::Quadratic({x}, {y})",
                api = self.api_path,
                x = self.size(x),
                y = self.size(y),
            ),
            PathElement::Cubic(a, b, c, d) => format!(
                "{api}::PathElement::Cubic({a}, {b}, {c}, {d})",
                api = self.api_path,
                a = self.size(a),
                b = self.size(b),
                c = self.size(c),
                d = self.size(d),
            ),
            PathElement::Close => format!("{api}::PathElement::Close", api = self.api_path),
        }
    }

    fn token(&self, token: &crate::Token) -> String {
        format!(
            "{mp}::Token {{ token_value: {token_value}, token_location: {token_location} }}",
            mp = self.manifest_path,
            token_value = rust_string(&token.token_value),
            token_location = self.option(&token.token_location, |value| self.location_info(value)),
        )
    }

    fn location_info(&self, location: &LocationInfo) -> String {
        format!(
            "{mp}::LocationInfo {{ start_line_col: ({start_line}usize, {start_col}usize), end_line_col: ({end_line}usize, {end_col}usize) }}",
            mp = self.manifest_path,
            start_line = location.start_line_col.0,
            start_col = location.start_line_col.1,
            end_line = location.end_line_col.0,
            end_col = location.end_line_col.1,
        )
    }

    fn template_node_id(&self, id: &TemplateNodeId) -> String {
        format!(
            "{mp}::TemplateNodeId::build({id}usize)",
            mp = self.manifest_path,
            id = id.as_usize(),
        )
    }

    fn type_id(&self, type_id: &TypeId) -> String {
        match type_id.get_pax_type() {
            PaxType::If => format!("{mp}::TypeId::build_if()", mp = self.manifest_path),
            PaxType::Router => format!("{mp}::TypeId::build_router()", mp = self.manifest_path),
            PaxType::Slot => format!("{mp}::TypeId::build_slot()", mp = self.manifest_path),
            PaxType::Repeat => format!("{mp}::TypeId::build_repeat()", mp = self.manifest_path),
            PaxType::Comment => format!("{mp}::TypeId::build_comment()", mp = self.manifest_path),
            PaxType::BlankComponent { pascal_identifier } => format!(
                "{mp}::TypeId::build_blank_component({pascal_identifier})",
                mp = self.manifest_path,
                pascal_identifier = rust_str(pascal_identifier),
            ),
            PaxType::Primitive { pascal_identifier } => format!(
                "{mp}::TypeId::build_primitive({pascal_identifier})",
                mp = self.manifest_path,
                pascal_identifier = rust_str(pascal_identifier),
            ),
            PaxType::Singleton { pascal_identifier } => {
                let import_path = type_id.import_path().unwrap_or_default();
                format!(
                    "{mp}::TypeId::build_singleton({import_path}, Some({pascal_identifier}))",
                    mp = self.manifest_path,
                    import_path = rust_str(&import_path),
                    pascal_identifier = rust_str(pascal_identifier),
                )
            }
            PaxType::Range { identifier } => format!(
                "{mp}::TypeId::build_range({identifier})",
                mp = self.manifest_path,
                identifier = rust_str(identifier),
            ),
            PaxType::Option { identifier } => format!(
                "{mp}::TypeId::build_option({identifier})",
                mp = self.manifest_path,
                identifier = rust_str(identifier),
            ),
            PaxType::Vector { elem_identifier } => format!(
                "{mp}::TypeId::build_vector({elem_identifier})",
                mp = self.manifest_path,
                elem_identifier = rust_str(elem_identifier),
            ),
            PaxType::Map {
                key_identifier,
                value_identifier,
            } => format!(
                "{mp}::TypeId::build_map({key_identifier}, {value_identifier})",
                mp = self.manifest_path,
                key_identifier = rust_str(key_identifier),
                value_identifier = rust_str(value_identifier),
            ),
            PaxType::Unknown => format!("{mp}::TypeId::default()", mp = self.manifest_path),
        }
    }

    fn option_box_value_definition(&self, value: &Option<Box<ValueDefinition>>) -> String {
        match value {
            Some(value) => format!("Some(Box::new({}))", self.value_definition(value)),
            None => "None".to_string(),
        }
    }

    fn option<T>(&self, value: &Option<T>, f: impl FnOnce(&T) -> String) -> String {
        match value {
            Some(value) => format!("Some({})", f(value)),
            None => "None".to_string(),
        }
    }

    fn vec<T>(&self, values: &[T], mut f: impl FnMut(&T) -> String) -> String {
        let mut output = String::from("vec![");
        for value in values {
            output.push_str(&f(value));
            output.push(',');
        }
        output.push(']');
        output
    }

    fn vec_deque<T>(
        &self,
        values: &std::collections::VecDeque<T>,
        mut f: impl FnMut(&T) -> String,
    ) -> String {
        if values.is_empty() {
            return "std::collections::VecDeque::new()".to_string();
        }

        let mut output = String::from("std::collections::VecDeque::from(vec![");
        for value in values {
            output.push_str(&f(value));
            output.push(',');
        }
        output.push_str("])");
        output
    }
}

fn map_from_entries(map_type: &str, entries: Vec<String>) -> String {
    if entries.is_empty() {
        return format!("{map_type}::new()");
    }

    format!("{map_type}::from([{}])", entries.join(","))
}

fn rust_str(value: &str) -> String {
    format!("{value:?}")
}

fn rust_string(value: &str) -> String {
    format!("{}.to_string()", rust_str(value))
}

fn f64_literal(value: f64) -> String {
    if value.is_nan() {
        "f64::NAN".to_string()
    } else if value == f64::INFINITY {
        "f64::INFINITY".to_string()
    } else if value == f64::NEG_INFINITY {
        "f64::NEG_INFINITY".to_string()
    } else {
        let mut output = String::new();
        write!(&mut output, "{value:?}").unwrap();
        output
    }
}

fn f32_literal(value: f32) -> String {
    if value.is_nan() {
        "f32::NAN".to_string()
    } else if value == f32::INFINITY {
        "f32::INFINITY".to_string()
    } else if value == f32::NEG_INFINITY {
        "f32::NEG_INFINITY".to_string()
    } else {
        let mut output = String::new();
        write!(&mut output, "{value:?}").unwrap();
        output
    }
}
