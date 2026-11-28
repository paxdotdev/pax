#[cfg(feature = "shaders")]
#[allow(unused_imports)]
use crate::{CloudGlass, Group, PoolFloorSurface, RimRefraction};
use pax_engine::api::{Event, NodeContext, Store, ViewportProximityChange};
use pax_engine::*;

/// Shared phase in seconds for animated button faces. Visible buttons follow the
/// host's clock. Holding the phase freezes the pattern while scene lights respond.
pub struct ButtonSurfacePhase(pub LocalProperty<f64>);
impl Store for ButtonSurfacePhase {}

/// Original name for the shared cloud-button clock.
pub use ButtonSurfacePhase as ButtonCloudPhase;

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
    /// Some(density) draws cloud glass with a 2 px rim when no pool density is set.
    /// With both densities unset the face uses the 1 px contour.
    /// For cloud glass, project an opaque silhouette and supply the CPU fallback as its paint.
    pub cloud_density: Property<Option<f64>>,
    /// Some(density) applies pool waves to the projected paint with a 2 px rim.
    /// Takes precedence over cloud glass. The shared surface phase controls wave time.
    pub pool_density: Property<Option<f64>>,
    /// Pool wave strength, from 0 (base paint) to 1 (default). Does not attenuate the rim.
    pub pool_amplitude: Property<Option<f64>>,
    pub _phase: Property<f64>,
}

impl ButtonRim {
    pub fn cloud_visibility(&mut self, ctx: &NodeContext, event: Event<ViewportProximityChange>) {
        let visible = event.current.is_in_viewport();
        if visible == event.previous.as_ref().is_some_and(|p| p.is_in_viewport()) {
            return;
        }
        if visible {
            if let Ok(phase) = ctx.with_store(|s: &mut ButtonSurfacePhase| s.0.clone()) {
                let deps = [phase.untyped()];
                self._phase
                    .replace_with(LocalProperty::computed_with_cutoff(
                        move || phase.get(),
                        &deps,
                        PartialEq::eq,
                    ));
            }
        } else {
            self.pause_cloud(ctx);
        }
    }

    pub fn pause_cloud(&mut self, _: &NodeContext) {
        self._phase
            .replace_with(LocalProperty::new(self._phase.get()));
    }
}
