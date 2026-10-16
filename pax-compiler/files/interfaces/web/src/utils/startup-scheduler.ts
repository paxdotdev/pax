// Startup work runs in regular tasks. A resolved Promise or queueMicrotask is
// not a browser yield and must not be used here.
export function enqueueStartupTask(job: () => void) {
    // Chaining scheduler.yield() continuations can outrank ordinary tasks for
    // the entire mount. Font-fetch continuations and resize events need turns too.
    const channel = new MessageChannel();
    channel.port1.onmessage = () => {
        channel.port1.close(); channel.port2.close(); job();
    };
    channel.port2.postMessage(undefined);
}

/** One FIFO of native patches, sharing the host slice's deadline with the engine.
 * Drain a batch before asking the engine for more: create/update/delete order is
 * preserved, and a slow browser cannot accumulate an unbounded patch backlog.
 */
export class StartupWork {
    private messages: any[] = [];
    private cursor = 0;
    constructor(private prepare: () => boolean,
                private serviceEngine: (budget: number) => any[],
                private apply: (message: any) => void,
                private now: () => number = () => performance.now()) {}

    get patchesPending() { return this.cursor < this.messages.length; }

    service(budget: number): boolean {
        const deadline = this.now() + Math.max(0, budget);
        let worked = false;
        if (!this.patchesPending && !this.prepare()) {
            this.messages = this.serviceEngine(Math.max(0, deadline - this.now()));
            this.cursor = 0;
            worked = true;
        }
        // At least one complete operation per slice; an indivisible operation
        // may overrun, but never starts another operation after its deadline.
        while (this.patchesPending && (!worked || this.now() < deadline)) {
            this.apply(this.messages[this.cursor++]);
            worked = true;
        }
        if (this.patchesPending) return true;
        this.messages = [];
        this.cursor = 0;
        return !this.prepare();
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
