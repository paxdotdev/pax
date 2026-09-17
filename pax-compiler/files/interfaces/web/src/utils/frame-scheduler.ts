/** Keeps a pending animation frame across suspension without replacing application state. */
export class FrameScheduler {
    private handle: number | null = null;
    private pending: (() => void) | null = null;

    constructor(
        public suspended: boolean,
        private request: (callback: () => void) => number = callback => requestAnimationFrame(callback),
        private cancel: (handle: number) => void = handle => cancelAnimationFrame(handle),
    ) {}

    schedule(callback: () => void) {
        this.pending = callback;
        if (this.suspended || this.handle !== null) return;
        this.handle = this.request(() => {
            this.handle = null;
            if (this.suspended) return;
            const next = this.pending;
            this.pending = null;
            next?.();
        });
    }

    setSuspended(suspended: boolean) {
        if (this.suspended === suspended) return;
        this.suspended = suspended;
        if (suspended) {
            if (this.handle !== null) this.cancel(this.handle);
            this.handle = null;
        } else if (this.pending) {
            this.schedule(this.pending);
        }
    }

    clear() {
        if (this.handle !== null) this.cancel(this.handle);
        this.handle = null;
        this.pending = null;
    }
}
