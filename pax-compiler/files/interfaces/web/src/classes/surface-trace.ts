import type {SurfaceCandidate, SurfaceDenial} from './surface-scheduler';
import type {NativeMessageTiming} from './native-message-profile';
import type {WheelInputSample} from '../events/listeners';
import {TraceRing} from './trace-ring';
import {SurfaceCapture, observeSurfaceReadiness} from './surface-capture';
import type {CaptureRect} from './surface-capture';
import {sharedWorkBudget} from '../utils/shared-work-budget';

export type SurfaceTraceEvent = {at: number; event: string; id?: string; generation?: string;
    contentRevision?: string; [key: string]: unknown};

// Opt-in, bounded, and attached to the app mount so multiple chassis never
// mingle their traces. No network reporting or per-frame console output.
export class SurfaceTrace {
    private events: TraceRing<SurfaceTraceEvent>;
    private samples: TraceRing<SurfaceTraceEvent>;
    private frame?: number;
    private phases = new Map<string, {count: number; totalMs: number; maxMs: number}>();
    private renderWork: Record<string, number> = {};
    private layerReconciliation: unknown[] = [];
    private layerDraws: unknown[] = [];
    private renderBlockers = new Set<string>();
    private admission = '';
    private demand = new Map<string, {signature: string; firstVisibleAt?: number; coverage: number; owner: number; denial?: string}>();
    private scrollDirections = new Map<number, {offset: number[]; direction: number[]}>();
    private inspector?: HTMLDetailsElement;
    private lastUnready = '';
    private probeFrame?: number;
    private profileFrame?: number;
    private workProfileTimer?: ReturnType<typeof setTimeout>;
    private profileStart = 0;
    private profileUntil = -Infinity;
    private profilePhases = new Map<string, {count: number; total: number; max: number; samples: number[]}>();
    private profileOutput?: HTMLPreElement;
    private lastProfile?: object;
    private probeUntil = -Infinity;
    private scrollUntil = -Infinity;
    private wheelInputs = new TraceRing<WheelInputSample>(128);
    private imageCounts: Record<string, number> = {};
    private completionRequests = {queued: 0, coalesced: 0, used: 0, disposed: 0};
    private imageStates = new Map<string, string>();
    private pendingImages = new Map<string, {node: number; path: string; id: string}>();
    private recordedImages = new Map<string, Set<string>>();
    private capture?: SurfaceCapture;
    private captureFrame?: number;
    private captureTimer?: ReturnType<typeof setTimeout>;
    private stopBudgetObservation?: () => void;
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

    constructor(private mount: HTMLElement, private capacity = 2048, inspectNode?: (id: number) => object | null | undefined,
        workProfile?: {begin: () => boolean; take: () => object | null}) {
        this.capacity = Math.max(1, Math.floor(capacity));
        this.events = new TraceRing(this.capacity);
        this.samples = new TraceRing(Math.min(this.capacity, 512));
        mount.addEventListener('pax-surface-trace', this.listener);
        // Readable through developer tools without retaining DOM nodes in the log.
        Object.defineProperty(mount, 'paxSurfaceTrace', {configurable: true, get: () => this.snapshot()});
        Object.defineProperty(mount, 'paxSurfaceCapture', {configurable: true, get: () => this.capture?.snapshot()});
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
            const profile = doc.createElement('button');
            profile.textContent = 'Profile 2 seconds';
            const profileOutput = doc.createElement('pre');
            profileOutput.id = 'pax-surface-profile';
            this.profileOutput = profileOutput;
            profile.addEventListener('click', () => { details.open = false; this.runProfile(2000); });
            details.append(profile, profileOutput);
            const nodeId = doc.createElement('input');
            nodeId.type = 'number'; nodeId.min = '0'; nodeId.max = '4294967295'; nodeId.step = '1';
            nodeId.placeholder = 'Engine node ID'; nodeId.setAttribute('aria-label', 'Engine node ID');
            const inspect = doc.createElement('button');
            inspect.textContent = 'Inspect node';
            const nodeOutput = doc.createElement('pre');
            nodeOutput.id = 'pax-node-inspection';
            inspect.addEventListener('click', () => {
                const id = nodeId.valueAsNumber;
                if (!Number.isInteger(id) || id < 0 || id > 4294967295) {
                    nodeOutput.textContent = 'Enter an unsigned 32-bit engine node ID.';
                    return;
                }
                const node = inspectNode?.(id);
                nodeOutput.textContent = JSON.stringify(node === undefined ? {status: 'Node inspection unavailable in this cartridge'}
                    : node === null ? {engine_id: id, status: 'Node is not in the runtime cache'} : node);
                this.record('node-inspection', {engine_id: id, node});
            });
            details.append(nodeId, inspect, nodeOutput);
            if (workProfile) {
                const capture = doc.createElement('button');
                capture.textContent = 'Profile CPU work 2 seconds';
                const workOutput = doc.createElement('pre');
                workOutput.id = 'pax-cpu-work-profile';
                capture.addEventListener('click', () => {
                    if (!workProfile.begin()) {
                        workOutput.textContent = 'CPU work profiling unavailable in this cartridge.';
                        return;
                    }
                    capture.disabled = true;
                    details.open = false;
                    this.workProfileTimer = setTimeout(() => {
                        this.workProfileTimer = undefined;
                        const report = workProfile.take();
                        workOutput.textContent = JSON.stringify(report);
                        this.record('cpu-work-profile', {report});
                        capture.disabled = false;
                        details.open = true;
                    }, 2000);
                });
                details.append(capture, workOutput);
            }
            const captureStatus = doc.createElement('span');
            const capture = doc.createElement('button');
            capture.textContent = 'Capture 20 seconds';
            capture.addEventListener('click', () => {
                this.startCapture(() => { captureStatus.textContent = ' Capture stopped; save JSON.'; });
                captureStatus.textContent = ' Recording (20 seconds maximum).';
                details.open = false;
            });
            const mark = doc.createElement('button');
            mark.textContent = 'Mark visible blank';
            mark.addEventListener('click', () => this.record('user-observed-blank'));
            const save = doc.createElement('button');
            save.textContent = 'Stop / save capture';
            save.addEventListener('click', () => {
                this.stopCapture();
                const report = this.capture?.snapshot();
                if (!report) return;
                const url = URL.createObjectURL(new Blob([JSON.stringify(report)], {type:'application/json'}));
                const link = doc.createElement('a');
                link.href = url; link.download = `pax-surfaces-${Date.now()}.json`; link.click();
                setTimeout(() => URL.revokeObjectURL(url), 1000);
                captureStatus.textContent = ' Capture saved locally.';
            });
            details.append(capture, mark, save, captureStatus);
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
                    this.record('presentation-completion-summary', {requests: {...this.completionRequests}});
                    this.record('wheel-input-summary', {samples: this.wheelInputs.snapshot()});
                    this.record('trace-overflow', {events: this.events.overwritten, samples: this.samples.overwritten});
                    output.textContent = JSON.stringify(this.snapshot());
                    profileOutput.textContent = JSON.stringify(this.lastProfile ?? {status:'No profile recorded'});
                }
            });
            doc.body.append(details);
            this.inspector = details;
        }
    }

    private startCapture(onStop: () => void) {
        this.stopCapture();
        const win = this.mount.ownerDocument.defaultView!;
        const now = performance.now();
        this.capture = new SurfaceCapture(now, {timeOrigin:performance.timeOrigin,
            userAgent:win.navigator.userAgent, url:win.location.href,
            viewport:[win.innerWidth,win.innerHeight], dpr:win.devicePixelRatio,
            hidden:this.mount.ownerDocument.hidden, focused:this.mount.ownerDocument.hasFocus()});
        this.stopBudgetObservation = sharedWorkBudget.observe(({at,event,...detail}) =>
            this.capture?.event(at,event,detail));
        const frame = () => {
            this.capture?.frame(performance.now(), this.mount.ownerDocument.hidden);
            if (this.capture?.active) this.captureFrame = requestAnimationFrame(frame);
        };
        this.captureFrame = requestAnimationFrame(frame);
        this.captureTimer = setTimeout(() => { this.stopCapture(); onStop(); }, 20000);
        this.record('capture-start');
    }

    private stopCapture() {
        this.stopBudgetObservation?.(); this.stopBudgetObservation = undefined;
        if (this.capture?.active) {
            this.flushFrame();
            this.capture.stop(performance.now());
        }
        if (this.captureFrame != null) cancelAnimationFrame(this.captureFrame);
        if (this.captureTimer != null) clearTimeout(this.captureTimer);
        this.captureFrame = undefined; this.captureTimer = undefined;
    }

    get capturing() { return this.capture?.active ?? false; }

    captureCoverage(candidates: SurfaceCandidate[], denied: SurfaceDenial[],
        canvases: Map<string, HTMLCanvasElement>, clips: Map<number, CaptureRect>, rootClip: CaptureRect) {
        if (!this.capturing) return;
        const start = performance.now();
        this.capture!.observe(start, observeSurfaceReadiness(candidates,denied,canvases,clips,rootClip));
        const elapsed = performance.now()-start;
        this.capture!.phase('diagnostic.coverage', elapsed, 1, elapsed);
    }

    // Keep a bounded aggregate separately from the noisy event ring. This
    // observes the chosen workload without injecting scroll or changing it.
    private runProfile(duration: number) {
        if (this.profileFrame != null) cancelAnimationFrame(this.profileFrame);
        this.profileStart = performance.now();
        this.profileUntil = this.profileStart + duration;
        this.profilePhases.clear();
        let previous = this.profileStart;
        const step = () => {
            const now = performance.now();
            this.addProfileSample('raf-interval', now - previous);
            previous = now;
            if (now < this.profileUntil) this.profileFrame = requestAnimationFrame(step);
            else {
                this.profileFrame = undefined;
                this.profileUntil = -Infinity;
                this.lastProfile = {duration: now - this.profileStart, phases: this.profileSnapshot()};
                if (this.profileOutput) this.profileOutput.textContent = JSON.stringify(this.lastProfile);
            }
        };
        this.profileFrame = requestAnimationFrame(step);
    }

    private addProfileSample(phase: string, duration: number) {
        if (!Number.isFinite(duration) || duration < 0) return;
        if (!this.profilePhases.has(phase)) {
            if (this.profilePhases.size >= 64) return;
            this.profilePhases.set(phase, {count:0,total:0,max:0,samples:[]});
        }
        const stat = this.profilePhases.get(phase)!;
        stat.count++; stat.total += duration; stat.max = Math.max(stat.max, duration);
        if (stat.samples.length < 2048) stat.samples.push(duration);
    }

    private profileSnapshot() {
        return Object.fromEntries([...this.profilePhases].map(([phase, stat]) => {
            const samples = [...stat.samples].sort((a,b)=>a-b);
            return [phase, {count:stat.count, mean:stat.total/stat.count, max:stat.max,
                p95:samples[Math.ceil(samples.length*0.95)-1], percentileSamples:samples.length}];
        }));
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
    }

    record(event: string, details: Record<string, unknown> = {}) {
        if (event.startsWith('image-') && event !== 'image-readiness-summary') {
            this.imageCounts[event] = (this.imageCounts[event] ?? 0) + 1;
            if (event === 'image-upload') this.imageCounts.uploadBytes = (this.imageCounts.uploadBytes ?? 0) + Number(details.bytes ?? 0);
        }
        const ring = ['scroll-sample', 'probe-frame', 'wheel-input', 'frame-summary', 'native-message-batch'].includes(event)
            || (event === 'occlusion-work' && !(details.delta as any)?.rebuilds && !(details.delta as any)?.structural_reconciliations)
            ? this.samples : this.events;
        const at = performance.now();
        ring.push({at, event, ...details});
        this.capture?.event(at,event,details);
    }

    recordPhase(phase: string, duration: number, count = 1, maximum = duration) {
        const value = this.phases.get(phase) ?? {count: 0, totalMs: 0, maxMs: 0};
        value.count += count; value.totalMs += duration; value.maxMs = Math.max(value.maxMs, maximum);
        this.phases.set(phase, value);
        this.capture?.phase(phase,duration,count,maximum);
        if (performance.now() <= this.profileUntil) this.addProfileSample(phase, duration);
    }

    beginFrame(epoch: number) {
        if (epoch === this.frame) return;
        this.flushFrame();
        this.frame = epoch;
    }

    private flushFrame() {
        if (!this.phases.size && !Object.keys(this.renderWork).length) return;
        this.record('frame-summary', {epoch: this.frame, phases: Object.fromEntries(this.phases),
            renderWork: this.renderWork, blockers: [...this.renderBlockers],
            layerReconciliation: this.layerReconciliation, layerDraws: this.layerDraws});
        this.phases.clear(); this.renderWork = {}; this.renderBlockers.clear();
        this.layerReconciliation = []; this.layerDraws = [];
    }

    snapshot() {
        this.flushFrame();
        return this.events.snapshot().concat(this.samples.snapshot()).sort((a,b) => a.at-b.at);
    }

    recordRenderWork(report: Record<string, any>, hostElapsedMs: number) {
        this.capture?.event(performance.now(),'render-service',{hostMs:hostElapsedMs,
            pending:report.pending, visiblePending:report.visible_pending,
            blocked:report.blocked, blocker:report.blocker, exhausted:report.allowance_exhausted,
            units:report.units, elapsedMs:report.elapsed_ms, maxUnitMs:report.max_unit_ms,
            submitted:report.preparation?.submitted, initialized:report.preparation?.initializations,
            setupMs:report.setup_ms, ordinaryMs:report.ordinary_ms, preparationMs:report.preparation?.elapsed_ms,
            preparationStages:report.preparation?.diagnostics?.stages,
            host:report.host});
        this.layerReconciliation.push(...(report.layer_reconciliation ?? []).slice(0, 64 - this.layerReconciliation.length));
        this.layerDraws.push(...(report.layer_draws ?? []).slice(0, 64 - this.layerDraws.length));
        const add = (key: string, value: number) => this.renderWork[key] = (this.renderWork[key] ?? 0) + value;
        add('services', 1); add('hostMs', hostElapsedMs);
        for (const key of ['elapsed_ms', 'setup_ms', 'ordinary_ms', 'units', 'ordinary_layers',
            'ordinary_drawn_layers', 'ordinary_flush_only_layers', 'ordinary_drawn_ms',
            'ordinary_flush_only_ms', 'atomic_fallbacks']) {
            add(key, report[key] ?? 0);
        }
        for (const key of ['initializations', 'recorded', 'submitted', 'urgent_units', 'elapsed_ms']) {
            add('preparation_' + key, report.preparation?.[key] ?? 0);
        }
        const diagnostics = report.preparation?.diagnostics;
        for (const [name, sample] of Object.entries(diagnostics?.stages ?? {}) as [string, any][]) {
            // Fixed stage names from the runtime; aggregate rather than retaining
            // per-primitive records or creating browser performance entries.
            this.recordPhase('preparation.' + name, sample.total_ms, sample.count, sample.max_ms);
        }
        for (const key of ['targeted_surfaces', 'additional_surfaces', 'image_uploads', 'image_upload_bytes']) {
            if (diagnostics) add('preparation_' + key, diagnostics[key] ?? 0);
        }
        add('exhausted', Number(!!report.allowance_exhausted));
        this.renderWork.max_unit_ms = Math.max(this.renderWork.max_unit_ms ?? 0, report.max_unit_ms ?? 0);
        if (report.blocker) this.renderBlockers.add(report.blocker);
    }

    recordAdmission(admitted: SurfaceCandidate[], denied: SurfaceDenial[], detail: Record<string, unknown>) {
        // Distance changes continuously. Record membership/readiness/class changes,
        // not full JSON on every continuation merely to compare it to old JSON.
        const signature = admitted.map(c => `${c.surface.id}:${c.rank.coverage}:${Number(c.rank.ready)}`)
            .concat(denied.map(d => `${d.id}:${d.reason}`)).join('|');
        if (signature === this.admission) return;
        this.admission = signature;
        this.record('admission', {...detail, admitted: admitted.map(c => ({id: c.surface.id,
            rank: {...c.rank}, owner: c.branch, rect: [c.surface.left, c.surface.top, c.surface.width, c.surface.height]})), denied});
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
        if (this.workProfileTimer != null) clearTimeout(this.workProfileTimer);
        this.workProfileTimer = undefined;
        this.stopCapture(); this.capture = undefined;
        if (this.probeFrame != null) cancelAnimationFrame(this.probeFrame);
        if (this.profileFrame != null) cancelAnimationFrame(this.profileFrame);
        this.profileUntil = -Infinity;
        this.profilePhases.clear(); this.lastProfile = undefined; this.profileOutput = undefined;
        this.mount.removeEventListener('pax-surface-trace', this.listener);
        this.inspector?.remove();
        delete (this.mount as any).paxSurfaceTrace;
        delete (this.mount as any).paxSurfaceCapture;
        this.events.clear(); this.samples.clear(); this.phases.clear();
        this.renderWork = {}; this.renderBlockers.clear(); this.admission = '';
        this.layerReconciliation = []; this.layerDraws = [];
        this.wheelInputs.clear();
        this.demand.clear();
        this.scrollDirections.clear();
        this.imageStates.clear(); this.pendingImages.clear(); this.recordedImages.clear(); this.imageCounts = {};
        this.completionRequests = {queued: 0, coalesced: 0, used: 0, disposed: 0};
    }
}
