import {ObjectManager} from "../pools/object-manager";
import {FONT} from "../pools/supported-objects";

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
        this.type = undefined;
        this.family = undefined;
        this.style = undefined;
        this.url = undefined;
        this.style = undefined;
        this.weight = undefined;
        this.path = undefined;
    }

    private fontKey(): string {
        return `${this.type}-${this.family}-${this.style}-${this.weight}`;
    }

    registerFontFace(registeredFontFaces: Set<string>) {
        const fontKey = this.fontKey();
        if (!registeredFontFaces.has(fontKey)) {
            registeredFontFaces.add(fontKey);

            if (this.type === "Web" && this.url && this.family) {
                if (this.url.includes("fonts.googleapis.com/css")) {
                    // Fetch the Google Fonts CSS file and create a <style> element to insert its content
                    fetch(this.url)
                        .then(response => response.text())
                        .then(css => {
                            const style = document.createElement("style");
                            style.textContent = css;
                            document.head.appendChild(style);
                        });
                } else {
                    const fontFace = new FontFace(this.family, `url(${this.url})`, {
                        style: this.style ? FontStyle[this.style] : undefined,
                        weight: this.weight ? FontWeight[this.weight] : undefined,
                    });

                    fontFace.load().then(loadedFontFace => {
                        document.fonts.add(loadedFontFace);
                    });
                }
            } else if (this.type === "Local" && this.path && this.family) {
                const fontFace = new FontFace(this.family, `url(${this.path})`, {
                    style: this.style ? FontStyle[this.style] : undefined,
                    weight: this.weight ? FontWeight[this.weight] : undefined,
                });

                fontFace.load().then(loadedFontFace => {
                    document.fonts.add(loadedFontFace);
                });
            }
        }
    }
    applyFontToDiv(div: HTMLDivElement) {
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
