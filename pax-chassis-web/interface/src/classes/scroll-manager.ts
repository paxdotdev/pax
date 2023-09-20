interface PredictedTrajectory {
    initialPosition: number;
    initialVelocity: number;
    startTime: number;
}

export class ScrollManager {
    private scrollContainer: HTMLDivElement;
    private interpolator: HermiteInterpolator;

    lastInterruptScrollTop: number = 0;
    lastScrollTop: number = 0;
    predicting: boolean = false;
    count = 0;
    stopped = false;


    constructor(parent: HTMLDivElement, isMobile: boolean) {
        this.scrollContainer = parent;

        this.interpolator = new HermiteInterpolator();
        console.log(isMobile);
        if (isMobile) {
            setInterval(() => {
                let currentScrollTop = this.scrollContainer.scrollTop;
                this.interpolator.update(Date.now(), currentScrollTop);
                if (currentScrollTop == this.lastScrollTop) {
                    this.count++;
                } else {
                    this.count = 0;
                    this.stopped = false;
                }
                if (this.count > 3) {
                    this.stopped = true;
                }
                this.lastScrollTop = currentScrollTop;
            }, 1);

            this.scrollContainer.addEventListener('touchstart', () => {
                this.predicting = false;
            });

            this.scrollContainer.addEventListener('touchend', () => {
                this.predicting = true;
            });
        } else {
            setInterval(() => {
                this.lastScrollTop = this.scrollContainer.scrollTop;
            }, 0);
        }
    }

    getScrollDelta() {
        let ret;
        if (!this.predicting || this.stopped) {
            ret = this.lastScrollTop - this.lastInterruptScrollTop;
            this.lastInterruptScrollTop = this.lastScrollTop;
        } else {
            const predictedScrollTop =  this.interpolator.predict(Date.now());
            ret = predictedScrollTop - this.lastInterruptScrollTop;
            this.lastInterruptScrollTop = predictedScrollTop;
       }
        return ret;
    }
}


interface ScrollData {
    timestamp: number;
    position: number;
    velocity: number;
}

class HermiteInterpolator {
    private buffer: ScrollData[] = [];
    private initialTimestamp: number | null = null;

    private normalizeTimestamp(timestamp: number): number {
        if (this.initialTimestamp === null) {
            this.initialTimestamp = timestamp;
        }
        return timestamp - this.initialTimestamp;
    }

    update(actualTimestamp: number, position: number): void {
        const timestamp = this.normalizeTimestamp(actualTimestamp);

        if (this.buffer.length === 100) {
            this.buffer.shift();
        }

        let velocity = 0;
        if (this.buffer.length === 2) {
            const prevDelta = position - this.buffer[1].position;
            const prevTime = timestamp - this.buffer[1].timestamp;
            const earlierDelta = this.buffer[1].position - this.buffer[0].position;
            const earlierTime = this.buffer[1].timestamp - this.buffer[0].timestamp;
            velocity = 0.5 * ((prevDelta / prevTime) + (earlierDelta / earlierTime));
        } else if (this.buffer.length === 1) {
            velocity = (position - this.buffer[0].position) / (timestamp - this.buffer[0].timestamp);
        }

        this.buffer.push({ timestamp, position, velocity });
    }

    predict(actualTimestamp: number): number {
        if (this.buffer.length < 2) {
            return this.buffer.length === 1 ? this.buffer[0].position : 0;
        }

        const timestamp = this.normalizeTimestamp(actualTimestamp);
        const t0 = this.buffer[this.buffer.length - 2].timestamp;
        const t1 = this.buffer[this.buffer.length - 1].timestamp;
        const y0 = this.buffer[this.buffer.length - 2].position;
        const y1 = this.buffer[this.buffer.length - 1].position;
        const m = (timestamp - t0) / (t1 - t0);

        // Hermite interpolation formula
        const h00 = (2 * m * m * m) - (3 * m * m) + 1;
        const h10 = m * m * m - 2 * m * m + m;
        const h01 = -2 * m * m * m + 3 * m * m;
        const h11 = m * m * m - m * m;

        const predictedPosition = h00 * y0 + h10 * (t1 - t0) * this.buffer[this.buffer.length - 2].velocity + h01 * y1 + h11 * (t1 - t0) * this.buffer[this.buffer.length - 1].velocity;

        // Calculate predicted velocity
        let predictedVelocity = 0;
        if (this.buffer.length >= 2) {
            predictedVelocity = (predictedPosition - this.buffer[this.buffer.length - 1].position) / (timestamp - t1);
        }

        // Store the predicted value in the buffer
        //this.buffer.push({ timestamp, position: predictedPosition, velocity: predictedVelocity });

        return predictedPosition;
    }
}