import type {TextFontLoader} from "./text";

/** Gate only a Text node's first paint. Hidden text still participates in browser
 * layout/ResizeObserver; lifecycle and subsequent updates continue normally.
 */
export class InitialTextPresentation {
    updated = false;
    presented = false;
    fontsReady = true;
    generation?: number;
    onPresent?: () => void;
    font?: TextFontLoader;
    linkFont?: TextFontLoader;
    private measured = true;
    private hasMeasurement = false;
    private frames = 0;
    private fontRevision = 0;
    private fonts: Promise<void>[] = [];

    private fontRequest?: {loader?: TextFontLoader; text: string; ready?: Promise<void>};
    private linkRequest?: {loader?: TextFontLoader; text: string; ready?: Promise<void>};

    waitForTextFonts(text: string, linkText: string, ready: () => void) {
        if (this.fontRequest?.loader !== this.font || this.fontRequest?.text !== text) {
            this.fontRequest = {loader: this.font, text, ready: text ? this.font?.(text) : undefined};
        }
        if (this.linkRequest?.loader !== this.linkFont || this.linkRequest?.text !== linkText) {
            this.linkRequest = {loader: this.linkFont, text: linkText, ready: linkText ? this.linkFont?.(linkText) : undefined};
        }
        this.waitForFonts([this.fontRequest?.ready, this.linkRequest?.ready].filter(
            (font): font is Promise<void> => font != null,
        ), ready);
    }

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

    get startupReady(): boolean {
        return this.updated && this.fontsReady && this.measured && this.generation == null;
    }

    afterFrame(startupSettled = false): boolean {
        if (this.presented || !this.updated || !this.fontsReady || !this.measured) return false;
        // Feed the browser's measurement back through layout before the first
        // paint, including the following parent/Stacker update.
        if (!startupSettled && ++this.frames < 2) return false;
        this.presented = true;
        return true;
    }
}
