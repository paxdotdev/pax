import {ObjectManager} from "../../pools/object-manager";
import { TEXT_STYLE } from "../../pools/supported-objects";
import { ColorGroup, TextStyle } from "../text";

export class RadioSetUpdatePatch {
    public id?: number;
    public size_x?: number;
    public size_y?: number;
    public background?: ColorGroup; 
    public transform?: number[];
    public selected_id?: number;
    public options?: string[];
    objectManager: ObjectManager;
    public style?: TextStyle;

    constructor(objectManager: ObjectManager) {
        this.objectManager = objectManager;
    }

    fromPatch(jsonMessage: any, registeredFontFaces: Set<string>) {
        this.id = jsonMessage["id"];
        this.size_x = jsonMessage["size_x"];
        this.size_y = jsonMessage["size_y"];
        this.transform = jsonMessage["transform"];
        this.options = jsonMessage["options"];
        this.background = jsonMessage["background"];
        this.selected_id = jsonMessage["selected_id"];
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
        this.options = [];
        this.selected_id = 0;
    }
}
