// Startup work runs in regular tasks. A resolved Promise or queueMicrotask is
// not a browser yield and must not be used as the fallback here.
export function enqueueStartupTask(job: () => void) {
    const scheduler = (globalThis as any).scheduler;
    if (scheduler?.yield) {
        void scheduler.yield().then(job);
    } else {
        const channel = new MessageChannel();
        channel.port1.onmessage = () => {
            channel.port1.close(); channel.port2.close(); job();
        };
        channel.port2.postMessage(undefined);
    }
}

export class StartupScheduler {
    private queued = false;
    private disposed = false;
    constructor(private service: () => boolean, private allowed: () => boolean,
                private enqueue: (job: () => void) => void = enqueueStartupTask) {}
    request() {
        if (this.disposed || this.queued) return;
        this.queued = true;
        this.enqueue(() => {
            this.queued = false;
            if (this.disposed || !this.allowed()) return;
            if (this.service()) this.request();
        });
    }
    dispose() { this.disposed = true; }
}
