//! Capture inputs are isolated scenes, composed by the same depth/opacity painter as
//! the main surface. Planning state and clip slots belong to the capture domain so
//! later draws cannot overwrite resources already referenced by a queued capture.
use super::*;
use crate::render_backend::capture::CaptureKey;

#[cfg(feature = "3d")]
#[derive(Default)]
pub(super) struct SpatialComposition {
    pub clip_keys: Vec<ClipArenaKey>,
    pub solid_clips: HashMap<u32, Vec<ClipReference>>,
    pub solid_clip_plans: HashMap<ClipArenaKey, solid::SolidClipPlan>,
    pub solid_clip_keys: Vec<ClipArenaKey>,
    pub region: Option<(usize, u64, u32)>,
    pub fades: spatial_fade::SpatialFades,
    pub hidden: HashSet<u32>,
    pub namespace: u64,
    // Capture membership is already explicit in sorted_nodes. Main-scene membership
    // instead excludes nodes owned by detached sources and shader containers.
    pub capture: bool,
    captures: HashSet<CaptureKey>,
}

impl WgpuRenderer<'_> {
    pub(super) fn belongs_to_draw_domain(&self, id: u32) -> bool {
        #[cfg(feature = "3d")]
        if self.spatial.capture {
            return true;
        }
        #[cfg(feature = "shaders")]
        if self.effect_nodes.contains_key(&id) {
            return false;
        }
        !self.source_nodes.contains_key(&id)
    }

    pub(super) fn hash_capture_content(&self, id: u32, hash: &mut DefaultHasher) {
        self.hash_opacity_content(id, hash);
        #[cfg(feature = "3d")]
        if let Some(plane) = self.depth_planes.get(&id) {
            for coefficient in plane.coefficients {
                coefficient.to_bits().hash(hash);
            }
            plane.backface.hash(hash);
            plane.presentation_layer.hash(hash);
        }
    }

    pub(super) fn draw_capture_scene(
        &mut self,
        key: CaptureKey,
        nodes: &[u32],
        active: &mut HashSet<CaptureKey>,
    ) {
        #[cfg(not(feature = "3d"))]
        {
            let _ = key;
            let plan = self.opacity_plan_at_depth(nodes, 0);
            collect_opacity_group_keys(&plan, active);
            self.draw_opacity_plan(&plan);
        }
        #[cfg(feature = "3d")]
        {
            let mut composition = self.capture_scenes.remove(&key).unwrap_or_default();
            let mut namespace = DefaultHasher::new();
            key.hash(&mut namespace);
            composition.namespace = namespace.finish();
            composition.capture = match key {
                CaptureKey::Presentation(..) => self.spatial.capture,
                _ => true,
            };
            let main = std::mem::replace(&mut self.spatial, composition);
            let main_nodes = std::mem::replace(
                &mut self.sorted_nodes,
                nodes
                    .iter()
                    .map(|id| (self.scene[id].z_index(), *id))
                    .collect(),
            );
            let mut captures = HashSet::new();
            self.draw_spatial_scene(&mut captures);
            self.spatial.captures = captures;
            self.sorted_nodes = main_nodes;
            let composition = std::mem::replace(&mut self.spatial, main);
            active.extend(&composition.captures);
            self.capture_scenes.insert(key, composition);
        }
    }

    pub(super) fn retain_capture_scene(
        &self,
        key: CaptureKey,
        _nodes: &[u32],
        active: &mut HashSet<CaptureKey>,
    ) {
        #[cfg(feature = "3d")]
        if let Some(composition) = self.capture_scenes.get(&key) {
            active.extend(&composition.captures);
        }
        #[cfg(not(feature = "3d"))]
        {
            let _ = key;
            // Preserve the existing 2D opacity-cache residency even when the outer
            // source hits its cache; a later paint update can reuse those groups.
            collect_opacity_group_keys(&self.opacity_plan_at_depth(_nodes, 0), active);
        }
    }

    #[cfg(feature = "3d")]
    pub(super) fn retire_capture_scenes(&mut self, active: &HashSet<CaptureKey>) {
        self.capture_scenes.retain(|key, composition| {
            if active.contains(key) {
                return true;
            }
            self.clip_arena.release_keys(&composition.clip_keys);
            self.clip_arena.release_keys(&composition.solid_clip_keys);
            false
        });
    }

    #[cfg(feature = "3d")]
    pub(super) fn composition_clip_keys(&self) -> impl Iterator<Item = &ClipArenaKey> {
        std::iter::once(&self.spatial)
            .chain(self.capture_scenes.values())
            .flat_map(|composition| {
                composition
                    .clip_keys
                    .iter()
                    .chain(&composition.solid_clip_keys)
            })
    }
}

#[cfg(all(test, feature = "3d"))]
impl WgpuRenderer<'_> {
    pub(crate) fn spatial_capture_resource_counts(&self) -> (usize, usize) {
        (
            self.capture_scenes.len(),
            self.capture_scenes
                .values()
                .map(|s| s.clip_keys.len() + s.solid_clip_keys.len())
                .sum(),
        )
    }
}
