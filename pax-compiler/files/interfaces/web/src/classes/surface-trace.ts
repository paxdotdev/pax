import type {SurfaceCandidate, SurfaceDenial} from './surface-scheduler';
import type {NativeMessageTiming} from './native-message-profile';
import type {WheelInputSample} from '../events/listeners';

export type SurfaceTraceEvent = {at: number; event: string; id?: string; generation?: string;
    contentRevision?: string; [key: string]: unknown};

// Opt-in, bounded, and attached to the app mount so multiple chassis never
// mingle their traces. No network reporting or per-frame console output.
export class SurfaceTrace {
    private events: SurfaceTraceEvent[] = [];
    private demand = new Map<string, {signature: string; firstVisibleAt?: number; coverage: number; owner: number; denial?: string}>();
    private scrollDirections = new Map<number, {offset: number[]; direction: number[]}>();
    private inspector?: HTMLDetailsElement;
    private lastUnready = '';
    private probeFrame?: number;
    private probeUntil = -Infinity;
    private scrollUntil = -Infinity;
    private wheelInputs: WheelInputSample[] = [];
    private imageCounts: Record<string, number> = {};
    private completionRequests = {queued: 0, coalesced: 0, used: 0, disposed: 0};
    private completionDeferredAt?: number;
    private completionWait = {opportunities: 0, totalMs: 0, maxMs: 0};
    private imageStates = new Map<string, string>();
    private pendingImages = new Map<string, {node: number; path: string; id: string}>();
    private recordedImages = new Map<string, Set<string>>();
    private listener = (event: Event) => {
        const canvas = event.target as HTMLCanvasElement;
        const name = canvas.dataset.surfaceEvent ?? 'unknown';
        const id = canvas.id || canvas.dataset.surfaceRetiredId || '';
        const node = Number(canvas.dataset.imageNode ?? 0);
        const key = `${id}|${canvas.dataset.surfaceGeneration}|${node}`;
        if (name.startsWith('image-') && canvas.dataset.imageNode != null) {
            const state = `${name}|${canvas.dataset.imagePath}|${canvas.dataset.imageVersion}`;
            if (this.imageStates.get(key) === state) return;
            // Diagnostics are bounded independently of the event ring.
            if (this.imageStates.size >= this.capacity && !this.imageStates.has(key)) {
                const oldest = this.imageStates.keys().next().value!;
                this.imageStates.delete(oldest); this.pendingImages.delete(oldest);
                for (const [id, nodes] of this.recordedImages) {
                    nodes.delete(oldest);
                    if (nodes.size === 0) this.recordedImages.delete(id);
                }
            }
            this.imageStates.set(key, state);
            if (name === 'image-waiting-decoded') this.pendingImages.set(key, {id, node, path: canvas.dataset.imagePath!});
            if (name === 'image-draw-ready') {
                this.pendingImages.delete(key);
                if (!this.recordedImages.has(id)) this.recordedImages.set(id, new Set());
                this.recordedImages.get(id)!.add(key);
            }
        }
        if (name === 'image-content-submit') this.recordedImages.delete(id);
        if (name === 'lease-released' || name === 'retirement') {
            this.recordedImages.delete(id);
            for (const key of this.imageStates.keys()) if (key.startsWith(id+'|')) {
                this.imageStates.delete(key); this.pendingImages.delete(key);
            }
        }
        this.record(name, {
            id: canvas.id || canvas.dataset.surfaceRetiredId, generation: canvas.dataset.surfaceGeneration,
            contentRevision: canvas.dataset.surfaceContentRevision ?? canvas.dataset.surfaceRetiredRevision,
            width: canvas.width, height: canvas.height,
            owners: Number(canvas.dataset.paxSurfaceOwners ?? 0),
            coverage: this.demand.get(id)?.coverage, owner: this.demand.get(id)?.owner,
            ...(name === 'retirement' ? {
                admission: !this.demand.has(id) ? 'not-candidate'
                    : this.demand.get(id)?.denial ?? 'admitted',
                submittedRevision: canvas.dataset.surfaceSubmittedRevision,
                submitted: canvas.dataset.surfaceContentRevision != null
                    && canvas.dataset.surfaceSubmittedRevision === canvas.dataset.surfaceContentRevision,
            } : {}),
            priority: canvas.dataset.replayPriority == null ? undefined : Number(canvas.dataset.replayPriority),
            ...(canvas.dataset.imageNode == null ? {} : {node, path: canvas.dataset.imagePath,
                version: Number(canvas.dataset.imageVersion), bytes: Number(canvas.dataset.imageBytes)}),
        });
    };

    constructor(private mount: HTMLElement, private capacity = 2048) {
        this.capacity = Math.max(1, Math.floor(capacity));
        mount.addEventListener('pax-surface-trace', this.listener);
        // Readable through developer tools without retaining DOM nodes in the log.
        (mount as any).paxSurfaceTrace = this.events;
        // An explicit snapshot avoids per-frame DOM serialization while making
        // transient gaps inspectable after native scrolling has settled.
        if (mount.ownerDocument) {
            const doc = mount.ownerDocument;
            const details = doc.createElement('details');
            details.setAttribute('data-pax-surface-inspector', '');
            const summary = doc.createElement('summary');
            const output = doc.createElement('pre');
            const scroller = doc.createElement('select');
            const duration = doc.createElement('select');
            scroller.setAttribute('aria-label','Probe scroller');
            duration.setAttribute('aria-label','Probe duration');
            for (const ms of [400,1000,200]) {
                const option=doc.createElement('option'); option.value=String(ms); option.textContent=ms+' ms'; duration.append(option);
            }
            summary.textContent = 'Surface trace snapshot';
            output.id = 'pax-surface-trace-snapshot';
            details.style.cssText = 'position:fixed;bottom:0;right:0;z-index:2147483647;background:#fff;color:#000;max-width:90vw;max-height:60vh;overflow:auto;font:11px monospace';
            details.append(summary, scroller, duration);
            for (const [name,direction] of [['Probe down',1],['Probe up',-1]] as const) {
                const button=doc.createElement('button'); button.textContent=name;
                button.addEventListener('click',()=>{
                    const leaf=doc.querySelector<HTMLElement>('.scroller-container[pax_id="'+scroller.value+'"]');
                    if (!leaf) return;
                    details.open=false;
                    this.runScrollProbe(leaf,Number(duration.value),direction);
                });
                details.append(button);
            }
            details.append(output);
            details.addEventListener('toggle', () => {
                if (details.open) {
                    const selected=scroller.value;
                    scroller.replaceChildren();
                    for (const leaf of doc.querySelectorAll<HTMLElement>('.scroller-container')) {
                        const r=leaf.getBoundingClientRect();
                        if (leaf.scrollHeight <= leaf.clientHeight + 1 || r.right<=0 || r.left>=window.innerWidth
                            || r.bottom<=0 || r.top>=window.innerHeight) continue;
                        const option=doc.createElement('option');
                        option.value=leaf.getAttribute('pax_id') ?? '';
                        option.textContent='Scroller '+option.value+' ('+Math.round(r.width)+'×'+Math.round(r.height)+')';
                        scroller.append(option);
                    }
                    if ([...scroller.options].some(o=>o.value===selected)) scroller.value=selected;
                    this.record('image-readiness-summary', {counts: {...this.imageCounts},
                        waitingDecoded: [...this.pendingImages.values()],
                        recordedNotSubmitted: [...this.recordedImages.entries()].map(([id, nodes]) => ({id, count:nodes.size}))});
                    this.record('presentation-completion-summary', {requests: {...this.completionRequests},
                        wait: {...this.completionWait}, pending: this.completionDeferredAt != null});
                    this.record('wheel-input-summary', {samples: [...this.wheelInputs]});
                    output.textContent = JSON.stringify(this.events);
                }
            });
            doc.body.append(details);
            this.inspector = details;
        }
    }

    // Opt-in reproducible input, not a native compositor/frame-capture proxy.
    // It exercises observed programmatic movement through the same scroll path;
    // missed callbacks remain visible as probe-frame intervals in the trace.
    private runScrollProbe(leaf: HTMLElement, duration: number, direction: number) {
        if (this.probeFrame != null) cancelAnimationFrame(this.probeFrame);
        const start=performance.now(), from=leaf.scrollTop;
        this.probeUntil = start + duration + 250;
        const to=Math.max(0,Math.min(leaf.scrollHeight-leaf.clientHeight,from+direction*leaf.clientHeight));
        let previous=start;
        this.record('probe-start',{scroller:leaf.getAttribute('pax_id'),duration,from,to});
        const step=()=>{
            // rAF's supplied timestamp can precede the button handler in the
            // same frame; sample the actual callback time for input and timing.
            const now=performance.now();
            const progress=Math.max(0,Math.min(1,(now-start)/duration));
            leaf.scrollTop=from+(to-from)*progress;
            this.record('probe-frame',{interval:now-previous,offset:leaf.scrollTop,progress});
            previous=now;
            if (progress<1) this.probeFrame=requestAnimationFrame(step);
            else { this.probeFrame=undefined; this.record('probe-end',{duration:now-start,offset:leaf.scrollTop}); }
        };
        this.probeFrame=requestAnimationFrame(step);
    }

    recordCompletionRequest(outcome: keyof SurfaceTrace['completionRequests']) {
        this.completionRequests[outcome]++;
        if (outcome === 'used' && this.completionDeferredAt == null) {
            this.completionDeferredAt = performance.now();
        }
    }

    recordPresentationOpportunity(blocked: boolean) {
        if (this.completionDeferredAt == null) return;
        const waitMs = Math.max(0, performance.now() - this.completionDeferredAt);
        this.completionDeferredAt = undefined;
        this.completionWait.opportunities++;
        this.completionWait.totalMs += waitMs;
        this.completionWait.maxMs = Math.max(this.completionWait.maxMs, waitMs);
        // This measures the next service opportunity, not completion of any
        // particular tile. The runtime may still reject unsettled scene state.
        this.record('presentation-completion-wait', {waitMs, blocked});
    }

    record(event: string, details: Record<string, unknown> = {}) {
        if (event.startsWith('image-') && event !== 'image-readiness-summary') {
            this.imageCounts[event] = (this.imageCounts[event] ?? 0) + 1;
            if (event === 'image-upload') this.imageCounts.uploadBytes = (this.imageCounts.uploadBytes ?? 0) + Number(details.bytes ?? 0);
        }
        if (this.events.length >= this.capacity) this.events.splice(0, this.events.length - this.capacity + 1);
        this.events.push({at: performance.now(), event, ...details});
    }

    recordPhase(phase: string, duration: number) {
        if (performance.now() <= this.probeUntil) this.record('probe-phase',{phase,duration});
        if (performance.now() <= this.scrollUntil) this.record('scroll-phase',{phase,duration});
        if (duration > 20) this.record('slow-frame-phase',{phase,duration});
    }

    recordNativeMessageBatch(types: NativeMessageTiming[]) {
        const duration = types.reduce((total, t) => total+t.totalMs, 0);
        const now = performance.now();
        if (now <= this.probeUntil || now <= this.scrollUntil || duration > 20) {
            this.record('native-message-batch', {types, duration});
        }
    }

    recordWheelInput(sample: WheelInputSample) {
        // Keep a small independent tail so noisy tile traces cannot erase the
        // input evidence before the inspector is opened. No event/DOM retention.
        if (this.wheelInputs.length >= 128) this.wheelInputs.shift();
        this.wheelInputs.push(sample);
        this.scrollUntil = performance.now() + 250;
        this.record('wheel-input', sample);
    }

    observeScroll(scroller: number, x: number, y: number, previousOffset?: [number, number]) {
        // Observe input; never synthesize movement. Timing begins when the main
        // thread samples it, not when the compositor first displayed the scroll.
        this.scrollUntil = performance.now() + 250;
        this.record('scroll-sample', {scroller, x, y});
        const known = this.scrollDirections.has(scroller);
        const old = this.scrollDirections.get(scroller)
            ?? (previousOffset ? {offset: previousOffset, direction: [0,0]} : undefined);
        const offset = [x,y], direction = old?.direction.slice() ?? [0,0];
        const reversedAxes: number[] = [];
        if (old) for (let axis=0; axis<2; axis++) {
            const delta=offset[axis]-old.offset[axis];
            if (Math.abs(delta)<=0.1) continue;
            const next=Math.sign(delta);
            if (direction[axis] !== 0 && direction[axis] !== next) reversedAxes.push(axis);
            direction[axis]=next;
        }
        if (reversedAxes.length) this.record('scroll-reversal', {scroller, from: old!.offset, to: offset, direction, reversedAxes});
        if (this.scrollDirections.size >= this.capacity && !known) this.scrollDirections.delete(this.scrollDirections.keys().next().value!);
        this.scrollDirections.set(scroller, {offset, direction});
    }

    observeDemand(candidates: SurfaceCandidate[], denied: SurfaceDenial[] = []) {
        const denials = new Map(denied.map(d => [d.id, d.reason]));
        const unready = candidates.filter(c => c.rank.coverage === 0 && !c.rank.ready)
            .map(c => c.surface.id).sort();
        const signature = unready.join('|');
        if (signature !== this.lastUnready) {
            this.record('visible-unready', {ids: unready});
            this.lastUnready = signature;
        }
        const next: typeof this.demand = new Map();
        for (const candidate of candidates) {
            const s = candidate.surface;
            const signature = `${s.hostSignature}|${s.surfaceSignature}|${s.transformSignature}`;
            const previous = this.demand.get(s.id);
            const state = {signature, firstVisibleAt: previous?.signature === signature ? previous.firstVisibleAt : undefined,
                coverage: candidate.rank.coverage, owner: candidate.branch, denial: denials.get(s.id)};
            if (candidate.rank.coverage === 0 && state.firstVisibleAt == null) {
                state.firstVisibleAt = Date.now();
                // Demand is observed before admission, including visible tiles
                // denied by overload or waiting for an old lease to retire.
                this.record('first-visible-demand', {id: s.id, signature, firstVisibleAt: state.firstVisibleAt});
            }
            next.set(s.id, state);
        }
        this.demand = next;
    }

    firstVisibleAt(id: string) { return this.demand.get(id)?.firstVisibleAt; }

    dispose() {
        if (this.probeFrame != null) cancelAnimationFrame(this.probeFrame);
        this.mount.removeEventListener('pax-surface-trace', this.listener);
        this.inspector?.remove();
        delete (this.mount as any).paxSurfaceTrace;
        this.events.length = 0;
        this.wheelInputs.length = 0;
        this.demand.clear();
        this.scrollDirections.clear();
        this.imageStates.clear(); this.pendingImages.clear(); this.recordedImages.clear(); this.imageCounts = {};
        this.completionRequests = {queued: 0, coalesced: 0, used: 0, disposed: 0};
        this.completionDeferredAt = undefined;
        this.completionWait = {opportunities: 0, totalMs: 0, maxMs: 0};
    }
}
