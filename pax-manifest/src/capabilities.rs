//! Capability checks shared by cartridge generation and template reload.
use crate::{
    PaxManifest, SettingElement, SettingsBlockElement, TimelineBlockElement,
    TimelineSelectorElement,
};

pub const REQUIRES_3D: &str = "Pax spatial properties require the `3d` feature. Enable `pax-kit = { ..., features = [\"3d\"] }` (or `pax-engine/3d` and `pax-std/3d` for direct dependencies), then rebuild the application.";

fn spatial_property(name: &str) -> bool {
    matches!(name, "rotate_x" | "rotate_y" | "translate_z")
}

fn settings_require_3d(settings: &[SettingElement]) -> bool {
    settings.iter().any(|entry| matches!(entry, SettingElement::Setting(key, _) if spatial_property(&key.token_value)))
}

fn blocks_require_3d(blocks: &[SettingsBlockElement]) -> bool {
    blocks.iter().any(|block| match block {
        SettingsBlockElement::SelectorBlock(_, body) => settings_require_3d(&body.elements),
        SettingsBlockElement::Conditional(conditional) => conditional
            .branches
            .iter()
            .any(|branch| blocks_require_3d(&branch.elements)),
        _ => false,
    })
}

impl PaxManifest {
    /// Whether any component authors spatial common properties, including dormant
    /// selectors, conditional branches and timeline tracks. Compiler type vocabulary
    /// alone does not imply a runtime requirement.
    pub fn requires_3d(&self) -> bool {
        for component in self.components.values() {
            if let Some(template) = &component.template {
                if template
                    .get_nodes()
                    .iter()
                    .any(|node| node.settings.as_deref().is_some_and(settings_require_3d))
                {
                    return true;
                }
            }
            if component.settings.as_deref().is_some_and(blocks_require_3d) {
                return true;
            }
            for timeline in &component.timelines {
                for block in &timeline.elements {
                    if let TimelineBlockElement::SelectorBlock(_, body) = block {
                        if body.elements.iter().any(|entry| matches!(entry,
                            TimelineSelectorElement::Track(key, _) if spatial_property(&key.token_value))) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Validate against the receiving runtime, not the Cargo features of a build tool.
    /// The same check applies to rich and release-baked manifests.
    pub fn validate_runtime_capabilities(&self, supports_3d: bool) -> Result<(), &'static str> {
        if !supports_3d && self.requires_3d() {
            Err(REQUIRES_3D)
        } else {
            Ok(())
        }
    }
}
