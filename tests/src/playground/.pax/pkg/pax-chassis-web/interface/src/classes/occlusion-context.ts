import {Layer} from "./layer";
import {ObjectManager} from "../pools/object-manager";
import {ARRAY, DIV, LAYER} from "../pools/supported-objects";

import type {PaxChassisWeb} from "../types/pax-chassis-web";
import { CLIPPING_CONTAINER } from "../utils/constants";
import { getQuadClipPolygonCommand } from "../utils/helpers";

export class OcclusionLayerManager {
    private layers?: Layer[];
    private canvasMap?: Map<string, HTMLCanvasElement>;
    private parent?: Element;
    private objectManager: ObjectManager;
    private chassis?: PaxChassisWeb;
    private containers: Map<number, Container>;

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

    // If a div for the container referenced already exists, returns it. if not,
    // create it (and all non-existent parents)
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
        let container = this.containers.get(id);
        if (container == null) {
            throw new Error("something referenced a container that doesn't exist");
        }
        let new_container: HTMLDivElement = this.objectManager.getFromPool(DIV);
        new_container.dataset.containerId = id.toString();

        // set styling, includes referencing the variable especially created for this container
        // (a container might need to exist on multiple native layers - variable results in less dom updates needed)
        new_container.setAttribute("class", CLIPPING_CONTAINER)
        let var_val = `var(${containerCssClipPathVar(id)})`;
        new_container.style.clipPath = var_val;
        (new_container.style as any).webkitClipPath = var_val;


        let parent_container = this.getOrCreateContainer(container.parentFrame, occlusionLayerId);
        parent_container.appendChild(new_container);
        return new_container;
    }

    addContainer(id: number, parentId: number | undefined) {
        this.containers.set(id, new Container(id, parentId));
    }

    updateContainer(id: number, styles: Partial<ContainerStyle>) {
        let container = this.containers.get(id);
        if (container == null) {
            throw new Error("tried to update non existent container");
        }
        container.updateClippingPath(styles);
    }

    removeContainer(id: number) {
        let container = this.containers.get(id);
        if (container == null) {
            throw new Error(`tried to delete non-existent container with id ${id}`);
        }
        this.containers.delete(id);

        let existing_layer_instantiations = document.querySelectorAll(`[data-container-id="${id}"]`);
        existing_layer_instantiations.forEach((elem, _key, _parent) => {
            let parent = elem.parentElement;
            if (elem.children.length > 0) {
                throw new Error(`tried to remove container width id ${id} while children still present`);
            }
            parent!.removeChild(elem);
        })
        let var_name = containerCssClipPathVar(id);
        document.documentElement.style.removeProperty(var_name);
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

class Container {
    id: number;
    parentFrame: number | undefined;
    styles: ContainerStyle;

    constructor(id: number, parentId: number | undefined) {
        this.parentFrame = parentId;
        this.styles = new ContainerStyle();
        this.id = id;
    }

    updateClippingPath(patch: Partial<ContainerStyle>) {
        this.styles = {...this.styles, ...patch};
        let polygonDef = getQuadClipPolygonCommand(this.styles.width!, this.styles.height!, this.styles.transform!)
        // element.style.clipPath = polygonDef;
        // element.style.webkitClipPath = polygonDef;
        let var_name = containerCssClipPathVar(this.id);
        document.documentElement.style.setProperty(var_name, polygonDef);
    }
}

function containerCssClipPathVar(id: number) {
    return `--container-${id}-clip-path`;
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
