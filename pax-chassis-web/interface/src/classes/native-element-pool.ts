// @ts-ignore
import {PaxChassisWeb} from '../dist/pax_chassis_web';import {Scroller} from "./scroller";
import {MOUNT_ID, NATIVE_LEAF_CLASS} from "../utils/constants";
import {AnyCreatePatch} from "./messages/any-create-patch";
// @ts-ignore
import snarkdown from 'snarkdown';
import {TextUpdatePatch} from "./messages/text-update-patch";
import {FrameUpdatePatch} from "./messages/frame-update-patch";
import {ScrollerUpdatePatch} from "./messages/scroller-update-patch";
import {ImageLoadPatch} from "./messages/image-load-patch";
import {OcclusionContext} from "./occlusion-context";
import {ObjectManager} from "../pools/object-manager";
import {DIV, OBJECT, OCCLUSION_CONTEXT, SCROLLER} from "../pools/supported-objects";
import {arrayToKey, packAffineCoeffsIntoMatrix3DString, readImageToByteBuffer} from "../utils/helpers";
import {getAlignItems, getJustifyContent, getTextAlign} from "./text";

export class NativeElementPool {
    private canvases: Map<string, HTMLCanvasElement>;
    private scrollers: Map<string, Scroller>;
    baseOcclusionContext: OcclusionContext;
    private textNodes = {};
    private chassis? : PaxChassisWeb;
    private objectManager: ObjectManager;
    registeredFontFaces: Set<string>;
    messageList:string[] = [];
    private isMobile = false;

    constructor(objectManager: ObjectManager) {
        this.objectManager = objectManager;
        this.canvases = new Map();
        this.scrollers = new Map();
        this.baseOcclusionContext = objectManager.getFromPool(OCCLUSION_CONTEXT, objectManager);
        this.registeredFontFaces = new Set<string>();
    }

    build(chassis: PaxChassisWeb, isMobile: boolean){
        this.isMobile = isMobile;
        this.chassis = chassis;
        let mount = document.querySelector("#" + MOUNT_ID)!;
        this.baseOcclusionContext.build(mount, undefined, chassis, this.canvases);
    }

    static addNativeElement(elem: HTMLElement, baseOcclusionContext: OcclusionContext, scrollers: Map<string, Scroller>,
                                     idChain: number[] , scrollerIdChain: number[] | undefined, zIndex: number){
        if(scrollerIdChain != undefined){
            let scroller: Scroller = scrollers.get(arrayToKey(scrollerIdChain))!;
            scroller.addElement(elem, zIndex);
        } else {
            baseOcclusionContext.addElement(elem, zIndex);
        }
    }

    clearCanvases(): void {
        this.canvases.forEach((canvas, key) => {
            let dpr = window.devicePixelRatio;
            const context = canvas.getContext('2d');
            if (context) {
                context.clearRect(0, 0, canvas.width, canvas.height);
            }
            if(canvas.width != (canvas.clientWidth * dpr) || canvas.height != (canvas.clientHeight * dpr)){
                canvas.width = (canvas.clientWidth * dpr);
                canvas.height = (canvas.clientHeight * dpr);
                if (context) {
                    context.scale(dpr, dpr);
                }
            }
        });
    }

    sendScrollerValues(){
        this.scrollers.forEach((scroller, id) => {
            // @ts-ignore
            let deltaX = 0;
            // @ts-ignore
            let deltaY = scroller.getTickScrollDelta();
            if(deltaY && Math.abs(deltaY) > 0){
                const scrollEvent = this.objectManager.getFromPool(OBJECT);
                const deltas: object = this.objectManager.getFromPool(OBJECT);
                // @ts-ignore
                deltas['delta_x'] = deltaX;
                // @ts-ignore
                deltas['delta_y'] = deltaY;
                // @ts-ignore
                scrollEvent.Scroll = deltas;
                const scrollEventStringified = JSON.stringify(scrollEvent);
                this.messageList.push(scrollEventStringified);
                this.chassis.interrupt(scrollEventStringified, []);
                this.objectManager.returnToPool(OBJECT, deltas);
                this.objectManager.returnToPool(OBJECT, scrollEvent);
            }
        });
    }

    textCreate(patch: AnyCreatePatch) {
        console.assert(patch.idChain != null);
        console.assert(patch.clippingIds != null);
        console.assert(patch.scrollerIds != null);
        console.assert(patch.zIndex != null);

        let runningChain: HTMLDivElement = this.objectManager.getFromPool(DIV);
        let textChild: HTMLDivElement = this.objectManager.getFromPool(DIV);
        runningChain.appendChild(textChild);
        runningChain.setAttribute("class", NATIVE_LEAF_CLASS)
        runningChain.setAttribute("id_chain", String(patch.idChain));

        let scroller_id;
        if(patch.scrollerIds != null){
            let length = patch.scrollerIds.length;
            if(length != 0) {
                scroller_id = patch.scrollerIds[length-1];
            }
        }

        if(patch.idChain != undefined && patch.zIndex != undefined){
            NativeElementPool.addNativeElement(runningChain, this.baseOcclusionContext,
                this.scrollers, patch.idChain, scroller_id, patch.zIndex);
        }

        // @ts-ignore
        this.textNodes[patch.idChain] = runningChain;
    }

    textUpdate(patch: TextUpdatePatch) {

        //console.log("Msg received", patch.id_chain, patch.content);
        //@ts-ignore
        window.textNodes = this.textNodes;
        // @ts-ignore
        let leaf = this.textNodes[patch.id_chain];
        console.assert(leaf !== undefined);

        let textChild = leaf.firstChild;

        // Apply TextStyle from patch.style
        if (patch.style) {
            const style = patch.style;
            if (style.font) {
                style.font.applyFontToDiv(leaf);
            }
            if (style.fill) {
                let newValue = "";
                if(style.fill.Rgba != null) {
                    let p = style.fill.Rgba;
                    newValue = `rgba(${p[0]! * 255.0},${p[1]! * 255.0},${p[2]! * 255.0},${p[3]! * 255.0})`;
                } else {
                    let p = style.fill.Hsla!;
                    newValue = `hsla(${p[0]! * 255.0},${p[1]! * 255.0},${p[2]! * 255.0},${p[3]! * 255.0})`;
                }
                textChild.style.color = newValue;
            }
            if (style.font_size) {
                textChild.style.fontSize = style.font_size + "px";
            }
            if (style.underline != null) {
                textChild.style.textDecoration = style.underline ? 'underline' : 'none';
            }
            if (style.align_horizontal) {
                leaf.style.display = "flex";
                leaf.style.justifyContent = getJustifyContent(style.align_horizontal);
            }
            if (style.align_vertical) {
                leaf.style.alignItems = getAlignItems(style.align_vertical);
            }
            if (style.align_multiline) {
                textChild.style.textAlign = getTextAlign(style.align_multiline);
            }
        }

        // Apply the content
        if (patch.content != null) {
            // @ts-ignore
            textChild.innerHTML = snarkdown(patch.content);

            // Apply the link styles if they exist
            if (patch.style_link) {
                let linkStyle = patch.style_link;
                const links = textChild.querySelectorAll('a');
                links.forEach((link: HTMLDivElement) => {
                    if (linkStyle.font) {
                        linkStyle.font.applyFontToDiv(link);
                    }
                    if (linkStyle.fill) {
                        let newValue = "";
                        if(linkStyle.fill.Rgba != null) {
                            let p = linkStyle.fill.Rgba;
                            newValue = `rgba(${p[0]! * 255.0},${p[1]! * 255.0},${p[2]! * 255.0},${p[3]! * 255.0})`;
                        } else {
                            let p = linkStyle.fill.Hsla!;
                            newValue = `hsla(${p[0]! * 255.0},${p[1]! * 255.0},${p[2]! * 255.0},${p[3]! * 255.0})`;
                        }
                        link.style.color = newValue;
                    }

                    if (linkStyle.align_horizontal) {
                        leaf.style.display = "flex";
                        leaf.style.justifyContent = getJustifyContent(linkStyle.align_horizontal);
                    }
                    if (linkStyle.align_vertical) {
                        leaf.style.alignItems = getAlignItems(linkStyle.align_vertical);
                    }
                    if (linkStyle.align_multiline) {
                        textChild.style.textAlign = getTextAlign(linkStyle.align_multiline);
                    }
                    if (linkStyle.underline != null) {
                        link.style.textDecoration = linkStyle.underline ? 'underline' : 'none';
                    }
                });
            }
        }

        // Handle size_x and size_y
        if (patch.size_x != null) {
            leaf.style.width = patch.size_x + "px";
        }
        if (patch.size_y != null) {
            leaf.style.height = patch.size_y + "px";
        }

        // Handle transform
        if (patch.transform != null) {
            leaf.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
        }
    }

    textDelete(id_chain: number[]) {
        // @ts-ignore
        let oldNode = this.textNodes[id_chain];
        if (oldNode){
            let parent = oldNode.parentElement;
            parent.removeChild(oldNode);
        }
    }

    frameCreate(patch: AnyCreatePatch) {
        // console.assert(patch.idChain != null);
        // console.assert(this.clippingNodes["id_chain"] === undefined);
        //
        // let attachPoint = getAttachPointFromClippingIds(patch.clippingIds);
        //
        // let newClip = document.createElement("div");
        // newClip.id = getStringIdFromClippingId("clip", patch.idChain);
        // newClip.classList.add(NATIVE_CLIPPING_CLASS);

        //attachPoint!.appendChild(newClip);
    }

    frameUpdate(patch: FrameUpdatePatch) {
        //@ts-ignore
        // let cacheContainer : FrameUpdatePatch = this.clippingValueCache[patch.id_chain] || new FrameUpdatePatch();
        //
        // let shouldRedraw = false;
        // if (patch.size_x != null) {
        //     shouldRedraw = true;
        //     cacheContainer.size_x = patch.size_x
        // }
        // if (patch.size_y != null) {
        //     shouldRedraw = true;
        //     cacheContainer.size_y = patch.size_y
        // }
        // if (patch.transform != null) {
        //     shouldRedraw = true;
        //     cacheContainer.transform = patch.transform;
        // }
        //
        // if (shouldRedraw) {
        //     let node : HTMLElement = document.querySelector("#" + getStringIdFromClippingId(CLIP_PREFIX, patch.id_chain!))!
        //
        //     // Fallback and/or perf optimizer: `polygon` instead of `path`.
        //     let polygonDef = getQuadClipPolygonCommand(cacheContainer.size_x!, cacheContainer.size_y!, cacheContainer.transform!)
        //     node.style.clipPath = polygonDef;
        //     //@ts-ignore
        //     node.style.webkitClipPath = polygonDef;
        //
        //     // PoC arbitrary path clipping (noticeably poorer perf in Firefox at time of authoring)
        //     // let pathDef = getQuadClipPathCommand(cacheContainer.size_x!, cacheContainer.size_y!, cacheContainer.transform!)
        //     // node.style.clipPath = pathDef;
        //     // //@ts-ignore
        //     // node.style.webkitClipPath = pathDef;
        // }
        // //@ts-ignore
        // this.clippingValueCache[patch.id_chain] = cacheContainer;
    }

    frameDelete(id_chain: number[]) {
        // NOTE: this should be supported, and may cause a memory leak if left unaddressed;
        //       was likely unplugged during v0 implementation due to some deeper bug that was interfering with 'hello world'

        // let oldNode = this.textNodes.get(id_chain);
        // console.assert(oldNode !== undefined);
        // this.textNodes.delete(id_chain);
        //
        // let nativeLayer = document.querySelector("#" + NATIVE_OVERLAY_CLASS);
        // nativeLayer?.removeChild(oldNode);
    }

    scrollerCreate(patch: AnyCreatePatch, chassis: PaxChassisWeb){
        //console.log(patch);
        let scroller_id;
        if(patch.scrollerIds != null){
            let length = patch.scrollerIds.length;
            if(length != 0) {
                scroller_id = patch.scrollerIds[length-1];
            }
        }
        let scroller: Scroller = this.objectManager.getFromPool(SCROLLER, this.objectManager);
        scroller.build(patch.idChain!, patch.zIndex!, scroller_id, this.chassis, this.scrollers,
            this.baseOcclusionContext, this.canvases, this.isMobile)
        // @ts-ignore
        this.scrollers.set(arrayToKey(patch.idChain),scroller);
    }

    scrollerUpdate(patch: ScrollerUpdatePatch){
            this.scrollers.get(arrayToKey(patch.idChain!))!.handleScrollerUpdate(patch);
    }

    scrollerDelete(idChain: number[]){
        if(this.scrollers.has(arrayToKey(idChain))){
            this.objectManager.returnToPool(SCROLLER, this.scrollers.get(arrayToKey(idChain)));
            this.scrollers.delete(arrayToKey(idChain));
        }
    }



    async imageLoad(patch: ImageLoadPatch, chassis: PaxChassisWeb) {

        //Check the full path of our index.js; use the prefix of this path also for our image assets
        function getScriptBasePath(scriptName: string) {
            const scripts = document.getElementsByTagName('script');
            for (let i = 0; i < scripts.length; i++) {
                if (scripts[i].src.indexOf(scriptName) > -1) {
                    // Extract path after the domain and port.
                    const path = new URL(scripts[i].src).pathname;
                    return path.replace(scriptName, '');
                }
            }
            return '/';
        }

        const BASE_PATH = getScriptBasePath('index.js');

        let path = (BASE_PATH + patch.path!).replace("//", "/");
        let image_data = await readImageToByteBuffer(path!)
        let message = {
            "Image": {
                "Data": {
                    "id_chain": patch.id_chain!,
                    "width": image_data.width,
                    "height": image_data.height,
                }
            }
        }
        chassis.interrupt(JSON.stringify(message), image_data.pixels);
    }

}