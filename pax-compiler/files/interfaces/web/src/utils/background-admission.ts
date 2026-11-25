import type {PaxChassisWeb} from '../types/pax-chassis-web';
import {sharedWorkBudget} from './shared-work-budget';

const framePeriod = 1000 / 120;
const mountAllowance = 2;
const minimumSlack = 0.25;

function inputPending() {
    const nav = navigator as Navigator & {scheduling?: {isInputPending(options: {includeContinuous: boolean}): boolean}};
    return nav.scheduling?.isInputPending({includeContinuous: true}) ?? false;
}

/** A grant prepares detached work. The next foreground tick owns settlement
 * and presentation. Each posted continuation rechecks the original submission. */
export class BackgroundAdmission {
    private phase: 'prepared' | 'draining' | 'settling' | 'finished' = 'prepared';
    private input = false;
    private spatial = false;
    private previouslyServed = -1;
    private ticks = 0;
    private lastGrantTick = 0;
    private foregroundLate = false;
    private heldPreparationMs = 0;
    private deadline = 0;
    private epoch = 0;
    private interrupt = 0;
    private readonly listeners = new AbortController();
    private gestureHost?: number;
    private gestureAt = -Infinity;

    constructor(private chassis: PaxChassisWeb, root: Element,
        indexed: boolean, private runContinuation: (job: () => void) => void,
        readonly shareDemand = false, private prepareWhileWaiting = false) {
        chassis.configure_admission_resumption(1, false);
        chassis.configure_visible_groups(true);
        if (indexed) chassis.enable_indexed_mounts(performance.now());
        if (this.shareDemand) chassis.share_viewport_demand();
        const options = {capture: true, passive: true, signal: this.listeners.signal};
        const intent = (event: Event, x: number, y: number) => {
            const horizontal = Math.abs(x) > Math.abs(y);
            const step = horizontal ? x : y;
            if (!step) return;
            this.gestureHost = undefined;
            for (const el of event.composedPath()) {
                if (!(el instanceof HTMLElement) || !el.classList.contains('scroller-container')) continue;
                const at = horizontal ? el.scrollLeft : el.scrollTop;
                const end = horizontal ? el.scrollWidth - el.clientWidth : el.scrollHeight - el.clientHeight;
                if (step < 0 ? at <= 0.5 : at >= end - 0.5) continue;
                const id = el.closest('[pax_id]')?.getAttribute('pax_id');
                if (id != null) this.gestureHost = Number(id);
                break;
            }
            this.gestureAt = performance.now();
            if (indexed) chassis.mount_scroll_intent(this.gestureHost, x, y);
        };
        root.addEventListener('wheel', e => {const w = e as WheelEvent; intent(e, w.deltaX, w.deltaY);}, options);
        let touch: [number, number] | undefined;
        root.addEventListener('touchstart', e => {
            const t = (e as TouchEvent).touches[0];
            touch = t ? [t.clientX, t.clientY] : undefined;
        }, options);
        root.addEventListener('touchmove', e => {
            const t = (e as TouchEvent).touches[0];
            if (t && touch) { intent(e, touch[0] - t.clientX, touch[1] - t.clientY); touch = [t.clientX,t.clientY]; }
        }, options);
        for (const type of ['wheel','scroll','pointerdown','pointermove','pointerup','touchstart','touchmove','keydown','input','resize','visibilitychange']) {
            window.addEventListener(type, () => this.interrupt++, options);
        }
    }

    begin(): boolean {
        if (!this.chassis.begin_background_mounting()) return false;
        this.phase = 'draining';
        return true;
    }
    enableInput() { this.input = true; }
    observedScroll(id: number, page: boolean) {
        if (!this.input || this.spatial || this.phase !== 'draining') return;
        if (!page && (id !== this.gestureHost || performance.now()-this.gestureAt > 500)) return;
        this.spatial = true;
        this.chassis.prioritize_background_owners();
    }
    frame(epoch: number, timestamp: number) {
        this.epoch = epoch;
        this.deadline = timestamp + framePeriod;
    }
    private slack() { return Math.min(sharedWorkBudget.remaining, this.deadline - performance.now()); }
    private grant(cap: number) {
        const allowance = Math.min(cap, this.slack());
        if (allowance <= minimumSlack) return false;
        this.previouslyServed = this.epoch;
        this.lastGrantTick = this.ticks;
        if (!sharedWorkBudget.run(allowance, b => this.chassis.service_background_mounts(b))) this.phase = 'settling';
        return this.phase === 'draining';
    }
    beforeTick() {
        this.ticks++;
        if (this.phase !== 'draining' || !this.input || this.previouslyServed === this.epoch || inputPending()) return;
        if (!this.spatial && this.ticks - this.lastGrantTick < 8) return;
        if (!this.chassis.has_pending_properties() || this.chassis.admission_can_continue()) this.grant(this.spatial ? mountAllowance : 0.5);
    }
    afterTick(duration: number) {
        this.foregroundLate = duration >= framePeriod;
        this.heldPreparationMs = 0;
        if (this.phase === 'draining' && !this.chassis.admission_can_continue() &&
            this.chassis.background_counts().slice(3).every(count => count === 0) && !this.chassis.service_background_mounts(0)) {
            this.phase = 'settling';
        }
    }
    // The host has just checked the sealed revision's fence. The displayed
    // scene stays frozen while detached attachments use spare time on this rAF.
    // Charge actual time until the next tick settles the resulting effects.
    waitingForSubmission() {
        if (!this.prepareWhileWaiting || this.phase !== 'draining' || !this.input || !this.spatial ||
            document.hidden || this.previouslyServed === this.epoch || inputPending() ||
            !this.chassis.admission_can_continue()) return;
        const cap = Math.min(1, mountAllowance - this.heldPreparationMs, this.slack());
        if (cap <= minimumSlack) return;
        const start = performance.now();
        this.grant(cap);
        this.heldPreparationMs += performance.now() - start;
    }
    afterSubmission(fence: bigint | undefined, currentFence: () => bigint | undefined) {
        if (this.phase !== 'draining' || !this.input || document.hidden || this.previouslyServed === this.epoch ||
            fence === undefined || (!this.spatial && this.foregroundLate) || inputPending() || this.chassis.web_submission_complete(fence)) return;
        const epoch = this.epoch, tick = this.ticks, interrupt = this.interrupt;
        if (!this.grant(mountAllowance)) return;
        const next = () => {
            if (this.phase !== 'draining' || this.epoch !== epoch || this.ticks !== tick || this.interrupt !== interrupt ||
                document.hidden || currentFence() !== fence || inputPending() ||
                this.chassis.web_submission_complete(fence) || !this.chassis.admission_can_continue()) return;
            this.runContinuation(() => this.grant(0.75));
        };
        const tasks = (globalThis as typeof globalThis & {scheduler?: {postTask(f: () => void, options: {priority: string}): Promise<unknown>}}).scheduler;
        if (tasks) void tasks.postTask(next, {priority: 'background'});
        else setTimeout(next, 0);
    }
    finishIf(ready: () => boolean) {
        if (this.phase !== 'settling' || !ready()) return false;
        this.phase = 'finished';
        if (!this.shareDemand) this.listeners.abort();
        return true;
    }
    dispose() { this.phase = 'finished'; this.listeners.abort(); }
}
