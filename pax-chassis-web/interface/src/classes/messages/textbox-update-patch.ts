import {ObjectManager} from "../../pools/object-manager";
import { TEXT_STYLE } from "../../pools/supported-objects";
import { TextStyle } from "../text";

export class TextboxUpdatePatch {
    public id_chain?: number[];
    public size_x?: number;
    public size_y?: number;
    public stroke_width?: number;
    public stroke_color?: ColorGroup;
    public background?: ColorGroup; 
    public border_radius: number;
    public focus_on_mount?: boolean;
    public transform?: number[];
    public text?: string;
    objectManager: ObjectManager;
    public style?: TextStyle;

    constructor(objectManager: ObjectManager) {
        this.objectManager = objectManager;
    }

    fromPatch(jsonMessage: any, registeredFontFaces: Set<string>) {
        this.id_chain = jsonMessage["id_chain"];
        this.size_x = jsonMessage["size_x"];
        this.size_y = jsonMessage["size_y"];
        this.transform = jsonMessage["transform"];
        this.text = jsonMessage["text"];
        this.stroke_color = jsonMessage["stroke_color"];
        this.stroke_width = jsonMessage["stroke_width"];
        this.background = jsonMessage["background"];
        this.border_radius = jsonMessage["border_radius"];
        this.focus_on_mount = jsonMessage["focus_on_mount"];
        const styleMessage = jsonMessage["style"];

        if (styleMessage) {
            this.style = this.objectManager.getFromPool(TEXT_STYLE, this.objectManager);
            this.style.build(styleMessage, registeredFontFaces)
        }
    }

    cleanUp(){
        this.id_chain = [];
        this.size_x = 0;
        this.size_y = 0;
        this.transform = [];
        this.text = "";
    }
}
