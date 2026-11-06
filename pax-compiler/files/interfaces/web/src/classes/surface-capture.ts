import {TraceRing} from './trace-ring';
import type {SurfaceCandidate, SurfaceDenial} from './surface-scheduler';

export type CaptureRect = [number, number, number, number]; // left, top, right, bottom
export type SurfaceWaitStage = 'assignment' | 'initializer' | 'configuration' | 'submission' | 'submitted';
type SurfaceObservation = {
    id: string; layer: number; owner: number; rect: CaptureRect;
    stage: SurfaceWaitStage; denial?: string; generation?: string; revision?: string;
};

function intersection(a: CaptureRect, b: CaptureRect): CaptureRect {
    return [Math.max(a[0], b[0]), Math.max(a[1], b[1]), Math.min(a[2], b[2]), Math.min(a[3], b[3])];
}
function valid(r: CaptureRect) { return r.every(Number.isFinite) && r[2] > r[0] && r[3] > r[1]; }

// Union only within one logical layer/coordinate space. Adding the areas of
// nested or overlapping layers would not measure missing screen pixels.
export function rectUnionArea(rects: CaptureRect[]): number {
    rects = rects.filter(valid);
    const xs = [...new Set(rects.flatMap(r => [r[0], r[2]]))].sort((a,b) => a-b);
    let area = 0;
    for (let i = 1; i < xs.length; i++) {
        const intervals = rects.filter(r => r[0] < xs[i] && r[2] > xs[i-1])
            .map(r => [r[1], r[3]]).sort((a,b) => a[0]-b[0]);
        let end = -Infinity, height = 0;
        for (const [top, bottom] of intervals) {
            height += Math.max(0, bottom - Math.max(top, end));
            end = Math.max(end, bottom);
        }
        area += (xs[i] - xs[i-1]) * height;
    }
    return area;
}

export function observeSurfaceReadiness(candidates: SurfaceCandidate[], denied: SurfaceDenial[],
    canvases: Map<string, HTMLCanvasElement>, clips: Map<number, CaptureRect>, rootClip: CaptureRect) {
    const denials = new Map(denied.map(d => [d.id, d.reason]));
    const surfaces: SurfaceObservation[] = [];
    for (const c of candidates) {
        if (c.rank.coverage !== 0) continue;
        const s = c.surface, clip = c.branch < 0 ? rootClip : clips.get(c.branch);
        if (!clip) continue;
        const rect = intersection([s.left, s.top, s.left+s.width, s.top+s.height], clip);
        if (!valid(rect)) continue;
        const d = canvases.get(s.id)?.dataset;
        const matching = d && d.hostSignature === s.hostSignature
            && d.surfaceSignature === s.surfaceSignature && d.transformSignature === s.transformSignature;
        const configured = matching && !!d.surfaceGeneration && d.surfaceConfiguredGeneration === d.surfaceGeneration;
        const stage: SurfaceWaitStage = !matching ? 'assignment'
            : configured ? d.surfaceContentRevision && d.surfaceSubmittedRevision === d.surfaceContentRevision
                ? 'submitted' : 'submission'
            : !Number(d.paxSurfaceOwners ?? 0) ? 'initializer' : 'configuration';
        surfaces.push({id:s.id, layer:c.layerId, owner:c.branch, rect, stage,
            denial:denials.get(s.id), generation:matching ? d.surfaceGeneration : undefined,
            revision:matching ? d.surfaceContentRevision : undefined});
    }
    const layers = [...new Set(surfaces.map(s => s.layer))].map(layer => {
        const members = surfaces.filter(s => s.layer === layer);
        return {layer, owner:members[0].owner, area:rectUnionArea(members.map(s => s.rect)),
            unsubmittedArea:rectUnionArea(members.filter(s => s.stage !== 'submitted').map(s => s.rect)),
            stages:Object.fromEntries((['assignment','initializer','configuration','submission','submitted'] as const)
                .map(stage => [stage, rectUnionArea(members.filter(s => s.stage === stage).map(s => s.rect))]))};
    });
    return {surfaces, layers};
}

// A finite diagnostic recording, independent of the continuously rolling trace.
// No DOM/GPU objects, screenshots or application content are retained here.
export class SurfaceCapture {
    private events: TraceRing<Record<string, unknown>>;
    private observations: TraceRing<Record<string, unknown>>;
    private phases = new Map<string, {count:number; totalMs:number; maxMs:number}>();
    private endedAt?: number;
    private previousObservation?: number;
    private previousFrame?: number;
    private maxObservationGapMs = 0;
    private frameCount = 0;
    private maxFrameGapMs = 0;
    private slowFrames = 0;
    private observationCount = 0;
    private unsubmittedObservations = 0;

    constructor(readonly startedAt: number, private metadata: Record<string, unknown>, capacity = 8192) {
        this.events = new TraceRing(capacity);
        this.observations = new TraceRing(Math.min(capacity, 2048));
    }
    get active() { return this.endedAt == null; }
    event(at: number, event: string, details: Record<string, unknown> = {}) {
        if (this.active) this.events.push({at, event, ...details});
    }
    phase(name: string, duration: number, count: number, maximum: number) {
        if (!this.active || !Number.isFinite(duration)) return;
        if (!this.phases.has(name) && this.phases.size >= 128) return;
        const p = this.phases.get(name) ?? {count:0, totalMs:0, maxMs:0};
        p.count += count; p.totalMs += duration; p.maxMs = Math.max(p.maxMs, maximum);
        this.phases.set(name,p);
        if (name === 'continuation.host_wait') {
            const at = performance.now();
            this.event(at,'host-task-delivered',{queuedAt:at-duration,waitMs:duration});
        }
    }
    frame(at: number, hidden: boolean) {
        if (!this.active) return;
        const interval = this.previousFrame == null ? undefined : at-this.previousFrame;
        this.previousFrame = at; this.frameCount++;
        if (interval != null) {
            this.maxFrameGapMs = Math.max(this.maxFrameGapMs, interval);
            if (interval > 50) this.slowFrames++;
        }
        this.observations.push({at,event:'display-callback',interval,hidden});
    }
    observe(at: number, observation: ReturnType<typeof observeSurfaceReadiness>) {
        if (!this.active) return;
        if (this.previousObservation != null) this.maxObservationGapMs = Math.max(this.maxObservationGapMs, at-this.previousObservation);
        this.previousObservation = at; this.observationCount++;
        if (observation.layers.some(l => l.unsubmittedArea > 0)) this.unsubmittedObservations++;
        this.observations.push({at,event:'planned-coverage',...observation});
    }
    stop(at: number) { this.endedAt ??= at; }
    snapshot() {
        return {version:1, startedAt:this.startedAt, endedAt:this.endedAt, capturedAt:performance.now(), metadata:this.metadata,
            semantics:'Main-thread observations, not GPU completion or displayed pixels. Areas are owner-local per layer; do not sum across layers. Coverage uses the latest planner geometry, not a separate compositor observation.',
            summary:{observationCount:this.observationCount, unsubmittedObservations:this.unsubmittedObservations,
                maxObservationGapMs:this.maxObservationGapMs, frameCount:this.frameCount,
                maxFrameGapMs:this.maxFrameGapMs, frameGapsOver50Ms:this.slowFrames,
                phases:Object.fromEntries(this.phases)},
            overwritten:{events:this.events.overwritten, observations:this.observations.overwritten},
            events:this.events.snapshot(), observations:this.observations.snapshot()};
    }
}
