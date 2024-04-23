// @ts-ignore
import {Scroller} from "./scroller";
import {BUTTON_CLASS, BUTTON_TEXT_CONTAINER_CLASS, NATIVE_LEAF_CLASS} from "../utils/constants";
import {AnyCreatePatch} from "./messages/any-create-patch";
import {OcclusionUpdatePatch} from "./messages/occlusion-update-patch";
// @ts-ignore
import snarkdown from 'snarkdown';
import {TextUpdatePatch} from "./messages/text-update-patch";
import {FrameUpdatePatch} from "./messages/frame-update-patch";
import {ScrollerUpdatePatch} from "./messages/scroller-update-patch";
import {ButtonUpdatePatch} from "./messages/button-update-patch";
import {ImageLoadPatch} from "./messages/image-load-patch";
import {OcclusionContext} from "./occlusion-context";
import {ObjectManager} from "../pools/object-manager";
import {INPUT, BUTTON, DIV, OBJECT, OCCLUSION_CONTEXT, SCROLLER} from "../pools/supported-objects";
import {arrayToKey, packAffineCoeffsIntoMatrix3DString, readImageToByteBuffer} from "../utils/helpers";
import {ColorGroup, TextStyle, getAlignItems, getJustifyContent, getTextAlign} from "./text";
import type {PaxChassisWeb} from "../types/pax-chassis-web";
import { CheckboxUpdatePatch } from "./messages/checkbox-update-patch";
import { TextboxUpdatePatch } from "./messages/textbox-update-patch";

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

    build(chassis: PaxChassisWeb, isMobile: boolean, mount: Element){
        this.isMobile = isMobile;
        this.chassis = chassis;
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
                this.chassis!.interrupt(scrollEventStringified, []);
                this.objectManager.returnToPool(OBJECT, deltas);
                this.objectManager.returnToPool(OBJECT, scrollEvent);
            }
        });
    }

    occlusionUpdate(patch: OcclusionUpdatePatch) {
        // @ts-ignore
        let node = this.textNodes[patch.idChain];
        if (node){
            let parent = node.parentElement;
            parent.removeChild(node);
            NativeElementPool.addNativeElement(node, this.baseOcclusionContext,
                // @ts-ignore
                this.scrollers, patch.idChain, undefined, patch.zIndex);
        }
    }

    checkboxCreate(patch: AnyCreatePatch) {
        console.assert(patch.idChain != null);
        console.assert(patch.clippingIds != null);
        console.assert(patch.scrollerIds != null);
        console.assert(patch.zIndex != null);
        
        const checkbox = this.objectManager.getFromPool(INPUT) as HTMLInputElement;
        checkbox.type = "checkbox";
        checkbox.style.margin = "0";
        checkbox.addEventListener("change", (event) => {
            //Reset the checkbox state (state changes only allowed through engine)
            const is_checked = (event.target as HTMLInputElement).checked;
            checkbox.checked = !is_checked;
            
            let message = {
                "FormCheckboxToggle": {
                    "id_chain": patch.idChain!,
                    "state": checkbox.checked,
                }
            }
            this.chassis!.interrupt(JSON.stringify(message), undefined);
        });

        let runningChain: HTMLDivElement = this.objectManager.getFromPool(DIV);
        runningChain.appendChild(checkbox);
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

    
    checkboxUpdate(patch: CheckboxUpdatePatch) {
        //@ts-ignore
        window.textNodes = this.textNodes;
        // @ts-ignore
        let leaf = this.textNodes[patch.id_chain];
        console.assert(leaf !== undefined);
        let checkbox = leaf.firstChild;
        if (patch.checked !== null) {
            checkbox.checked = patch.checked;
        }
        // Handle size_x and size_y
        if (patch.size_x != null) {
            checkbox.style.width = patch.size_x + "px";
        }
        if (patch.size_y != null) {
            checkbox.style.height = patch.size_y + "px";
        }
        // Handle transform
        if (patch.transform != null) {
            leaf.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
        }
    }

    checkboxDelete(id_chain: number[]) {
        // @ts-ignore
        let oldNode = this.textNodes[id_chain];
        if (oldNode){
            let parent = oldNode.parentElement;
            parent.removeChild(oldNode);
        }
    }

    textboxCreate(patch: AnyCreatePatch) {
        const textbox = this.objectManager.getFromPool(INPUT) as HTMLInputElement;
        textbox.type = "text";
        textbox.addEventListener("input", (_event) => {
            let message = {
                "FormTextboxInput": {
                    "id_chain": patch.idChain!,
                    "text": textbox.value,
                }
            }
            this.chassis!.interrupt(JSON.stringify(message), undefined);
        });

        textbox.addEventListener("change", (_event) => {
            let message = {
                "FormTextboxChange": {
                    "id_chain": patch.idChain!,
                    "text": textbox.value,
                }
            }
            this.chassis!.interrupt(JSON.stringify(message), undefined);
        });

        let runningChain: HTMLDivElement = this.objectManager.getFromPool(DIV);
        runningChain.appendChild(textbox);
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

    
    textboxUpdate(patch: TextboxUpdatePatch) {
        //@ts-ignore
        window.textNodes = this.textNodes;
        // @ts-ignore
        let leaf = this.textNodes[patch.id_chain];
        console.assert(leaf !== undefined);
        let textbox = leaf.firstChild;

        applyTextTyle(textbox, textbox, patch.style);

        //We may support styles other than solid in the future; this is a better default than the browser's for now
        textbox.style["border-style"] = "solid";

        if (patch.background) {
            textbox.style.background = toCssColor(patch.background);
        }

        if (patch.border_radius) {
            textbox.style["border-radius"] = patch.border_radius + "px";
        }

        if (patch.stroke_color) {
            textbox.style["border-color"] = toCssColor(patch.stroke_color);
        }

        if (patch.stroke_width) {
            textbox.style["border-width"] = patch.stroke_width + "px";

        }

        // Apply the content
        if (patch.text != null) {

            // Check if the input element is focused — we want to maintain the user's cursor position if so
            if (document.activeElement === textbox) {
                // Get the current selection range
                const selectionStart = textbox.selectionStart || 0;

                // Update the content of the input
                textbox.value = patch.text;

                // Calculate the new cursor position, clamped to the new length of the input value
                const newCursorPosition = Math.min(selectionStart, patch.text.length);

                // Set the cursor position to the beginning of the former selection range
                textbox.setSelectionRange(newCursorPosition, newCursorPosition);
            } else {
                // If the textbox isn't selected, just update its content
                textbox.value = patch.text;
            }


        }
       
        // Handle size_x and size_y
        if (patch.size_x != null) {
            textbox.style.width = patch.size_x - 1 + "px";
        }
        if (patch.size_y != null) {
            textbox.style.height = patch.size_y + "px";
        }
        // Handle transform
        if (patch.transform != null) {
            leaf.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
        }

        if (patch.focus_on_mount) {
            setTimeout(() => { textbox.focus(); }, 10);
        }
    }

    textboxDelete(id_chain: number[]) {
        // @ts-ignore
        let oldNode = this.textNodes[id_chain];
        if (oldNode){
            let parent = oldNode.parentElement;
            parent.removeChild(oldNode);
        }
    }

    buttonCreate(patch: AnyCreatePatch) {
        console.assert(patch.idChain != null);
        console.assert(patch.clippingIds != null);
        console.assert(patch.scrollerIds != null);
        console.assert(patch.zIndex != null);
        
        const button = this.objectManager.getFromPool(BUTTON) as HTMLButtonElement;
        const textContainer = this.objectManager.getFromPool(DIV) as HTMLDivElement;
        const textChild = this.objectManager.getFromPool(DIV) as HTMLDivElement;
        button.setAttribute("class", BUTTON_CLASS);
        textContainer.setAttribute("class", BUTTON_TEXT_CONTAINER_CLASS);
        textChild.style.margin = "0";
        button.addEventListener("click", (_event) => {
            let message = {
                "FormButtonClick": {
                    "id_chain": patch.idChain!,
                }
            }
            this.chassis!.interrupt(JSON.stringify(message), undefined);
        });

        let runningChain: HTMLDivElement = this.objectManager.getFromPool(DIV);
        textContainer.appendChild(textChild);
        button.appendChild(textContainer);
        runningChain.appendChild(button);
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

    
    buttonUpdate(patch: ButtonUpdatePatch) {
        //@ts-ignore
        window.textNodes = this.textNodes;
        // @ts-ignore
        let leaf = this.textNodes[patch.id_chain];
        console.assert(leaf !== undefined);
        let button = leaf.firstChild;
        let textContainer = button.firstChild;
        let textChild = textContainer.firstChild;


        // Apply the content
        if (patch.content != null) {
            // @ts-ignore
            textChild.innerHTML = snarkdown(patch.content);
        }

        
        applyTextTyle(textContainer, textChild, patch.style);

        // Handle size_x and size_y
        if (patch.size_x != null) {
            button.style.width = patch.size_x - 1 + "px";
        }
        if (patch.size_y != null) {
            button.style.height = patch.size_y + "px";
        }
        // Handle transform
        if (patch.transform != null) {
            leaf.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
        }
    }

    buttonDelete(id_chain: number[]) {
        // @ts-ignore
        let oldNode = this.textNodes[id_chain];
        if (oldNode){
            let parent = oldNode.parentElement;
            parent.removeChild(oldNode);
        }
    }

    textCreate(patch: AnyCreatePatch) {
        console.assert(patch.idChain != null);
        console.assert(patch.clippingIds != null);
        console.assert(patch.scrollerIds != null);
        console.assert(patch.zIndex != null);

        let runningChain: HTMLDivElement = this.objectManager.getFromPool(DIV);
        let textChild: HTMLDivElement = this.objectManager.getFromPool(DIV);
        textChild.addEventListener("input", (_event) => {
            let message = {
              "TextInput": {
                "id_chain": patch.idChain!,
                // why all the replaces?
                // see: https://stackoverflow.com/questions/13762863/contenteditable-field-to-maintain-newlines-upon-database-entry
                "text": textChild.innerHTML
                        .replace(/<br\s*\/*>/ig, '\n') 
                        .replace(/(<(p|div))/ig, '\n$1') 
                        .replace(/(<([^>]+)>)/ig, "")?? '',
              }
            };

            this.chassis!.interrupt(JSON.stringify(message), undefined);
        });
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
        textChild.style.userSelect = "none";

        if(patch.idChain != undefined && patch.zIndex != undefined) {
            NativeElementPool.addNativeElement(runningChain, this.baseOcclusionContext,
                this.scrollers, patch.idChain, scroller_id, patch.zIndex);
        }

        // @ts-ignore
        this.textNodes[patch.idChain] = runningChain;
    }

    textUpdate(patch: TextUpdatePatch) {
        //@ts-ignore
        window.textNodes = this.textNodes;
        // @ts-ignore
        let leaf = this.textNodes[patch.id_chain];
        console.assert(leaf !== undefined);

        let textChild = leaf.firstChild;
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

        if (patch.editable != null) {
            textChild.setAttribute("contenteditable", patch.editable.toString());
            if (patch.editable == true) {
                textChild.style.outline = "none";
                // without these the editable div doesn't register
                // clicks in the entire outside div as a request to edit
                textChild.style.width = "inherit";
                textChild.style.height = "inherit";
            }
        }

        applyTextTyle(leaf, textChild, patch.style);

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
                            newValue = `rgba(${p[0]! * 255.0},${p[1]! * 255.0},${p[2]! * 255.0},${p[3]!})`; //note that alpha channel expects [0.0, 1.0] in CSS
                        } else {
                            console.warn("Unsupported Color Format");
                        }
                        link.style.color = newValue;
                    }

                    if (linkStyle.align_horizontal) {
                        leaf.style.display = "flex";
                        leaf.style.justifyContent = getJustifyContent(linkStyle.align_horizontal);
                    }
                    if (linkStyle.font_size) {
                        textChild.style.fontSize = linkStyle.font_size + "px";
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
    }

    textDelete(id_chain: number[]) {
        // @ts-ignore
        let oldNode = this.textNodes[id_chain];
        if (oldNode){
            let parent = oldNode.parentElement;
            parent.removeChild(oldNode);
        }
    }

    frameCreate(_patch: AnyCreatePatch) {
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

    frameUpdate(_patch: FrameUpdatePatch) {
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

    frameDelete(_id_chain: number[]) {
        // NOTE: this should be supported, and may cause a memory leak if left unaddressed;
        //       was likely unplugged during v0 implementation due to some deeper bug that was interfering with 'hello world'

        // let oldNode = this.textNodes.get(id_chain);
        // console.assert(oldNode !== undefined);
        // this.textNodes.delete(id_chain);
        //
        // let nativeLayer = document.querySelector("#" + NATIVE_OVERLAY_CLASS);
        // nativeLayer?.removeChild(oldNode);
    }

    scrollerCreate(patch: AnyCreatePatch){
        window.textNodes = this.textNodes;
        console.assert(patch.idChain != null);
        console.assert(patch.clippingIds != null);
        console.assert(patch.scrollerIds != null);
        console.assert(patch.zIndex != null);

        let runningChain: HTMLDivElement = this.objectManager.getFromPool(DIV);
        let scroller: HTMLDivElement = this.objectManager.getFromPool(DIV);
        runningChain.addEventListener("scroll", (_event) => {
            // TODO send interrupt
            // console.log("scrolling!");
        });

        runningChain.appendChild(scroller);
        runningChain.setAttribute("class", NATIVE_LEAF_CLASS)
        runningChain.style.overflow = "scroll";
        runningChain.setAttribute("id_chain", String(patch.idChain));


        let scroller_id;
        if(patch.scrollerIds != null){
            let length = patch.scrollerIds.length;
            if(length != 0) {
                scroller_id = patch.scrollerIds[length-1];
            }
        }
        if(patch.idChain != undefined && patch.zIndex != undefined) {
            NativeElementPool.addNativeElement(runningChain, this.baseOcclusionContext,
                this.scrollers, patch.idChain, scroller_id, patch.zIndex);
        }
        // @ts-ignore
        this.textNodes[patch.idChain] = runningChain;
    }

    scrollerUpdate(patch: ScrollerUpdatePatch){
        // @ts-ignore
        let leaf = this.textNodes[patch.id_chain];
        console.assert(leaf !== undefined);

        let scroller_inner = leaf.firstChild;
        // Handle size_x and size_y
        if (patch.size_x != null) {
            leaf.style.width = patch.size_x + "px";
        }
        if (patch.size_y != null) {
            leaf.style.height = patch.size_y + "px";
        }
        if (patch.scroll_x != null) {
            leaf.scrollLeft = patch.scroll_x;
        }
        if (patch.scroll_y != null) {
            leaf.scrollTop = patch.scroll_y;
        }
        // Handle transform
        if (patch.transform != null) {
            leaf.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
        }

        if (patch.size_inner_pane_x != null) {
            scroller_inner.style.width = patch.size_inner_pane_x + "px";
        }
        if (patch.size_inner_pane_y != null) {
            scroller_inner.style.height = patch.size_inner_pane_y + "px";
        }

        if (patch.scroll_enabled_x != null) {
           // TODO enable/disable 
        }
        if (patch.scroll_enabled_x != null) {
           // TODO enable/disable 
        }
    }

    scrollerDelete(idChain: number[]){
        // @ts-ignore
        let oldNode = this.textNodes[id_chain];
        if (oldNode){
            let parent = oldNode.parentElement;
            parent.removeChild(oldNode);
        }
    }



    async imageLoad(patch: ImageLoadPatch, chassis: PaxChassisWeb) {

        if (chassis.image_loaded(patch.path)) {
            return
        }
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

        const BASE_PATH = getScriptBasePath('pax-chassis-web-interface.js');

        let path = (BASE_PATH + patch.path!).replace("//", "/");
        let image_data = await readImageToByteBuffer(path!)
        let message = {
            "Image": {
                "Data": {
                    "id_chain": patch.id_chain!,
                    "path": patch.path!,
                    "width": image_data.width,
                    "height": image_data.height,
                }
            }
        }
        chassis.interrupt(JSON.stringify(message), image_data.pixels);
    }

}

function toCssColor(color: ColorGroup): string {
    if (color.Rgba != null) {
        let p = color.Rgba;
        return `rgba(${p[0] * 255},${p[1] * 255},${p[2] * 255},${p[3]})`; //Note that alpha channel expects [0.0, 1.0] in CSS
    } else {
        throw new TypeError("Unsupported Color Format");
    }        
}

function applyTextTyle(textContainer: HTMLDivElement, textElem: HTMLDivElement, style: TextStyle | undefined) {
    
// Apply TextStyle from patch.style
    if (style) {
        if (style.font) {
            style.font.applyFontToDiv(textContainer);
        }
        if (style.fill) {
            textElem.style.color = toCssColor(style.fill);
        }
        if (style.font_size) {
            textElem.style.fontSize = style.font_size + "px";
        }
        if (style.underline != null) {
            textElem.style.textDecoration = style.underline ? 'underline' : 'none';
        }
        if (style.align_horizontal) {
            textContainer.style.display = "flex";
            textContainer.style.justifyContent = getJustifyContent(style.align_horizontal);
        }
        if (style.align_vertical) {
            textContainer.style.alignItems = getAlignItems(style.align_vertical);
        }
        if (style.align_multiline) {
            textElem.style.textAlign = getTextAlign(style.align_multiline);
        }
    }
}


