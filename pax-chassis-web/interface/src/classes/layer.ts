// @ts-ignore

import {CANVAS_CLASS, NATIVE_OVERLAY_CLASS} from '../utils/constants';
import {ObjectManager} from "../pools/object-manager";
import {ARRAY, CANVAS, DIV, UINT32ARRAY} from "../pools/supported-objects";
import {generateLocationId} from "../utils/helpers";

export class Layer {
    canvas?: HTMLCanvasElement;
    canvasMap?: Map<string, HTMLCanvasElement>;
    native?: HTMLDivElement;
    scrollerId?: number[];
    zIndex?: number;
    chassis?: any;
    objectManager: ObjectManager;


    constructor(objectManager: ObjectManager) {
        this.objectManager = objectManager;
    }

    build(parent: Element, zIndex: number, scroller_id: number[] | undefined, chassis: any, canvasMap: Map<string, HTMLCanvasElement>) {
        this.zIndex = zIndex;
        this.scrollerId = scroller_id;
        this.chassis = chassis;
        this.canvasMap = canvasMap;

        this.canvas = this.objectManager.getFromPool(CANVAS);
        this.native = this.objectManager.getFromPool(DIV);

        this.canvas.id = generateLocationId(scroller_id, zIndex);
        this.canvas.style.zIndex = String(1000 - zIndex);
        parent.appendChild(this.canvas);
        // @ts-ignore
        canvasMap.set(this.canvas.id, this.canvas);
        chassis.add_context(this.canvas.id);

        this.native.className = NATIVE_OVERLAY_CLASS;
        this.native.style.zIndex = String(1000 - zIndex);
        parent.appendChild(this.native);
        if (scroller_id != undefined) {
            this.canvas.style.position = "sticky";
            if (zIndex > 0) {
                this.canvas.style.marginTop = String(-this.canvas.style.height) + 'px';
            }
            this.native.style.position = "sticky";
            this.native.style.marginTop = String(-this.canvas.style.height) + 'px';
        }
    }

    public cleanUp() {
        if (this.canvas != undefined && this.chassis != undefined && this.zIndex != undefined) {
            this.chassis.remove_context(generateLocationId(this.scrollerId, this.zIndex));
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
        this.scrollerId = [];
        this.zIndex = undefined;
    }

    public updateCanvas(width: number, height: number) {
        requestAnimationFrame(() => {
            if (this.scrollerId != undefined && (this.zIndex != undefined && this.zIndex > 0)) {
                if (this.canvas != undefined) {
                    this.canvas.style.marginTop = String(-height) + "px";
                }
            }
        });

    }

    public updateNativeOverlay(width: number, height: number) {
        requestAnimationFrame(() => {
            if (this.native != undefined) {
                if (this.scrollerId != undefined) {
                    this.native.style.marginTop = String(-height) + "px";
                }
                this.native.style.width = String(width) + 'px';
                this.native.style.height = String(height) + 'px';
            }
        });
    }
}