import {ObjectManager} from "../pools/object-manager";
import {FONT} from "../pools/supported-objects";

type RegisteredFontDescriptor = {
    family: string;
    style: string;
    weight: string;
};

export type TextFontLoader = (text: string) => Promise<void>;

type FontRegistration = {load: TextFontLoader; prepare: () => Promise<void>};
const fontLoads = new Map<string, FontRegistration>();
const stylesheetLoads = new Map<string, Promise<void>>();
const pendingFontLoads = new Map<string, Promise<void>>();
const registeredFontCss = new Map<string, string>();
const registeredFontDescriptors = new Map<string, RegisteredFontDescriptor>();
const embeddedFontCss = new Map<string, Promise<string>>();
const embeddedFontAssetUrls = new Map<string, Promise<string>>();
const FONT_STYLE_DATA_ATTRIBUTE = "data-pax-font-key";

function getDocumentFonts(targetDocument: Document): FontFaceSet | undefined {
    return targetDocument.fonts as FontFaceSet | undefined;
}

function quoteFontFamily(family: string): string {
    return `"${family.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
}

function buildFontShorthand(descriptor: RegisteredFontDescriptor): string {
    return `${descriptor.style} ${descriptor.weight} 16px ${quoteFontFamily(descriptor.family)}`;
}

function appendFontCss(targetDocument: Document, fontKey: string, css: string) {
    const existingStyles = Array.from(
        targetDocument.head.querySelectorAll(`style[${FONT_STYLE_DATA_ATTRIBUTE}]`),
    );
    if (existingStyles.some((styleNode) => styleNode.getAttribute(FONT_STYLE_DATA_ATTRIBUTE) === fontKey)) {
        return;
    }

    const style = targetDocument.createElement("style");
    style.setAttribute(FONT_STYLE_DATA_ATTRIBUTE, fontKey);
    style.textContent = css;
    targetDocument.head.appendChild(style);
}

async function waitForDocumentFonts(targetDocument: Document): Promise<void> {
    const fonts = getDocumentFonts(targetDocument);
    if (!fonts?.ready) {
        return;
    }
    await fonts.ready;
}

async function loadFontDescriptor(targetDocument: Document, descriptor: RegisteredFontDescriptor, text = " "): Promise<void> {
    const fonts = getDocumentFonts(targetDocument);
    if (!fonts) {
        return;
    }

    await fonts.load(buildFontShorthand(descriptor), text);
}

function arrayBufferToBase64(buffer: ArrayBuffer): string {
    const bytes = new Uint8Array(buffer);
    const chunkSize = 0x8000;
    let binary = "";
    for (let index = 0; index < bytes.length; index += chunkSize) {
        const chunk = bytes.subarray(index, index + chunkSize);
        let chunkBinary = "";
        chunk.forEach((value) => {
            chunkBinary += String.fromCharCode(value);
        });
        binary += chunkBinary;
    }
    return btoa(binary);
}

function fetchAssetAsDataUrl(url: string): Promise<string> {
    let existing = embeddedFontAssetUrls.get(url);
    if (existing) {
        return existing;
    }

    const request = fetch(url)
        .then(async (response) => {
            if (!response.ok) {
                throw new Error(`Failed to fetch font asset ${url}: ${response.status}`);
            }

            const contentType = response.headers.get("content-type") || "font/woff2";
            const buffer = await response.arrayBuffer();
            return `data:${contentType};base64,${arrayBufferToBase64(buffer)}`;
        }).catch(error => {
            embeddedFontAssetUrls.delete(url);
            throw error;
        });
    embeddedFontAssetUrls.set(url, request);
    return request;
}

async function inlineExternalFontUrls(css: string, baseUrl: string): Promise<string> {
    const matches = Array.from(css.matchAll(/url\((['"]?)([^'")]+)\1\)/g));
    if (matches.length === 0) {
        return css;
    }

    let embeddedCss = css;
    const replacements = await Promise.all(matches.map(async (match) => {
        const original = match[0];
        const rawUrl = match[2];
        if (!original || !rawUrl || rawUrl.startsWith("data:")) {
            return null;
        }

        const resolvedUrl = new URL(rawUrl, baseUrl).toString();
        const dataUrl = await fetchAssetAsDataUrl(resolvedUrl);
        return [original, `url("${dataUrl}")`] as const;
    }));

    replacements.forEach((replacement) => {
        if (!replacement) {
            return;
        }
        embeddedCss = embeddedCss.replaceAll(replacement[0], replacement[1]);
    });

    return embeddedCss;
}

// Keep stylesheet URLs absolute when moving CSS into the document or a capture.
// Do not fetch the referenced binaries: the browser selects faces and unicode ranges.
function resolveFontUrls(css: string, baseUrl: string): string {
    return css.replace(/url\((['"]?)([^'")]+)\1\)/g, (_match, _quote, url) =>
        `url("${new URL(url, baseUrl).href}")`);
}

function isFontStylesheet(url: string): boolean {
    const source = new URL(url, document.baseURI);
    return source.pathname.endsWith(".css") ||
        (source.hostname === "fonts.googleapis.com" && source.pathname.startsWith("/css"));
}

function loadFontStylesheet(url: string): Promise<void> {
    let pending = stylesheetLoads.get(url);
    if (!pending) {
        pending = fetch(url).then(async response => {
            if (!response.ok) throw new Error(`Failed to fetch font CSS: ${response.status}`);
            const css = resolveFontUrls(await response.text(), response.url || new URL(url, document.baseURI).href);
            registeredFontCss.set(url, css);
            appendFontCss(document, url, css);
        }).catch(error => {
            stylesheetLoads.delete(url);
            throw error;
        });
        stylesheetLoads.set(url, pending);
    }
    return pending;
}

/** Fetch statically known font stylesheets alongside the cartridge download.
 * Font binaries remain demand-loaded for the text that uses them.
 * Runtime-created fonts still follow the ordinary registration path below.
 */
export async function preloadManifestFonts(baseUrl: string): Promise<void> {
    try {
        const response = await fetch(new URL("pax-fonts.json", baseUrl));
        if (!response.ok) return; // Older/custom hosts need not supply the catalog.
        const sources: {family: string; url: string}[] = await response.json();
        await Promise.allSettled(sources.map(async ({url}) => {
            if (isFontStylesheet(url)) await loadFontStylesheet(url);
        }));
    } catch (error) {
        console.warn("Failed to preload manifest fonts", error);
    }
}

function trackFontLoad(fontKey: string, loadPromise: Promise<void>) {
    const trackedPromise = loadPromise.finally(() => {
        if (pendingFontLoads.get(fontKey) === trackedPromise) pendingFontLoads.delete(fontKey);
    });
    pendingFontLoads.set(fontKey, trackedPromise);
}

export async function waitForRegisteredFonts(): Promise<void> {
    const pendingLoads = Array.from(pendingFontLoads.values());
    if (pendingLoads.length > 0) {
        await Promise.allSettled(pendingLoads);
    }

    const descriptors = Array.from(registeredFontDescriptors.values());
    await Promise.allSettled(
        descriptors.map(async (descriptor) => {
            try {
                await loadFontDescriptor(document, descriptor);
            } catch (err) {
                console.warn(`Failed to load font ${descriptor.family}`, err);
            }
        }),
    );

    await waitForDocumentFonts(document);
}

export async function syncRegisteredFontsToDocument(targetDocument: Document): Promise<void> {
    for (const [fontKey, css] of registeredFontCss.entries()) {
        appendFontCss(targetDocument, fontKey, await embedFontCss(fontKey, css));
    }

    const descriptors = Array.from(registeredFontDescriptors.values());
    await Promise.allSettled(
        descriptors.map(async (descriptor) => {
            try {
                await loadFontDescriptor(targetDocument, descriptor);
            } catch (err) {
                console.warn(`Failed to clone font ${descriptor.family}`, err);
            }
        }),
    );

    await waitForDocumentFonts(targetDocument);
}

function embedFontCss(key: string, css: string): Promise<string> {
    let pending = embeddedFontCss.get(key);
    if (!pending) {
        pending = inlineExternalFontUrls(css, document.baseURI).catch(error => {
            embeddedFontCss.delete(key);
            throw error;
        });
        embeddedFontCss.set(key, pending);
    }
    return pending;
}

// Embedding is a capture concern; ordinary display never waits for unused faces.
export async function getRegisteredFontCssText(): Promise<string> {
    return (await Promise.all(Array.from(registeredFontCss, ([key, css]) => embedFontCss(key, css)))).join("\n");
}

enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    Normal,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}



export class Font {
    private textLoader: TextFontLoader = async () => {};
    private prepare: () => Promise<void> = async () => {};

    // The returned function captures immutable values, not this pooled Font.
    getTextLoader(): TextFontLoader { return this.textLoader; }
    public type?: string;
    public family?: string;
    public style?: FontStyle;
    public weight?: FontWeight;
    public url?: string; // for WebFontMessage
    public path?: string; // for LocalFontMessage

    mapFontWeight(fontWeight : FontWeight) {
        switch (fontWeight) {
            case FontWeight.Thin:
                return 100;
            case FontWeight.ExtraLight:
                return 200;
            case FontWeight.Light:
                return 300;
            case FontWeight.Normal:
                return 400;
            case FontWeight.Medium:
                return 500;
            case FontWeight.SemiBold:
                return 600;
            case FontWeight.Bold:
                return 700;
            case FontWeight.ExtraBold:
                return 800;
            case FontWeight.Black:
                return 900;
            default:
                return 400; // Return a default value if fontWeight is not found
        }
    }

    mapFontStyle(fontStyle: FontStyle) {
        switch (fontStyle) {
            case FontStyle.Normal:
                return 'normal';
            case FontStyle.Italic:
                return 'italic';
            case FontStyle.Oblique:
                return 'oblique';
            default:
                return 'normal'; // Return a default value if fontStyle is not found
        }
    }
    fromFontPatch(fontPatch: any, registeredFontFaces: Set<string>) {
        const type = Object.keys(fontPatch)[0];
        const data = fontPatch[type];
        this.type = type;
        if (type === "System") {
            this.family = data.family;
            this.style = FontStyle[data.style as keyof typeof FontStyle];
            this.weight = FontWeight[data.weight as keyof typeof FontWeight];
        } else if (type === "Web") {
            this.family = data.family;
            this.url = data.url;
            this.style = FontStyle[data.style as keyof typeof FontStyle];
            this.weight = FontWeight[data.weight as keyof typeof FontWeight];
        } else if (type === "Local") {
            this.family = data.family;
            this.path = data.path;
            this.style = FontStyle[data.style as keyof typeof FontStyle];
            this.weight = FontWeight[data.weight as keyof typeof FontWeight];
        }
        this.registerFontFace(registeredFontFaces);
    }

    cleanUp(){
        this.textLoader = async () => {};
        this.prepare = async () => {};
        this.type = undefined;
        this.family = undefined;
        this.style = undefined;
        this.url = undefined;
        this.style = undefined;
        this.weight = undefined;
        this.path = undefined;
    }

    private fontKey(): string {
        return JSON.stringify([this.type, this.family, this.style, this.weight, this.url ?? this.path]);
    }

    registerFontFace(registeredFontFaces: Set<string>) {
        const fontKey = this.fontKey();
        registeredFontFaces.add(fontKey);
        const existing = fontLoads.get(fontKey);
        if (existing) { this.textLoader = existing.load; this.prepare = existing.prepare; return; }
        const {family, type} = this;
        const url = this.url ?? this.path;
        const style = this.style != undefined ? this.mapFontStyle(this.style) : 'normal';
        const weight = this.weight != undefined ? String(this.mapFontWeight(this.weight)) : '400';
        const descriptor = family ? {family, style, weight} : undefined;
        let registration: Promise<void> | undefined;
        let failed = false;
        const register = async () => {
            if (descriptor) registeredFontDescriptors.set(fontKey, descriptor);
            if (!family || !url || (type !== "Web" && type !== "Local")) return;
            if (type === "Web" && isFontStylesheet(url)) {
                await loadFontStylesheet(url);
            } else {
                const absoluteUrl = new URL(url, document.baseURI).href;
                const css = `@font-face {font-family:${quoteFontFamily(family)};font-style:${style};font-weight:${weight};src:url("${absoluteUrl}");}`;
                registeredFontCss.set(fontKey, css);
                const face = await new FontFace(family, `url("${absoluteUrl}")`, {style, weight}).load();
                document.fonts.add(face);
            }
        };
        const fallback = (error: unknown) => {
            if (!failed) console.warn(`Failed to load font ${family}`, error);
            failed = true;
        };
        const prepare = () => registration ??= register().catch(fallback);
        this.prepare = prepare;
        this.textLoader = (text) => {
            // Unused link styles and empty text must not start font downloads.
            if (!text || !descriptor || failed) return Promise.resolve();
            const pending = prepare().then(() => {
                if (!failed) return loadFontDescriptor(document, descriptor, text);
            }).catch(fallback);
            trackFontLoad(fontKey, pending);
            return pending;
        };
        fontLoads.set(fontKey, {load: this.textLoader, prepare});
    }

    applyFontToDiv(div: HTMLElement) {
        void this.prepare();
        if (this.family != undefined) {
            div.style.fontFamily = this.family;
        }
        if (this.style != undefined) {
            div.style.fontStyle = this.mapFontStyle(this.style);
        }
        if (this.weight != undefined) {
            div.style.fontWeight = String(this.mapFontWeight(this.weight));
        }
    }
}

export class TextStyle {
    public font?: Font;
    public fill?: ColorGroup;
    public font_size?: number;
    public underline?: boolean;
    public align_multiline?: TextAlignHorizontal;
    public align_horizontal?: TextAlignHorizontal;
    public align_vertical?: TextAlignVertical;
    objectManager: ObjectManager;

    constructor(objectManager: ObjectManager) {
        this.objectManager = objectManager;
    }

    build(styleMessage: any, registeredFontFaces: Set<string>) {
        if (styleMessage["font"]) {
            const font: Font = this.objectManager.getFromPool(FONT);
            font.fromFontPatch(styleMessage["font"], registeredFontFaces);
            this.font = font;
        }
        this.fill = styleMessage["fill"];
        this.font_size = styleMessage["font_size"];
        this.underline = styleMessage["underline"];
        this.align_multiline = styleMessage["align_multiline"];
        this.align_horizontal = styleMessage["align_horizontal"];
        this.align_vertical = styleMessage["align_vertical"];
    }

    cleanUp(){
        if(this.font){
            this.objectManager.returnToPool(FONT, this.font!);
            this.font = undefined;
        }
        this.fill = undefined;
        this.font_size = undefined;
        this.underline = undefined;
        this.align_multiline = undefined;
        this.align_horizontal = undefined;
        this.align_vertical = undefined;
    }
}

enum TextAlignHorizontal {
    Left = "Left",
    Center = "Center",
    Right = "Right",
}

export function getJustifyContent(horizontalAlignment: string): string {
    switch (horizontalAlignment) {
        case TextAlignHorizontal.Left:
            return 'flex-start';
        case TextAlignHorizontal.Center:
            return 'center';
        case TextAlignHorizontal.Right:
            return 'flex-end';
        default:
            return 'flex-start';
    }
}

export function getTextAlign(paragraphAlignment: string): string {
    switch (paragraphAlignment) {
        case TextAlignHorizontal.Left:
            return 'left';
        case TextAlignHorizontal.Center:
            return 'center';
        case TextAlignHorizontal.Right:
            return 'right';
        default:
            return 'left';
    }
}

enum TextAlignVertical {
    Top = "Top",
    Center = "Center",
    Bottom = "Bottom",
}

export function getAlignItems(verticalAlignment: string): string {
    switch (verticalAlignment) {
        case TextAlignVertical.Top:
            return 'flex-start';
        case TextAlignVertical.Center:
            return 'center';
        case TextAlignVertical.Bottom:
            return 'flex-end';
        default:
            return 'flex-start';
    }
}

export class ColorGroup {
    Rgba?: number[];
}
