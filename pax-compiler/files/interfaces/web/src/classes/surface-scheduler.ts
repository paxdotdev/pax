import type {SurfaceCanvasDescriptor, LayerCanvasPlan} from './surface-host-policy';

export type SurfaceRank = {
    // Visible, moving-ahead, reversal-near, retained-trailing, stationary-near, forward-reserve,
    // farther-preparation, outside-preparation. Ordering belongs to Rust.
    coverage: 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7;
    distance: number;
    retained: boolean;
    ready?: boolean;
};
export type SurfaceCandidate = {
    layerId: number;
    branch: number;
    surface: SurfaceCanvasDescriptor;
    rank: SurfaceRank;
};
export type SurfaceDenial = {id: string; reason: string};
export type SharedSurfaceDecision = {
    admitted: number[];
    denied: [number, string][];
    ranks: SurfaceRank[];
    overloaded: boolean;
    backingPixels: number;
};

// Translation only: the shared Rust core owns ranking and admission. Preserve
// its order and attach the original descriptors without a browser policy fork.
export function materializeSurfaceSchedule(candidates: SurfaceCandidate[], decision: SharedSurfaceDecision) {
    candidates.forEach((c, i) => {c.rank = decision.ranks[i];});
    const admitted = decision.admitted.map(i => candidates[i]);
    const denied: SurfaceDenial[] = decision.denied.map(([i, reason]) => ({id: candidates[i].surface.id, reason}));
    const plans = new Map<number, LayerCanvasPlan>();
    admitted.forEach((candidate, priority) => {
        const plan = plans.get(candidate.layerId) ?? {layerId: candidate.layerId, active: true, surfaces: []};
        // An ordinal encodes the single global rank for existing renderer APIs.
        // It is preparation order, never draw/z order or surface identity.
        plan.surfaces.push({...candidate.surface,
            // Existing replay batching uses <= 0 for visible coverage.
            replayPriority: candidate.rank.coverage === 0 ? priority - admitted.length : priority + 1});
        plans.set(candidate.layerId, plan);
    });
    return {plans, admitted, denied, overloaded: decision.overloaded, backingPixels: decision.backingPixels};
}
