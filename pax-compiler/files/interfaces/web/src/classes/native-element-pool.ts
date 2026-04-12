import {BUTTON_CLASS, BUTTON_TEXT_CONTAINER_CLASS,
    NATIVE_LEAF_CLASS, CHECKBOX_CLASS, RADIO_SET_CLASS, SCROLLER_CONTAINER,
    CANVAS_CLASS, NATIVE_OVERLAY_CLASS, INNER_PANE} from "../utils/constants";
import {AnyCreatePatch} from "./messages/any-create-patch";
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
import {
    affineMultiply,
    invertAffineCoeffs,
    packAffineCoeffsIntoMatrix3DString,
    readImageToByteBuffer,
} from "../utils/helpers";
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
import {
    describeSurfaceHost,
    estimateWarmLayerCanvasCount,
    isIOSWebKitBrowser,
} from "./surface-host-policy";

const SCREENSHOT_FONT_STYLE_ATTRIBUTE = 'data-pax-screenshot-font-style';


export class NativeElementPool {
    private canvases: Map<string, HTMLCanvasElement>;
    private lastCanvasSurfaceSignatures = new Map<string, string>();
    private lastCanvasTransformSignatures = new Map<string, string>();
    private lastCanvasLayerCounts = new Map<number, number>();
    layers: OcclusionLayerManager;
    private nodesLookup = new Map<number, HTMLElement>();
    private scrollerHosts = new Map<number, ScrollerDomHosts>();
    private scrollerMeasurementStates = new Map<number, ScrollerMeasurementState>();
    private presentationRecords = new Map<number, PresentationRecord>();
    private pendingScrollerUpdates = new Map<number, PendingScrollerUpdate>();
    private surfaceRefreshPending = false;
    private chassis?: PaxChassisWeb;
    private mount?: HTMLElement;
    private activePageScrollScrollerId?: number;
    private pageScrollActivityListenersInstalled = false;
    private readonly pageScrollActivityListener: () => void;
    private perfTraceEnabled = false;
    private perfTraceSequence = 0;
    private lastPerfLifecycleSignature = "";
    private lastPerfLifecycleLogAt = 0;
    private lastPerfCanvasSignature = "";
    private lastPerfCanvasLogAt = 0;
    private objectManager: ObjectManager;
    private resizeObserver: ResizeObserver;
    registeredFontFaces: Set<string>;

    constructor(objectManager: ObjectManager) {
        this.objectManager = objectManager;
        this.canvases = new Map();
        this.layers = objectManager.getFromPool(OCCLUSION_CONTEXT, objectManager);
        this.registeredFontFaces = new Set<string>();
        this.pageScrollActivityListener = () => {
            if (this.activePageScrollScrollerId == null) {
                return;
            }
            this.activateScrollerMeasurement(this.activePageScrollScrollerId);
        };
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
        this.mount = mount instanceof HTMLElement ? mount : undefined;
        this.perfTraceEnabled = isScrollPerfTraceEnabled();
        this.layers.attach(mount, this.canvases);
    }

    hasActivePageScrollDelegation() {
        return this.activePageScrollScrollerId != null;
    }

    sampleFrameInputs() {
        this.scrollerMeasurementStates.forEach((state, id) => {
            let leaf = this.nodesLookup.get(id);
            if (leaf == null) {
                this.scrollerMeasurementStates.delete(id);
                return;
            }
            if (!state.active && state.stableFrames >= 2 && !isIOSWebKitBrowser()) {
                return;
            }

            let measurement = this.measureScrollerPosition(leaf, state);
            let moved =
                Math.abs(measurement.scrollX - state.lastMeasuredScrollX) > 0.1
                || Math.abs(measurement.scrollY - state.lastMeasuredScrollY) > 0.1
                || Math.abs(measurement.presentationScrollX - state.lastMeasuredPresentationScrollX) > 0.1
                || Math.abs(measurement.presentationScrollY - state.lastMeasuredPresentationScrollY) > 0.1;

            state.lastMeasuredScrollX = measurement.scrollX;
            state.lastMeasuredScrollY = measurement.scrollY;
            state.lastMeasuredPresentationScrollX = measurement.presentationScrollX;
            state.lastMeasuredPresentationScrollY = measurement.presentationScrollY;

            if (state.active || moved) {
                this.emitScrollerPosition(id, measurement, state);
            }

            state.stableFrames = moved ? 0 : state.stableFrames + 1;
            if (state.stableFrames >= 2) {
                state.active = false;
            }
        });
    }

    private applyLeafPlacement(
        leaf: HTMLElement,
        patch: { parentFrame?: number | null; zIndex?: number | null },
    ) {
        if (patch.parentFrame !== undefined) {
            if (this.layers.addElement(leaf, patch.parentFrame ?? undefined, 0)) {
                this.surfaceRefreshPending = true;
            }
        }
        if (patch.zIndex != null) {
            leaf.style.zIndex = patch.zIndex.toString();
            const focusableElements = leaf.querySelectorAll('input, button, select, textarea, a[href]');
            focusableElements.forEach((element) => {
                element.setAttribute('tabindex', (1000000 - patch.zIndex!).toString());
            });
        }
    }

    private applyContainerPlacement(
        id: number,
        patch: { parentFrame?: number | null },
    ) {
        if (patch.parentFrame !== undefined) {
            this.layers.updateContainerParent(id, patch.parentFrame ?? undefined);
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
        this.applyLeafPlacement(leaf!, patch);
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
        this.applyLeafPlacement(leaf!, patch);
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
        this.applyLeafPlacement(leaf!, patch);
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
        this.applyLeafPlacement(leaf!, patch);
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
        this.applyLeafPlacement(leaf!, patch);
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
        this.applyLeafPlacement(leaf!, patch);
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
        this.applyLeafPlacement(leaf!, patch);
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
        this.applyLeafPlacement(leaf!, patch);
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
        this.applyLeafPlacement(leaf, patch);
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
        this.updatePresentationRecord(
            patch.id!,
            patch.presentedBounds,
            patch.presentedClipBounds,
        );
        this.applyContainerPlacement(patch.id!, patch);

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
        this.presentationRecords.delete(id);
        this.layers.removeContainer(id);
    }

    private getScrollerIdFromLeaf(leaf: HTMLElement) {
        let id = Number.parseInt(leaf.getAttribute("pax_id") ?? "", 10);
        return Number.isFinite(id) ? id : undefined;
    }

    private getScrollerInnerPane(leaf: HTMLElement) {
        let scrollerId = this.getScrollerIdFromLeaf(leaf);
        let hosts = scrollerId != null ? this.scrollerHosts.get(scrollerId) : undefined;
        return hosts?.innerPane
            ?? (leaf.querySelector(`:scope > .${INNER_PANE}`) as HTMLElement | null);
    }

    private getScrollerCanvasHost(leaf: HTMLElement) {
        let scrollerId = this.getScrollerIdFromLeaf(leaf);
        let hosts = scrollerId != null ? this.scrollerHosts.get(scrollerId) : undefined;
        return hosts?.canvasHost
            ?? (leaf.querySelector(`:scope > .${INNER_PANE} > [data-role="scroller-canvas-host"]`) as HTMLElement | null);
    }

    private getScrollerContentHost(leaf: HTMLElement) {
        let scrollerId = this.getScrollerIdFromLeaf(leaf);
        let hosts = scrollerId != null ? this.scrollerHosts.get(scrollerId) : undefined;
        return hosts?.contentHost
            ?? (leaf.querySelector(`:scope > .${INNER_PANE} > [data-role="scroller-content-host"]`) as HTMLElement | null);
    }

    private shouldKeepScrollerHostsActive(leaf: HTMLElement, state: ScrollerMeasurementState) {
        if (typeof window === "undefined") {
            return true;
        }
        if (state.isRootScroller || state.delegatesToPageScroll) {
            return true;
        }
        let scrollerId = this.getScrollerIdFromLeaf(leaf);
        if (scrollerId == null) {
            return true;
        }
        let record = this.getPresentationRecordForLeaf(scrollerId, leaf);
        let horizontalScrollable = leaf.scrollWidth > leaf.clientWidth + 0.5;
        let verticalScrollable = leaf.scrollHeight > leaf.clientHeight + 0.5;
        let paddedClipBounds = expandRect(
            record.presentedClipBounds,
            horizontalScrollable
                ? Math.max(
                    leaf.clientWidth * ACTIVE_SCROLLABLE_PAD_X_MULTIPLIER,
                    ACTIVE_SCROLLABLE_MIN_PAD_X,
                )
                : 120,
            verticalScrollable
                ? Math.max(
                    leaf.clientHeight * ACTIVE_SCROLLABLE_PAD_Y_MULTIPLIER,
                    ACTIVE_SCROLLABLE_MIN_PAD_Y,
                )
                : 120,
        );
        return rectsIntersect(record.presentedBounds, paddedClipBounds);
    }

    private getPresentationRecordForLeaf(
        scrollerId: number,
        leaf: HTMLElement,
    ): { presentedBounds: AxisAlignedRect; presentedClipBounds: AxisAlignedRect } {
        let record = this.presentationRecords.get(scrollerId);
        if (record?.presentedBounds && record.presentedClipBounds) {
            return {
                presentedBounds: record.presentedBounds,
                presentedClipBounds: record.presentedClipBounds,
            };
        }
        let rect = leaf.getBoundingClientRect();
        let viewport = {
            left: 0,
            top: 0,
            right: window.innerWidth ?? rect.right,
            bottom: window.innerHeight ?? rect.bottom,
        };
        let clipBounds = viewport;
        if (isIOSWebKitBrowser()) {
            let node: HTMLElement | null = leaf;
            while (node != null) {
                if (node.classList.contains(SCROLLER_CONTAINER)) {
                    let nodeRect = node.getBoundingClientRect();
                    clipBounds = intersectRects(clipBounds, {
                        left: nodeRect.left,
                        top: nodeRect.top,
                        right: nodeRect.right,
                        bottom: nodeRect.bottom,
                    });
                }
                if (node === this.mount) {
                    break;
                }
                node = node.parentElement;
            }
        }
        return {
            presentedBounds: {
                left: rect.left,
                top: rect.top,
                right: rect.right,
                bottom: rect.bottom,
            },
            presentedClipBounds: clipBounds,
        };
    }

    private updatePresentationRecord(
        id: number,
        presentedBounds?: number[] | null,
        presentedClipBounds?: number[] | null,
    ) {
        let record = this.presentationRecords.get(id) ?? {};
        if (presentedBounds != null) {
            record.presentedBounds = rectFromMessageBounds(presentedBounds);
        }
        if (presentedClipBounds != null) {
            record.presentedClipBounds = rectFromMessageBounds(presentedClipBounds);
        }
        this.presentationRecords.set(id, record);
    }

    private parkScrollerHosts(hosts: ScrollerDomHosts) {
        if (hosts.parked) {
            return;
        }
        hosts.parked = true;
        hosts.canvasHost.dataset.renderState = "parked";
        hosts.contentHost.dataset.renderState = "parked";
        hosts.canvasHost.style.visibility = "hidden";
        hosts.contentHost.style.visibility = "hidden";
        this.surfaceRefreshPending = true;
        this.logScrollPerf(`park id=${hosts.id}`);
    }

    private unparkScrollerHosts(hosts: ScrollerDomHosts) {
        if (!hosts.parked) {
            return;
        }
        hosts.parked = false;
        hosts.canvasHost.dataset.renderState = "active";
        hosts.contentHost.dataset.renderState = "active";
        hosts.canvasHost.style.visibility = "";
        hosts.contentHost.style.visibility = "";
        this.surfaceRefreshPending = true;
        this.logScrollPerf(`unpark id=${hosts.id}`);
    }

    private promoteScrollerHosts(hosts: ScrollerDomHosts) {
        if (hosts.warmed) {
            return false;
        }
        hosts.warmed = true;
        hosts.canvasHost.dataset.warmState = "warm";
        this.surfaceRefreshPending = true;
        return true;
    }

    private demoteScrollerHosts(hosts: ScrollerDomHosts) {
        this.parkScrollerHosts(hosts);
        if (!hosts.warmed) {
            return false;
        }
        hosts.warmed = false;
        hosts.canvasHost.dataset.warmState = "cold";
        this.surfaceRefreshPending = true;
        return true;
    }

    private estimateScrollerWarmSurfaceCost(hosts: ScrollerDomHosts, state: ScrollerMeasurementState) {
        let layerIds = this.layers.estimateScrollerIslandOwnedLayers(
            hosts.id,
            state.contentLayerId,
        );
        if (layerIds.length === 0) {
            return 1;
        }
        return Math.max(
            1,
            layerIds.reduce((total, layerId) => {
                return total + Math.max(1, estimateWarmLayerCanvasCount(layerId, hosts.canvasHost));
            }, 0),
        );
    }

    private collectPrewarmCandidates() {
        let candidates: Array<{ id: number; distance: number }> = [];
        this.scrollerHosts.forEach((hosts, id) => {
            if (hosts.warmed) {
                return;
            }
            let state = this.scrollerMeasurementStates.get(id);
            if (state == null || state.isRootScroller || state.delegatesToPageScroll) {
                return;
            }
            let record = this.getPresentationRecordForLeaf(id, hosts.leaf);
            let horizontalScrollable = hosts.leaf.scrollWidth > hosts.leaf.clientWidth + 0.5;
            let verticalScrollable = hosts.leaf.scrollHeight > hosts.leaf.clientHeight + 0.5;
        let prewarmClipBounds = expandRect(
            record.presentedClipBounds,
            horizontalScrollable
                ? Math.max(
                    hosts.leaf.clientWidth * PREWARM_SCROLLABLE_PAD_X_MULTIPLIER,
                    PREWARM_SCROLLABLE_MIN_PAD_X,
                )
                : 240,
            verticalScrollable
                ? Math.max(
                    hosts.leaf.clientHeight * PREWARM_SCROLLABLE_PAD_Y_MULTIPLIER,
                    PREWARM_SCROLLABLE_MIN_PAD_Y,
                )
                : 240,
        );
        candidates.push({
            id,
            distance: rectCenterDistance(record.presentedBounds, record.presentedClipBounds),
        });
    });
        candidates.sort((left, right) => left.distance - right.distance || left.id - right.id);
        return candidates;
    }

    private syncWarmScrollerHosts() {
        let promotedIds: number[] = [];
        let demotedIds: number[] = [];
        let nestedSurfaceBudget = browserNestedScrollerSurfaceBudget();
        let totalCanvasBudget = browserTotalCanvasBudget();
        if (nestedSurfaceBudget != null) {
            let candidates: Array<{
                id: number;
                hosts: ScrollerDomHosts;
                shouldWarm: boolean;
                distance: number;
                surfaceCost: number;
                mandatory: boolean;
            }> = [];
            let rootCanvasCost = 0;
            if (totalCanvasBudget != null && this.mount != null) {
                rootCanvasCost = Math.max(1, estimateWarmLayerCanvasCount(0, this.mount));
            }
            let mandatoryCost = 0;
            this.scrollerHosts.forEach((hosts, id) => {
                let state = this.scrollerMeasurementStates.get(id);
                if (state == null) {
                    this.unparkScrollerHosts(hosts);
                    return;
                }
                if (!hosts.vectorIslandEnabled) {
                    this.unparkScrollerHosts(hosts);
                    return;
                }
                let mandatory = state.isRootScroller || state.delegatesToPageScroll;
                let shouldWarm = mandatory
                    ? true
                    : this.shouldKeepScrollerHostsActive(hosts.leaf, state);
                let record = this.getPresentationRecordForLeaf(id, hosts.leaf);
                let horizontalScrollable = hosts.leaf.scrollWidth > hosts.leaf.clientWidth + 0.5;
                let verticalScrollable = hosts.leaf.scrollHeight > hosts.leaf.clientHeight + 0.5;
                let prewarmClipBounds = expandRect(
                    record.presentedClipBounds,
                    horizontalScrollable
                        ? Math.max(
                            hosts.leaf.clientWidth * PREWARM_SCROLLABLE_PAD_X_MULTIPLIER,
                            PREWARM_SCROLLABLE_MIN_PAD_X,
                        )
                        : 240,
                    verticalScrollable
                        ? Math.max(
                            hosts.leaf.clientHeight * PREWARM_SCROLLABLE_PAD_Y_MULTIPLIER,
                            PREWARM_SCROLLABLE_MIN_PAD_Y,
                        )
                        : 240,
                );
                let surfaceCost = this.estimateScrollerWarmSurfaceCost(hosts, state);
                if (mandatory) {
                    mandatoryCost += surfaceCost;
                }
                candidates.push({
                    id,
                    hosts,
                    shouldWarm,
                    distance: rectCenterDistance(record.presentedBounds, record.presentedClipBounds),
                    surfaceCost,
                    mandatory,
                });
            });
            candidates.sort((left, right) => {
                let rankDelta = Number(left.shouldWarm) - Number(right.shouldWarm);
                if (rankDelta !== 0) {
                    return -rankDelta;
                }
                // On iOS, keep the warm set tied to proximity so later scrollers can take over
                // a limited surface budget as the user advances through a long list.
                let distanceDelta = left.distance - right.distance;
                if (distanceDelta !== 0) {
                    return distanceDelta;
                }
                let warmedDelta = Number(left.hosts.warmed) - Number(right.hosts.warmed);
                if (warmedDelta !== 0) {
                    return -warmedDelta;
                }
                return left.id - right.id;
            });
            let selected = new Set<number>();
            let remainingBudget = nestedSurfaceBudget;
            if (totalCanvasBudget != null) {
                remainingBudget = Math.min(
                    remainingBudget,
                    Math.max(0, totalCanvasBudget - rootCanvasCost),
                );
            }
            remainingBudget = Math.max(0, remainingBudget - mandatoryCost);
            for (let candidate of candidates) {
                if (!candidate.mandatory) {
                    continue;
                }
                selected.add(candidate.id);
            }
            for (let candidate of candidates) {
                if (candidate.mandatory) {
                    continue;
                }
                let cost = Math.max(1, candidate.surfaceCost);
                let canAfford = remainingBudget >= cost;
                let mustKeep = totalCanvasBudget == null
                    && candidate.shouldWarm
                    && selected.size === 0;
                if (!canAfford && !mustKeep) {
                    continue;
                }
                selected.add(candidate.id);
                remainingBudget = Math.max(0, remainingBudget - cost);
            }
            for (let candidate of candidates) {
                if (selected.has(candidate.id)) {
                    if (this.promoteScrollerHosts(candidate.hosts)) {
                        promotedIds.push(candidate.id);
                    }
                    if (candidate.shouldWarm || candidate.mandatory) {
                        this.unparkScrollerHosts(candidate.hosts);
                    } else {
                        this.parkScrollerHosts(candidate.hosts);
                    }
                } else if (this.demoteScrollerHosts(candidate.hosts)) {
                    demotedIds.push(candidate.id);
                }
            }
            if (promotedIds.length > 0) {
                this.logScrollPerf(`promote ids=${promotedIds.join(",")}`);
            }
            if (demotedIds.length > 0) {
                this.logScrollPerf(`demote ids=${demotedIds.join(",")}`);
            }
            this.logWarmScrollerLifecycleSummary();
            return;
        }
        this.scrollerHosts.forEach((hosts, id) => {
            let state = this.scrollerMeasurementStates.get(id);
            if (state == null) {
                this.unparkScrollerHosts(hosts);
                return;
            }
            if (!hosts.vectorIslandEnabled) {
                this.unparkScrollerHosts(hosts);
                return;
            }
            let shouldWarm = this.shouldKeepScrollerHostsActive(hosts.leaf, state);
            if (state.isRootScroller) {
                hosts.warmed = true;
                hosts.canvasHost.dataset.warmState = "warm";
            } else if (!hosts.warmed && shouldWarm) {
                // Grow the warm island pool on demand: the first near-viewport visit upgrades a
                // nested scroller from cold to warm, and we keep that warm state for the rest of
                // the session instead of thrashing back to zero-target cold starts.
                if (this.promoteScrollerHosts(hosts)) {
                    promotedIds.push(id);
                }
            }
            if (shouldWarm) {
                this.unparkScrollerHosts(hosts);
            } else {
                this.parkScrollerHosts(hosts);
            }
        });
        let coldCandidates = this.collectPrewarmCandidates();
        let prewarmBudget =
            coldCandidates.length <= EAGER_WARM_SCROLLER_LIMIT
                ? coldCandidates.length
                : Math.min(PREWARM_PROMOTION_BUDGET, coldCandidates.length);
        for (let index = 0; index < prewarmBudget; index += 1) {
            let candidate = coldCandidates[index];
            let hosts = this.scrollerHosts.get(candidate.id);
            if (hosts != null && this.promoteScrollerHosts(hosts)) {
                promotedIds.push(candidate.id);
            }
        }
        if (promotedIds.length > 0) {
            this.logScrollPerf(`promote ids=${promotedIds.join(",")}`);
        }
        this.logWarmScrollerLifecycleSummary();
    }

    syncRenderSurfaceLayouts() {
        this.syncDelegatedPageScrollViewport();
        this.withProfileMeasure("syncWarmScrollerHosts", () => {
            this.syncWarmScrollerHosts();
        });
        this.withProfileMeasure("syncLayerCanvasLayouts", () => {
            this.layers.syncLayerCanvasLayouts();
        });
        this.logWarmCanvasSummary();
        let surfaceChanged = this.surfaceRefreshPending;
        let nextSurfaceSignatures = new Map<string, string>();
        let nextTransformSignatures = new Map<string, string>();
        let nextLayerCounts = new Map<number, number>();
        let surfaceChangedLayers = new Set<number>();
        let transformChangedLayers = new Set<number>();
        let transformChanged = false;
        this.withProfileMeasure("scanCanvasSurfaceSignatures", () => {
            this.canvases.forEach((canvas, id) => {
                let parent = canvas.parentElement;
                if (parent == null) {
                    return;
                }
                let hostSignature = describeSurfaceHost(parent);
                let layerId = Number.parseInt(canvas.dataset.layerId ?? "", 10);
                let surfaceSignature =
                    `${canvas.dataset.surfaceSignature
                        ?? `${canvas.clientWidth}x${canvas.clientHeight}@${hostSignature}`}#host=${hostSignature}`;
                let transformSignature =
                    canvas.dataset.transformSignature
                    ?? `${canvas.dataset.tileOriginX ?? "0"},${canvas.dataset.tileOriginY ?? "0"}`;
                nextSurfaceSignatures.set(id, surfaceSignature);
                nextTransformSignatures.set(id, transformSignature);
                if (Number.isFinite(layerId)) {
                    nextLayerCounts.set(layerId, (nextLayerCounts.get(layerId) ?? 0) + 1);
                }
                if (this.lastCanvasSurfaceSignatures.get(id) !== surfaceSignature) {
                    surfaceChanged = true;
                    if (Number.isFinite(layerId)) {
                        surfaceChangedLayers.add(layerId);
                    }
                }
                if (this.lastCanvasTransformSignatures.get(id) !== transformSignature) {
                    transformChanged = true;
                    if (Number.isFinite(layerId)) {
                        transformChangedLayers.add(layerId);
                    }
                }
            });
        });
        if (nextSurfaceSignatures.size !== this.lastCanvasSurfaceSignatures.size) {
            surfaceChanged = true;
        }
        if (nextTransformSignatures.size !== this.lastCanvasTransformSignatures.size) {
            transformChanged = true;
        }
        let layerCountsChanged = false;
        this.lastCanvasLayerCounts.forEach((count, layerId) => {
            if ((nextLayerCounts.get(layerId) ?? 0) !== count) {
                surfaceChangedLayers.add(layerId);
                layerCountsChanged = true;
            }
        });
        nextLayerCounts.forEach((count, layerId) => {
            if ((this.lastCanvasLayerCounts.get(layerId) ?? 0) !== count) {
                surfaceChangedLayers.add(layerId);
                layerCountsChanged = true;
            }
        });
        if (layerCountsChanged) {
            surfaceChanged = true;
        }
        if (this.surfaceRefreshPending && surfaceChangedLayers.size === 0) {
            nextLayerCounts.forEach((_count, layerId) => {
                surfaceChangedLayers.add(layerId);
            });
        }
        this.lastCanvasSurfaceSignatures = nextSurfaceSignatures;
        this.lastCanvasTransformSignatures = nextTransformSignatures;
        this.lastCanvasLayerCounts = nextLayerCounts;
        if (surfaceChanged) {
            // Browser-owned scroller hosts can resize independently of the retained scene. When
            // that host signature changes, ask Rust to reconfigure the backing surfaces before the
            // next render pass.
            this.surfaceRefreshPending = false;
            let start = performance.now();
            let layers = Array.from(surfaceChangedLayers).sort((left, right) => left - right);
            if (layers.length > 0) {
                this.withProfileMeasure("refreshRenderSurfacesForLayers", () => {
                    this.chassis?.refresh_render_surfaces_for_layers(new Uint32Array(layers));
                });
            } else {
                this.withProfileMeasure("refreshRenderSurfaces", () => {
                    this.chassis?.refresh_render_surfaces();
                });
            }
            this.logScrollPerf(
                `refresh reason=surface canvases=${nextSurfaceSignatures.size} duration_ms=${(performance.now() - start).toFixed(1)}`,
            );
        } else if (transformChanged) {
            // Sliding a keyed tile slot onto a new origin reuses the same browser surface, but the
            // retained vector scene currently bakes per-node transforms/bounds against the prior
            // slot origin. Refresh only the affected logical layers so outer seam crossings do not
            // force a full-scene dirty/reconfigure pass across every warmed scroller island.
            let layers = Array.from(transformChangedLayers).sort((left, right) => left - right);
            let start = performance.now();
            if (layers.length > 0) {
                this.withProfileMeasure("refreshRenderSurfacesForLayers", () => {
                    this.chassis?.refresh_render_surfaces_for_layers(new Uint32Array(layers));
                });
            } else {
                this.withProfileMeasure("refreshRenderSurfaces", () => {
                    this.chassis?.refresh_render_surfaces();
                });
            }
            this.logScrollPerf(
                `refresh reason=transform layers=${layers.join(",")} canvases=${nextSurfaceSignatures.size} duration_ms=${(performance.now() - start).toFixed(1)}`,
            );
        }
    }

    private logWarmScrollerLifecycleSummary() {
        if (!this.perfTraceEnabled || typeof performance === "undefined") {
            return;
        }
        let cold = 0;
        let warmActive = 0;
        let warmParked = 0;
        this.scrollerHosts.forEach(hosts => {
            if (!hosts.warmed) {
                cold += 1;
            } else if (hosts.parked) {
                warmParked += 1;
            } else {
                warmActive += 1;
            }
        });
        let signature = `${cold}/${warmActive}/${warmParked}/${this.canvases.size}`;
        let now = performance.now();
        if (
            signature === this.lastPerfLifecycleSignature
            && now - this.lastPerfLifecycleLogAt < 250
        ) {
            return;
        }
        this.lastPerfLifecycleSignature = signature;
        this.lastPerfLifecycleLogAt = now;
        this.logScrollPerf(
            `lifecycle cold=${cold} warm_active=${warmActive} warm_parked=${warmParked} canvases=${this.canvases.size}`,
        );
    }

    private logWarmCanvasSummary() {
        if (!this.perfTraceEnabled || typeof performance === "undefined") {
            return;
        }
        let missing: Array<{ id: number; layer: number }> = [];
        let warmCount = 0;
        this.scrollerHosts.forEach((hosts, id) => {
            let state = this.scrollerMeasurementStates.get(id);
            if (!hosts.warmed || state?.contentLayerId == null) {
                return;
            }
            warmCount += 1;
            let count = this.countLayerCanvases(state.contentLayerId);
            if (count === 0) {
                missing.push({ id, layer: state.contentLayerId });
            }
        });
        let signature = `${warmCount}/${missing.length}`;
        let now = performance.now();
        if (
            signature === this.lastPerfCanvasSignature
            && now - this.lastPerfCanvasLogAt < 250
        ) {
            return;
        }
        this.lastPerfCanvasSignature = signature;
        this.lastPerfCanvasLogAt = now;
        let details = missing.slice(0, 6).map(entry => `${entry.id}:${entry.layer}`).join(",");
        this.logScrollPerf(
            `warm-canvases warm=${warmCount} missing=${missing.length}${details.length ? ` ids=${details}` : ""}`,
        );
    }

    private logScrollPerf(message: string) {
        if (!this.perfTraceEnabled) {
            return;
        }
        console.info(`[pax-scroll-perf] ${message}`);
    }

    private countLayerCanvases(layerId: number) {
        let total = 0;
        let layerMarker = String(layerId);
        this.canvases.forEach((canvas) => {
            if (canvas.dataset.layerId === layerMarker) {
                total += 1;
            }
        });
        return total;
    }

    private withProfileMeasure<T>(name: string, fn: () => T): T {
        if (
            !this.perfTraceEnabled
            || typeof performance === "undefined"
            || typeof performance.mark !== "function"
            || typeof performance.measure !== "function"
        ) {
            return fn();
        }
        let token = `${name}:${this.perfTraceSequence++}`;
        let startMark = `pax:${token}:start`;
        let endMark = `pax:${token}:end`;
        performance.mark(startMark);
        try {
            return fn();
        } finally {
            performance.mark(endMark);
            performance.measure(`pax:${name}`, startMark, endMark);
            performance.clearMarks(startMark);
            performance.clearMarks(endMark);
        }
    }

    private installPageScrollActivityListeners() {
        if (this.pageScrollActivityListenersInstalled) {
            return;
        }
        window.addEventListener("scroll", this.pageScrollActivityListener, { passive: true });
        window.addEventListener("wheel", this.pageScrollActivityListener, { passive: true });
        window.addEventListener("touchstart", this.pageScrollActivityListener, { passive: true });
        window.addEventListener("touchmove", this.pageScrollActivityListener, { passive: true });
        this.pageScrollActivityListenersInstalled = true;
    }

    private uninstallPageScrollActivityListeners() {
        if (!this.pageScrollActivityListenersInstalled) {
            return;
        }
        window.removeEventListener("scroll", this.pageScrollActivityListener);
        window.removeEventListener("wheel", this.pageScrollActivityListener);
        window.removeEventListener("touchstart", this.pageScrollActivityListener);
        window.removeEventListener("touchmove", this.pageScrollActivityListener);
        this.pageScrollActivityListenersInstalled = false;
    }

    private readPageScrollPosition() {
        let scrollingElement = document.scrollingElement as HTMLElement | null;
        return {
            scrollX: scrollingElement?.scrollLeft ?? window.scrollX ?? 0,
            scrollY: scrollingElement?.scrollTop ?? window.scrollY ?? 0,
        };
    }

    private readVisiblePageViewport() {
        let root = document.documentElement;
        let visualViewport = window.visualViewport;
        if (visualViewport != null) {
            return {
                width: visualViewport.width,
                height: visualViewport.height,
                offsetX: visualViewport.offsetLeft,
                offsetY: visualViewport.offsetTop,
            };
        }
        return {
            width: window.innerWidth ?? root.clientWidth ?? this.mount?.clientWidth ?? 0,
            height: window.innerHeight ?? root.clientHeight ?? this.mount?.clientHeight ?? 0,
            offsetX: 0,
            offsetY: 0,
        };
    }

    private syncDelegatedPageScrollViewport() {
        if (this.activePageScrollScrollerId == null) {
            return;
        }
        let leaf = this.nodesLookup.get(this.activePageScrollScrollerId);
        let canvasHost = leaf != null ? this.getScrollerCanvasHost(leaf) : null;
        if (leaf == null || canvasHost == null) {
            return;
        }
        // Root page-scroll delegation keeps layout pinned to the layout viewport, but the visible
        // tile window on iOS Safari still follows the browser's visual viewport as toolbars move.
        // Feed that live visible viewport into the tiled root surface host without relaying it
        // through engine viewport updates, which would relayout the scene mid-gesture.
        let visibleViewport = this.readVisiblePageViewport();
        let pageScroll = this.readPageScrollPosition();
        let approvedScrollX = pageScroll.scrollX + visibleViewport.offsetX;
        let approvedScrollY = pageScroll.scrollY + visibleViewport.offsetY;

        if (this.mount != null) {
            this.mount.dataset.viewportWidth = String(visibleViewport.width);
            this.mount.dataset.viewportHeight = String(visibleViewport.height);
            this.mount.dataset.approvedScrollX = String(approvedScrollX);
            this.mount.dataset.approvedScrollY = String(approvedScrollY);
        }

        canvasHost.dataset.viewportWidth = String(visibleViewport.width);
        canvasHost.dataset.viewportHeight = String(visibleViewport.height);
        canvasHost.dataset.approvedScrollX = String(approvedScrollX);
        canvasHost.dataset.approvedScrollY = String(approvedScrollY);
    }

    private setPageScrollPosition(nextScrollX?: number, nextScrollY?: number) {
        let current = this.readPageScrollPosition();
        let targetX = nextScrollX ?? current.scrollX;
        let targetY = nextScrollY ?? current.scrollY;
        window.scrollTo(targetX, targetY);
    }

    private setDocumentPageScrollMode(
        active: boolean,
        viewportWidth?: number,
        viewportHeight?: number,
        contentHeight?: number,
        scrollX?: number,
        scrollY?: number,
    ) {
        let root = document.documentElement;
        if (active) {
            root.style.overflow = "auto";
            root.style.height = "auto";
            document.body.style.overflow = "auto";
            document.body.style.height = "auto";
            if (this.mount != null) {
                this.mount.style.overflow = "visible";
                this.mount.style.height = `${Math.max(contentHeight ?? 0, viewportHeight ?? 0)}px`;
                this.mount.dataset.viewportWidth = String(viewportWidth ?? 0);
                this.mount.dataset.viewportHeight = String(viewportHeight ?? 0);
                this.mount.dataset.approvedScrollX = String(scrollX ?? 0);
                this.mount.dataset.approvedScrollY = String(scrollY ?? 0);
            }
        } else {
            root.style.overflow = "";
            root.style.height = "";
            document.body.style.overflow = "";
            document.body.style.height = "";
            if (this.mount != null) {
                this.mount.style.overflow = "";
                this.mount.style.height = "";
                delete this.mount.dataset.viewportWidth;
                delete this.mount.dataset.viewportHeight;
                delete this.mount.dataset.approvedScrollX;
                delete this.mount.dataset.approvedScrollY;
            }
        }
        this.layers.setRootPageScrollMode(active);
    }

    private setLeafPageScrollMode(leaf: HTMLElement, active: boolean) {
        if (active) {
            leaf.dataset.pageScrollDelegated = "true";
            leaf.style.overflowX = "visible";
            leaf.style.overflowY = "visible";
            // Root page-scroll delegation needs the browser to account for overflow outside the
            // viewport-sized scroller host, so we intentionally relax paint containment here.
            leaf.style.contain = "layout style";
            leaf.style.willChange = "auto";
        } else if (leaf.dataset.pageScrollDelegated === "true") {
            delete leaf.dataset.pageScrollDelegated;
            leaf.style.contain = "";
            leaf.style.willChange = "";
        }
    }

    private setPageScrollDelegation(
        scrollerId: number,
        leaf: HTMLElement,
        state: ScrollerMeasurementState,
        active: boolean,
        viewportWidth: number,
        viewportHeight: number,
        contentHeight: number,
        approvedScrollX: number,
        approvedScrollY: number,
    ) {
        if (state.delegatesToPageScroll === active && this.activePageScrollScrollerId === (active ? scrollerId : undefined)) {
            this.setLeafPageScrollMode(leaf, active);
            if (active) {
                this.setDocumentPageScrollMode(
                    true,
                    viewportWidth,
                    viewportHeight,
                    contentHeight,
                    approvedScrollX,
                    approvedScrollY,
                );
            }
            return;
        }

        if (!active && this.activePageScrollScrollerId === scrollerId) {
            let current = this.readPageScrollPosition();
            leaf.scrollLeft = current.scrollX;
            leaf.scrollTop = current.scrollY;
            window.scrollTo(0, 0);
        }

        if (active) {
            if (this.activePageScrollScrollerId != null && this.activePageScrollScrollerId !== scrollerId) {
                let previousLeaf = this.nodesLookup.get(this.activePageScrollScrollerId);
                let previousState = this.scrollerMeasurementStates.get(this.activePageScrollScrollerId);
                if (previousLeaf != null && previousState != null) {
                    previousState.delegatesToPageScroll = false;
                    this.setLeafPageScrollMode(previousLeaf, false);
                }
            }
            this.activePageScrollScrollerId = scrollerId;
            this.installPageScrollActivityListeners();
            this.setDocumentPageScrollMode(
                true,
                viewportWidth,
                viewportHeight,
                contentHeight,
                approvedScrollX,
                approvedScrollY,
            );
        } else if (this.activePageScrollScrollerId === scrollerId) {
            this.activePageScrollScrollerId = undefined;
            this.uninstallPageScrollActivityListeners();
            this.setDocumentPageScrollMode(false);
        }

        state.delegatesToPageScroll = active;
        this.setLeafPageScrollMode(leaf, active);
        this.activateScrollerMeasurement(scrollerId);
    }

    private shouldDelegateToPageScroll(
        leaf: HTMLElement,
        patch: ScrollerUpdatePatch,
        state: ScrollerMeasurementState,
    ) {
        if (!supportsPageScrollDelegation()) {
            return false;
        }
        if (!state.isRootScroller) {
            return false;
        }
        if (state.delegatesToPageScroll) {
            // Scroller updates are delta-shaped, so fields like contentLayerId may be omitted on
            // ordinary scroll ticks. Once the root scroller has been promoted to page-scroll
            // delegation, keep that ownership stable unless the scroller is recreated.
            return true;
        }
        if (state.contentLayerId == null) {
            return false;
        }
        let shouldClip = patch.clipContent ?? true;
        if (!shouldClip) {
            return false;
        }
        let transform = patch.transform ?? state.transform;
        if (!isViewportAnchoredTransform(transform)) {
            return false;
        }
        let viewportWidth = (patch.sizeX ?? leaf.clientWidth) || 0;
        let viewportHeight = (patch.sizeY ?? leaf.clientHeight) || 0;
        let root = document.documentElement;
        let targetWidth = window.innerWidth ?? root.clientWidth ?? this.mount?.clientWidth ?? 0;
        let targetHeight = window.innerHeight ?? root.clientHeight ?? this.mount?.clientHeight ?? 0;
        return nearlyEqual(viewportWidth, targetWidth) && nearlyEqual(viewportHeight, targetHeight);
    }

    private emitScrollerPosition(
        id: number,
        measurement: ScrollerMeasurement,
        state?: ScrollerMeasurementState,
    ) {
        if (
            state != null
            && Math.abs(state.lastSentScrollX - measurement.scrollX) <= 0.1
            && Math.abs(state.lastSentScrollY - measurement.scrollY) <= 0.1
            && Math.abs(state.lastSentPresentationScrollX - measurement.presentationScrollX) <= 0.1
            && Math.abs(state.lastSentPresentationScrollY - measurement.presentationScrollY) <= 0.1
        ) {
            return;
        }
        if (state != null) {
            state.lastSentScrollX = measurement.scrollX;
            state.lastSentScrollY = measurement.scrollY;
            state.lastSentPresentationScrollX = measurement.presentationScrollX;
            state.lastSentPresentationScrollY = measurement.presentationScrollY;
        }
        let message = {
            "Scrollbar": {
                "id": id,
                "scroll_x": measurement.scrollX,
                "scroll_y": measurement.scrollY,
                "presentation_scroll_x": measurement.presentationScrollX,
                "presentation_scroll_y": measurement.presentationScrollY,
            }
        };
        this.chassis!.interrupt(JSON.stringify(message), undefined);
    }

    private activateScrollerMeasurement(id: number) {
        let state = this.scrollerMeasurementStates.get(id);
        if (state == null) {
            return;
        }
        state.active = true;
        state.stableFrames = 0;
    }

    private queueScrollerUpdate(patch: ScrollerUpdatePatch) {
        if (patch.id == null) {
            return;
        }
        let queued = this.pendingScrollerUpdates.get(patch.id) ?? { id: patch.id };
        if (patch.sizeX != null) {
            queued.sizeX = patch.sizeX;
        }
        if (patch.sizeY != null) {
            queued.sizeY = patch.sizeY;
        }
        if (patch.clipContent != null) {
            queued.clipContent = patch.clipContent;
        }
        if (patch.sizeInnerPaneX != null) {
            queued.sizeInnerPaneX = patch.sizeInnerPaneX;
        }
        if (patch.sizeInnerPaneY != null) {
            queued.sizeInnerPaneY = patch.sizeInnerPaneY;
        }
        if (patch.transform != null) {
            queued.transform = [...patch.transform];
        }
        if (patch.opacity != null) {
            queued.opacity = patch.opacity;
        }
        if (patch.scrollX != null) {
            queued.scrollX = patch.scrollX;
        }
        if (patch.scrollY != null) {
            queued.scrollY = patch.scrollY;
        }
        if (patch.presentationScrollX != null) {
            queued.presentationScrollX = patch.presentationScrollX;
        }
        if (patch.presentationScrollY != null) {
            queued.presentationScrollY = patch.presentationScrollY;
        }
        if (patch.contentLayerId != null) {
            queued.contentLayerId = patch.contentLayerId;
        }
        if (patch.presentedBounds != null) {
            queued.presentedBounds = [...patch.presentedBounds];
        }
        if (patch.presentedClipBounds != null) {
            queued.presentedClipBounds = [...patch.presentedClipBounds];
        }
        this.pendingScrollerUpdates.set(patch.id, queued);
    }

    private replayPendingScrollerUpdate(id: number) {
        let queued = this.pendingScrollerUpdates.get(id);
        if (queued == null) {
            return;
        }
        this.pendingScrollerUpdates.delete(id);
        this.scrollerUpdate(queued);
    }

    private measureScrollerPosition(
        leaf: HTMLElement,
        state: ScrollerMeasurementState,
    ): ScrollerMeasurement {
        if (state.delegatesToPageScroll) {
            let pageScroll = this.readPageScrollPosition();
            return {
                scrollX: pageScroll.scrollX,
                scrollY: pageScroll.scrollY,
                presentationScrollX: pageScroll.scrollX,
                presentationScrollY: pageScroll.scrollY,
            };
        }
        let scrollX = leaf.scrollLeft;
        let scrollY = leaf.scrollTop;
        let presentationScrollX = scrollX;
        let presentationScrollY = scrollY;
        return {
            scrollX,
            scrollY,
            presentationScrollX,
            presentationScrollY,
        };
    }

    scrollerCreate(patch: AnyCreatePatch){
        console.assert(patch.id != null);
        console.assert(patch.occlusionLayerId != null);

        let scrollerDiv: HTMLDivElement = this.objectManager.getFromPool(DIV);
        let innerPane: HTMLDivElement = this.objectManager.getFromPool(DIV);
        let canvasHost: HTMLDivElement = this.objectManager.getFromPool(DIV);
        let contentHost: HTMLDivElement = this.objectManager.getFromPool(DIV);
        let scrollerId = patch.id!;
        let vectorIslandEnabled = browserOwnedVectorScrollerIslandsEnabled();
        innerPane.setAttribute("class", INNER_PANE);
        canvasHost.dataset.role = "scroller-canvas-host";
        canvasHost.dataset.scrollerId = String(scrollerId);
        canvasHost.style.position = "absolute";
        canvasHost.style.top = "0";
        canvasHost.style.left = "0";
        canvasHost.style.width = "100%";
        canvasHost.style.height = "100%";
        canvasHost.style.pointerEvents = "none";
        canvasHost.style.overflow = "visible";
        canvasHost.style.zIndex = "0";
        canvasHost.dataset.viewportWidth = "0";
        canvasHost.dataset.viewportHeight = "0";
        canvasHost.dataset.approvedScrollX = "0";
        canvasHost.dataset.approvedScrollY = "0";
        canvasHost.dataset.warmState = patch.parentFrame == null ? "warm" : "cold";
        canvasHost.dataset.renderState = "active";
        contentHost.dataset.role = "scroller-content-host";
        contentHost.dataset.scrollerId = String(scrollerId);
        contentHost.dataset.renderState = "active";
        contentHost.style.position = "absolute";
        contentHost.style.top = "0";
        contentHost.style.left = "0";
        contentHost.style.width = "100%";
        contentHost.style.height = "100%";
        contentHost.style.transformOrigin = "top left";
        contentHost.style.pointerEvents = "none";
        // Keep the native overlay host above the canvas host inside browser-owned scroller
        // islands. Individual layer canvases/native elements can still order locally within those
        // hosts, but the hosts themselves should never invert.
        contentHost.style.zIndex = "1";
        let scheduleMeasurement = () => this.activateScrollerMeasurement(scrollerId);
        scrollerDiv.addEventListener("scroll", scheduleMeasurement, { passive: true });
        scrollerDiv.addEventListener("wheel", scheduleMeasurement, { passive: true });
        scrollerDiv.addEventListener("touchmove", scheduleMeasurement, { passive: true });
        scrollerDiv.addEventListener("touchstart", scheduleMeasurement, { passive: true });

        innerPane.appendChild(canvasHost);
        innerPane.appendChild(contentHost);
        scrollerDiv.appendChild(innerPane);
        scrollerDiv.setAttribute("class", NATIVE_LEAF_CLASS + " " + SCROLLER_CONTAINER)
        scrollerDiv.setAttribute("pax_id", String(patch.id));
        // Blink can cull descendant browser-owned canvases inside a paint-contained scrolling leaf
        // even while the WebGPU/WebGL surfaces continue rendering. Scrollers that host island
        // canvases still need layout containment, but relaxing paint containment keeps nested
        // vector layers composited correctly in Chrome.
        scrollerDiv.style.contain = "layout style";


        if(patch.id != undefined && patch.occlusionLayerId != undefined){
            this.layers.addElement(scrollerDiv, patch.parentFrame, patch.occlusionLayerId);
            this.layers.registerParentFrameHost(patch.id, contentHost);
            if (
                vectorIslandEnabled
                && this.layers.registerScrollerIslandHosts(patch.id, canvasHost, contentHost)
            ) {
                this.surfaceRefreshPending = true;
            }
            this.nodesLookup.set(patch.id!, scrollerDiv);
            this.scrollerHosts.set(patch.id, {
                id: patch.id,
                leaf: scrollerDiv,
                innerPane,
                canvasHost,
                contentHost,
                parked: false,
                warmed: patch.parentFrame == null,
                vectorIslandEnabled,
            });
            this.scrollerMeasurementStates.set(patch.id, {
                active: true,
                stableFrames: 0,
                isRootScroller: patch.parentFrame == null,
                delegatesToPageScroll: false,
                contentLayerId: undefined,
                transform: [1, 0, 0, 1, 0, 0],
                lastMeasuredScrollX: scrollerDiv.scrollLeft,
                lastMeasuredScrollY: scrollerDiv.scrollTop,
                lastMeasuredPresentationScrollX: scrollerDiv.scrollLeft,
                lastMeasuredPresentationScrollY: scrollerDiv.scrollTop,
                lastSentScrollX: Number.NaN,
                lastSentScrollY: Number.NaN,
                lastSentPresentationScrollX: Number.NaN,
                lastSentPresentationScrollY: Number.NaN,
            });
            this.replayPendingScrollerUpdate(patch.id);
        } else {
            throw new Error("undefined id or occlusionLayer");
        }
    }

    scrollerUpdate(patch: ScrollerUpdatePatch){
        let leaf = this.nodesLookup.get(patch.id!);
        // Ordering sometimes result in updates being sent after deletes.
        // could fix this ordering, but simply "skipping" this works for now.
        if (leaf == undefined) {
            this.queueScrollerUpdate(patch);
            return;
        }
        let scrollerInner = this.getScrollerInnerPane(leaf);
        let canvasHost = this.getScrollerCanvasHost(leaf);
        let contentHost = this.getScrollerContentHost(leaf);
        if (scrollerInner == null || canvasHost == null || contentHost == null) {
            return;
        }
        this.updatePresentationRecord(
            patch.id!,
            patch.presentedBounds,
            patch.presentedClipBounds,
        );
        this.applyLeafPlacement(leaf, patch);
        // Scrollers expose a dedicated descendant host via `registerParentFrameHost(...)` rather
        // than a layer-manager container record. Reparenting the scroller leaf is sufficient to
        // move that host with it; trying to route scroller placement through container-parent
        // updates trips the frame-only container API.

        // Handle size_x and size_y
        if (patch.sizeX != null) {
            leaf.style.width = patch.sizeX + "px";
        }
        if (patch.sizeY != null) {
            leaf.style.height = patch.sizeY + "px";
        }

        if (patch.sizeInnerPaneX != null) {
            scrollerInner.style.width = patch.sizeInnerPaneX + "px";
            canvasHost.style.width = patch.sizeInnerPaneX + "px";
            contentHost.style.width = patch.sizeInnerPaneX + "px";
        }
        if (patch.sizeInnerPaneY != null) {
            scrollerInner.style.height = patch.sizeInnerPaneY + "px";
            canvasHost.style.height = patch.sizeInnerPaneY + "px";
            contentHost.style.height = patch.sizeInnerPaneY + "px";
        }

        const shouldClip = patch.clipContent ?? true;
        const viewportWidth = (patch.sizeX ?? parseFloat(leaf.style.width)) || 0;
        const viewportHeight = (patch.sizeY ?? parseFloat(leaf.style.height)) || 0;
        const contentWidth = (patch.sizeInnerPaneX ?? parseFloat(scrollerInner.style.width)) || 0;
        const contentHeight = (patch.sizeInnerPaneY ?? parseFloat(scrollerInner.style.height)) || 0;
        let state = this.scrollerMeasurementStates.get(patch.id!);
        let hosts = this.scrollerHosts.get(patch.id!);
        canvasHost.dataset.viewportWidth = String(viewportWidth);
        canvasHost.dataset.viewportHeight = String(viewportHeight);
        // Scroller patches are delta-shaped. Remember the resolved content layer once it arrives so
        // later scroll-only updates can keep root page-scroll delegation active.
        if (state != null && patch.contentLayerId != null) {
            state.contentLayerId = patch.contentLayerId;
        }
        if (hosts?.vectorIslandEnabled && patch.contentLayerId != null) {
            let claimed = this.layers.claimLayerForScrollerIsland(patch.contentLayerId, patch.id!);
            if (claimed) {
                this.surfaceRefreshPending = true;
            }
        }
        let delegateToPageScroll = state != null && this.shouldDelegateToPageScroll(leaf, patch, state);
        let approvedScrollX = patch.presentationScrollX
            ?? patch.scrollX
            ?? (state?.lastMeasuredPresentationScrollX ?? leaf.scrollLeft);
        let approvedScrollY = patch.presentationScrollY
            ?? patch.scrollY
            ?? (state?.lastMeasuredPresentationScrollY ?? leaf.scrollTop);
        if (state != null) {
            this.setPageScrollDelegation(
                patch.id!,
                leaf,
                state,
                delegateToPageScroll,
                viewportWidth,
                viewportHeight,
                contentHeight,
                approvedScrollX,
                approvedScrollY,
            );
        }
        if (!delegateToPageScroll) {
            leaf.style.overflowX = shouldClip
                ? (contentWidth > viewportWidth ? "auto" : "hidden")
                : "visible";
            leaf.style.overflowY = shouldClip
                ? (contentHeight > viewportHeight ? "auto" : "hidden")
                : "visible";
        }

        const ignoreActiveScrollPatch =
            isIOSWebKitBrowser() && !delegateToPageScroll && state?.active;
        if (patch.scrollX != null) {
            if (!ignoreActiveScrollPatch) {
                if (delegateToPageScroll) {
                    let current = this.readPageScrollPosition();
                    // During delegated root scrolling, engine scroll patches are usually an echo of the
                    // browser's own page-scroll state from the previous frame. On iOS Safari, browser
                    // chrome collapse can advance that page scroll asynchronously, so writing the stale
                    // engine value back into `window.scrollTo` creates a visible tug-of-war. Only push
                    // page scroll from the engine once the delegated scroller is idle again.
                    if (!state?.active && Math.abs(current.scrollX - patch.scrollX) > 0.5) {
                        this.setPageScrollPosition(patch.scrollX, undefined);
                    }
                    if (this.mount != null) {
                        this.mount.dataset.approvedScrollX = String(approvedScrollX);
                    }
                } else if (Math.abs(leaf.scrollLeft - patch.scrollX) > 0.5) {
                    leaf.scrollLeft = patch.scrollX;
                }
            }
            canvasHost.dataset.approvedScrollX = String(approvedScrollX);
            this.activateScrollerMeasurement(patch.id!);
        }
        if (patch.scrollY != null) {
            if (!ignoreActiveScrollPatch) {
                if (delegateToPageScroll) {
                    let current = this.readPageScrollPosition();
                    // See scrollX note above: while Safari is animating browser chrome, keep the page
                    // scroll browser-owned and only allow engine-originated corrections after motion
                    // has settled.
                    if (!state?.active && Math.abs(current.scrollY - patch.scrollY) > 0.5) {
                        this.setPageScrollPosition(undefined, patch.scrollY);
                    }
                    if (this.mount != null) {
                        this.mount.dataset.approvedScrollY = String(approvedScrollY);
                    }
                } else if (Math.abs(leaf.scrollTop - patch.scrollY) > 0.5) {
                    leaf.scrollTop = patch.scrollY;
                }
            }
            canvasHost.dataset.approvedScrollY = String(approvedScrollY);
            this.activateScrollerMeasurement(patch.id!);
        }
        if (patch.presentationScrollX != null && patch.scrollX == null) {
            canvasHost.dataset.approvedScrollX = String(approvedScrollX);
            if (delegateToPageScroll && this.mount != null) {
                this.mount.dataset.approvedScrollX = String(approvedScrollX);
            }
            this.activateScrollerMeasurement(patch.id!);
        }
        if (patch.presentationScrollY != null && patch.scrollY == null) {
            canvasHost.dataset.approvedScrollY = String(approvedScrollY);
            if (delegateToPageScroll && this.mount != null) {
                this.mount.dataset.approvedScrollY = String(approvedScrollY);
            }
            this.activateScrollerMeasurement(patch.id!);
        }
        // Handle transform
        if (patch.transform != null) {
            let state = this.scrollerMeasurementStates.get(patch.id!);
            if (state != null) {
                state.transform = [...patch.transform];
            }
            leaf.style.transform = packAffineCoeffsIntoMatrix3DString(patch.transform);
            const inverseTransform = invertAffineCoeffs(patch.transform);
            if (inverseTransform != null) {
                contentHost.style.transform = packAffineCoeffsIntoMatrix3DString(inverseTransform);
            } else {
                contentHost.style.transform = "";
            }
            this.activateScrollerMeasurement(patch.id!);
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
        let state = this.scrollerMeasurementStates.get(id);
        if (state?.delegatesToPageScroll) {
            this.setPageScrollDelegation(id, oldNode, state, false);
        }
        this.scrollerMeasurementStates.delete(id);
        this.pendingScrollerUpdates.delete(id);
        this.presentationRecords.delete(id);
        let hosts = this.scrollerHosts.get(id);
        if (this.layers.unregisterScrollerIslandHosts(
            id,
            hosts?.canvasHost ?? this.getScrollerCanvasHost(oldNode) ?? undefined,
            hosts?.contentHost ?? this.getScrollerContentHost(oldNode) ?? undefined,
        )) {
            this.surfaceRefreshPending = true;
        }
        this.layers.unregisterParentFrameHost(id, hosts?.contentHost ?? this.getScrollerContentHost(oldNode) ?? undefined);
        hosts?.canvasHost.parentElement?.removeChild(hosts.canvasHost);
        hosts?.contentHost.parentElement?.removeChild(hosts.contentHost);
        this.scrollerHosts.delete(id);
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
        this.applyLeafPlacement(leaf, patch);
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

type ScrollerMeasurement = {
    scrollX: number;
    scrollY: number;
    presentationScrollX: number;
    presentationScrollY: number;
};

type ScrollerMeasurementState = {
    active: boolean;
    stableFrames: number;
    isRootScroller: boolean;
    delegatesToPageScroll: boolean;
    // `ScrollerUpdate` messages only include contentLayerId when it changes, but root page-scroll
    // delegation needs to remember that layer identity across later scroll-only deltas.
    contentLayerId?: number;
    transform: number[];
    lastMeasuredScrollX: number;
    lastMeasuredScrollY: number;
    lastMeasuredPresentationScrollX: number;
    lastMeasuredPresentationScrollY: number;
    lastSentScrollX: number;
    lastSentScrollY: number;
    lastSentPresentationScrollX: number;
    lastSentPresentationScrollY: number;
};

type PendingScrollerUpdate = {
    id: number;
    sizeX?: number;
    sizeY?: number;
    clipContent?: boolean;
    sizeInnerPaneX?: number;
    sizeInnerPaneY?: number;
    transform?: number[];
    opacity?: number;
    scrollX?: number;
    scrollY?: number;
    presentationScrollX?: number;
    presentationScrollY?: number;
    contentLayerId?: number;
    presentedBounds?: number[];
    presentedClipBounds?: number[];
};

type ScrollerDomHosts = {
    id: number;
    leaf: HTMLElement;
    innerPane: HTMLDivElement;
    canvasHost: HTMLDivElement;
    contentHost: HTMLDivElement;
    parked: boolean;
    warmed: boolean;
    vectorIslandEnabled: boolean;
};

type AxisAlignedRect = {
    left: number;
    top: number;
    right: number;
    bottom: number;
};

type PresentationRecord = {
    presentedBounds?: AxisAlignedRect;
    presentedClipBounds?: AxisAlignedRect;
};

const EAGER_WARM_SCROLLER_LIMIT = 24;
const PREWARM_PROMOTION_BUDGET = 6;
const ACTIVE_SCROLLABLE_PAD_X_MULTIPLIER = 4.0;
const ACTIVE_SCROLLABLE_PAD_Y_MULTIPLIER = 2.0;
const ACTIVE_SCROLLABLE_MIN_PAD_X = 960;
const ACTIVE_SCROLLABLE_MIN_PAD_Y = 480;
const PREWARM_SCROLLABLE_PAD_X_MULTIPLIER = 8.0;
const PREWARM_SCROLLABLE_PAD_Y_MULTIPLIER = 4.0;
const PREWARM_SCROLLABLE_MIN_PAD_X = 1920;
const PREWARM_SCROLLABLE_MIN_PAD_Y = 960;
// WebKit's WebGL context cap is tight enough that we need a little headroom
// beyond the nominal canvas budget to avoid eviction when layers churn.
const IOS_NESTED_BROWSER_SURFACE_BUDGET = 6;
const IOS_TOTAL_CANVAS_BUDGET = 6;

function browserNestedScrollerSurfaceBudget() {
    return isIOSWebKitBrowser() ? IOS_NESTED_BROWSER_SURFACE_BUDGET : undefined;
}

function browserTotalCanvasBudget() {
    return isIOSWebKitBrowser() ? IOS_TOTAL_CANVAS_BUDGET : undefined;
}

function browserOwnedVectorScrollerIslandsEnabled() {
    return true;
}

function rectFromMessageBounds(bounds: number[]): AxisAlignedRect {
    return {
        left: bounds[0] ?? 0,
        top: bounds[1] ?? 0,
        right: bounds[2] ?? 0,
        bottom: bounds[3] ?? 0,
    };
}

function expandRect(rect: AxisAlignedRect, expandX: number, expandY: number): AxisAlignedRect {
    return {
        left: rect.left - expandX,
        top: rect.top - expandY,
        right: rect.right + expandX,
        bottom: rect.bottom + expandY,
    };
}

function intersectRects(a: AxisAlignedRect, b: AxisAlignedRect): AxisAlignedRect {
    return {
        left: Math.max(a.left, b.left),
        top: Math.max(a.top, b.top),
        right: Math.min(a.right, b.right),
        bottom: Math.min(a.bottom, b.bottom),
    };
}

function rectsIntersect(a: AxisAlignedRect, b: AxisAlignedRect): boolean {
    return !(
        a.right < b.left
        || a.bottom < b.top
        || a.left > b.right
        || a.top > b.bottom
    );
}

function rectSeparationDistance(a: AxisAlignedRect, b: AxisAlignedRect): number {
    let dx = 0;
    if (a.right < b.left) {
        dx = b.left - a.right;
    } else if (b.right < a.left) {
        dx = a.left - b.right;
    }

    let dy = 0;
    if (a.bottom < b.top) {
        dy = b.top - a.bottom;
    } else if (b.bottom < a.top) {
        dy = a.top - b.bottom;
    }

    return Math.hypot(dx, dy);
}

function rectCenterDistance(a: AxisAlignedRect, b: AxisAlignedRect): number {
    let aCenterX = (a.left + a.right) * 0.5;
    let aCenterY = (a.top + a.bottom) * 0.5;
    let bCenterX = (b.left + b.right) * 0.5;
    let bCenterY = (b.top + b.bottom) * 0.5;
    return Math.hypot(aCenterX - bCenterX, aCenterY - bCenterY);
}

function isScrollPerfTraceEnabled() {
    if (typeof window === "undefined") {
        return false;
    }
    try {
        let params = new URLSearchParams(window.location.search);
        let value = params.get("pax_scroll_perf");
        return value === "1" || value === "true";
    } catch (_err) {
        return false;
    }
}

function isViewportAnchoredTransform(transform: number[]) {
    return transform.length >= 6
        && nearlyEqual(transform[0], 1)
        && nearlyEqual(transform[1], 0)
        && nearlyEqual(transform[2], 0)
        && nearlyEqual(transform[3], 1)
        && nearlyEqual(transform[4], 0)
        && nearlyEqual(transform[5], 0);
}

function nearlyEqual(a: number, b: number, tolerance = 1.5) {
    return Math.abs(a - b) <= tolerance;
}

function supportsPageScrollDelegation() {
    if (typeof navigator === "undefined") {
        return false;
    }
    let userAgent = navigator.userAgent;
    let isiOS = /iPhone|iPad|iPod/i.test(userAgent);
    return isiOS && /AppleWebKit/i.test(userAgent) && !/CriOS|FxiOS|EdgiOS/i.test(userAgent);
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
