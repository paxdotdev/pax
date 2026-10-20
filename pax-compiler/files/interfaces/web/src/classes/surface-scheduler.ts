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

// Diagnostic control only: keep a small named corridor of submitted surfaces
// after candidate exit. Never allocate, override a denial, displace normal work,
// or exceed the scheduler's existing borrowing ceiling. Not a residency policy.
export function retainSubmittedProbeTiles(
    requested: string[], previous: Map<number, LayerCanvasPlan>,
    candidates: SurfaceCandidate[], decision: ReturnType<typeof materializeSurfaceSchedule>,
    canvases: Map<string, HTMLCanvasElement>, limit: [number, number],
) {
    const retained: string[] = [];
    const present = new Set(candidates.map(c => c.surface.id));
    for (const id of [...new Set(requested)].slice(0, 4)) {
        if (present.has(id)) continue;
        const old = [...previous.values()].find(p => p.surfaces.some(s => s.id === id));
        const surface = old?.surfaces.find(s => s.id === id);
        const canvas = canvases.get(id);
        if (!old || !surface || !canvas) continue;
        const d = canvas.dataset;
        // A current sibling proves the host, cell geometry and density still
        // agree. Cold, removed, resized, and reparented layers drop the control.
        const sibling = candidates.find(c => c.layerId === old.layerId
            && c.surface.hostSignature === surface.hostSignature
            && c.surface.width === surface.width && c.surface.height === surface.height
            && c.surface.backingWidth === surface.backingWidth
            && c.surface.backingHeight === surface.backingHeight);
        if (!sibling || !d.surfaceContentRevision
            || d.surfaceSubmittedRevision !== d.surfaceContentRevision
            || d.surfaceSignature !== surface.surfaceSignature
            || d.transformSignature !== surface.transformSignature
            || d.hostSignature !== surface.hostSignature) continue;
        const pixels = (surface.backingWidth ?? 0) * (surface.backingHeight ?? 0);
        if (!(pixels > 0) || decision.admitted.length >= limit[0]
            || decision.backingPixels + pixels > limit[1]) continue;
        const candidate: SurfaceCandidate = {layerId: old.layerId, branch: sibling.branch,
            surface: {...surface, replayPriority: 1_000_000},
            rank: {coverage: 7, distance: 0, retained: true, ready: true}};
        decision.admitted.push(candidate);
        const plan = decision.plans.get(old.layerId) ?? {layerId: old.layerId, active: true, surfaces: []};
        plan.surfaces.push(candidate.surface);
        decision.plans.set(old.layerId, plan);
        decision.backingPixels += pixels;
        retained.push(id);
    }
    return retained;
}
