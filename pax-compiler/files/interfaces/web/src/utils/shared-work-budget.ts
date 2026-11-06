/** Background hosts still need to finish mounting when rAF is suspended. */
export function requestPreparationFrame(job: () => void): number {
    return document.hidden ? window.setTimeout(job, 100) : requestAnimationFrame(job);
}

type BudgetObservation = {at:number; event:'shared-budget-reset'|'shared-budget-work';
    usedMs:number; remainingMs:number; elapsedMs?:number; grantedMs?:number};

/** One cooperative allowance across hosted applications per display opportunity.
 * An indivisible Rust callback can overrun; subsequent work waits for a frame.
 */
export class SharedWorkBudget {
    private used = 0;
    private resetQueued = false;
    private observers = new Set<(event: BudgetObservation) => void>();
    constructor(private allowance = 4,
        private now = () => performance.now(),
        private frame = requestPreparationFrame) {}
    get remaining() { return Math.max(0, this.allowance - this.used); }
    /** Capture-only observation; does not request frames or replenish allowance. */
    observe(listener: (event: BudgetObservation) => void) {
        this.observers.add(listener);
        return () => { this.observers.delete(listener); };
    }
    private report(event: BudgetObservation['event'], elapsedMs?:number, grantedMs?:number) {
        if (!this.observers.size) return;
        const sample = {at:this.now(), event, usedMs:this.used, remainingMs:this.remaining, elapsedMs, grantedMs};
        for (const listener of this.observers) listener(sample);
    }
    run<T>(maximum: number, operation: (budget: number) => T): T {
        if (!this.resetQueued) {
            this.resetQueued = true;
            this.frame(() => {
                this.used = 0; this.resetQueued = false;
                this.report('shared-budget-reset');
            });
        }
        const start = this.now();
        const granted = Math.min(maximum, this.remaining);
        try { return operation(granted); }
        finally {
            const elapsed = Math.max(0, this.now() - start);
            this.used += elapsed;
            this.report('shared-budget-work', elapsed, granted);
        }
    }
}

export const sharedWorkBudget = new SharedWorkBudget();
