import {ObjectManager} from "../pools/object-manager";
import {FONT} from "../pools/supported-objects";

type RegisteredFontDescriptor = {
    family: string;
    style: string;
    weight: string;
};

const fontLoads = new Map<string, Promise<void>>();
const stylesheetLoads = new Map<string, Promise<void>>();
const pendingFontLoads = new Map<string, Promise<void>>();
const registeredFontCss = new Map<string, string>();
const registeredFontDescriptors = new Map<string, RegisteredFontDescriptor>();
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

async function loadFontDescriptor(targetDocument: Document, descriptor: RegisteredFontDescriptor): Promise<void> {
    const fonts = getDocumentFonts(targetDocument);
    if (!fonts) {
        return;
    }

    await fonts.load(buildFontShorthand(descriptor));
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

// A stylesheet commonly declares several weights. Fetch, embed and register it
// once even when many Text styles request different weights in the same frame.
function loadFontStylesheet(url: string): Promise<void> {
    let pending = stylesheetLoads.get(url);
    if (!pending) {
        pending = fetch(url).then(async response => {
            if (!response.ok) throw new Error(`Failed to fetch font CSS: ${response.status}`);
            const css = await inlineExternalFontUrls(await response.text(), url);
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

/** Overlap statically known stylesheet/font loading with the cartridge download.
 * Runtime-created fonts still follow the ordinary registration path below.
 */
export async function preloadManifestFonts(baseUrl: string): Promise<void> {
    try {
        const response = await fetch(new URL("pax-fonts.json", baseUrl));
        if (!response.ok) return; // Older/custom hosts need not supply the catalog.
        const sources: {family: string; url: string}[] = await response.json();
        await Promise.allSettled(sources.map(async ({family, url}) => {
            if (!url.includes("fonts.googleapis.com/css")) return;
            await loadFontStylesheet(url);
            const loads: Promise<FontFace>[] = [];
            document.fonts.forEach(face => {
                if (face.family.replace(/^['"]|['"]$/g, "") === family) loads.push(face.load());
            });
            await Promise.allSettled(loads);
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
        appendFontCss(targetDocument, fontKey, css);
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

export function getRegisteredFontCssText(): string {
    return Array.from(registeredFontCss.values()).join("\n");
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
    public ready: Promise<void> = Promise.resolve();
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
        this.ready = Promise.resolve();
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
        if (existing) { this.ready = existing; return; }
        // Font instances are pooled: async work must capture values, not `this`.
        const {family, type} = this;
        const url = this.url ?? this.path;
        const style = this.style != undefined ? this.mapFontStyle(this.style) : 'normal';
        const weight = this.weight != undefined ? String(this.mapFontWeight(this.weight)) : '400';
        const descriptor = family ? {family, style, weight} : undefined;
        if (descriptor) registeredFontDescriptors.set(fontKey, descriptor);
        const load = async () => {
            if (!family || !url || (type !== "Web" && type !== "Local")) return;
            if (type === "Web" && url.includes("fonts.googleapis.com/css")) {
                await loadFontStylesheet(url);
            } else {
                const face = await new FontFace(family, `url(${url})`, {style, weight}).load();
                (document.fonts as any).add(face);
            }
            if (descriptor) await loadFontDescriptor(document, descriptor);
        };
        this.ready = load().catch(error => {
            // Cache fallback readiness too: retrying on every animated style
            // patch could keep resetting a new Text node's first-paint gate.
            console.warn(`Failed to load font ${family}`, error);
        });
        fontLoads.set(fontKey, this.ready);
        trackFontLoad(fontKey, this.ready);
    }
    applyFontToDiv(div: HTMLElement) {
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
