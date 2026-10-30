/** Background hosts still need to finish mounting when rAF is suspended. */
export function requestPreparationFrame(job: () => void): number {
    return document.hidden ? window.setTimeout(job, 100) : requestAnimationFrame(job);
}

/** One cooperative allowance across hosted applications per display opportunity.
 * An indivisible Rust callback can overrun; subsequent work waits for a frame.
 */
export class SharedWorkBudget {
    private used = 0;
    private resetQueued = false;
    constructor(private allowance = 4,
        private now = () => performance.now(),
        private frame = requestPreparationFrame) {}
    get remaining() { return Math.max(0, this.allowance - this.used); }
    run<T>(maximum: number, operation: (budget: number) => T): T {
        if (!this.resetQueued) {
            this.resetQueued = true;
            this.frame(() => { this.used = 0; this.resetQueued = false; });
        }
        const start = this.now();
        try { return operation(Math.min(maximum, this.remaining)); }
        finally { this.used += Math.max(0, this.now() - start); }
    }
}

export const sharedWorkBudget = new SharedWorkBudget();
