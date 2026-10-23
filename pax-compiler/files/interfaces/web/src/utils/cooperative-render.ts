export type RenderWorkResult = {
    pending: boolean;
    blocked?: boolean;
    allowance_exhausted?: boolean;
};

// Unlike queueMicrotask, both paths return control to the host task scheduler.
// A host boundary permits input; it does not promise that the browser paints.
let channel: MessageChannel | undefined;
const tasks: (() => void)[] = [];
export function enqueueRenderTask(job: () => void) {
    const scheduler = (globalThis as any).scheduler;
    if (typeof scheduler?.yield === 'function') {
        scheduler.yield().then(job);
        return;
    }
    channel ??= new MessageChannel();
    channel.port1.onmessage = () => tasks.shift()?.();
    tasks.push(job);
    channel.port2.postMessage(null);
}

/** One coalescing pump for ordinary drawing and completion-driven preparation.
 * No scene/node/lease selection crosses a host boundary. Invalidating cancels
 * intent, not physical browser tasks; an already queued task may service a newer
 * frame only after beginFrame explicitly establishes fresh work.
 */
export class CooperativeRenderScheduler {
    private queued = false;
    private running = false;
    private disposed = false;
    private armed = false;
    private wanted = false;

    constructor(private service: (fromFrame: boolean) => RenderWorkResult, private allowed: () => boolean,
                private enqueue: (job: () => void) => void = enqueueRenderTask,
                private observeHostWait?: (milliseconds: number) => void) {}

    beginFrame() {
        this.invalidate();
        this.armed = true;
        this.wanted = true;
        this.run(true);
    }

    request(): 'disposed' | 'coalesced' | 'used' | 'queued' {
        if (this.disposed) return 'disposed';
        if (!this.armed) return 'used';
        this.wanted = true;
        if (this.queued || this.running) return 'coalesced';
        this.queue();
        return 'queued';
    }

    private queue() {
        if (this.queued || this.disposed || !this.wanted) return;
        this.queued = true;
        const queuedAt = this.observeHostWait ? performance.now() : undefined;
        this.enqueue(() => {
            this.queued = false;
            this.run(false, queuedAt);
        });
    }

    private run(fromFrame = false, queuedAt?: number) {
        if (this.disposed || this.running || !this.wanted || !this.allowed()) return;
        if (queuedAt !== undefined) this.observeHostWait?.(performance.now() - queuedAt);
        this.running = true;
        this.wanted = false;
        let result: RenderWorkResult;
        try {
            result = this.service(fromFrame);
        } finally {
            this.running = false;
        }
        // Blocked/empty/exhausted work never schedules itself indefinitely.
        // A fresh completion or normal frame can wake it again.
        this.wanted = this.wanted || (result.pending && !result.blocked && !result.allowance_exhausted);
        this.queue();
    }

    invalidate() { this.wanted = false; this.armed = false; }
    dispose() { this.invalidate(); this.disposed = true; }
}
