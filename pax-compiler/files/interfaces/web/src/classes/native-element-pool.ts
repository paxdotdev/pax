import {BUTTON_CLASS, BUTTON_TEXT_CONTAINER_CLASS,
    NATIVE_LEAF_CLASS, CHECKBOX_CLASS, RADIO_SET_CLASS, SCROLLER_CONTAINER,
    CANVAS_CLASS, NATIVE_OVERLAY_CLASS} from "../utils/constants";
import {AnyCreatePatch} from "./messages/any-create-patch";
import {OcclusionUpdatePatch} from "./messages/occlusion-update-patch";
import snarkdown from 'snarkdown';
import {TextUpdatePatch} from "./messages/text-update-patch";
import {FrameUpdatePatch} from "./messages/frame-update-patch";
import {ScrollerUpdatePatch} from "./messages/scroller-update-patch";
import {ButtonUpdatePatch} from "./messages/button-update-patch";
import {ImageLoadPatch} from "./messages/image-load-patch";
import {ContainerStyle, OcclusionLayerManager, setLeafLocalOpacity} from "./occlusion-context";
import {ObjectManager} from "../pools/object-manager";
import {
    IMAGE,
    INPUT,
    BUTTON,
    DIV,
    OCCLUSION_CONTEXT,
    SELECT,
    YOUTUBE_VIDEO,
    YOUTUBE_VIDEO_UPDATE_PATCH
} from "../pools/supported-objects";
import {packAffineCoeffsIntoMatrix3DString, readImageToByteBuffer} from "../utils/helpers";
import {
    ColorGroup,
    TextStyle,
    getRegisteredFontCssText,
    getAlignItems,
    getJustifyContent,
    getTextAlign,
    syncRegisteredFontsToDocument,
    waitForRegisteredFonts,
} from "./text";
import type {PaxChassisWeb} from "../types/pax-chassis-web";
import { CheckboxUpdatePatch } from "./messages/checkbox-update-patch";
import { TextboxUpdatePatch } from "./messages/textbox-update-patch";
import { RadioSetUpdatePatch } from "./messages/radio-set-update-patch";
import { DropdownUpdatePatch } from "./messages/dropdown-update-patch";
import { SliderUpdatePatch } from "./messages/slider-update-patch";
import { EventBlockerUpdatePatch } from "./messages/event-blocker-update-patch";
import { NavigationPatch } from "./messages/navigation-patch";
import { NativeImageUpdatePatch } from "./messages/native-image-update-patch";
import { YoutubeVideoUpdatePatch } from "./messages/youtube-video-update-patch";
import { SetCursorPatch } from "./messages/set-cursor-patch";
import { ScreenshotPatch } from "./messages/screenshot-patch";
import { NativeMaskUpdatePatch } from "./messages/native-mask-update-patch";

import html2canvas from 'html2canvas';

const SCREENSHOT_FONT_STYLE_ATTRIBUTE = 'data-pax-screenshot-font-style';


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

    occlusionUpdate(patch: OcclusionUpdatePatch) {
        let node: HTMLElement = this.nodesLookup.get(patch.id!)!;
        if (node){
            this.layers.addElement(node, patch.parentFrame, patch.occlusionLayerId!);
            node.style.zIndex = patch.zIndex!.toString();
            const focusableElements = node.querySelectorAll('input, button, select, textarea, a[href]');
            focusableElements.forEach((element, _index) => {
                element.setAttribute('tabindex', (1000000 - patch.zIndex!).toString());
            });
        } else {
            // must be container
            this.layers.updateContainerParent(patch.id!, patch.parentFrame);
        }
    }

    nativeMaskUpdate(patch: NativeMaskUpdatePatch) {
        let node: HTMLElement = this.nodesLookup.get(patch.id!)!;
        if (!node) {
            return;
        }
        this.layers.updateElementMask(node, patch.id!, patch.entries, patch.sizeX, patch.sizeY);
    }

    checkboxCreate(patch: AnyCreatePatch) {
        console.assert(patch.id != null);
        console.assert(patch.occlusionLayerId != null);
        
        const checkbox = this.objectManager.getFromPool(INPUT) as HTMLInputElement;
        checkbox.type = "checkbox";
        checkbox.style.margin = "0";
        checkbox.setAttribute("class", CHECKBOX_CLASS);
        checkbox.addEventListener("change", (event) => {
            //Reset the checkbox state (state changes only allowed through engine)
            const is_checked = (event.target as HTMLInputElement).checked;
            checkbox.checked = !is_checked;
            
            let message = {
                "FormCheckboxToggle": {
                    "id": patch.id,
                    "state": is_checked,
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

        updateCommonProps(leaf!, patch);

        if (patch.checked !== null) {
            checkbox.checked = patch.checked!;
        }

        if (patch.background != null) {
            checkbox.style.background = toCssColor(patch.background);
        }

        if (patch.borderRadius != null) {
            checkbox.style.borderRadius = patch.borderRadius + "px";
        }

        if (patch.outlineWidth !== undefined) {
            checkbox.style.borderWidth = patch.outlineWidth + "px";
        }

        if (patch.outlineColor != null) {
            checkbox.style.borderColor = toCssColor(patch.outlineColor);
        }

        if (patch.backgroundChecked != null) {
            checkbox.style.setProperty("--checked-color", toCssColor(patch.backgroundChecked));
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

    nativeImageCreate(patch: AnyCreatePatch) {
        console.assert(patch.id != null);
        console.assert(patch.occlusionLayerId != null);
        
        const nativeImage = this.objectManager.getFromPool(IMAGE) as HTMLInputElement;
        nativeImage.style.margin = "0";

        let nativeImage_div: HTMLDivElement = this.objectManager.getFromPool(DIV);
        nativeImage_div.appendChild(nativeImage);
        nativeImage_div.setAttribute("class", NATIVE_LEAF_CLASS)
        nativeImage_div.setAttribute("pax_id", String(patch.id));
        if(patch.id != undefined && patch.occlusionLayerId != undefined){
            this.layers.addElement(nativeImage_div, patch.parentFrame, patch.occlusionLayerId);
        }
        this.nodesLookup.set(patch.id!, nativeImage_div);
    }

    
    nativeImageUpdate(patch: NativeImageUpdatePatch) {
        let leaf = this.nodesLookup.get(patch.id!);
        let nativeImage = leaf!.firstChild as HTMLInputElement;
        updateCommonProps(leaf!, patch);
        if (patch.url != null) {
            nativeImage.setAttribute("src", patch.url);
        }
        if (patch.fit != null) {
            nativeImage.style.objectFit = patch.fit;
        }
    }

    nativeImageDelete(id: number) {
        let oldNode = this.nodesLookup.get(id);
        if (oldNode){
            let parent = oldNode.parentElement;
            parent!.removeChild(oldNode);
            this.nodesLookup.delete(id);
        }
    }

    youtubeVideoCreate(patch: AnyCreatePatch) {
        console.assert(patch.id != null);
        console.assert(patch.occlusionLayerId != null);

        const youtubeVideo = this.objectManager.getFromPool(YOUTUBE_VIDEO) as HTMLIFrameElement;
        youtubeVideo.width = "560";
        youtubeVideo.height = "315";
        youtubeVideo.title = "YouTube video player";
        youtubeVideo.frameBorder = "0";
        youtubeVideo.allow = "accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share";
        youtubeVideo.referrerPolicy = "strict-origin-when-cross-origin";
        youtubeVideo.allowFullscreen = true;

        let youtubeVideo_div: HTMLDivElement = this.objectManager.getFromPool(DIV);
        youtubeVideo_div.appendChild(youtubeVideo as Node);
        //The above fails: 'appendChild' on 'Node': parameter 1 is not of type 'Node'

        youtubeVideo_div.setAttribute("class", NATIVE_LEAF_CLASS)
        youtubeVideo_div.setAttribute("pax_id", String(patch.id));
        if(patch.id != undefined && patch.occlusionLayerId != undefined){
            this.layers.addElement(youtubeVideo_div, patch.parentFrame, patch.occlusionLayerId);
        }
        this.nodesLookup.set(patch.id!, youtubeVideo_div);
    }

    youtubeVideoUpdate(patch: YoutubeVideoUpdatePatch) {
        //retrieve the iframe; update its width, height, and src
        let leaf = this.nodesLookup.get(patch.id!);
        let youtubeVideo = leaf!.firstChild as HTMLIFrameElement;
        updateCommonProps(leaf!, patch);
        if (patch.url != null) {
            youtubeVideo.src = patch.url;
        }
        if (patch.size_x != null) {
            youtubeVideo.width = patch.size_x.toString();
        }
        if (patch.size_y != null) {
            youtubeVideo.height = patch.size_y.toString();
        }
    }

    youtubeVideoDelete(id: number) {
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
        textbox.style.margin = "0";
        textbox.style.padding = "0";
        textbox.style.paddingInline = "5px 5px";
        textbox.style.paddingBlock = "0";
        textbox.style.borderWidth = "0";
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
        updateCommonProps(leaf!, patch);
        // set to 10px less to give space for left-padding
        if (patch.size_x != null) {
            (leaf!.firstChild! as HTMLElement).style.width = (patch.size_x - 10) + "px";
        }
        let textbox = leaf!.firstChild as HTMLTextAreaElement;

        applyTextStyle(textbox, textbox, patch.style);

        if (patch.background != null) {
            textbox.style.background = toCssColor(patch.background);
        }

        if (patch.outline_width != null) {
            if (patch.outline_width < 0.1) {
                textbox.style.outline = "none";
            } else {
                textbox.style.outlineWidth = patch.outline_width + "px";
                if (patch.outline_color != null) {
                    textbox.style.outlineColor = toCssColor(patch.outline_color);
                }
            }
        }

        if (patch.stroke_width != null) {
            if (patch.stroke_width < 0.1) {
                textbox.style.border = "none";
            } else {
                //We may support styles other than solid in the future; this is a better default than the browser's for now
                textbox.style.borderStyle = "solid";
                textbox.style.borderWidth = patch.stroke_width + "px";
                if (patch.stroke_color != null) {
                    textbox.style.borderColor = toCssColor(patch.stroke_color);
                }
                if (patch.border_radius != null) {
                    textbox.style.borderRadius = patch.border_radius + "px";
                }
            }
        }

        if (patch.placeholder != null) {
            textbox.placeholder = patch.placeholder;
        }

        // Apply the content
        if (patch.text != null) {
            // Check if the input element is focused — we want to maintain the user's cursor position if so
            if (document.activeElement === textbox) {
                let new_text = patch.text!;
                // Get the current selection range
                const selectionStart = textbox.selectionStart || 0;

                // Update the content of the input
                textbox.value = new_text;

                // Calculate the new cursor position, clamped to the new length of the input value
                const newCursorPosition = Math.min(selectionStart, new_text.length);

                // Set the cursor position to the beginning of the former selection range
                textbox.setSelectionRange(newCursorPosition, newCursorPosition);
            } else {
                // If the textbox isn't selected, just update its content
                textbox.value = patch.text;
            }
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


    
    radioSetCreate(patch: AnyCreatePatch) {
        let fields = document.createElement('fieldset') as HTMLFieldSetElement;
        fields.style.border = "0";
        fields.style.margin = "0";
        fields.style.padding = "0";
        fields.addEventListener('change', (event) => {
            let target = event.target as HTMLElement | undefined;
            if (target && target.matches("input[type='radio']")) {
                // get the index of the triggered radio button in the fieldset
                let container = target.parentNode as Element;
                let index = Array.from(container!.parentNode!.children).indexOf(container);
                let message = {
                    "FormRadioSetChange": {
                        "id": patch.id!,
                        "selected_id": index,
                    }
                }
                this.chassis!.interrupt(JSON.stringify(message), undefined);
            }
        });

        let radioSetDiv: HTMLDivElement = this.objectManager.getFromPool(DIV);
        radioSetDiv.setAttribute("class", NATIVE_LEAF_CLASS)
        radioSetDiv.setAttribute("pax_id", String(patch.id));
        radioSetDiv.appendChild(fields);

        if(patch.id != undefined && patch.occlusionLayerId != undefined){
            this.layers.addElement(radioSetDiv, patch.parentFrame, patch.occlusionLayerId);
            this.nodesLookup.set(patch.id!, radioSetDiv);
        } else {
            throw new Error("undefined id or occlusionLayer");
        }

    }

    
    radioSetUpdate(patch: RadioSetUpdatePatch) {
        let leaf = this.nodesLookup.get(patch.id!);
        updateCommonProps(leaf!, patch);
        if (patch.style != null) {
            applyTextStyle(leaf!, leaf!, patch.style);
        }

        let fields = leaf!.firstChild as HTMLFieldSetElement;
        if (patch.options != null) {
            fields!.innerHTML = "";
            patch.options.forEach((optionText, _index) => {
                let div = document.createElement('div') as HTMLDivElement;
                div.style.alignItems = "center";
                div.style.display = "flex";
                div.style.marginBottom = "3px";
                const option = document.createElement('input') as HTMLInputElement;
                option.type = "radio";
                option.name = `radio-${patch.id}`;
                option.value = optionText.toString();
                option.setAttribute("class", RADIO_SET_CLASS);
                div.appendChild(option);
                const label = document.createElement('label') as HTMLLabelElement;
                label.innerHTML = optionText.toString();
                div.appendChild(label);
                fields.appendChild(div);
            });
        }

        if (patch.selected_id != null) {
            let radio = fields.children[patch.selected_id].firstChild as HTMLInputElement;
            if (radio.checked == false) {
                radio.checked = true;
            }
        }

        if (patch.background != null) {
           fields.style.setProperty("--background-color", toCssColor(patch.background));
        }

        if (patch.backgroundChecked != null) {
           fields.style.setProperty("--selected-color", toCssColor(patch.backgroundChecked));
        }

        if (patch.outlineWidth != null) {
            fields.style.setProperty("--border-width", patch.outlineWidth + "px");
        }

        if (patch.outlineColor != null) {
            fields.style.setProperty("--border-color",  toCssColor(patch.outlineColor));
        }
    }

    radioSetDelete(id: number) {
        let oldNode = this.nodesLookup.get(id);
        if (oldNode){
            let parent = oldNode.parentElement;
            parent!.removeChild(oldNode);
            this.nodesLookup.delete(id);
        }
    }

    
    sliderCreate(patch: AnyCreatePatch) {
        const slider = this.objectManager.getFromPool(INPUT) as HTMLInputElement;
        slider.type = "range";
        slider.style.padding = "0px";
        slider.style.margin = "0px";
        slider.style.appearance = "none";
        slider.style.display = "block";
        slider.addEventListener("input", (_event) => {
            let message = {
                "FormSliderChange": {
                    "id": patch.id!,
                    "value": parseFloat(slider.value),
                }
            }
            this.chassis!.interrupt(JSON.stringify(message), undefined);
        });

        let sliderDiv: HTMLDivElement = this.objectManager.getFromPool(DIV);
        sliderDiv.appendChild(slider);
        sliderDiv.setAttribute("class", NATIVE_LEAF_CLASS)
        sliderDiv.style.overflow = "visible";
        sliderDiv.style.contain = "layout style";
        sliderDiv.setAttribute("pax_id", String(patch.id));

        if(patch.id != undefined && patch.occlusionLayerId != undefined){
            this.layers.addElement(sliderDiv, patch.parentFrame, patch.occlusionLayerId);
            this.nodesLookup.set(patch.id!, sliderDiv);
        } else {
            throw new Error("undefined id or occlusionLayer");
        }

    }

    
    sliderUpdate(patch: SliderUpdatePatch) {
        let leaf = this.nodesLookup.get(patch.id!);
        updateCommonProps(leaf!, patch);
        let slider = leaf!.firstChild as HTMLInputElement;
        slider.style.height = "";

        if (patch.step != null && patch.step.toString() != slider.step) {
            slider.step = patch.step.toString();
        }
        if (patch.min != null && patch.min.toString() != slider.min) {
            slider.min = patch.min.toString();
        }
        if (patch.max != null && patch.max.toString() != slider.max) {
            slider.max = patch.max.toString();
        }
        if (patch.value != null && patch.value.toString() != slider.value) {
            slider.value = patch.value.toString();
        }

        if (patch.accent != null) {
            let color =  toCssColor(patch.accent);   
            slider.style.accentColor = color;
        }

        if (patch.background != null) {
            let color =  toCssColor(patch.background);   
            slider.style.backgroundColor = color;
        }

        if (patch.borderRadius != null) {
            slider.style.borderRadius = patch.borderRadius + "px";
        }
    }

    sliderDelete(id: number) {
        let oldNode = this.nodesLookup.get(id);
        if (oldNode){
            let parent = oldNode.parentElement;
            parent!.removeChild(oldNode);
            this.nodesLookup.delete(id);
        }
    }

    dropdownCreate(patch: AnyCreatePatch) {
        const dropdown = this.objectManager.getFromPool(SELECT) as HTMLSelectElement;
        dropdown.addEventListener("change", (event) => {
            let message = {
                "FormDropdownChange": {
                    "id": patch.id!,
                    "selected_id": (event.target! as any).selectedIndex,
                }
            }
            this.chassis!.interrupt(JSON.stringify(message), undefined);
        });


        let textboxDiv: HTMLDivElement = this.objectManager.getFromPool(DIV);
        textboxDiv.appendChild(dropdown);
        textboxDiv.setAttribute("class", NATIVE_LEAF_CLASS)
        textboxDiv.setAttribute("pax_id", String(patch.id));

        if(patch.id != undefined && patch.occlusionLayerId != undefined){
            this.layers.addElement(textboxDiv, patch.parentFrame, patch.occlusionLayerId);
            this.nodesLookup.set(patch.id!, textboxDiv);
        } else {
            throw new Error("undefined id or occlusionLayer");
        }

    }

    
    dropdownUpdate(patch: DropdownUpdatePatch) {
        let leaf = this.nodesLookup.get(patch.id!);
        updateCommonProps(leaf!, patch);
        let dropdown = leaf!.firstChild as HTMLSelectElement;
        applyTextStyle(dropdown, dropdown, patch.style);
        dropdown.style.borderStyle = "solid";

        if (patch.background != null) {
            dropdown.style.backgroundColor = toCssColor(patch.background);
        }
        if (patch.stroke_color != null) {
            dropdown.style.borderColor = toCssColor(patch.stroke_color);
        }
        if (patch.stroke_width != null) {
            dropdown.style.borderWidth = patch.stroke_width + "px";
        }

        if (patch.borderRadius != null) {
            dropdown.style.borderRadius = patch.borderRadius + "px";
        }

        // Apply the content
        if (patch.options != null) {
            // Iterate over the options array and create option elements

            //clear children
            dropdown.innerHTML = "";

            patch.options.forEach((optionText, index) => {
                const option = document.createElement('option') as HTMLOptionElement;
                option.value = index.toString();
                option.textContent = optionText;
                dropdown.appendChild(option);
            });
        }

        if (patch.selected_id != null && dropdown.options.selectedIndex != patch.selected_id) {
            dropdown.options.selectedIndex = patch.selected_id;
        }
    }

    dropdownDelete(id: number) {
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
        updateCommonProps(leaf!, patch);
        console.assert(leaf !== undefined);
        let button = leaf!.firstChild as HTMLElement;
        let textContainer = button!.firstChild as HTMLElement;
        let textChild = textContainer.firstChild as HTMLElement;


        // Apply the content
        if (patch.content != null) {
            textChild.innerHTML = snarkdown(patch.content);
        }
        // if not applied, rendering moves button down
        if (textChild.innerHTML.length == 0) {
            textChild.innerHTML = " ";
        }

        if (patch.color != null) {
            button.style.background = toCssColor(patch.color);
        }

        if (patch.hoverColor != null) {
            let color = toCssColor(patch.hoverColor);
            button.style.setProperty("--hover-color", color);
        }

        if (patch.borderRadius != null) {
            button.style.borderRadius = patch.borderRadius + "px";
        }

        if (patch.outlineStrokeColor != null) {
            button.style.borderColor = toCssColor(patch.outlineStrokeColor);
        }

        if (patch.outlineStrokeWidth != null) {
            button.style.borderWidth = patch.outlineStrokeWidth + "px";
        }
        
        applyTextStyle(textContainer, textChild, patch.style);
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
        // Text should be allowed to paint outside its measured box by default;
        // otherwise large headings get clipped by native leaf paint containment.
        textDiv.style.overflow = "visible";
        textDiv.style.contain = "layout style";
        textChild.style.overflow = "visible";
        textDiv.addEventListener("click", (_event) => {
            if (textDiv.contentEditable != "false") {
                textChild.focus();
            }
        });
        textChild.addEventListener("input", (_event) => {
            let message = {
              "TextInput": {
                "id": patch.id!,
                "text": textChild.innerText,
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
        const applyClip = (clip: boolean) => {
            const overflow = clip ? "hidden" : "visible";
            leaf.style.overflow = overflow;
            textChild.style.overflow = overflow;
        };
        // should be start listening to this elements size and
        // send interrupts to the engine, or not?
        let start_listening = false;

        // Handle size_x and size_y
        if (patch.size_x != null) {

            // if size_x = -1.0, the engine wants to know
            // this elements size from the chassi.
            if (patch.size_x == -1.0) {
                start_listening = true;
                leaf!.style.width = "auto";
            } else {
                leaf!.style.width = patch.size_x + "px";
            }
        }
        if (patch.size_y != null) {
            if (patch.size_y == -1.0) {
                start_listening = true;
                leaf!.style.height = "auto";
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

        if (patch.opacity != null) {
            setLeafLocalOpacity(leaf!, patch.opacity);
        }

        if (patch.editable != null) {
            if (patch.editable == true) {
                const selection = window.getSelection();
                selection!.removeAllRanges();
                textChild.setAttribute("contenteditable", "plaintext-only");
                textChild.style.outline = "none";


                setTimeout(() => {
                    textChild.focus();

                    // Move the cursor to the end of the text
                    const range = document.createRange();
                    range.selectNodeContents(textChild);
                    const selection = window.getSelection();
                    selection!.removeAllRanges();
                    selection!.addRange(range)                    
                }, 1);
                // Focus on the editable div
            } else {
                textChild.setAttribute("contenteditable", "false");
            }
        }

        if (patch.selectable != null) {
            textChild.style.userSelect = patch.selectable ? "auto" : "none";
        }

        if (patch.clip != null) {
            applyClip(patch.clip);
        }

        applyTextStyle(leaf, textChild, patch.style);

        // Apply the content
        if (patch.content != null) {
            if (textChild.innerText != patch.content) {
                if (patch.markdown) {
                    textChild.innerHTML = snarkdown(patch.content);
                } else {
                    textChild.innerText = patch.content;
                }
            }
            // Apply the link styles if they exist
            if (patch.style_link != null) {
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
                    //force underlining for now since we don't currently offer an API that offers sane (underlined) defaults.
                    link.style.textDecoration = 'underline';
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
         if (patch.clipContent != null) {
             styles.clipContent = patch.clipContent;
         }
         if (patch.clipPath != null) {
             styles.clipPath = patch.clipPath;
         }
         if (patch.opacity != null) {
             styles.opacity = patch.opacity;
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
        scroller.style.pointerEvents = "none";
        scrollerDiv.addEventListener("scroll", (_event) => {
            let message = {
                "Scrollbar": {
                    "id": patch.id!,
                    "scroll_x": scrollerDiv.scrollLeft,
                    "scroll_y": scrollerDiv.scrollTop
                }
            }
            this.chassis!.interrupt(JSON.stringify(message), undefined);
        });

        scrollerDiv.appendChild(scroller);
        scrollerDiv.setAttribute("class", NATIVE_LEAF_CLASS + " " + SCROLLER_CONTAINER)
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
        // Ordering sometimes result in updates being sent after deletes.
        // could fix this ordering, but simply "skipping" this works for now.
        if (leaf == undefined) {
            return;
        }
        let scroller_inner = leaf.firstChild as HTMLElement;

        // Handle size_x and size_y
        if (patch.sizeX != null) {
            leaf.style.width = patch.sizeX + "px";
        }
        if (patch.sizeY != null) {
            leaf.style.height = patch.sizeY + "px";
        }

        if (patch.sizeInnerPaneX != null) {
            if (patch.sizeInnerPaneX! <= parseFloat(leaf.style.width)) {
                leaf.style.overflowX = "hidden";
            } else {
                leaf.style.overflowX = "auto";
            }
            scroller_inner.style.width = patch.sizeInnerPaneX + "px";
        }
        if (patch.sizeInnerPaneY != null) {
            if (patch.sizeInnerPaneY! <= parseFloat(leaf.style.height)) {
                leaf.style.overflowY = "hidden";
            } else {
                leaf.style.overflowY = "auto";
            }
            scroller_inner.style.height = patch.sizeInnerPaneY + "px";
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

        if (patch.opacity != null) {
            setLeafLocalOpacity(leaf, patch.opacity);
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

    eventBlockerCreate(patch: AnyCreatePatch){
        console.assert(patch.id != null);
        console.assert(patch.occlusionLayerId != null);

        let eventBlockerDiv: HTMLDivElement = this.objectManager.getFromPool(DIV);
        // let eventBlocker: HTMLDivElement = this.objectManager.getFromPool(DIV);
        eventBlockerDiv.setAttribute("class", NATIVE_LEAF_CLASS)
        eventBlockerDiv.setAttribute("pax_id", String(patch.id));


        if(patch.id != undefined && patch.occlusionLayerId != undefined){
            this.layers.addElement(eventBlockerDiv, patch.parentFrame, patch.occlusionLayerId);
            this.nodesLookup.set(patch.id!, eventBlockerDiv);
        } else {
            throw new Error("undefined id or occlusionLayer");
        }


    }

    eventBlockerUpdate(patch: EventBlockerUpdatePatch){
        let leaf = this.nodesLookup.get(patch.id!);
        if (leaf == undefined) {
            throw new Error("tried to update non-existent event blocker");
        }
        // Handle size_x and size_y
        if (patch.sizeX != null) {
            leaf.style.width = patch.sizeX + "px";
        }
        if (patch.sizeY != null) {
            leaf.style.height = patch.sizeY + "px";
        }
        // Handle transform
        if (patch.transform != null) {
            leaf.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
        }
        if (patch.opacity != null) {
            setLeafLocalOpacity(leaf, patch.opacity);
        }
    }

    eventBlockerDelete(id: number){
        let oldNode = this.nodesLookup.get(id);
        if (oldNode == undefined) {
            throw new Error("tried to delete non-existent event blocker");
        }
        let parent = oldNode.parentElement!;
        parent.removeChild(oldNode);
        this.nodesLookup.delete(id);
    }

    async screenshot(patch: ScreenshotPatch, chassis: PaxChassisWeb) {
        let canvas: HTMLCanvasElement | null = null;
        let ctx: CanvasRenderingContext2D | null = null;
        let responded = false;

        const sendScreenshot = () => {
            if (responded || !canvas || !ctx) {
                return;
            }

            const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
            const pixels = new Uint8Array(imageData.data.buffer);

            const message = {
                "Screenshot": {
                    "Data": {
                        "id": patch.id!,
                        "path": "",
                        "width": canvas.width,
                        "height": canvas.height,
                    }
                }
            };

            chassis.interrupt(JSON.stringify(message), pixels);
            responded = true;
        };

        try {
            type ScreenshotLayer = {
                element: HTMLCanvasElement | HTMLDivElement;
                index: number;
            };
            type LayerScreenshotData = {
                data: Uint8Array | number[];
                height: number;
                id: number;
                width: number;
            };

            const mount = this.layers.parent;
            if (!(mount instanceof HTMLElement)) {
                console.warn('Could not resolve screenshot mount');
                return;
            }

            const scale = patch.scale ?? 1;
            canvas = document.createElement('canvas');
            canvas.width = Math.max(1, Math.round(mount.clientWidth * scale));
            canvas.height = Math.max(1, Math.round(mount.clientHeight * scale));

            ctx = canvas.getContext('2d');
            if (!ctx) {
                console.log('Could not get canvas context');
                return;
            }

            const layers = Array
                .from(mount.children)
                .reduce<ScreenshotLayer[]>((acc, element, index) => {
                    if (element instanceof HTMLCanvasElement && element.classList.contains(CANVAS_CLASS)) {
                        acc.push({ element, index });
                    } else if (element instanceof HTMLDivElement && element.classList.contains(NATIVE_OVERLAY_CLASS)) {
                        acc.push({ element, index });
                    }
                    return acc;
                }, [])
                .sort((left, right) => {
                    const leftZIndex = Number.parseInt(window.getComputedStyle(left.element).zIndex || '0', 10);
                    const rightZIndex = Number.parseInt(window.getComputedStyle(right.element).zIndex || '0', 10);
                    const safeLeftZIndex = Number.isNaN(leftZIndex) ? 0 : leftZIndex;
                    const safeRightZIndex = Number.isNaN(rightZIndex) ? 0 : rightZIndex;
                    if (safeLeftZIndex !== safeRightZIndex) {
                        return safeLeftZIndex - safeRightZIndex;
                    }
                    return left.index - right.index;
                });

            const requestId = patch.id!;
            const canvasLayerIds = new Set<number>();
            for (const { element } of layers) {
                if (!(element instanceof HTMLCanvasElement)) {
                    continue;
                }
                const layerId = Number.parseInt(element.id, 10);
                if (Number.isNaN(layerId)) {
                    continue;
                }
                canvasLayerIds.add(layerId);
                chassis.request_layer_screenshot(layerId, requestId);
            }

            const nextAnimationFrame = () => new Promise<void>((resolve) => {
                window.requestAnimationFrame(() => resolve());
            });
            const waitForPaint = async (frames: number = 1) => {
                for (let frame = 0; frame < frames; frame += 1) {
                    await nextAnimationFrame();
                }
            };

            const drawLayerScreenshot = (
                screenshot: LayerScreenshotData,
            ) => {
                const layerCanvas = document.createElement('canvas');
                layerCanvas.width = screenshot.width;
                layerCanvas.height = screenshot.height;
                const layerContext = layerCanvas.getContext('2d');
                if (!layerContext) {
                    return;
                }
                const imageData = new ImageData(
                    new Uint8ClampedArray(screenshot.data),
                    screenshot.width,
                    screenshot.height,
                );
                layerContext.putImageData(imageData, 0, 0);
                ctx.drawImage(layerCanvas, 0, 0, canvas.width, canvas.height);
            };

            const waitForLayerScreenshot = async (layerId: number) => {
                for (let attempt = 0; attempt < 10; attempt += 1) {
                    const screenshot = chassis.take_layer_screenshot(layerId, requestId) as LayerScreenshotData | null;
                    if (screenshot !== null) {
                        return screenshot;
                    }
                    await nextAnimationFrame();
                }
                console.warn(`Timed out waiting for Pax canvas screenshot for layer ${layerId}`);
                return null;
            };

            await waitForRegisteredFonts();
            await waitForPaint(2);

            const canvasScreenshots = new Map<number, LayerScreenshotData>();
            for (const layerId of canvasLayerIds) {
                const screenshot = await waitForLayerScreenshot(layerId);
                if (screenshot !== null) {
                    canvasScreenshots.set(layerId, screenshot);
                }
            }

            let hasNativeOverlayContent = false;
            for (const { element } of layers) {
                if (element instanceof HTMLCanvasElement) {
                    const layerId = Number.parseInt(element.id, 10);
                    if (Number.isNaN(layerId)) {
                        continue;
                    }
                    const screenshot = canvasScreenshots.get(layerId);
                    if (screenshot === undefined) {
                        continue;
                    }
                    drawLayerScreenshot(screenshot);
                    continue;
                }

                if (element.childElementCount === 0) {
                    continue;
                }
                hasNativeOverlayContent = true;
            }

            if (hasNativeOverlayContent) {
                const overlayCanvas = await html2canvas(mount, {
                    backgroundColor: null,
                    useCORS: true,
                    allowTaint: false,
                    foreignObjectRendering: true,
                    scale,
                    width: mount.clientWidth,
                    height: mount.clientHeight,
                    windowWidth: mount.clientWidth,
                    windowHeight: mount.clientHeight,
                    onclone: (async (clonedDocument: Document, clonedMount: HTMLElement) => {
                        await syncRegisteredFontsToDocument(clonedDocument);
                        injectRegisteredFontCssIntoElement(clonedDocument, clonedMount);

                        clonedDocument.documentElement.style.width = `${mount.clientWidth}px`;
                        clonedDocument.documentElement.style.height = `${mount.clientHeight}px`;
                        clonedDocument.body.style.width = `${mount.clientWidth}px`;
                        clonedDocument.body.style.height = `${mount.clientHeight}px`;
                        clonedMount.style.width = `${mount.clientWidth}px`;
                        clonedMount.style.height = `${mount.clientHeight}px`;

                        clonedMount
                            .querySelectorAll(`.${NATIVE_OVERLAY_CLASS}`)
                            .forEach((overlayNode) => {
                                if (!(overlayNode instanceof HTMLElement)) {
                                    return;
                                }
                                overlayNode.style.width = `${mount.clientWidth}px`;
                                overlayNode.style.height = `${mount.clientHeight}px`;
                            });

                        clonedMount
                            .querySelectorAll(`canvas.${CANVAS_CLASS}`)
                            .forEach((canvasNode) => {
                                if (!(canvasNode instanceof HTMLElement)) {
                                    return;
                                }
                                canvasNode.style.display = 'none';
                            });
                    }) as unknown as (document: Document, element: HTMLElement) => void,
                    ignoreElements: (overlayElement: Element) => isScreenshotIgnoredElement(overlayElement),
                }).catch((err) => {
                    console.warn('Proceeding without native overlay capture', err);
                    return null;
                });
                if (overlayCanvas) {
                    ctx.drawImage(overlayCanvas, 0, 0, canvas.width, canvas.height);
                }
                drawPlainTextNativeLeaves(ctx, mount, canvas.width, canvas.height);
            }

            sendScreenshot();
        } catch (err) {
            console.error('html2canvas error:', err);
            sendScreenshot();
        }
    }
    
    async imageLoad(patch: ImageLoadPatch, chassis: PaxChassisWeb) {
        if (chassis.image_loaded(patch.path ?? "")) {
            return
        }
        //Check the full path of our index.js; use the prefix of this path also for our image assets
        function getBasePath() {
            const baseURI = document.baseURI;
            const url = new URL(baseURI);
            return url.pathname.substring(0, url.pathname.lastIndexOf('/') + 1);
        }
    
        const BASE_PATH = getBasePath();

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

    navigate(patch: NavigationPatch) {
        let name: string;
        switch (patch.target) {
            case "current":
                name = "_self";
                break;
            case "new":
                name = "_blank";
                break;
            default:
                console.error("no valid url target!");
                name = "_self";
        }
        window.open(patch.url, name);
    }

    setCursor(patch: SetCursorPatch) {
        document.body.style.cursor = patch.cursor!;
    }
}

function isScreenshotIgnoredElement(element: Element): boolean {
    if (element.tagName === 'IMG') {
        return true;
    }

    if (isPlainTextNativeLeaf(element)) {
        return true;
    }

    return element.tagName === 'CANVAS'
        && element.classList.contains(CANVAS_CLASS);
}

function injectRegisteredFontCssIntoElement(targetDocument: Document, targetElement: HTMLElement) {
    if (targetElement.querySelector(`style[${SCREENSHOT_FONT_STYLE_ATTRIBUTE}]`)) {
        return;
    }

    const fontCss = getRegisteredFontCssText().trim();
    if (fontCss.length === 0) {
        return;
    }

    // html2canvas serializes the cloned mount subtree into the foreignObject image;
    // styles injected only into clonedDocument.head are not carried into that snapshot.
    const style = targetDocument.createElement('style');
    style.setAttribute(SCREENSHOT_FONT_STYLE_ATTRIBUTE, 'true');
    style.textContent = fontCss;
    targetElement.prepend(style);
}

function getElementComputedStyle(element: Element): CSSStyleDeclaration | null {
    const defaultView = element.ownerDocument?.defaultView;
    if (!defaultView) {
        return null;
    }

    return defaultView.getComputedStyle(element);
}

function isElementWithTextContent(element: Element): element is HTMLElement {
    return element.nodeType === Node.ELEMENT_NODE && 'innerText' in element;
}

function isMaskedNativeLeaf(element: Element): boolean {
    const computedStyle = getElementComputedStyle(element);
    if (!computedStyle) {
        return false;
    }

    const webkitMaskImage = (computedStyle as CSSStyleDeclaration & { webkitMaskImage?: string }).webkitMaskImage;
    return computedStyle.maskImage !== 'none'
        || (!!webkitMaskImage && webkitMaskImage !== 'none');
}

function isPlainTextNativeLeaf(element: Element): boolean {
    if (element.nodeType !== Node.ELEMENT_NODE) {
        return false;
    }

    if (!element.classList.contains(NATIVE_LEAF_CLASS)) {
        return false;
    }

    if (element.querySelector('input, button, select, textarea, iframe, img, video, canvas')) {
        return false;
    }

    const textChild = element.firstElementChild;
    if (!(textChild instanceof HTMLElement)) {
        return false;
    }

    if (isMaskedNativeLeaf(element)) {
        return false;
    }

    return isElementWithTextContent(textChild)
        && textChild.children.length === 0
        && textChild.innerText.trim().length > 0;
}

function drawPlainTextNativeLeaves(
    ctx: CanvasRenderingContext2D,
    mount: HTMLElement,
    outputWidth: number,
    outputHeight: number,
) {
    const scaleX = outputWidth / Math.max(mount.clientWidth, 1);
    const scaleY = outputHeight / Math.max(mount.clientHeight, 1);

    mount.querySelectorAll(`.${NATIVE_LEAF_CLASS}`).forEach((leafNode) => {
        if (!(leafNode instanceof HTMLElement) || !isPlainTextNativeLeaf(leafNode)) {
            return;
        }

        const textChild = leafNode.firstElementChild;
        if (!(textChild instanceof HTMLElement)) {
            return;
        }

        const leafRect = leafNode.getBoundingClientRect();
        const textStyle = window.getComputedStyle(textChild);
        const leafStyle = window.getComputedStyle(leafNode);
        const text = textChild.innerText;
        if (text.length === 0) {
            return;
        }

        ctx.save();
        ctx.globalAlpha *= Number.parseFloat(leafStyle.opacity || '1') || 1;
        ctx.fillStyle = textStyle.color;
        ctx.font = textStyle.font;
        ctx.textAlign = (textStyle.textAlign as CanvasTextAlign) || 'left';
        ctx.textBaseline = 'middle';
        ctx.direction = (textStyle.direction as CanvasDirection) || 'ltr';
        applyCanvasTypography(ctx, textStyle);

        const transform = leafStyle.transform !== 'none'
            ? new DOMMatrixReadOnly(leafStyle.transform)
            : new DOMMatrixReadOnly();
        const fontSize = Number.parseFloat(textStyle.fontSize || '16');
        const lineHeight = Number.parseFloat(textStyle.lineHeight || '') || fontSize * 1.2;
        const localWidth = Number.parseFloat(leafStyle.width || '') || leafNode.offsetWidth || leafRect.width;
        const localHeight = Number.parseFloat(leafStyle.height || '') || leafNode.offsetHeight || leafRect.height;
        const lines = wrapPlainText(text, localWidth, ctx);
        const totalTextHeight = lineHeight * lines.length;

        ctx.setTransform(scaleX, 0, 0, scaleY, 0, 0);
        ctx.transform(
            transform.a,
            transform.b,
            transform.c,
            transform.d,
            transform.e,
            transform.f,
        );

        let x = 0;
        switch (ctx.textAlign) {
            case 'center':
                x = localWidth / 2;
                break;
            case 'right':
            case 'end':
                x = localWidth;
                break;
            default:
                x = 0;
        }

        const startY = (localHeight - totalTextHeight) / 2 + lineHeight / 2;
        lines.forEach((line, index) => {
            ctx.fillText(line, x, startY + index * lineHeight, localWidth);
        });
        ctx.restore();
    });
}

function applyCanvasTypography(ctx: CanvasRenderingContext2D, textStyle: CSSStyleDeclaration) {
    const typographyContext = ctx as CanvasRenderingContext2D & {
        fontKerning?: string;
        fontStretch?: string;
        fontVariantCaps?: string;
        letterSpacing?: string;
        wordSpacing?: string;
        textRendering?: string;
    };

    typographyContext.fontKerning = textStyle.fontKerning;
    typographyContext.fontStretch = textStyle.fontStretch;
    typographyContext.fontVariantCaps = textStyle.fontVariantCaps;
    typographyContext.letterSpacing = textStyle.letterSpacing;
    typographyContext.wordSpacing = textStyle.wordSpacing;
    typographyContext.textRendering = textStyle.textRendering;
}

function wrapPlainText(text: string, maxWidth: number, ctx: CanvasRenderingContext2D): string[] {
    const paragraphs = text.split('\n');
    const lines: string[] = [];

    paragraphs.forEach((paragraph) => {
        const words = paragraph.split(/\s+/).filter((word) => word.length > 0);
        if (words.length === 0) {
            lines.push('');
            return;
        }

        let currentLine = words[0]!;
        for (let index = 1; index < words.length; index += 1) {
            const candidate = `${currentLine} ${words[index]}`;
            if (ctx.measureText(candidate).width <= maxWidth) {
                currentLine = candidate;
            } else {
                lines.push(currentLine);
                currentLine = words[index]!;
            }
        }
        lines.push(currentLine);
    });

    return lines;
}

function toCssColor(color: ColorGroup): string {
    if (color.Rgba != null) {
        let p = color.Rgba;
        return `rgba(${p[0] * 255},${p[1] * 255},${p[2] * 255},${p[3]})`; //Note that alpha channel expects [0.0, 1.0] in CSS
    } else {
        throw new TypeError("Unsupported Color Format");
    }        
}

function applyTextStyle(textContainer: HTMLElement, textElem: HTMLElement, style: TextStyle | undefined) {
    
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

function updateCommonProps(leaf: HTMLElement, patch: any) {
    let elem = leaf!.firstChild as any;
    // Handle size_x and size_y
    if (patch.size_x != null) {
        elem!.style.width = patch.size_x + "px";
    }
    if (patch.size_y != null) {
        elem!.style.height = patch.size_y + "px";
    }
    // Handle transform
    if (patch.transform != null) {
        leaf!.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
    }
    if (patch.opacity != null) {
        setLeafLocalOpacity(leaf, patch.opacity);
    }
}
