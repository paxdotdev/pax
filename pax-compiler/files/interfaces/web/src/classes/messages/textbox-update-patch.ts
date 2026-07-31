import {ObjectManager} from "../../pools/object-manager";
import { TEXT_STYLE } from "../../pools/supported-objects";
import { ColorGroup, TextStyle } from "../text";

export class TextboxUpdatePatch {
    public id?: number;
    public parentFrame?: number;
    public zIndex?: number;
    public size_x?: number;
    public size_y?: number;
    public stroke_width?: number;
    public stroke_color?: ColorGroup;
    public background?: ColorGroup; 
    public corner_radius?: number;
    public focus_on_mount?: boolean;
    public transform?: number[];
    public opacity?: number;
    public text?: string;
    public placeholder?: string;
    objectManager: ObjectManager;
    public style?: TextStyle;
    public outline_width?: number;
    public outline_color?: ColorGroup;
    public is_text_area?: boolean;

    constructor(objectManager: ObjectManager) {
        this.objectManager = objectManager;
    }

    fromPatch(jsonMessage: any, registeredFontFaces: Set<string>) {
        this.id = jsonMessage["id"];
        this.parentFrame = jsonMessage["parent_frame"];
        this.zIndex = jsonMessage["z_index"];
        this.size_x = jsonMessage["size_x"];
        this.size_y = jsonMessage["size_y"];
        this.transform = jsonMessage["transform"];
        this.opacity = jsonMessage["opacity"];
        this.text = jsonMessage["text"];
        this.stroke_color = jsonMessage["stroke_color"];
        this.stroke_width = jsonMessage["stroke_width"];
        this.background = jsonMessage["background"];
        this.corner_radius = jsonMessage["corner_radius"];
        this.focus_on_mount = jsonMessage["focus_on_mount"];
        this.placeholder = jsonMessage["placeholder"];
        const styleMessage = jsonMessage["style"];
        this.outline_width = jsonMessage["outline_width"];
        this.outline_color = jsonMessage["outline_color"];
        this.is_text_area = jsonMessage["is_text_area"];

        if (styleMessage) {
            this.style = this.objectManager.getFromPool(TEXT_STYLE, this.objectManager);
            this.style.build(styleMessage, registeredFontFaces)
        }
    }

    cleanUp(){
        this.id = undefined;
        this.parentFrame = undefined;
        this.zIndex = undefined;
        this.size_x = 0;
        this.size_y = 0;
        this.transform = [];
        this.opacity = undefined;
        this.text = "";
        this.is_text_area = undefined;
    }
}
