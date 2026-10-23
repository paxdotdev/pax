import {describeSurfaceHost} from "./surface-host-policy";
import type {LayerCanvasPlan} from "./surface-host-policy";
import type {RenderLayerManager} from "./render-layer-context";
import type {SurfaceTrace} from './surface-trace';
import type {CanvasPool} from './canvas-pool';

export type InstalledSurfacePlan = {
    version: 2; revision: number; pending: string[];
    surfaces: {id: string; layerId: number; generation: string; key: string; hostSignature: string;
        left: number; top: number; width: number; height: number;
        backingWidth: number; backingHeight: number; densityClamped: boolean;
        replayPriority: number; active: boolean}[];
};

// Executes one plan transaction. Selection policy belongs to the scheduler;
// this owner only reconciles DOM assignments and notifies renderer ownership.
export class SurfaceCoordinator {
    private lastCanvasSurfaceSignatures = new Map<string, string>();
    private lastCanvasTransformSignatures = new Map<string, string>();
    private lastCanvasPriorities = new Map<string, number>();
    private lastCanvasActiveStates = new Map<string, boolean>();
    private lastCanvasLayerCounts = new Map<number, number>();
    private revision = 0;

    constructor(private canvases: Map<string, HTMLCanvasElement>,
        private layers: RenderLayerManager, private refresh: (layers: number[]) => void,
        private publish: (plan: InstalledSurfacePlan) => void = () => {},
        private begin: () => void = () => {}, private trace?: SurfaceTrace,
        private capacity?: () => ReturnType<CanvasPool['capacitySnapshot']> | undefined) {}

    sync(planProvider: (layer: number) => LayerCanvasPlan | null | undefined) {
        this.begin();
        const desired: string[] = [];
        this.layers.syncLayerCanvasLayouts(layer => {
            const plan = planProvider(layer);
            if (plan) desired.push(...plan.surfaces.map(s => s.id));
            return plan;
        }, (retiredLayers) => this.refresh(retiredLayers));
        // Publication follows every retirement/allocation/placement operation,
        // and precedes any initializer observing the newly installed targets.
        const installed: InstalledSurfacePlan = {version: 2, revision: ++this.revision,
            pending: desired.filter(id => !this.canvases.has(id)), surfaces: []};
        this.canvases.forEach((canvas, id) => {
            const d = canvas.dataset;
            if (this.trace) {
                d.surfaceTrace = '1';
                if (Number(d.replayPriority) <= 0 && !d.surfaceFirstVisibleAt) {
                    d.surfaceFirstVisibleAt = String(this.trace.firstVisibleAt(id) ?? Date.now());
                    this.trace.record('first-visible', {id, generation: d.surfaceGeneration,
                        contentRevision: d.surfaceContentRevision});
                }
                if (!Number(d.paxSurfaceOwners ?? 0)
                    && d.surfaceQueuedRevision !== d.surfaceContentRevision) {
                    d.surfaceQueuedRevision = d.surfaceContentRevision;
                    this.trace.record('initialization-queued', {id, generation: d.surfaceGeneration,
                        contentRevision: d.surfaceContentRevision, priority: Number(d.replayPriority)});
                }
            }
            const width = Number(d.logicalWidth), height = Number(d.logicalHeight);
            const backingWidth = Number(d.expectedBackingWidth), backingHeight = Number(d.expectedBackingHeight);
            if (!(width > 0 && height > 0 && backingWidth > 0 && backingHeight > 0) || !canvas.parentElement) return;
            installed.surfaces.push({id, layerId: Number(d.layerId), generation: d.surfaceGeneration ?? '',
                key: d.tileKey ?? id, hostSignature: d.hostSignature ?? '',
                left: Number(d.tileOriginX ?? 0), top: Number(d.tileOriginY ?? 0), width, height,
                backingWidth, backingHeight, densityClamped: d.densityClamped === 'true',
                replayPriority: Number(d.replayPriority ?? 0),
                active: canvas.parentElement.dataset.renderState !== 'parked'});
        });
        this.publish(installed);
        const capacity = this.capacity?.();
        this.trace?.record('installed', {revision: installed.revision,
            surfaces: installed.surfaces, pending: installed.pending,
            capacity, pendingReason: installed.pending.length
                ? capacity?.retiring ? 'awaiting-lease-release' : 'awaiting-capacity' : undefined});
        const backingMismatchLayers = this.collectBackingMismatchLayers();
        let nextSurfaceSignatures = new Map<string, string>();
        let nextTransformSignatures = new Map<string, string>();
        let nextPriorities = new Map<string, number>();
        let nextActiveStates = new Map<string, boolean>();
        let nextLayerCounts = new Map<number, number>();
        let surfaceChangedLayers = new Set<number>(backingMismatchLayers);
        let transformChangedLayers = new Set<number>();
        let priorityChangedLayers = new Set<number>();
        let activeChangedLayers = new Set<number>();
        this.canvases.forEach((canvas, id) => {
            let parent = canvas.parentElement;
            if (parent == null) {
                return;
            }
            let hostSignature = canvas.dataset.hostSignature ?? describeSurfaceHost(parent);
            let layerId = Number.parseInt(canvas.dataset.layerId ?? "", 10);
            let surfaceSignature =
                `${canvas.dataset.surfaceSignature
                    ?? `${canvas.clientWidth}x${canvas.clientHeight}@${hostSignature}`}#host=${hostSignature}`
                + `#generation=${canvas.dataset.surfaceGeneration ?? ""}#layer=${layerId}`
                + `#key=${canvas.dataset.tileKey ?? id}`
                + `#size=${canvas.dataset.logicalWidth},${canvas.dataset.logicalHeight}`
                + `#backing=${canvas.dataset.expectedBackingWidth},${canvas.dataset.expectedBackingHeight}`
                + `#clamped=${canvas.dataset.densityClamped ?? 'false'}`;
            let transformSignature =
                `${canvas.dataset.transformSignature ?? ''}`
                + `#origin=${canvas.dataset.tileOriginX ?? "0"},${canvas.dataset.tileOriginY ?? "0"}`;
            const priority = Number(canvas.dataset.replayPriority ?? 0);
            const active = parent.dataset?.renderState !== 'parked';
            nextSurfaceSignatures.set(id, surfaceSignature);
            nextTransformSignatures.set(id, transformSignature);
            nextPriorities.set(id, priority);
            nextActiveStates.set(id, active);
            if (Number.isFinite(layerId)) {
                nextLayerCounts.set(layerId, (nextLayerCounts.get(layerId) ?? 0) + 1);
            }
            if (this.lastCanvasSurfaceSignatures.get(id) !== surfaceSignature) {
                if (Number.isFinite(layerId)) {
                    surfaceChangedLayers.add(layerId);
                }
            }
            if (this.lastCanvasTransformSignatures.get(id) !== transformSignature) {
                if (Number.isFinite(layerId)) {
                    transformChangedLayers.add(layerId);
                }
            }
            if (Number.isFinite(layerId)) {
                if (this.lastCanvasPriorities.get(id) !== priority) priorityChangedLayers.add(layerId);
                if (this.lastCanvasActiveStates.get(id) !== active) activeChangedLayers.add(layerId);
            }
        });
        this.lastCanvasLayerCounts.forEach((count, layerId) => {
            if ((nextLayerCounts.get(layerId) ?? 0) !== count) {
                surfaceChangedLayers.add(layerId);
            }
        });
        nextLayerCounts.forEach((count, layerId) => {
            if ((this.lastCanvasLayerCounts.get(layerId) ?? 0) !== count) {
                surfaceChangedLayers.add(layerId);
            }
        });
        this.lastCanvasSurfaceSignatures = nextSurfaceSignatures;
        this.lastCanvasTransformSignatures = nextTransformSignatures;
        this.lastCanvasPriorities = nextPriorities;
        this.lastCanvasActiveStates = nextActiveStates;
        this.lastCanvasLayerCounts = nextLayerCounts;
        // Recomputing admission does not invalidate every installed surface.
        // Priority-only changes still notify their layer so queued warm replay
        // is reclassified, without refreshing unrelated geometry or ownership.
        let layers = Array.from(new Set([
            ...surfaceChangedLayers, ...transformChangedLayers,
            ...priorityChangedLayers, ...activeChangedLayers,
        ])).sort((left, right) => left - right);
        if (layers.length > 0) {
            this.refresh(layers);
            this.trace?.record('refresh', {layers, backingMismatch: [...backingMismatchLayers],
                surfaceChanged: [...surfaceChangedLayers], transformChanged: [...transformChangedLayers],
                priorityChanged: [...priorityChangedLayers], activeChanged: [...activeChangedLayers]});
        }
    }

    collectBackingMismatchLayers() {
        let layers = new Set<number>();
        this.canvases.forEach((canvas) => {
            // An assignment is published before its renderer exists. Until that
            // renderer acknowledges configuration, a 1x1 backing is expected.
            // Acknowledgements from a prior lease cannot validate this assignment.
            const generation = canvas.dataset.surfaceGeneration;
            if (generation && canvas.dataset.surfaceConfiguredGeneration !== generation) return;
            let layerId = Number.parseInt(canvas.dataset.layerId ?? "", 10);
            if (!Number.isFinite(layerId)) {
                return;
            }
            // Backing sizes are exported by the same Rust helper that allocates
            // them, including fractional rounding and backend hard limits.
            const width = Number(canvas.dataset.expectedBackingWidth);
            const height = Number(canvas.dataset.expectedBackingHeight);
            if (Number.isFinite(width) && Number.isFinite(height) && width > 0 && height > 0
                && (canvas.width !== width || canvas.height !== height)) {
                layers.add(layerId);
            }
        });
        return layers;
    }

}
