#[cfg(feature = "shaders")]
#[allow(unused_imports)]
use crate::RimRefraction;
use pax_engine::*;

/// Draw a thin scene-lit CMY contour on the alpha silhouette of a button face.
/// Project vector artwork here and keep native controls and labels as siblings.
/// Requires the shaders feature, a GPU renderer, and a scoped direct light.
#[pax]
#[engine_import_path("pax_engine")]
#[cfg_attr(feature = "shaders", file("reference/button_rim.pax"))]
#[cfg_attr(not(feature = "shaders"), inlined(slot()))]
#[pax_document(children)]
pub struct ButtonRim {
    /// Enable the contour. Defaults to false, which draws the face directly.
    pub enabled: Property<bool>,
}
