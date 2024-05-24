import {ObjectManager} from "../../pools/object-manager";
import { TEXT_STYLE } from "../../pools/supported-objects";
import { ColorGroup, TextStyle } from "../text";

export class ButtonUpdatePatch {
    public id?: number;
    public size_x?: number;
    public size_y?: number;
    public transform?: number[];
    public content?: string;
    public color?: ColorGroup; 
    public style?: TextStyle;

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
        this.color = jsonMessage["color"];
        const styleMessage = jsonMessage["style"];

        if (styleMessage) {
            this.style = this.objectManager.getFromPool(TEXT_STYLE, this.objectManager);
            this.style.build(styleMessage, registeredFontFaces)
        }
    }

    cleanUp(){
        this.id = undefined;
        this.size_x = 0;
        this.size_y = 0;
        this.transform = [];
        this.objectManager.returnToPool(TEXT_STYLE, this.style);
        this.style = undefined;
    }
}
