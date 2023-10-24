import {ObjectManager} from "../../pools/object-manager";
import {TEXT_STYLE} from "../../pools/supported-objects";
import {TextStyle} from "../text";

export class CheckboxUpdatePatch {
    public id_chain?: number[];
    public size_x?: number;
    public size_y?: number;
    public transform?: number[];
    public checked?: boolean;
    //public style?: TextStyle;
    objectManager: ObjectManager;

    constructor(objectManager: ObjectManager) {
        this.objectManager = objectManager;
    }

    fromPatch(jsonMessage: any) {
        this.id_chain = jsonMessage["id_chain"];
        this.size_x = jsonMessage["size_x"];
        this.size_y = jsonMessage["size_y"];
        this.transform = jsonMessage["transform"];
        this.checked = jsonMessage["checked"];
    }

    cleanUp(){
        this.id_chain = [];
        this.size_x = 0;
        this.size_y = 0;
        this.transform = [];
        this.checked = undefined;
    }
}