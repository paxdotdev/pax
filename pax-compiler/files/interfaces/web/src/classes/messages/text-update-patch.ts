import {ObjectManager} from "../../pools/object-manager";
import {TEXT_STYLE} from "../../pools/supported-objects";
import {TextStyle} from "../text";

export class TextUpdatePatch {
    public id?: number;
    public content?: string;
    public size_x?: number;
    public size_y?: number;
    public transform?: number[];
    public style?: TextStyle;
    public style_link?: TextStyle;
    public depth?: number;
    public editable?: boolean;
    public selectable?: boolean;
    objectManager: ObjectManager;

    constructor(objectManager: ObjectManager) {
        this.objectManager = objectManager;
    }

    fromPatch(jsonMessage: any, registeredFontFaces: Set<string>) {
        this.id = jsonMessage["id"];
        this.content = jsonMessage["content"];
        this.size_x = jsonMessage["size_x"];
        this.size_y = jsonMessage["size_y"];
        this.transform = jsonMessage["transform"];
        this.depth = jsonMessage["depth"];
        this.editable = jsonMessage["editable"];
        this.selectable = jsonMessage["selectable"];

        const styleMessage = jsonMessage["style"];
        if (styleMessage) {
            this.style = this.objectManager.getFromPool(TEXT_STYLE, this.objectManager);
            this.style.build(styleMessage, registeredFontFaces)
        }

        const styleLinkMessage = jsonMessage["style_link"];
        if (styleLinkMessage) {
            this.style_link = this.objectManager.getFromPool(TEXT_STYLE, this.objectManager);
            this.style_link.build(styleLinkMessage, registeredFontFaces);
        }
    }

    cleanUp(){
        this.id = undefined;
        this.content = '';
        this.size_x = 0;
        this.size_y = 0;
        this.transform = [];
        this.objectManager.returnToPool(TEXT_STYLE, this.style);
        this.style = undefined;
        this.objectManager.returnToPool(TEXT_STYLE, this.style_link);
        this.style_link = undefined;
        this.editable = false;
        this.selectable = false;
    }
}
