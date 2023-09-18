// @ts-ignore
import {PaxChassisWeb} from '../../dist/pax_chassis_web';
import {Layer} from "./layer";
import {ObjectManager} from "../pools/object-manager";
import {ARRAY, LAYER} from "../pools/supported-objects";
export class OcclusionContext {
    private layers?: Layer[];
    private canvasMap?: Map<string, HTMLCanvasElement>;
    private parent?: Element;
    private zIndex?: number;
    private scrollerId?: number[];
    private objectManager: ObjectManager;
    private chassis?: PaxChassisWeb;

    constructor(objectManager: ObjectManager) {
        this.objectManager = objectManager;
    }

    build(parent: Element, scrollerId : number[] | undefined, chassis: PaxChassisWeb, canvasMap: Map<string, HTMLCanvasElement>) {
        this.layers = this.objectManager.getFromPool(ARRAY);
        this.parent = parent;
        this.zIndex = -1;
        this.scrollerId = scrollerId;
        this.chassis = chassis;
        this.canvasMap = canvasMap;
        this.growTo(0);
    }

    growTo(zIndex: number) {
        if(this.parent == undefined || this.canvasMap == undefined ||
            this.layers == undefined || this.chassis == undefined){
            return
        }
        if(this.zIndex != undefined && this.zIndex < zIndex){
            for(let i = this.zIndex+1; i <= zIndex; i++) {
                let newLayer: Layer = this.objectManager.getFromPool(LAYER, this.objectManager);
                newLayer.build(this.parent, i, this.scrollerId, this.chassis, this.canvasMap);
                this.layers.push(newLayer);
            }
            this.zIndex = zIndex;
        }
    }

    shrinkTo(zIndex: number){
        if(this.layers == undefined){
            return
        }
        if(this.zIndex != undefined && this.zIndex > zIndex) {
            for(let i = this.zIndex; i > zIndex; i--){
                this.objectManager.returnToPool(LAYER, this.layers[i]);
                this.layers.pop();
            }
            this.zIndex = zIndex;
        }
    }

    addElement(element: HTMLElement, zIndex: number){
        if(this.zIndex != undefined){
            if(zIndex > this.zIndex){
                this.growTo(zIndex);
            }
            element.style.zIndex = String(1000-zIndex);
            // @ts-ignore
            this.layers[zIndex].native.prepend(element);
        }
    }

    updateCanvases(width: number, height: number){
        if(this.layers != undefined){
            this.layers.forEach((layer)=>{layer.updateCanvas(width, height)});
        }
    }

    cleanUp(){
        if(this.layers != undefined){
            this.layers.forEach((layer) => {
                this.objectManager.returnToPool(LAYER, layer);
            });
        }
        this.canvasMap = undefined;
        this.parent = undefined;
        this.zIndex = undefined;
        this.scrollerId = undefined;
    }

    updateNativeOverlays(width: number, height: number){
        if(this.layers != undefined) {
            this.layers.forEach((layer) => {
                layer.updateNativeOverlay(width, height)
            });
        }
    }
}
