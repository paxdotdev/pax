import {ObjectManager} from "../../pools/object-manager";

export class CheckboxUpdatePatch {
    public id?: number;
    public size_x?: number;
    public size_y?: number;
    public transform?: number[];
    public checked?: boolean;
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
    }

    cleanUp(){
        this.id = undefined;
        this.size_x = 0;
        this.size_y = 0;
        this.transform = [];
        this.checked = undefined;
    }
}
