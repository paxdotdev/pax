use super::*;

/// Conservative dependencies of retained vector draws. Images and unlit materials
/// contribute nothing; radius/clip intersection is deliberately not inferred here.
#[derive(Clone, Copy, Default)]
pub(super) struct LightingDependencies {
    direct: u32,
    lit: bool,
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
        assert_eq!(LightingDependencies::default().signature(&lighting), 0);

        let ambient_only = LightingDependencies {
            direct: 0,
            lit: true,
        };
        assert_eq!(ambient_only.signature(&lighting), 0);
        lighting.meta[2] = 1;
        let ambient = ambient_only.signature(&lighting);
        assert_ne!(ambient, 0);
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
