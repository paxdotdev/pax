import {NATIVE_OVERLAY_CLASS} from '../utils/constants';
import {ObjectManager} from "../pools/object-manager";
import {CANVAS, DIV} from "../pools/supported-objects";
import type {PaxChassisWeb} from "../types/pax-chassis-web";


export class Layer {
    canvas?: HTMLCanvasElement;
    canvasMap?: Map<string, HTMLCanvasElement>;
    native?: HTMLDivElement;
    occlusionLayerId?: number;
    chassis?: PaxChassisWeb;
    objectManager: ObjectManager;


    constructor(objectManager: ObjectManager) {
        this.objectManager = objectManager;
    }

    build(parent: Element, occlusionLayerId: number, chassis: PaxChassisWeb, canvasMap: Map<string, HTMLCanvasElement>) {
        this.occlusionLayerId = occlusionLayerId;
        this.chassis = chassis;
        this.canvasMap = canvasMap;
        this.canvas = this.objectManager.getFromPool(CANVAS);
        this.native = this.objectManager.getFromPool(DIV);

        this.canvas.style.zIndex = String(occlusionLayerId);
        this.canvas.id = String(occlusionLayerId);
        parent.appendChild(this.canvas);

        canvasMap.set(this.canvas.id, this.canvas);
        chassis.add_context(this.canvas.id);

        this.native.className = NATIVE_OVERLAY_CLASS;
        this.native.style.zIndex = String(occlusionLayerId);
        parent.appendChild(this.native);
    }

    public cleanUp() {
        if (this.canvas != undefined && this.chassis != undefined && this.occlusionLayerId != undefined) {
            this.chassis.remove_context(this.occlusionLayerId.toString());
            this.canvasMap?.delete(this.canvas.id);
            let parent = this.canvas.parentElement;
            parent!.removeChild(this.canvas);
            this.objectManager.returnToPool(CANVAS, this.canvas);
        }
        if (this.native != undefined) {
            let parent = this.native.parentElement;
            parent!.removeChild(this.native);
            this.objectManager.returnToPool(DIV, this.native);
        }
        this.occlusionLayerId = undefined;
    }
}
