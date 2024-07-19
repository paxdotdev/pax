import {ObjectManager} from "../../pools/object-manager";
import { ColorGroup } from "../text";

export class CheckboxUpdatePatch {
    public id?: number;
    public size_x?: number;
    public size_y?: number;
    public transform?: number[];
    public checked?: boolean;
    public borderRadius?: number;
    public outlineColor?: ColorGroup;
    public outlineWidth?: number;
    public background?: ColorGroup;
    public backgroundChecked?: ColorGroup;

    objectManager: ObjectManager;

    constructor(objectManager: ObjectManager) {
        this.objectManager = objectManager;
    }

    fromPatch(jsonMessage: any) {
        this.id = jsonMessage["id"];
        this.size_x = jsonMessage["size_x"];
        this.size_y = jsonMessage["size_y"];
        this.transform = jsonMessage["transform"];
        this.checked = jsonMessage["checked"];
        this.borderRadius = jsonMessage["border_radius"];
        this.outlineColor = jsonMessage["outline_color"];
        this.outlineWidth = jsonMessage["outline_width"];
        this.background = jsonMessage["background"];
        this.backgroundChecked = jsonMessage["background_checked"];
    }

    cleanUp(){
        this.id = undefined;
        this.size_x = 0;
        this.size_y = 0;
        this.transform = [];
        this.checked = undefined;
    }
}
