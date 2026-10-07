// Coalesce asynchronous surface completions without creating another lifecycle
// frame or a self-sustaining microtask loop. One extra pass per ordinary frame.
export class SurfacePresentationScheduler {
    private used = false;
    private queued = false;
    private disposed = false;

    constructor(private run: () => void, private allowed: () => boolean,
                private enqueue: (job: () => void) => void = job => queueMicrotask(job)) {}

    request() {
        if (this.disposed || this.queued || this.used) return;
        this.queued = true;
        this.enqueue(() => { this.queued = false; this.present(); });
    }

    present() {
        if (this.disposed || this.used || !this.allowed()) return;
        this.used = true;
        this.run();
    }

    afterFrame() { this.used = false; }
    dispose() { this.disposed = true; }
}
