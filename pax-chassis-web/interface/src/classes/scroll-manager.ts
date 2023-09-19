export class ScrollManager {

    private scrollContainer: HTMLDivElement;
    private interpolator: HermiteInterpolator;
    lastInterruptScrollTop: number = 0;
    lastScrollTop: number = 0;
    touching: boolean = false;
    count = 0;


    constructor(parent: HTMLDivElement){
        this.scrollContainer = parent;
        this.interpolator = new HermiteInterpolator();

        setInterval(() => {
            // @ts-ignore
            let currentScrollTop = this.scrollContainer.scrollTop;
            if (currentScrollTop == this.lastScrollTop){
                this.count++;
            } else {
                this.count = 0;
            }
            if(this.count < 3){
                this.interpolator.update(Date.now(), currentScrollTop);
            }
            this.lastScrollTop = currentScrollTop;
        }, 0);

        this.scrollContainer.addEventListener('touchstart', ()=>{
            this.touching = true;
        });

        this.scrollContainer.addEventListener('touchend', ()=>{
            this.touching = false;
        });
    }

    getScrollDelta() {
        let ret;
        if (this.touching) {
            ret = this.lastScrollTop - this.lastInterruptScrollTop;
            this.lastInterruptScrollTop = this.lastScrollTop;
        } else {
            const predictedScrollTop = this.interpolator.predict(Date.now());
            const lerpFactor = 0.95; 
            const smoothedScrollTop = this.lerp(this.lastInterruptScrollTop, predictedScrollTop, lerpFactor);
            ret = smoothedScrollTop - this.lastInterruptScrollTop;
            this.lastInterruptScrollTop = smoothedScrollTop;
        }
        return ret;
    }

    private lerp(start: number, end: number, factor: number): number {
        return start + factor * (end - start);
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
        
        let predictedVelocity = 0;

        if (this.buffer.length >= 2) {
            predictedVelocity = (predictedPosition - y1) / (timestamp - t1);
        }
        if (predictedVelocity < 10){
            return y1;
        }

        return predictedPosition;
    }
}
