import {BUTTON_CLASS, BUTTON_TEXT_CONTAINER_CLASS, NATIVE_LEAF_CLASS} from "../utils/constants";
import {AnyCreatePatch} from "./messages/any-create-patch";
import {OcclusionUpdatePatch} from "./messages/occlusion-update-patch";
import snarkdown from 'snarkdown';
import {TextUpdatePatch} from "./messages/text-update-patch";
import {FrameUpdatePatch} from "./messages/frame-update-patch";
import {ScrollerUpdatePatch} from "./messages/scroller-update-patch";
import {ButtonUpdatePatch} from "./messages/button-update-patch";
import {ImageLoadPatch} from "./messages/image-load-patch";
import {ContainerStyle, OcclusionLayerManager} from "./occlusion-context";
import {ObjectManager} from "../pools/object-manager";
import {INPUT, BUTTON, DIV, OCCLUSION_CONTEXT} from "../pools/supported-objects";
import {packAffineCoeffsIntoMatrix3DString, readImageToByteBuffer} from "../utils/helpers";
import {ColorGroup, TextStyle, getAlignItems, getJustifyContent, getTextAlign} from "./text";
import type {PaxChassisWeb} from "../types/pax-chassis-web";
import { CheckboxUpdatePatch } from "./messages/checkbox-update-patch";
import { TextboxUpdatePatch } from "./messages/textbox-update-patch";

export class NativeElementPool {
    private canvases: Map<string, HTMLCanvasElement>;
    layers: OcclusionLayerManager;
    private nodesLookup = new Map<number, HTMLElement>();
    private chassis?: PaxChassisWeb;
    private objectManager: ObjectManager;
    private resizeObserver: ResizeObserver;
    registeredFontFaces: Set<string>;

    constructor(objectManager: ObjectManager) {
        this.objectManager = objectManager;
        this.canvases = new Map();
        this.layers = objectManager.getFromPool(OCCLUSION_CONTEXT, objectManager);
        this.registeredFontFaces = new Set<string>();
        this.resizeObserver = new ResizeObserver(entries => {
            let resize_requests = [];
            for (const entry of entries) {
                let node = entry.target as HTMLElement;
                let id = parseInt(node.getAttribute("pax_id")!);
                let width = entry.contentRect.width;
                let height = entry.contentRect.height;
                let message ={
                    "id": id,
                    "width": width,
                    "height": height,
                }
                resize_requests.push(message);
            }
            this.chassis!.interrupt!(JSON.stringify({
                "ChassisResizeRequestCollection": resize_requests,
            }), undefined);
        });
    }

    attach(chassis: PaxChassisWeb, mount: Element){
        this.chassis = chassis;
        this.layers.attach(mount, chassis, this.canvases);
    }

    clearCanvases(): void {
        this.canvases.forEach((canvas, _key) => {
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

    occlusionUpdate(patch: OcclusionUpdatePatch) {
        let node: HTMLElement = this.nodesLookup.get(patch.id!)!;
        if (node){
            let parent = node.parentElement;
            let id_str = parent?.dataset.containerId;
            let id;
            if (id_str !== undefined) {
                id = parseInt(id_str!);
            } else {
                id = undefined;
            }
            this.layers.addElement(node, id, patch.occlusionLayerId!);
        }
    }

    checkboxCreate(patch: AnyCreatePatch) {
        console.assert(patch.id != null);
        console.assert(patch.occlusionLayerId != null);
        
        const checkbox = this.objectManager.getFromPool(INPUT) as HTMLInputElement;
        checkbox.type = "checkbox";
        checkbox.style.margin = "0";
        checkbox.addEventListener("change", (event) => {
            //Reset the checkbox state (state changes only allowed through engine)
            const is_checked = (event.target as HTMLInputElement).checked;
            checkbox.checked = !is_checked;
            
            let message = {
                "FormCheckboxToggle": {
                    "id": patch.id,
                    "state": checkbox.checked,
                }
            }
            this.chassis!.interrupt(JSON.stringify(message), undefined);
        });

        let checkbox_div: HTMLDivElement = this.objectManager.getFromPool(DIV);
        checkbox_div.appendChild(checkbox);
        checkbox_div.setAttribute("class", NATIVE_LEAF_CLASS)
        checkbox_div.setAttribute("pax_id", String(patch.id));
        if(patch.id != undefined && patch.occlusionLayerId != undefined){
            this.layers.addElement(checkbox_div, patch.parentFrame, patch.occlusionLayerId);
        }
        this.nodesLookup.set(patch.id!, checkbox_div);
    }

    
    checkboxUpdate(patch: CheckboxUpdatePatch) {
        let leaf = this.nodesLookup.get(patch.id!);
        let checkbox = leaf!.firstChild as HTMLInputElement;
        if (patch.checked !== null) {
            checkbox!.checked = patch.checked!;
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
            leaf!.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
        }
    }

    checkboxDelete(id: number) {
        let oldNode = this.nodesLookup.get(id);
        if (oldNode){
            let parent = oldNode.parentElement;
            parent!.removeChild(oldNode);
            this.nodesLookup.delete(id);
        }
    }

    textboxCreate(patch: AnyCreatePatch) {
        const textbox = this.objectManager.getFromPool(INPUT) as HTMLInputElement;
        textbox.type = "text";
        textbox.addEventListener("input", (_event) => {
            let message = {
                "FormTextboxInput": {
                    "id": patch.id!,
                    "text": textbox.value,
                }
            }
            this.chassis!.interrupt(JSON.stringify(message), undefined);
        });

        textbox.addEventListener("change", (_event) => {
            let message = {
                "FormTextboxChange": {
                    "id": patch.id!,
                    "text": textbox.value,
                }
            }
            this.chassis!.interrupt(JSON.stringify(message), undefined);
        });

        let textboxDiv: HTMLDivElement = this.objectManager.getFromPool(DIV);
        textboxDiv.appendChild(textbox);
        textboxDiv.setAttribute("class", NATIVE_LEAF_CLASS)
        textboxDiv.setAttribute("pax_id", String(patch.id));

        if(patch.id != undefined && patch.occlusionLayerId != undefined){
            this.layers.addElement(textboxDiv, patch.parentFrame, patch.occlusionLayerId);
            this.nodesLookup.set(patch.id!, textboxDiv);
        } else {
            throw new Error("undefined id or occlusionLayer");
        }

    }

    
    textboxUpdate(patch: TextboxUpdatePatch) {
        let leaf = this.nodesLookup.get(patch.id!);
        let textbox = leaf!.firstChild as HTMLTextAreaElement;

        applyTextTyle(textbox, textbox, patch.style);

        //We may support styles other than solid in the future; this is a better default than the browser's for now
        textbox.style.borderStyle = "solid";

        if (patch.background) {
            textbox.style.background = toCssColor(patch.background);
        }

        if (patch.border_radius) {
            textbox.style.borderRadius = patch.border_radius + "px";
        }

        if (patch.stroke_color) {
            textbox.style.borderColor = toCssColor(patch.stroke_color);
        }

        if (patch.stroke_width) {
            textbox.style.borderWidth = patch.stroke_width + "px";

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
            leaf!.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
        }

        if (patch.focus_on_mount) {
            setTimeout(() => { textbox.focus(); }, 10);
        }
    }

    textboxDelete(id: number) {
        let oldNode = this.nodesLookup.get(id);
        if (oldNode){
            let parent = oldNode.parentElement;
            parent!.removeChild(oldNode);
            this.nodesLookup.delete(id);
        }
    }

    buttonCreate(patch: AnyCreatePatch) {
        console.assert(patch.id != null);
        console.assert(patch.occlusionLayerId != null);
        
        const button = this.objectManager.getFromPool(BUTTON) as HTMLButtonElement;
        const textContainer = this.objectManager.getFromPool(DIV) as HTMLDivElement;
        const textChild = this.objectManager.getFromPool(DIV) as HTMLDivElement;
        button.setAttribute("class", BUTTON_CLASS);
        textContainer.setAttribute("class", BUTTON_TEXT_CONTAINER_CLASS);
        textChild.style.margin = "0";
        button.addEventListener("click", (_event) => {
            let message = {
                "FormButtonClick": {
                    "id": patch.id!,
                }
            }
            this.chassis!.interrupt(JSON.stringify(message), undefined);
        });

        let buttonDiv: HTMLDivElement = this.objectManager.getFromPool(DIV);
        textContainer.appendChild(textChild);
        button.appendChild(textContainer);
        buttonDiv.appendChild(button);
        buttonDiv.setAttribute("class", NATIVE_LEAF_CLASS)
        buttonDiv.setAttribute("pax_id", String(patch.id));
        if(patch.id != undefined && patch.occlusionLayerId != undefined){
            this.layers.addElement(buttonDiv, patch.parentFrame, patch.occlusionLayerId);
            this.nodesLookup.set(patch.id!, buttonDiv);
        } else {
            throw new Error("undefined id or occlusionLayer");
        }
    }

    
    buttonUpdate(patch: ButtonUpdatePatch) {
        let leaf = this.nodesLookup.get(patch.id!);
        console.assert(leaf !== undefined);
        let button = leaf!.firstChild as HTMLElement;
        let textContainer = button!.firstChild as HTMLElement;
        let textChild = textContainer.firstChild as HTMLElement;


        // Apply the content
        if (patch.content != null) {
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
            leaf!.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
        }
    }

    buttonDelete(id: number) {
        let oldNode = this.nodesLookup.get(id);
        if (oldNode){
            let parent = oldNode.parentElement;
            parent!.removeChild(oldNode);
            this.nodesLookup.delete(id);
        }
    }

    textCreate(patch: AnyCreatePatch) {
        console.assert(patch.id != null);
        console.assert(patch.occlusionLayerId != null);

        let textDiv: HTMLDivElement = this.objectManager.getFromPool(DIV);
        let textChild: HTMLDivElement = this.objectManager.getFromPool(DIV);
        textChild.addEventListener("input", (_event) => {
            let message = {
              "TextInput": {
                "id": patch.id!,
                "text": sanitizeContentEditableString(textChild.innerHTML),
              }
            };

            this.chassis!.interrupt(JSON.stringify(message), undefined);
        });
        textDiv.appendChild(textChild);
        textDiv.setAttribute("class", NATIVE_LEAF_CLASS)
        textDiv.setAttribute("pax_id", String(patch.id));

        if(patch.id != undefined && patch.occlusionLayerId != undefined){
            this.layers.addElement(textDiv, patch.parentFrame, patch.occlusionLayerId);
            this.nodesLookup.set(patch.id!, textDiv);
        } else {
            throw new Error("undefined id or occlusionLayer");
        }
    }

    textUpdate(patch: TextUpdatePatch) {
        let leaf = this.nodesLookup.get(patch.id!) as HTMLElement;
        let textChild = leaf!.firstChild as HTMLElement;
        // should be start listening to this elements size and
        // send interrupts to the engine, or not?
        let start_listening = false;

        // Handle size_x and size_y
        if (patch.size_x != null) {

            // if size_x = -1.0, the engine wants to know
            // this elements size from the chassi.
            if (patch.size_x == -1.0) {
                start_listening = true;
            } else {
                leaf!.style.width = patch.size_x + "px";
            }
        }
        if (patch.size_y != null) {
            if (patch.size_y == -1.0) {
                start_listening = true;
            } else {
                leaf!.style.height = patch.size_y + "px";
            }
        }

        if (start_listening) {
            this.resizeObserver.observe(leaf);
        }

        // Handle transform
        if (patch.transform != null) {
            leaf!.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
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
            if (sanitizeContentEditableString(textChild.innerHTML) != patch.content) {
                textChild.innerHTML = patch.content;
            }
            // Apply the link styles if they exist
            if (patch.style_link) {
                let linkStyle = patch.style_link;
                const links = textChild.querySelectorAll('a');
                links.forEach((link: HTMLElement) => {
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

    textDelete(id: number) {
        let oldNode = this.nodesLookup.get(id);
        this.resizeObserver.unobserve(oldNode!);
        if (oldNode){
            let parent = oldNode.parentElement;
            parent!.removeChild(oldNode);
            this.nodesLookup.delete(id);
        }
    }

    frameCreate(patch: AnyCreatePatch) {
        console.assert(patch.id != null);
        this.layers.addContainer(patch.id!, patch.parentFrame);
    }

    frameUpdate(patch: FrameUpdatePatch) {
        console.assert(patch.id != null);

        let styles: Partial<ContainerStyle> = {};
         if (patch.sizeX != null) {
             styles.width = patch.sizeX;
         }
         if (patch.sizeY != null) {
             styles.height = patch.sizeY;
         }
         if (patch.transform != null) {
            styles.transform = patch.transform;
         }
        
        this.layers.updateContainer(patch.id!, styles);
    }

    frameDelete(id: number) {
        this.layers.removeContainer(id);
    }

    scrollerCreate(patch: AnyCreatePatch){
        console.assert(patch.id != null);
        console.assert(patch.occlusionLayerId != null);

        let scrollerDiv: HTMLDivElement = this.objectManager.getFromPool(DIV);
        let scroller: HTMLDivElement = this.objectManager.getFromPool(DIV);
        scrollerDiv.addEventListener("scroll", (_event) => {
            // TODO send interrupt
            // console.log("scrolling!");
        });

        scrollerDiv.appendChild(scroller);
        scrollerDiv.setAttribute("class", NATIVE_LEAF_CLASS)
        scrollerDiv.style.overflow = "scroll";
        scrollerDiv.setAttribute("pax_id", String(patch.id));


        if(patch.id != undefined && patch.occlusionLayerId != undefined){
            this.layers.addElement(scrollerDiv, patch.parentFrame, patch.occlusionLayerId);
            this.nodesLookup.set(patch.id!, scrollerDiv);
        } else {
            throw new Error("undefined id or occlusionLayer");
        }
    }

    scrollerUpdate(patch: ScrollerUpdatePatch){
        let leaf = this.nodesLookup.get(patch.id!);
        if (leaf == undefined) {
            throw new Error("tried to update non-existent scroller");
        }
        let scroller_inner = leaf.firstChild as HTMLElement;

        // Handle size_x and size_y
        if (patch.sizeX != null) {
            leaf.style.width = patch.sizeX + "px";
        }
        if (patch.sizeY != null) {
            leaf.style.height = patch.sizeY + "px";
        }
        if (patch.scrollX != null) {
            leaf.scrollLeft = patch.scrollX;
        }
        if (patch.scrollY != null) {
            leaf.scrollTop = patch.scrollY;
        }
        // Handle transform
        if (patch.transform != null) {
            leaf.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
        }

        if (patch.sizeInnerPaneX != null) {
            scroller_inner.style.width = patch.sizeInnerPaneX + "px";
        }
        if (patch.sizeInnerPaneY != null) {
            scroller_inner.style.height = patch.sizeInnerPaneY + "px";
        }
    }

    scrollerDelete(id: number){
        let oldNode = this.nodesLookup.get(id);
        if (oldNode == undefined) {
            throw new Error("tried to delete non-existent scroller");
        }
        let parent = oldNode.parentElement!;
        parent.removeChild(oldNode);
        this.nodesLookup.delete(id);
    }



    async imageLoad(patch: ImageLoadPatch, chassis: PaxChassisWeb) {

        if (chassis.image_loaded(patch.path ?? "")) {
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
                    "id": patch.id!,
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

function applyTextTyle(textContainer: HTMLElement, textElem: HTMLElement, style: TextStyle | undefined) {
    
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


// Function to set selection in content-editable text div
// see https://stackoverflow.com/questions/6240139/highlight-text-range-using-javascript/6242538#6242538


// why all the replaces?:
// see: https://stackoverflow.com/questions/13762863/contenteditable-field-to-maintain-newlines-upon-database-entry
function sanitizeContentEditableString(string: string): string {
    return (string
        .replace(/<br\s*\/*>/ig, '\n') 
        .replace(/(<(p|div))/ig, '\n$1') 
        .replace(/(<([^>]+)>)/ig, "")?? '');
}
