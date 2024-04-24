// @ts-ignore
import {Layer} from "./layer";
import {ObjectManager} from "../pools/object-manager";
import {ARRAY, LAYER} from "../pools/supported-objects";

import type {PaxChassisWeb} from "../types/pax-chassis-web";

export class OcclusionContext {
    private layers?: Layer[];
    private canvasMap?: Map<string, HTMLCanvasElement>;
    private parent?: Element;
    private nextOcclusionLayerId?: number;
    private objectManager: ObjectManager;
    private chassis?: PaxChassisWeb;

    constructor(objectManager: ObjectManager) {
        this.objectManager = objectManager;
    }

    attach(parent: Element, chassis: PaxChassisWeb, canvasMap: Map<string, HTMLCanvasElement>) {
        this.layers = this.objectManager.getFromPool(ARRAY);
        this.parent = parent;
        this.nextOcclusionLayerId = -1;
        this.chassis = chassis;
        this.canvasMap = canvasMap;
        this.growTo(0);
    }

    growTo(newOcclusionLayerId: number) {
        let occlusionLayerId = newOcclusionLayerId + 1;
        if(this.parent == undefined || this.canvasMap == undefined ||
            this.layers == undefined || this.chassis == undefined){
            return
        }
        if(this.nextOcclusionLayerId != undefined && this.nextOcclusionLayerId < occlusionLayerId){
            for(let i = this.nextOcclusionLayerId+1; i <= occlusionLayerId; i++) {
                let newLayer: Layer = this.objectManager.getFromPool(LAYER, this.objectManager);
                newLayer.build(this.parent, i, this.chassis, this.canvasMap);
                this.layers.push(newLayer);
            }
            this.nextOcclusionLayerId = occlusionLayerId;
        }
    }

    shrinkTo(occlusionLayerId: number){
        if(this.layers == undefined){
            return
        }
        if(this.nextOcclusionLayerId != undefined && this.nextOcclusionLayerId > occlusionLayerId) {
            for(let i = this.nextOcclusionLayerId; i > occlusionLayerId; i--){
                this.objectManager.returnToPool(LAYER, this.layers[i]);
                this.layers.pop();
            }
            this.nextOcclusionLayerId = occlusionLayerId;
        }
    }

    addElement(element: HTMLElement, occlusionLayerId: number){
        if(this.nextOcclusionLayerId != undefined){
            this.growTo(occlusionLayerId);
            element.style.zIndex = String(occlusionLayerId);
            this.layers![occlusionLayerId]!.native!.prepend(element);
        }
    }

    // TODO needed?
    // updateCanvases(width: number, height: number){
    //     if(this.layers != undefined){
    //         this.layers.forEach((layer)=>{layer.updateCanvas(width, height)});
    //     }
    // }

    cleanUp(){
        if(this.layers != undefined){
            this.layers.forEach((layer) => {
                this.objectManager.returnToPool(LAYER, layer);
            });
        }
        this.canvasMap = undefined;
        this.parent = undefined;
        this.nextOcclusionLayerId = undefined;
    }

    // TODO needed?
    // updateNativeOverlays(width: number, height: number){
    //     if(this.layers != undefined) {
    //         this.layers.forEach((layer) => {
    //             layer.updateNativeOverlay(width, height)
    //         });
    //     }
    // }
}
