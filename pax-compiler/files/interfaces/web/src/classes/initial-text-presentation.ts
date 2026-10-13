/** Gate only a Text node's first paint. Hidden text still participates in browser
 * layout/ResizeObserver; lifecycle and subsequent updates continue normally.
 */
export class InitialTextPresentation {
    updated = false;
    presented = false;
    fontsReady = true;
    generation?: number;
    onPresent?: () => void;
    font?: Promise<void>;
    linkFont?: Promise<void>;
    private measured = true;
    private hasMeasurement = false;
    private frames = 0;
    private fontRevision = 0;
    private fonts: Promise<void>[] = [];

    waitForFonts(fonts: Promise<void>[], ready: () => void) {
        if (fonts.length === this.fonts.length && fonts.every((font, i) => font === this.fonts[i])) return;
        this.fonts = fonts;
        const revision = ++this.fontRevision;
        this.fontsReady = false;
        this.frames = 0;
        Promise.allSettled(fonts).then(() => {
            if (revision !== this.fontRevision) return;
            this.fontsReady = true;
            ready();
        });
    }

    requestMeasurement(generation: number) {
        this.generation = generation;
        if (!this.hasMeasurement) {
            this.measured = false;
            this.frames = 0;
        }
    }

    fixedSize() {
        if (this.generation != null) {
            this.generation = undefined;
            this.measured = true;
            this.frames = 0;
        }
    }

    waitForFontMeasurement() {
        if (this.presented) return;
        this.measured = false;
        this.hasMeasurement = false;
        this.frames = 0;
    }

    measuredSize(): number | undefined {
        const generation = this.generation;
        this.generation = undefined;
        this.measured = true;
        // An animated size/style can produce a new generation every frame.
        // Once initial metrics exist, later requests must not starve first paint.
        if (!this.hasMeasurement) this.frames = 0;
        this.hasMeasurement = true;
        return generation;
    }

    afterFrame(): boolean {
        if (this.presented || !this.updated || !this.fontsReady || !this.measured) return false;
        // Feed the browser's measurement back through layout before the first
        // paint, including the following parent/Stacker update.
        if (++this.frames < 2) return false;
        this.presented = true;
        return true;
    }
}
