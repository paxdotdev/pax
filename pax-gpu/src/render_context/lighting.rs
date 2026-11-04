use super::*;

pub(super) fn to_gpu_tile_lighting(
    lighting: &SceneLighting,
    surface_transform: Transform2D,
) -> GpuSceneLighting {
    let mut gpu = to_gpu_scene_lighting(lighting);
    // The shader receives transformed (tile-local) positions, not logical-layer
    // coordinates. Undo only the surface base; authored node transforms remain.
    // A singular surface has no drawable area, so its mapping cannot contribute.
    if let Some(inverse) = surface_transform.inverse() {
        let world = inverse.then(&Transform2D::from_array(lighting.surface_to_world));
        gpu.surface_to_world_x = [world.m11, world.m21, world.m31, 0.];
        gpu.surface_to_world_y = [world.m12, world.m22, world.m32, 0.];
    }
    gpu
}

/// Conservative dependencies of retained vector draws. Images and unlit materials
/// contribute nothing; radius/clip intersection is deliberately not inferred here.
#[derive(Clone, Copy, Default)]
pub(super) struct LightingDependencies {
    pub(super) direct: u32,
    pub(super) lit: bool,
}

impl LightingDependencies {
    pub(super) fn from_ops(ops: &[PendingVectorOp]) -> Self {
        let mut result = Self::default();
        for op in ops.iter().filter(|op| !op.material.unlit) {
            result.lit = true;
            result.direct |= op.light_mask;
        }
        result
    }

    fn include(&mut self, other: Self) {
        self.direct |= other.direct;
        self.lit |= other.lit;
    }

    pub(super) fn signature(self, lighting: &GpuSceneLighting) -> u64 {
        let active_mask = (1u32 << lighting.meta[1]) - 1;
        let eligible = self.direct & active_mask;
        // Match geometry.wgsl's early returns, including the default unlit
        // appearance when neither direct nor authored ambient light can reach a draw.
        if !self.lit || lighting.meta[0] == 0 || (eligible == 0 && lighting.meta[2] == 0) {
            return 0;
        }
        let mut hash = DefaultHasher::new();
        lighting.meta[2].hash(&mut hash);
        bytemuck::bytes_of(&lighting.ambient).hash(&mut hash);
        eligible.hash(&mut hash);
        if eligible != 0 {
            bytemuck::bytes_of(&lighting.view).hash(&mut hash);
            bytemuck::bytes_of(&lighting.surface_to_world_x).hash(&mut hash);
            bytemuck::bytes_of(&lighting.surface_to_world_y).hash(&mut hash);
        }
        for (slot, light) in lighting.lights.iter().enumerate() {
            if eligible & (1 << slot) != 0 {
                bytemuck::bytes_of(light).hash(&mut hash);
            }
        }
        hash.finish()
    }
}

impl WgpuRenderer<'_> {
    pub(super) fn lighting_dependencies(&mut self) -> LightingDependencies {
        *self.lighting_dependencies.get_or_insert_with(|| {
            let mut dependencies = LightingDependencies::default();
            // Include detached capture sources: their pixels can affect a visible
            // mask even when the source itself is absent from the visible draw list.
            for node in self.scene.values() {
                #[cfg(feature = "3d")]
                if let RetainedNode::Solid(n) = node {
                    dependencies.include(LightingDependencies {
                        direct: n.light_mask,
                        lit: !n.material.unlit,
                    });
                }
                if let RetainedNode::Vector(node) = node {
                    dependencies.include(node.lighting_dependencies);
                }
            }
            dependencies
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_lighting_preserves_logical_world_coordinates() {
        let lighting = SceneLighting {
            surface_to_world: [0., 2., -2., 0., 1000., 2000.],
            ..Default::default()
        };
        let base = Transform2D::translation(-128., -256.);
        let gpu = to_gpu_tile_lighting(&lighting, base);
        let logical = lyon::geom::point(140., 280.);
        let tile = base.transform_point(logical);
        let expected = Transform2D::from_array(lighting.surface_to_world).transform_point(logical);
        assert_eq!(
            gpu.surface_to_world_x[0] * tile.x
                + gpu.surface_to_world_x[1] * tile.y
                + gpu.surface_to_world_x[2],
            expected.x
        );
        assert_eq!(
            gpu.surface_to_world_y[0] * tile.x
                + gpu.surface_to_world_y[1] * tile.y
                + gpu.surface_to_world_y[2],
            expected.y
        );
        assert_eq!(
            to_gpu_tile_lighting(&lighting, Transform2D::identity()).surface_to_world_x,
            to_gpu_scene_lighting(&lighting).surface_to_world_x
        );
    }

    #[test]
    fn dependencies_follow_shader_eligibility_and_ambient_transitions() {
        let deps = LightingDependencies {
            direct: 1,
            lit: true,
        };
        let mut lighting = GpuSceneLighting::default();
        lighting.meta = [1, 2, 0, 0];
        lighting.ambient = [1.0, 1.0, 1.0, 0.35];
        let before = deps.signature(&lighting);
        lighting.lights[1].position[0] = 30.0;
        assert_eq!(deps.signature(&lighting), before);
        lighting.lights[0].position[0] = 30.0;
        assert_ne!(deps.signature(&lighting), before);
        let before_camera = deps.signature(&lighting);
        lighting.view = [20., 30., 200., 1.];
        assert_ne!(
            deps.signature(&lighting),
            before_camera,
            "camera motion invalidates lit captures"
        );
        assert_eq!(LightingDependencies::default().signature(&lighting), 0);

        let ambient_only = LightingDependencies {
            direct: 0,
            lit: true,
        };
        assert_eq!(ambient_only.signature(&lighting), 0);
        lighting.meta[2] = 1;
        let ambient = ambient_only.signature(&lighting);
        assert_ne!(ambient, 0);
        lighting.view = [0., 0., 1., 0.];
        assert_eq!(
            ambient_only.signature(&lighting),
            ambient,
            "ambient-only captures do not depend on the camera"
        );
        lighting.lights[0].position[0] = 60.0;
        assert_eq!(ambient_only.signature(&lighting), ambient);
        lighting.ambient[3] = 0.5;
        assert_ne!(ambient_only.signature(&lighting), ambient);
        lighting.meta[2] = 0;
        lighting.meta[1] = 0;
        assert_eq!(deps.signature(&lighting), 0, "last direct light removed");
        lighting.meta[1] = 2;
        lighting.meta[0] = 0;
        assert_eq!(deps.signature(&lighting), 0, "scene disabled");
    }
}
