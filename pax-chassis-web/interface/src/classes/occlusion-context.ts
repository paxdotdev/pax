// @ts-ignore
import {Layer} from "./layer";
import {ObjectManager} from "../pools/object-manager";
import {ARRAY, DIV, LAYER} from "../pools/supported-objects";

import type {PaxChassisWeb} from "../types/pax-chassis-web";
import { NATIVE_LEAF_CLASS, CLIPPING_CONTAINER } from "../utils/constants";
import { getQuadClipPolygonCommand } from "../utils/helpers";

class ContainerData {
    parentFrame: number | undefined;
    styles: ContainerStyle;

    constructor(parentId: number | undefined) {
        this.parentFrame = parentId;
        this.styles = new ContainerStyle();
    }
}

export class ContainerStyle {
    transform: number[];
    width: number;
    height: number;

    constructor() {
        this.transform = [0, 0, 0, 0, 0, 0];
        this.width = 0;
        this.height = 0;
    }
}

export function setClippingPath(element: HTMLElement, styles: ContainerStyle) {
    let polygonDef = getQuadClipPolygonCommand(styles.width!, styles.height!, styles.transform!)
    element.style.clipPath = polygonDef;
    //@ts-ignore
    element.style.webkitClipPath = polygonDef;
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
            for(let i = this.layers!.length; i <= occlusionLayerId; i++) {
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
        let attach_point = this.getOrCreateContainer(parent_container, occlusionLayerId);
        attach_point.appendChild(element);
    }

    getOrCreateContainer(id: number | undefined, occlusionLayerId: number) {
        let layer = this.layers![occlusionLayerId]!.native!;
        if (id == undefined) {
            return layer;
        }

        // see if there already is a dom node corresponding to this container in this layer
        let elem = layer.querySelector(`[data-container-id="${id}"]`);
        if (elem != undefined) {
            return elem!;
        }

        //ok doesn't seem to exist, we need to create it
        let data = this.containers.get(id)!;
        let new_container: HTMLDivElement = this.objectManager.getFromPool(DIV);
        new_container.setAttribute("class", CLIPPING_CONTAINER)
        new_container.dataset.containerId = id.toString();
        setClippingPath(new_container, data.styles);

        let parent_container = this.getOrCreateContainer(data.parentFrame, occlusionLayerId);
        parent_container.appendChild(new_container);
        return new_container;
    }

    addContainer(id: number, parentId: number | undefined) {
        this.containers.set(id, new ContainerData(parentId));
    }

    updateContainer(id: number, styles: Partial<ContainerStyle>) {
        let container = this.containers.get(id);
        if (container == null) {
            throw new Error("tried to update non existent container");
        }
        container.styles = {...container.styles, ...styles};
        // update underlying data used to create new dom trees for a container
        // update already existing instances of this container
        let existing_layer_instantiations = document.querySelectorAll(`[data-container-id="${id}"]`);
        existing_layer_instantiations.forEach((elem, _key, _parent) => {
            setClippingPath(elem as HTMLElement, container!.styles);
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
