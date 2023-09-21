// @ts-ignore
import {PaxChassisWeb} from '../../dist/pax_chassis_web';
import {OcclusionContext} from "./occlusion-context";
import {INNER_PANE, SCROLLER_CONTAINER} from "../utils/constants";
import {ObjectManager} from "../pools/object-manager";
import {DIV, OBJECT, OCCLUSION_CONTEXT} from "../pools/supported-objects";
import {packAffineCoeffsIntoMatrix3DString} from "../utils/helpers";
import {ScrollerUpdatePatch} from "./messages/scroller-update-patch";
import {NativeElementPool} from "./native-element-pool";
import { ScrollManager } from './scroll-manager';

export class Scroller {
    private idChain?: number[];
    private parentScrollerId?:  number[] | undefined;
    private zIndex ?: number;
    container?: HTMLDivElement;
    private innerPane?: HTMLDivElement;
    private occlusionContext?: OcclusionContext;
    private sizeX?: number;
    private sizeY?: number;
    private sizeInnerPaneX?: number;
    private sizeInnerPaneY?: number;
    private transform?: number[];
    private scrollX?: boolean;
    private scrollY?: boolean;
    scrollOffsetX?: number;
    scrollOffsetY?: number;
    unsentX = 0;
    unsentY= 0;
    private subtreeDepth?: number;
    private objectManager: ObjectManager;
    private scrollManager?: ScrollManager;
    private isMobile = false;


    constructor(objectManager: ObjectManager) {
        this.objectManager = objectManager;
    }

    build(idChain: number[], zIndex: number, scrollerId: number[] | undefined, chassis: PaxChassisWeb,
          scrollers: Map<string, Scroller>, baseOcclusionContext: OcclusionContext, canvasMap: Map<string, HTMLCanvasElement>, isMobile: boolean) {
        this.isMobile = isMobile;
        this.idChain = idChain;
        this.parentScrollerId = scrollerId;
        this.zIndex = zIndex;
        this.scrollOffsetX = 0;
        this.scrollOffsetY = 0;
        this.sizeX = 0;
        this.sizeY = 0;
        this.sizeInnerPaneX = 0;
        this.sizeInnerPaneY = 0;

        this.container = this.objectManager.getFromPool(DIV);
        this.container.className = SCROLLER_CONTAINER;
        NativeElementPool.addNativeElement(this.container, baseOcclusionContext, scrollers, idChain, scrollerId, zIndex);
        this.scrollManager = new ScrollManager(this.container, isMobile);

        this.innerPane = this.objectManager.getFromPool(DIV);
        this.innerPane.className = INNER_PANE;
        this.container.appendChild(this.innerPane);

        this.occlusionContext = this.objectManager.getFromPool(OCCLUSION_CONTEXT, this.objectManager);
        this.occlusionContext.build(this.container, idChain, chassis, canvasMap);
    }

    getTickScrollDelta(){
        return this.scrollManager?.getScrollDelta();
    }

    cleanUp(){
        if(this.occlusionContext != undefined){
            this.occlusionContext.cleanUp();
            this.occlusionContext = undefined;
        }
        if(this.innerPane != undefined){
            this.objectManager.returnToPool(DIV, this.innerPane);
            this.innerPane = undefined;
        }
        if(this.container != undefined){
            let parent = this.container.parentElement;
            parent?.removeChild(this.container);
            this.objectManager.returnToPool(DIV, this.container);
            this.container = undefined;
        }

        this.idChain = undefined;
        this.parentScrollerId = undefined;
        this.zIndex = undefined;
        this.sizeX = undefined;
        this.sizeY = undefined;
        this.sizeInnerPaneX = undefined;
        this.sizeInnerPaneY = undefined;
        this.transform = undefined;
        this.scrollX = undefined;
        this.scrollY = undefined;
        this.scrollOffsetX = undefined;
        this.scrollOffsetY = undefined;
        this.subtreeDepth = undefined;
    }

    handleScrollerUpdate(msg: ScrollerUpdatePatch){
        if(this.container != undefined && this.occlusionContext != undefined
            && this.innerPane != undefined){
            if(msg.sizeX != null){
                this.sizeX = msg.sizeX;
                this.container.style.width = msg.sizeX + "px";
            }
            if(msg.sizeY != null){
                this.sizeY = msg.sizeY;
                this.container.style.height = msg.sizeY + "px";
            }
            if(msg.sizeInnerPaneX != null){
                this.sizeInnerPaneX = msg.sizeInnerPaneX;
            }
            if(msg.sizeInnerPaneY != null){
                this.sizeInnerPaneY = msg.sizeInnerPaneY;
            }
            if(msg.scrollX != null){
                this.scrollX = msg.scrollX;
                if(!msg.scrollX){
                    this.container.style.overflowX = "hidden";
                }
            }
            if(msg.scrollY != null){
                this.scrollY = msg.scrollY;
                if(!msg.scrollY){
                    this.container.style.overflowY = "hidden";
                }
            }
            if(msg.subtreeDepth != null){
                this.subtreeDepth = msg.subtreeDepth;
                this.occlusionContext.shrinkTo(msg.subtreeDepth);
            }
            if(msg.transform != null){
                this.container.style.transform = packAffineCoeffsIntoMatrix3DString(msg.transform);
                this.transform = msg.transform;
            }
            if(msg.sizeX != null || msg.sizeY != null){
                // @ts-ignore
                this.occlusionContext.updateCanvases(this.sizeX, this.sizeY);
                // @ts-ignore
                this.occlusionContext.updateNativeOverlays(this.sizeX, this.sizeY);
            }
            if(msg.sizeInnerPaneX != null || msg.sizeInnerPaneY != null){
                // @ts-ignore
                this.innerPane.style.width = String(this.sizeInnerPaneX)+'px';
                // @ts-ignore
                this.innerPane.style.height = String(this.sizeInnerPaneY)+'px';
            }

        }
    }
    addElement(elem: HTMLElement, zIndex: number){
        if(this.occlusionContext != undefined){
            this.occlusionContext.addElement(elem, zIndex);
        }
    }
}