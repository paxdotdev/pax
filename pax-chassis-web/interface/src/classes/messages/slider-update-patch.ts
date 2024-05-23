import {ObjectManager} from "../../pools/object-manager";
import { ColorGroup } from "../text";

export class SliderUpdatePatch {
    public id?: number;
    public size_x?: number;
    public size_y?: number;
    public accent?: ColorGroup;
    public transform?: number[];
    public value?: number;
    public step?: number;
    public min?: number;
    public max?: number;
    objectManager: ObjectManager;

    constructor(objectManager: ObjectManager) {
        this.objectManager = objectManager;
    }

    fromPatch(jsonMessage: any) {
        this.id = jsonMessage["id"];
        this.size_x = jsonMessage["size_x"];
        this.size_y = jsonMessage["size_y"];
        this.transform = jsonMessage["transform"];
        this.accent = jsonMessage["accent"];
        this.value = jsonMessage["value"];
        this.step = jsonMessage["step"];
        this.min= jsonMessage["min"];
        this.max = jsonMessage["max"];
    }

    cleanUp(){
        this.id = undefined;
        this.size_x = 0;
        this.size_y = 0;
        this.value = 0;
        this.step = 0;
        this.min = 0;
        this.max = 0;
        this.transform = [];
    }
}
