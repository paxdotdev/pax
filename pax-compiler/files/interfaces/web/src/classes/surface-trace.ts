import type {SurfaceCandidate} from './surface-scheduler';

export type SurfaceTraceEvent = {at: number; event: string; id?: string; generation?: string;
    contentRevision?: string; [key: string]: unknown};

// Opt-in, bounded, and attached to the app mount so multiple chassis never
// mingle their traces. No network reporting or per-frame console output.
export class SurfaceTrace {
    private events: SurfaceTraceEvent[] = [];
    private demand = new Map<string, {signature: string; firstVisibleAt?: number}>();
    private inspector?: HTMLDetailsElement;
    private lastUnready = '';
    private probeFrame?: number;
    private probeUntil = -Infinity;
    private scrollUntil = -Infinity;
    private imageCounts: Record<string, number> = {};
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

    observeScroll(scroller: number, x: number, y: number) {
        // Observe input; never synthesize movement. Timing begins when the main
        // thread samples it, not when the compositor first displayed the scroll.
        this.scrollUntil = performance.now() + 250;
        this.record('scroll-sample', {scroller, x, y});
    }

    observeDemand(candidates: SurfaceCandidate[]) {
        const unready = candidates.filter(c => c.rank.coverage === 0 && !c.rank.ready)
            .map(c => c.surface.id).sort();
        const signature = unready.join('|');
        if (signature !== this.lastUnready) {
            this.record('visible-unready', {ids: unready});
            this.lastUnready = signature;
        }
        const next = new Map<string, {signature: string; firstVisibleAt?: number}>();
        for (const candidate of candidates) {
            const s = candidate.surface;
            const signature = `${s.hostSignature}|${s.surfaceSignature}|${s.transformSignature}`;
            const previous = this.demand.get(s.id);
            const state = previous?.signature === signature ? previous : {signature, firstVisibleAt: undefined};
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
        this.demand.clear();
        this.imageStates.clear(); this.pendingImages.clear(); this.recordedImages.clear(); this.imageCounts = {};
    }
}
