// @ts-ignore
import {Layer} from "./layer";
import {ObjectManager} from "../pools/object-manager";
import {ARRAY, LAYER} from "../pools/supported-objects";

import type {PaxChassisWeb} from "../types/pax-chassis-web";
import { applyStyles} from "../utils/helpers";

class ContainerData {
    parentFrame: number;
    styles: Partial<CSSStyleDeclaration>;

    constructor(parentId: number, styles: Partial<CSSStyleDeclaration>) {
        this.parentFrame = parentId;
        this.styles = styles;
    }
}

export class OcclusionLayerManager {
    private layers?: Layer[];
    private canvasMap?: Map<string, HTMLCanvasElement>;
    private parent?: Element;
    private objectManager: ObjectManager;
    private chassis?: PaxChassisWeb;
    /// Frame id to id to data. Whenever a new frame is added, this struct is
    /// updated. Whenever a node that references a id in this frame is added, a
    /// chain of divs are created to correspond to this frames location
    /// if size info assosiated with a frame changes. ie if there is a colliding
    private containers: Map<number, ContainerData>;

    constructor(objectManager: ObjectManager) {
        this.objectManager = objectManager;
        this.containers = new Map();
    }

    attach(parent: Element, chassis: PaxChassisWeb, canvasMap: Map<string, HTMLCanvasElement>) {
        this.layers = this.objectManager.getFromPool(ARRAY);
        this.parent = parent;
        this.chassis = chassis;
        this.canvasMap = canvasMap;
        this.growTo(0);
    }

    growTo(newOcclusionLayerId: number) {
        let occlusionLayerId = newOcclusionLayerId + 1;
        if(this.layers!.length <= occlusionLayerId){
            for(let i = this.layers!.length; i < occlusionLayerId; i++) {
                let newLayer: Layer = this.objectManager.getFromPool(LAYER, this.objectManager);
                newLayer.build(this.parent!, i, this.chassis!, this.canvasMap!);
                this.layers!.push(newLayer);
            }
        }
    }

    shrinkTo(occlusionLayerId: number){
        if(this.layers == undefined){
            return
        }
        if(this.layers.length >= occlusionLayerId) {
            for(let i = this.layers!.length - 1; i > occlusionLayerId; i--){
                this.objectManager.returnToPool(LAYER, this.layers[i]);
                this.layers.pop();
            }
        }
    }

    addElement(element: HTMLElement, parent_container: number | undefined, occlusionLayerId: number){
        this.growTo(occlusionLayerId);
        // TODO add the element to the correct location if parent_container is != null
        let attach_point = this.getOrCreateContainer(parent_container, occlusionLayerId);
        attach_point.appendChild(element);
    }

    getOrCreateContainer(id: number | undefined, occlusionLayerId: number) {
        let layer = this.layers![occlusionLayerId]!.native!;
        if (id === undefined) {
            return layer;
        }

        // see if there already is a dom node corresponding to this container in this layer
        let elem = layer.querySelector(`[data-container-id="${id}"]`);
        if (elem !== undefined) {
            return elem!;
        }

        //ok doesn't seem to exist, we need to create it
        let data = this.containers.get(id)!;
        let new_container = document.createElement("div");
        new_container.dataset.containerId = id.toString();
        applyStyles(new_container, data.styles);

        let parent_container = this.getOrCreateContainer(data.parentFrame, occlusionLayerId);
        parent_container.appendChild(new_container);

        return new_container
    }

    addContainer(id: number, parentId: number, style: Partial<CSSStyleDeclaration>) {
        this.containers.set(id, new ContainerData(parentId, style));
    }

    updateContainer(id: number, styles: Partial<CSSStyleDeclaration>) {
        let container = this.containers.get(id);
        container!.styles = styles;
        // update underlying data used to create new dom trees for a container
        // update already existing instances of this container
        let existing_layer_instantiations = document.querySelectorAll(`[data-container-id="${id}"]`);
        existing_layer_instantiations.forEach((elem, _key, _parent) => {
            applyStyles(elem as HTMLElement, styles);
        })
    }

    removeContainer(_id: number) {
        throw new Error("TODO");
    }

    cleanUp(){
        if(this.layers != undefined){
            this.layers.forEach((layer) => {
                this.objectManager.returnToPool(LAYER, layer);
            });
        }
        this.canvasMap = undefined;
        this.parent = undefined;
    }
}
