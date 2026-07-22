import {BUTTON_CLASS, BUTTON_TEXT_CONTAINER_CLASS,
    NATIVE_LEAF_CLASS, CHECKBOX_CLASS, RADIO_LIST_CLASS, SCROLLER_CONTAINER,
    CANVAS_CLASS, NATIVE_OVERLAY_CLASS, INNER_PANE} from "../utils/constants";
import {AnyCreatePatch} from "./messages/any-create-patch";
import snarkdown from 'snarkdown';
import {TextUpdatePatch} from "./messages/text-update-patch";
import {FrameUpdatePatch} from "./messages/frame-update-patch";
import {ScrollerUpdatePatch} from "./messages/scroller-update-patch";
import {ButtonUpdatePatch} from "./messages/button-update-patch";
import {PhotoPickerUpdatePatch} from "./messages/photo-picker-update-patch";
import {ImageLoadPatch} from "./messages/image-load-patch";
import {ContainerStyle, RenderLayerManager, setLeafLocalOpacity} from "./render-layer-context";
import {ObjectManager} from "../pools/object-manager";
import {
    IMAGE,
    INPUT,
    BUTTON,
    DIV,
    RENDER_LAYER_MANAGER,
    SELECT,
    YOUTUBE_VIDEO,
    YOUTUBE_VIDEO_UPDATE_PATCH
} from "../pools/supported-objects";
import {
    affineMultiply,
    HIDDEN_TAB_FRAME_FALLBACK_MS,
    invertAffineCoeffs,
    packAffineCoeffsIntoMatrix3DString,
    readImageToByteBuffer,
    waitForDocumentFrame,
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
import { RadioListUpdatePatch } from "./messages/radio-list-update-patch";
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
import type { LayerCanvasPlan } from "./surface-host-policy";
import { CanvasPool } from "./canvas-pool";
import { pushRouteHistoryState, serializeRouteLocation } from "../utils/route-location";

const SCREENSHOT_FONT_STYLE_ATTRIBUTE = 'data-pax-screenshot-font-style';
const SCROLLER_CHROME_STYLE_ATTRIBUTE = 'data-pax-scroller-chrome-style';
const SCREENSHOT_OVERLAY_BLACK = '#000000';
const SCREENSHOT_OVERLAY_WHITE = '#ffffff';
const TILED_SCROLLER_SURFACE_DPR = 1.0;
const IOS_BROWSER_SURFACE_DIMENSION_CAP = 2048;
const IOS_NESTED_LAYER_MIN_DPR = 0.25;


export class NativeElementPool {
    private canvases: Map<string, HTMLCanvasElement>;
    private lastCanvasSurfaceSignatures = new Map<string, string>();
    private lastCanvasTransformSignatures = new Map<string, string>();
    private lastCanvasLayerCounts = new Map<number, number>();
    private layerCanvasPlanCache = new Map<number, LayerCanvasPlan | null>();
    layers: RenderLayerManager;
    private nodesLookup = new Map<number, HTMLElement>();
    private scrollerHosts = new Map<number, ScrollerDomHosts>();
    private scrollerMeasurementStates = new Map<number, ScrollerMeasurementState>();
    private presentationRecords = new Map<number, PresentationRecord>();
    private pendingScrollerUpdates = new Map<number, PendingScrollerUpdate>();
    private surfaceRefreshPending = false;
    private lastLayerCanvasPlanGeneration?: number;
    private lastLayerCanvasPlanDevicePixelRatio?: number;
    private chassis?: PaxChassisWeb;
    private mount?: HTMLElement;
    private activePageScrollScrollerId?: number;
    private pageSnapHost?: HTMLDivElement;
    private pageSnapOwnerScrollerId?: number;
    private pageScrollActivityListenersInstalled = false;
    private readonly pageScrollActivityListener: () => void;
    private postAsyncInterruptFlush?: () => void;
    private objectManager: ObjectManager;
    private resizeObserver: ResizeObserver;
    registeredFontFaces: Set<string>;
    private canvasPool?: CanvasPool;

    constructor(objectManager: ObjectManager) {
        this.objectManager = objectManager;
        this.canvases = new Map();
        this.layers = objectManager.getFromPool(RENDER_LAYER_MANAGER, objectManager);
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
            this.chassis!.interrupt({
                "ChassisResizeRequestCollection": resize_requests,
            }, undefined);
        });
    }

    attach(chassis: PaxChassisWeb, mount: Element){
        this.chassis = chassis;
        this.mount = mount instanceof HTMLElement ? mount : undefined;
        injectScrollerChromeCss(mount.ownerDocument ?? document);
        this.canvasPool = new CanvasPool(
            this.objectManager,
            browserCanvasPoolBudget(),
            browserCanvasPoolReuseCooldownMs(),
        );
        this.canvasPool.attach(mount);
        this.layers.attach(mount, this.canvases, this.canvasPool);
    }

    setPostAsyncInterruptFlush(callback: (() => void) | undefined) {
        this.postAsyncInterruptFlush = callback;
    }

    dispose() {
        if (this.activePageScrollScrollerId != null) {
            let leaf = this.nodesLookup.get(this.activePageScrollScrollerId);
            let state = this.scrollerMeasurementStates.get(this.activePageScrollScrollerId);
            if (leaf != null && state != null) {
                this.setPageScrollDelegation(
                    this.activePageScrollScrollerId,
                    leaf,
                    state,
                    false,
                    0,
                    0,
                    0,
                    0,
                    0,
                );
            } else {
                this.activePageScrollScrollerId = undefined;
                this.uninstallPageScrollActivityListeners();
                this.setDocumentPageScrollMode(false);
            }
        } else {
            this.uninstallPageScrollActivityListeners();
            this.setDocumentPageScrollMode(false);
        }

        this.resizeObserver.disconnect();
        if (this.pageSnapHost != null) {
            while (this.pageSnapHost.firstChild) {
                this.pageSnapHost.removeChild(this.pageSnapHost.firstChild);
            }
        }
        if (this.mount != null) {
            this.mount.innerHTML = "";
        }

        this.canvases.clear();
        this.lastCanvasSurfaceSignatures.clear();
        this.lastCanvasTransformSignatures.clear();
        this.lastCanvasLayerCounts.clear();
        this.layerCanvasPlanCache.clear();
        this.nodesLookup.clear();
        this.scrollerHosts.clear();
        this.scrollerMeasurementStates.clear();
        this.presentationRecords.clear();
        this.pendingScrollerUpdates.clear();
        this.surfaceRefreshPending = false;
        this.lastLayerCanvasPlanGeneration = undefined;
        this.lastLayerCanvasPlanDevicePixelRatio = undefined;
        this.activePageScrollScrollerId = undefined;
        this.pageSnapHost = undefined;
        this.pageSnapOwnerScrollerId = undefined;
        this.postAsyncInterruptFlush = undefined;
        this.chassis = undefined;
        this.mount = undefined;
        this.canvasPool = undefined;
    }

    hasActivePageScrollDelegation() {
        return this.activePageScrollScrollerId != null;
    }

    sampleFrameInputs() {
        this.syncDelegatedPageScrollViewport();
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

            if (
                isIOSWebKitBrowser()
                && (state.active || moved)
            ) {
                state.lastWarmAt =
                    typeof performance !== "undefined" && typeof performance.now === "function"
                        ? performance.now()
                        : Date.now();
            }

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
        console.assert(patch.renderLayerId != null);
        
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
            this.chassis!.interrupt(message, undefined);
        });

        let checkbox_div: HTMLDivElement = this.objectManager.getFromPool(DIV);
        checkbox_div.appendChild(checkbox);
        checkbox_div.setAttribute("class", NATIVE_LEAF_CLASS)
        checkbox_div.setAttribute("pax_id", String(patch.id));
        if(patch.id != undefined && patch.renderLayerId != undefined){
            this.layers.addElement(checkbox_div, patch.parentFrame, patch.renderLayerId);
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
        console.assert(patch.renderLayerId != null);
        
        const nativeImage = this.objectManager.getFromPool(IMAGE) as HTMLInputElement;
        nativeImage.style.margin = "0";

        let nativeImage_div: HTMLDivElement = this.objectManager.getFromPool(DIV);
        nativeImage_div.appendChild(nativeImage);
        nativeImage_div.setAttribute("class", NATIVE_LEAF_CLASS)
        nativeImage_div.setAttribute("pax_id", String(patch.id));
        if(patch.id != undefined && patch.renderLayerId != undefined){
            this.layers.addElement(nativeImage_div, patch.parentFrame, patch.renderLayerId);
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
        console.assert(patch.renderLayerId != null);

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
        if(patch.id != undefined && patch.renderLayerId != undefined){
            this.layers.addElement(youtubeVideo_div, patch.parentFrame, patch.renderLayerId);
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

    createTextboxElement(isTextArea: boolean): HTMLInputElement | HTMLTextAreaElement {
        const textbox = isTextArea
            ? document.createElement("textarea") as HTMLTextAreaElement
            : this.objectManager.getFromPool(INPUT) as HTMLInputElement;

        if (!isTextArea) {
            (textbox as HTMLInputElement).type = "text";
        } else {
            (textbox as HTMLTextAreaElement).rows = 2;
            textbox.style.resize = "none";
        }
        textbox.style.margin = "0";
        textbox.style.padding = "0";
        textbox.style.paddingInline = "5px 5px";
        textbox.style.paddingBlock = "0";
        textbox.style.borderWidth = "0";
        textbox.style.boxSizing = "border-box";
        return textbox;
    }

    attachTextboxListeners(textbox: HTMLInputElement | HTMLTextAreaElement, id: number) {
        textbox.addEventListener("input", (_event) => {
            let message = {
                "FormTextboxInput": {
                    "id": id,
                    "text": textbox.value,
                }
            }
            this.chassis!.interrupt(message, undefined);
        });

        textbox.addEventListener("change", (_event) => {
            let message = {
                "FormTextboxChange": {
                    "id": id,
                    "text": textbox.value,
                }
            }
            this.chassis!.interrupt(message, undefined);
        });
    }

    textboxCreate(patch: AnyCreatePatch) {
        const textbox = this.createTextboxElement(false);
        this.attachTextboxListeners(textbox, patch.id!);

        let textboxDiv: HTMLDivElement = this.objectManager.getFromPool(DIV);
        textboxDiv.appendChild(textbox);
        textboxDiv.setAttribute("class", NATIVE_LEAF_CLASS)
        textboxDiv.setAttribute("pax_id", String(patch.id));

        if(patch.id != undefined && patch.renderLayerId != undefined){
            this.layers.addElement(textboxDiv, patch.parentFrame, patch.renderLayerId);
            this.nodesLookup.set(patch.id!, textboxDiv);
        } else {
            throw new Error("undefined id or renderLayer");
        }

    }

    
    textboxUpdate(patch: TextboxUpdatePatch) {
        let leaf = this.nodesLookup.get(patch.id!);
        this.applyLeafPlacement(leaf!, patch);
        updateCommonProps(leaf!, patch);
        let textbox = leaf!.firstChild as HTMLInputElement | HTMLTextAreaElement;

        if (patch.is_text_area != null) {
            const shouldUseTextarea = patch.is_text_area;
            const isTextarea = textbox instanceof HTMLTextAreaElement;
            if (shouldUseTextarea !== isTextarea) {
                const replacement = this.createTextboxElement(shouldUseTextarea);
                replacement.value = textbox.value;
                replacement.placeholder = textbox.placeholder;
                this.attachTextboxListeners(replacement, patch.id!);
                leaf!.replaceChild(replacement, textbox);
                textbox = replacement;
            }
        }

        // set to 10px less to give space for left-padding
        if (patch.size_x != null) {
            (leaf!.firstChild! as HTMLElement).style.width = (patch.size_x - 10) + "px";
        }
        if (patch.size_y != null) {
            (leaf!.firstChild! as HTMLElement).style.height = patch.size_y + "px";
        }

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


    
    radioListCreate(patch: AnyCreatePatch) {
        let fields = document.createElement('fieldset') as HTMLFieldSetElement;
        fields.style.border = "0";
        fields.style.margin = "0";
        fields.style.padding = "0";
        fields.style.display = "flex";
        fields.style.flexDirection = "column";
        fields.style.justifyContent = "center";
        fields.style.width = "100%";
        fields.style.height = "100%";
        fields.style.boxSizing = "border-box";
        fields.addEventListener('change', (event) => {
            let target = event.target as HTMLElement | undefined;
            if (target && target.matches("input[type='radio']")) {
                // get the index of the triggered radio button in the fieldset
                let container = target.parentNode as Element;
                let index = Array.from(container!.parentNode!.children).indexOf(container);
                let message = {
                    "FormRadioListChange": {
                        "id": patch.id!,
                        "selected_id": index,
                    }
                }
                this.chassis!.interrupt(message, undefined);
            }
        });

        let radioListDiv: HTMLDivElement = this.objectManager.getFromPool(DIV);
        radioListDiv.setAttribute("class", NATIVE_LEAF_CLASS)
        radioListDiv.setAttribute("pax_id", String(patch.id));
        radioListDiv.appendChild(fields);

        if(patch.id != undefined && patch.renderLayerId != undefined){
            this.layers.addElement(radioListDiv, patch.parentFrame, patch.renderLayerId);
            this.nodesLookup.set(patch.id!, radioListDiv);
        } else {
            throw new Error("undefined id or renderLayer");
        }

    }

    
    radioListUpdate(patch: RadioListUpdatePatch) {
        let leaf = this.nodesLookup.get(patch.id!);
        this.applyLeafPlacement(leaf!, patch);
        updateCommonProps(leaf!, patch);
        if (patch.style != null) {
            applyTextStyle(leaf!, leaf!, patch.style);
        }

        let fields = leaf!.firstChild as HTMLFieldSetElement;
        if (patch.options != null) {
            fields!.innerHTML = "";
            patch.options.forEach((optionText, index) => {
                let row = document.createElement('label') as HTMLLabelElement;
                row.style.alignItems = "center";
                row.style.display = "flex";
                row.style.flex = "1 1 auto";
                row.style.width = "100%";
                row.style.boxSizing = "border-box";
                row.style.paddingRight = "12px";
                row.style.cursor = "pointer";
                row.style.userSelect = "none";
                const option = document.createElement('input') as HTMLInputElement;
                option.type = "radio";
                option.name = `radio-${patch.id}`;
                option.value = optionText.toString();
                option.id = `radio-${patch.id}-${index}`;
                option.setAttribute("class", RADIO_LIST_CLASS);
                row.appendChild(option);
                const labelText = document.createElement('span') as HTMLSpanElement;
                labelText.textContent = optionText.toString();
                labelText.style.flex = "1 1 auto";
                row.appendChild(labelText);
                fields.appendChild(row);
            });
        }

        if (patch.selected_id != null) {
            let radio = fields.children[patch.selected_id]
                ?.querySelector("input[type='radio']") as HTMLInputElement | null;
            if (radio != null && radio.checked == false) {
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

    radioListDelete(id: number) {
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
            this.chassis!.interrupt(message, undefined);
        });

        let sliderDiv: HTMLDivElement = this.objectManager.getFromPool(DIV);
        sliderDiv.appendChild(slider);
        sliderDiv.setAttribute("class", NATIVE_LEAF_CLASS)
        sliderDiv.style.overflow = "visible";
        sliderDiv.style.contain = "layout style";
        sliderDiv.setAttribute("pax_id", String(patch.id));

        if(patch.id != undefined && patch.renderLayerId != undefined){
            this.layers.addElement(sliderDiv, patch.parentFrame, patch.renderLayerId);
            this.nodesLookup.set(patch.id!, sliderDiv);
        } else {
            throw new Error("undefined id or renderLayer");
        }

    }

    
    sliderUpdate(patch: SliderUpdatePatch) {
        let leaf = this.nodesLookup.get(patch.id!);
        this.applyLeafPlacement(leaf!, patch);
        updateCommonProps(leaf!, patch);
        let slider = leaf!.firstChild as HTMLInputElement;

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
            this.chassis!.interrupt(message, undefined);
        });


        let textboxDiv: HTMLDivElement = this.objectManager.getFromPool(DIV);
        textboxDiv.appendChild(dropdown);
        textboxDiv.setAttribute("class", NATIVE_LEAF_CLASS)
        textboxDiv.setAttribute("pax_id", String(patch.id));

        if(patch.id != undefined && patch.renderLayerId != undefined){
            this.layers.addElement(textboxDiv, patch.parentFrame, patch.renderLayerId);
            this.nodesLookup.set(patch.id!, textboxDiv);
        } else {
            throw new Error("undefined id or renderLayer");
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
        console.assert(patch.renderLayerId != null);
        
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
            this.chassis!.interrupt(message, undefined);
        });

        let buttonDiv: HTMLDivElement = this.objectManager.getFromPool(DIV);
        textContainer.appendChild(textChild);
        button.appendChild(textContainer);
        buttonDiv.appendChild(button);
        buttonDiv.setAttribute("class", NATIVE_LEAF_CLASS)
        buttonDiv.setAttribute("pax_id", String(patch.id));
        if(patch.id != undefined && patch.renderLayerId != undefined){
            this.layers.addElement(buttonDiv, patch.parentFrame, patch.renderLayerId);
            this.nodesLookup.set(patch.id!, buttonDiv);
        } else {
            throw new Error("undefined id or renderLayer");
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

    photoPickerCreate(patch: AnyCreatePatch) {
        console.assert(patch.id != null);
        console.assert(patch.renderLayerId != null);

        const input = this.objectManager.getFromPool(INPUT) as HTMLInputElement;
        input.type = "file";
        input.accept = "image/*";
        input.multiple = true;
        input.dataset.source = "library";
        input.dataset.requestId = "0";
        input.dataset.lastTrigger = "0";
        input.dataset.includeBytes = "true";
        input.dataset.maxBytesPerPhoto = String(25 * 1024 * 1024);
        input.style.position = "absolute";
        input.style.inset = "0";
        input.style.width = "100%";
        input.style.height = "100%";
        input.style.opacity = "0";
        input.style.cursor = "pointer";
        input.style.pointerEvents = "auto";
        input.addEventListener("change", () => {
            const files = input.files;
            queueMicrotask(async () => {
                await this.dispatchPhotoPickerSelection(patch.id!, input, files);
                input.value = "";
            });
        });

        let leaf: HTMLDivElement = this.objectManager.getFromPool(DIV);
        leaf.appendChild(input);
        leaf.setAttribute("class", NATIVE_LEAF_CLASS);
        leaf.setAttribute("pax_id", String(patch.id));
        leaf.style.pointerEvents = "auto";
        leaf.style.overflow = "hidden";

        if(patch.id != undefined && patch.renderLayerId != undefined){
            this.layers.addElement(leaf, patch.parentFrame, patch.renderLayerId);
            this.nodesLookup.set(patch.id!, leaf);
        } else {
            throw new Error("undefined id or renderLayer");
        }
    }

    photoPickerUpdate(patch: PhotoPickerUpdatePatch) {
        let leaf = this.nodesLookup.get(patch.id!);
        console.assert(leaf !== undefined);
        this.applyLeafPlacement(leaf!, patch);
        updateCommonProps(leaf!, patch);

        const input = leaf!.firstChild as HTMLInputElement;
        if (patch.accept != null) {
            input.accept = patch.accept;
        }
        if (patch.allowMultiple != null) {
            input.multiple = patch.allowMultiple;
        }
        if (patch.source != null) {
            input.dataset.source = patch.source;
            if (patch.source === "camera") {
                input.setAttribute("capture", "environment");
            } else {
                input.removeAttribute("capture");
            }
        }
        if (patch.includeBytes != null) {
            input.dataset.includeBytes = String(patch.includeBytes);
        }
        if (patch.maxBytesPerPhoto != null) {
            input.dataset.maxBytesPerPhoto = String(patch.maxBytesPerPhoto);
        }
        if (patch.trigger != null) {
            const nextTrigger = String(patch.trigger);
            const previousTrigger = input.dataset.lastTrigger;
            input.dataset.requestId = nextTrigger;
            input.dataset.lastTrigger = nextTrigger;
            if (previousTrigger != null && previousTrigger !== nextTrigger && patch.trigger > 0) {
                input.click();
            }
        }
    }

    photoPickerDelete(id: number) {
        let oldNode = this.nodesLookup.get(id);
        if (oldNode){
            let parent = oldNode.parentElement;
            parent!.removeChild(oldNode);
            this.nodesLookup.delete(id);
        }
    }

    private async dispatchPhotoPickerSelection(id: number, input: HTMLInputElement, files: FileList | null) {
        const fileList = Array.from(files ?? []);
        if (fileList.length === 0) {
            this.dispatchPhotoPickerResult(id, input, "cancelled", null, [], []);
            return;
        }

        const includeBytes = input.dataset.includeBytes !== "false";
        const maxBytes = Number(input.dataset.maxBytesPerPhoto ?? 0);
        const photos: any[] = [];
        const payloads: Uint8Array[] = [];
        const rejected: string[] = [];
        const failures: string[] = [];

        for (let index = 0; index < fileList.length; index++) {
            const file = fileList[index]!;
            if (maxBytes > 0 && file.size > maxBytes) {
                rejected.push(file.name || `photo ${index + 1}`);
                continue;
            }

            try {
                const handle = URL.createObjectURL(file);
                const dimensions = await this.readPhotoDimensions(file, handle);
                const bytes = includeBytes ? new Uint8Array(await file.arrayBuffer()) : new Uint8Array();
                payloads.push(bytes);
                photos.push({
                    temp_id: `${id}-${input.dataset.requestId ?? "0"}-${index}-${file.lastModified}-${file.size}`,
                    file_name: file.name || null,
                    mime_type: file.type || "image/*",
                    byte_size: file.size,
                    width: dimensions.width,
                    height: dimensions.height,
                    source_kind: input.dataset.source === "camera" ? "camera" : "file",
                    handle,
                });
            } catch (err) {
                failures.push(err instanceof Error ? err.message : String(err));
            }
        }

        const hasRejected = rejected.length > 0;
        const status = hasRejected && photos.length === 0
            ? "size_limit_exceeded"
            : (photos.length === 0 && failures.length > 0 ? "failed" : "selected");
        const message = hasRejected
            ? `Skipped ${rejected.length} photo(s) over the configured byte limit.`
            : (failures[0] ?? null);
        this.dispatchPhotoPickerResult(id, input, status, message, photos, payloads);
    }

    private dispatchPhotoPickerResult(
        id: number,
        input: HTMLInputElement,
        status: string,
        message: string | null,
        photos: any[],
        payloads: Uint8Array[],
    ) {
        this.chassis!.interrupt({
            "PhotoPicker": {
                "id": id,
                "request_id": Number(input.dataset.requestId ?? 0),
                "status": status,
                "message": message,
                "photos": photos,
            }
        }, payloads);
        this.postAsyncInterruptFlush?.();
    }

    private async readPhotoDimensions(file: File, handle: string): Promise<{ width?: number, height?: number }> {
        try {
            if ("createImageBitmap" in window) {
                const bitmap = await createImageBitmap(file);
                const dimensions = { width: bitmap.width, height: bitmap.height };
                bitmap.close();
                return dimensions;
            }
        } catch (_err) {}

        return new Promise(resolve => {
            const image = new Image();
            image.onload = () => resolve({ width: image.naturalWidth, height: image.naturalHeight });
            image.onerror = () => resolve({});
            image.src = handle;
        });
    }

    textCreate(patch: AnyCreatePatch) {
        console.assert(patch.id != null);
        console.assert(patch.renderLayerId != null);

        let textDiv: HTMLDivElement = this.objectManager.getFromPool(DIV);
        let textChild: HTMLDivElement = this.objectManager.getFromPool(DIV);
        // Text should be allowed to paint outside its measured box by default;
        // otherwise large headings get clipped by native leaf paint containment.
        textDiv.style.overflow = "visible";
        textDiv.style.contain = "layout style";
        textChild.style.overflow = "visible";
        textChild.setAttribute("contenteditable", "false");
        textChild.innerHTML = "";
        delete textChild.dataset.paxRenderedContentSignature;
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

            this.chassis!.interrupt(message, undefined);
        });
        textDiv.appendChild(textChild);
        textDiv.setAttribute("class", NATIVE_LEAF_CLASS)
        textDiv.setAttribute("pax_id", String(patch.id));

        if(patch.id != undefined && patch.renderLayerId != undefined){
            this.layers.addElement(textDiv, patch.parentFrame, patch.renderLayerId);
            this.nodesLookup.set(patch.id!, textDiv);
        } else {
            throw new Error("undefined id or renderLayer");
        }
    }

    textUpdate(patch: TextUpdatePatch) {
        let leaf = this.nodesLookup.get(patch.id!) as HTMLElement;
        let textChild = leaf!.firstChild as HTMLElement;
        this.applyLeafPlacement(leaf, patch);
        const syncTextPointerEvents = () => {
            const editable = textChild.getAttribute("contenteditable") !== "false";
            const selectable = textChild.style.userSelect !== "none";
            leaf.style.pointerEvents = editable || selectable ? "auto" : "none";
        };
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
            syncTextPointerEvents();
        }

        if (patch.selectable != null) {
            textChild.style.userSelect = patch.selectable ? "auto" : "none";
            syncTextPointerEvents();
        }

        if (patch.clip != null) {
            applyClip(patch.clip);
        }

        if (patch.wrap != null) {
            textChild.style.whiteSpace = patch.wrap ? "normal" : "pre";
        }

        applyTextStyle(leaf, textChild, patch.style);

        // Apply the content
        if (patch.content != null) {
            const renderSignature = `${patch.markdown ? "markdown" : "plain"}:${patch.content}`;
            if (textChild.dataset.paxRenderedContentSignature !== renderSignature) {
                if (patch.markdown) {
                    textChild.innerHTML = renderMarkdownTextContent(patch.content);
                } else {
                    textChild.innerText = patch.content;
                }
                textChild.dataset.paxRenderedContentSignature = renderSignature;
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
        // defer debug overlay until after layout updates
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
         if (patch.borderRadius != null) {
             styles.borderRadius = patch.borderRadius;
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

    private getScrollerSnapHost(leaf: HTMLElement) {
        let scrollerId = this.getScrollerIdFromLeaf(leaf);
        let hosts = scrollerId != null ? this.scrollerHosts.get(scrollerId) : undefined;
        return hosts?.snapHost
            ?? (leaf.querySelector(`:scope > .${INNER_PANE} > [data-role="scroller-snap-host"]`) as HTMLElement | null);
    }

    private ensurePageSnapHost(): HTMLDivElement | null {
        if (this.mount == null) {
            return null;
        }
        if (this.pageSnapHost != null) {
            return this.pageSnapHost;
        }
        let host = document.createElement("div");
        host.dataset.role = "page-scroll-snap-host";
        host.style.position = "absolute";
        host.style.top = "0";
        host.style.left = "0";
        host.style.width = "1px";
        host.style.height = "1px";
        host.style.pointerEvents = "none";
        host.style.zIndex = "0";
        this.mount.appendChild(host);
        this.pageSnapHost = host;
        return host;
    }

    private getParentScrollerId(hosts: ScrollerDomHosts) {
        let parentLeaf = hosts.leaf.parentElement?.closest(`.${SCROLLER_CONTAINER}`);
        if (!(parentLeaf instanceof HTMLElement)) {
            return undefined;
        }
        let parentId = parseInt(parentLeaf.getAttribute("pax_id") ?? "", 10);
        if (!Number.isFinite(parentId) || parentId === hosts.id) {
            return undefined;
        }
        return parentId;
    }

    private getWarmClipBaseForCandidate(
        candidate: ScrollerWarmCandidate,
        candidates: Map<number, ScrollerWarmCandidate>,
    ): { bounds: AxisAlignedRect; viewportWidth: number; viewportHeight: number } {
        if (candidate.parentId != null) {
            let parentCandidate = candidates.get(candidate.parentId);
            if (parentCandidate != null) {
                let bounds = parentCandidate.record.presentedBounds;
                return {
                    bounds,
                    viewportWidth: parentCandidate.hosts.leaf.clientWidth || rectWidth(bounds),
                    viewportHeight: parentCandidate.hosts.leaf.clientHeight || rectHeight(bounds),
                };
            }

            let parentHosts = this.scrollerHosts.get(candidate.parentId);
            let parentState = this.scrollerMeasurementStates.get(candidate.parentId);
            if (parentHosts != null && parentState != null) {
                let record = this.getWarmthPresentationRecord(
                    candidate.parentId,
                    parentHosts.leaf,
                    parentState,
                );
                return {
                    bounds: record.presentedBounds,
                    viewportWidth: parentHosts.leaf.clientWidth || rectWidth(record.presentedBounds),
                    viewportHeight: parentHosts.leaf.clientHeight || rectHeight(record.presentedBounds),
                };
            }
        }

        let bounds = candidate.record.presentedClipBounds;
        return {
            bounds,
            viewportWidth: rectWidth(bounds),
            viewportHeight: rectHeight(bounds),
        };
    }

    private buildWarmScrollerCandidates(now: number) {
        let candidates = new Map<number, ScrollerWarmCandidate>();
        this.scrollerHosts.forEach((hosts, id) => {
            let state = this.scrollerMeasurementStates.get(id);
            if (state == null || !hosts.vectorIslandEnabled) {
                return;
            }
            let record = this.getWarmthPresentationRecord(id, hosts.leaf, state);
            candidates.set(id, {
                id,
                hosts,
                state,
                record,
                parentId: this.getParentScrollerId(hosts),
                intent: "cold",
                distance: rectCenterDistance(record.presentedBounds, record.presentedClipBounds),
                surfaceCost: this.estimateScrollerWarmSurfaceCost(hosts, state),
                mandatory: state.isRootScroller || state.delegatesToPageScroll,
            });
        });

        let resolving = new Set<number>();
        let resolved = new Set<number>();
        let resolveIntent = (candidate: ScrollerWarmCandidate): ScrollerWarmIntent => {
            if (resolved.has(candidate.id)) {
                return candidate.intent;
            }
            if (resolving.has(candidate.id)) {
                candidate.intent = "active";
                return candidate.intent;
            }
            resolving.add(candidate.id);

            if (candidate.mandatory) {
                candidate.intent = "active";
                candidate.state.lastWarmAt = now;
                resolving.delete(candidate.id);
                resolved.add(candidate.id);
                return candidate.intent;
            }

            let parentIntent: ScrollerWarmIntent = "active";
            if (candidate.parentId != null) {
                let parentCandidate = candidates.get(candidate.parentId);
                if (parentCandidate != null) {
                    parentIntent = resolveIntent(parentCandidate);
                }
            }

            if (parentIntent === "cold") {
                candidate.intent = "cold";
                resolving.delete(candidate.id);
                resolved.add(candidate.id);
                return candidate.intent;
            }

            // Warmth propagates recursively: a child island must be inside its parent scroller's
            // own runway, not just inside the page-level viewport runway.
            let clipBase = this.getWarmClipBaseForCandidate(candidate, candidates);
            let activeClipBounds = expandRect(
                clipBase.bounds,
                activeScrollablePadX(clipBase.viewportWidth),
                activeScrollablePadY(clipBase.viewportHeight),
            );
            let prewarmClipBounds = expandRect(
                clipBase.bounds,
                prewarmScrollablePadX(clipBase.viewportWidth),
                prewarmScrollablePadY(clipBase.viewportHeight),
            );
            let activeEligible = rectsIntersect(candidate.record.presentedBounds, activeClipBounds);
            let recentlyWarm =
                isIOSWebKitBrowser()
                && now - candidate.state.lastWarmAt <= IOS_WARM_LINGER_MS;
            if (activeEligible) {
                candidate.state.lastWarmAt = now;
                candidate.intent = parentIntent === "active" ? "active" : "prewarm";
            } else if (recentlyWarm) {
                candidate.intent = parentIntent === "active" ? "active" : "prewarm";
            } else if (rectsIntersect(candidate.record.presentedBounds, prewarmClipBounds)) {
                candidate.intent = "prewarm";
            } else {
                candidate.intent = "cold";
            }
            candidate.distance = rectCenterDistance(candidate.record.presentedBounds, clipBase.bounds);
            resolving.delete(candidate.id);
            resolved.add(candidate.id);
            return candidate.intent;
        };

        candidates.forEach(candidate => resolveIntent(candidate));
        return candidates;
    }

    private getPresentationRecordForLeaf(
        scrollerId: number,
        leaf: HTMLElement,
    ): { presentedBounds: AxisAlignedRect; presentedClipBounds: AxisAlignedRect } {
        let rect = leaf.getBoundingClientRect();
        let viewport = {
            left: 0,
            top: 0,
            right: window.innerWidth ?? rect.right,
            bottom: window.innerHeight ?? rect.bottom,
        };
        let clipBounds = viewport;
        if (!isIOSWebKitBrowser()) {
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
        let record = this.presentationRecords.get(scrollerId);
        if (record?.presentedBounds && record.presentedClipBounds) {
            return {
                presentedBounds: record.presentedBounds,
                presentedClipBounds: record.presentedClipBounds,
            };
        }
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

    private getWarmthPresentationRecord(
        scrollerId: number,
        leaf: HTMLElement,
        state: ScrollerMeasurementState,
    ): { presentedBounds: AxisAlignedRect; presentedClipBounds: AxisAlignedRect } {
        if (isIOSWebKitBrowser() && this.activePageScrollScrollerId != null) {
            let transform = state.transform ?? [1, 0, 0, 1, 0, 0];
            let width = leaf.clientWidth || 0;
            let height = leaf.clientHeight || 0;
            let point0 = affineMultiply([0, 0], transform);
            let point1 = affineMultiply([width, 0], transform);
            let point2 = affineMultiply([width, height], transform);
            let point3 = affineMultiply([0, height], transform);
            let left = Math.min(point0[0], point1[0], point2[0], point3[0]);
            let right = Math.max(point0[0], point1[0], point2[0], point3[0]);
            let top = Math.min(point0[1], point1[1], point2[1], point3[1]);
            let bottom = Math.max(point0[1], point1[1], point2[1], point3[1]);

            let visibleViewport = this.readVisiblePageViewport();
            let pageScroll = this.readPageScrollPosition();
            let approvedScrollX = pageScroll.scrollX + visibleViewport.offsetX;
            let approvedScrollY = pageScroll.scrollY + visibleViewport.offsetY;
            return {
                presentedBounds: { left, top, right, bottom },
                presentedClipBounds: {
                    left: approvedScrollX,
                    top: approvedScrollY,
                    right: approvedScrollX + visibleViewport.width,
                    bottom: approvedScrollY + visibleViewport.height,
                },
            };
        }
        return this.getPresentationRecordForLeaf(scrollerId, leaf);
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
        if (
            hosts.parked
            && hosts.canvasHost.dataset.renderState === "parked"
            && hosts.contentHost.dataset.renderState === "parked"
        ) {
            return;
        }
        hosts.parked = true;
        hosts.canvasHost.dataset.renderState = "parked";
        hosts.contentHost.dataset.renderState = "parked";
        hosts.canvasHost.style.visibility = "hidden";
        hosts.contentHost.style.visibility = "hidden";
        this.surfaceRefreshPending = true;
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
    }

    private prewarmScrollerHosts(hosts: ScrollerDomHosts) {
        if (
            hosts.parked
            && hosts.canvasHost.dataset.renderState === "active"
            && hosts.contentHost.dataset.renderState === "parked"
            && hosts.canvasHost.style.visibility === "hidden"
        ) {
            return;
        }
        hosts.parked = true;
        hosts.canvasHost.dataset.renderState = "active";
        hosts.contentHost.dataset.renderState = "parked";
        hosts.canvasHost.style.visibility = "hidden";
        hosts.contentHost.style.visibility = "hidden";
        this.surfaceRefreshPending = true;
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
                return total + Math.max(1, this.estimateWarmLayerCanvasCount(layerId, hosts.canvasHost));
            }, 0),
        );
    }

    private syncWarmScrollerHosts() {
        let now =
            typeof performance !== "undefined" && typeof performance.now === "function"
                ? performance.now()
                : Date.now();
        let candidatesById = this.buildWarmScrollerCandidates(now);
        let candidates = Array.from(candidatesById.values());
        this.scrollerHosts.forEach((hosts, id) => {
            if (candidatesById.has(id)) {
                return;
            }
            let state = this.scrollerMeasurementStates.get(id);
            if (state == null || !hosts.vectorIslandEnabled) {
                this.unparkScrollerHosts(hosts);
            }
        });

        let nestedSurfaceBudget = browserNestedScrollerSurfaceBudget();
        let totalCanvasBudget = browserTotalCanvasBudget();
        if (nestedSurfaceBudget != null) {
            let rootCanvasCost = 0;
            if (totalCanvasBudget != null && this.mount != null) {
                rootCanvasCost = Math.max(1, this.estimateWarmLayerCanvasCount(0, this.mount));
            }
            let mandatoryCost = candidates.reduce((total, candidate) => {
                return candidate.mandatory
                    ? total + Math.max(1, candidate.surfaceCost)
                    : total;
            }, 0);
            candidates.sort((left, right) => {
                let rankDelta = scrollerWarmIntentRank(left.intent) - scrollerWarmIntentRank(right.intent);
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
            let mandatorySelectionCount = selected.size;
            let trySelectCandidate = (candidate: ScrollerWarmCandidate): boolean => {
                if (selected.has(candidate.id)) {
                    return true;
                }
                if (candidate.mandatory) {
                    selected.add(candidate.id);
                    return true;
                }
                if (candidate.intent === "cold" && !candidate.hosts.warmed) {
                    return false;
                }
                if (candidate.parentId != null) {
                    let parentCandidate = candidatesById.get(candidate.parentId);
                    if (parentCandidate != null && !trySelectCandidate(parentCandidate)) {
                        return false;
                    }
                }
                let cost = Math.max(1, candidate.surfaceCost);
                let canAfford = remainingBudget >= cost;
                let mustKeep = totalCanvasBudget == null
                    && candidate.intent === "active"
                    && selected.size === mandatorySelectionCount;
                if (!canAfford && !mustKeep) {
                    return false;
                }
                selected.add(candidate.id);
                remainingBudget = Math.max(0, remainingBudget - cost);
                return true;
            };
            for (let candidate of candidates) {
                if (candidate.mandatory) {
                    continue;
                }
                if (
                    candidate.intent === "cold"
                    && !candidate.hosts.warmed
                ) {
                    continue;
                }
                trySelectCandidate(candidate);
            }
            for (let candidate of candidates) {
                if (selected.has(candidate.id)) {
                    this.promoteScrollerHosts(candidate.hosts);
                    if (candidate.intent === "active" || candidate.mandatory) {
                        this.unparkScrollerHosts(candidate.hosts);
                    } else {
                        this.prewarmScrollerHosts(candidate.hosts);
                    }
                } else {
                    this.demoteScrollerHosts(candidate.hosts);
                }
            }
            return;
        }
        for (let candidate of candidates) {
            if (candidate.intent === "active" || candidate.mandatory) {
                this.promoteScrollerHosts(candidate.hosts);
                this.unparkScrollerHosts(candidate.hosts);
            } else if (candidate.intent === "prewarm") {
                this.promoteScrollerHosts(candidate.hosts);
                this.prewarmScrollerHosts(candidate.hosts);
            } else if (candidate.hosts.warmed) {
                this.prewarmScrollerHosts(candidate.hosts);
            } else {
                this.parkScrollerHosts(candidate.hosts);
            }
        }
    }

    syncRenderSurfaceLayouts() {
        let layerCanvasPlanGeneration = this.chassis?.layer_canvas_plan_generation() ?? -1;
        let devicePixelRatio =
            typeof window !== "undefined" ? Math.max(window.devicePixelRatio || 1, 1) : 1;
        let planInputsChanged =
            this.lastLayerCanvasPlanGeneration !== layerCanvasPlanGeneration
            || this.lastLayerCanvasPlanDevicePixelRatio !== devicePixelRatio;
        if (planInputsChanged) {
            this.layerCanvasPlanCache.clear();
        }
        this.syncWarmScrollerHosts();
        let backingMismatchLayers = this.collectCanvasBackingMismatchLayers(devicePixelRatio);
        if (backingMismatchLayers.size > 0) {
            this.surfaceRefreshPending = true;
        }
        if (
            !this.surfaceRefreshPending
            && !planInputsChanged
        ) {
            return;
        }
        this.lastLayerCanvasPlanGeneration = layerCanvasPlanGeneration;
        this.lastLayerCanvasPlanDevicePixelRatio = devicePixelRatio;
        this.layers.syncLayerCanvasLayouts((layerId) => {
            if (!this.chassis) {
                return undefined;
            }
            let plan = this.chassis.get_layer_canvas_plan(layerId);
            this.layerCanvasPlanCache.set(layerId, plan ?? null);
            if (plan && layerId !== 0) {
                let warmState = this.layerWarmState(layerId);
                if (warmState === "cold") {
                    return {
                        layerId: plan.layerId ?? layerId,
                        active: false,
                        surfaces: [],
                    };
                }
            }
            return plan;
        });
        backingMismatchLayers = this.collectCanvasBackingMismatchLayers(devicePixelRatio);
        let surfaceChanged = this.surfaceRefreshPending || backingMismatchLayers.size > 0;
        let nextSurfaceSignatures = new Map<string, string>();
        let nextTransformSignatures = new Map<string, string>();
        let nextLayerCounts = new Map<number, number>();
        let surfaceChangedLayers = new Set<number>(backingMismatchLayers);
        let transformChangedLayers = new Set<number>();
        let transformChanged = false;
        this.canvases.forEach((canvas, id) => {
            let parent = canvas.parentElement;
            if (parent == null) {
                return;
            }
            let hostSignature = canvas.dataset.hostSignature ?? describeSurfaceHost(parent);
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
            this.surfaceRefreshPending = false;
            let layers = Array.from(surfaceChangedLayers).sort((left, right) => left - right);
            this.requestLayerSurfaceRefresh(layers);
        } else if (transformChanged) {
            let layers = Array.from(transformChangedLayers).sort((left, right) => left - right);
            this.requestLayerSurfaceRefresh(layers);
        }
    }

    private requestLayerSurfaceRefresh(layers: number[]) {
        if (!this.chassis) {
            return;
        }
        // Surface host/layout changes are fed back through the interrupt seam so the engine and
        // render backend stay in lockstep.
        if (layers.length === 0) {
            this.chassis.interrupt({
                "RenderSurfaceUpdate": {},
            }, undefined);
            return;
        }
        for (let layerId of layers) {
            this.chassis.interrupt({
                "RenderSurfaceUpdate": {
                    "layer_id": layerId,
                },
            }, undefined);
        }
    }

    private layerWarmState(layerId: number) {
        for (let [id, hosts] of this.scrollerHosts) {
            let state = this.scrollerMeasurementStates.get(id);
            if (state == null || state.contentLayerId == null) {
                continue;
            }
            if (state.contentLayerId === layerId) {
                return hosts.warmed ? "warm" : "cold";
            }
        }
        return "unknown";
    }

    private collectCanvasBackingMismatchLayers(devicePixelRatio: number) {
        let layers = new Set<number>();
        this.canvases.forEach((canvas) => {
            let layerId = Number.parseInt(canvas.dataset.layerId ?? "", 10);
            if (!Number.isFinite(layerId)) {
                return;
            }
            let logicalWidth = Number.parseFloat(canvas.dataset.logicalWidth ?? "");
            let logicalHeight = Number.parseFloat(canvas.dataset.logicalHeight ?? "");
            if (
                !Number.isFinite(logicalWidth)
                || !Number.isFinite(logicalHeight)
                || logicalWidth <= 0
                || logicalHeight <= 0
            ) {
                return;
            }

            let parentRole = canvas.parentElement?.dataset.role;
            let desiredDpr =
                parentRole === "scroller-canvas-host" && canvas.dataset.tileKey !== "single"
                    ? TILED_SCROLLER_SURFACE_DPR
                    : devicePixelRatio;
            let maxSurfaceDimension = Number.POSITIVE_INFINITY;
            let minimumDpr = 1.0;
            if (isIOSWebKitBrowser() && layerId > 0) {
                maxSurfaceDimension = IOS_BROWSER_SURFACE_DIMENSION_CAP;
                minimumDpr = IOS_NESTED_LAYER_MIN_DPR;
            }
            let expected = expectedCanvasBackingSize(
                logicalWidth,
                logicalHeight,
                desiredDpr,
                maxSurfaceDimension,
                minimumDpr,
            );
            if (
                Math.abs(canvas.width - expected.width) > 1
                || Math.abs(canvas.height - expected.height) > 1
            ) {
                layers.add(layerId);
            }
        });
        return layers;
    }


    private estimateWarmLayerCanvasCount(layerId: number, host?: HTMLElement) {
        let cachedPlan = this.layerCanvasPlanCache.get(layerId);
        if (cachedPlan && Array.isArray(cachedPlan.surfaces)) {
            return cachedPlan.surfaces.length;
        }
        if (this.chassis) {
            let plan = this.chassis.get_layer_canvas_plan(layerId);
            this.layerCanvasPlanCache.set(layerId, plan ?? null);
            if (plan && Array.isArray(plan.surfaces)) {
                return plan.surfaces.length;
            }
        }
        return estimateWarmLayerCanvasCount(layerId, host);
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
        let visualViewport = window.visualViewport;
        if (visualViewport != null) {
            let pageTop = visualViewport.pageTop;
            let pageLeft = visualViewport.pageLeft;
            if (Number.isFinite(pageTop) && Number.isFinite(pageLeft)) {
                let offsetX = visualViewport.offsetLeft;
                let offsetY = visualViewport.offsetTop;
                return {
                    scrollX: pageLeft - offsetX,
                    scrollY: pageTop - offsetY,
                };
            }
        }
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
        let approvedScrollX = pageScroll.scrollX;
        let approvedScrollY = pageScroll.scrollY;

        this.chassis?.interrupt({
            "VisualViewportUpdate": {
                "width": visibleViewport.width,
                "height": visibleViewport.height,
                "offset_x": visibleViewport.offsetX,
                "offset_y": visibleViewport.offsetY,
                "page_scroll_x": pageScroll.scrollX,
                "page_scroll_y": pageScroll.scrollY,
            },
        }, undefined);

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

    private pageScrollActivationPosition(approvedScrollX: number, approvedScrollY: number) {
        let current = this.readPageScrollPosition();
        return {
            scrollX: Math.abs(approvedScrollX) > 0.5 ? approvedScrollX : current.scrollX,
            scrollY: Math.abs(approvedScrollY) > 0.5 ? approvedScrollY : current.scrollY,
        };
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
            let activationScroll = this.pageScrollActivationPosition(approvedScrollX, approvedScrollY);
            this.activePageScrollScrollerId = scrollerId;
            this.installPageScrollActivityListeners();
            this.setDocumentPageScrollMode(
                true,
                viewportWidth,
                viewportHeight,
                contentHeight,
                activationScroll.scrollX,
                activationScroll.scrollY,
            );
            this.setPageScrollPosition(activationScroll.scrollX, activationScroll.scrollY);
            this.syncDelegatedPageScrollViewport();
            this.emitScrollerPosition(
                scrollerId,
                {
                    scrollX: activationScroll.scrollX,
                    scrollY: activationScroll.scrollY,
                    presentationScrollX: activationScroll.scrollX,
                    presentationScrollY: activationScroll.scrollY,
                },
                state,
            );
        } else if (this.activePageScrollScrollerId === scrollerId) {
            this.activePageScrollScrollerId = undefined;
            this.uninstallPageScrollActivityListeners();
            this.setDocumentPageScrollMode(false);
            let visibleViewport = this.readVisiblePageViewport();
            let pageScroll = this.readPageScrollPosition();
            this.chassis?.interrupt({
                "VisualViewportUpdate": {
                    "width": visibleViewport.width,
                    "height": visibleViewport.height,
                    "offset_x": 0,
                    "offset_y": 0,
                    "page_scroll_x": pageScroll.scrollX,
                    "page_scroll_y": pageScroll.scrollY,
                },
            }, undefined);
        }

        state.delegatesToPageScroll = active;
        this.setLeafPageScrollMode(leaf, active);
        this.activateScrollerMeasurement(scrollerId);
    }

    private syncScrollerSnap(
        patch: { snapPointsX?: number[] | null; snapPointsY?: number[] | null },
        state: ScrollerMeasurementState,
        leaf: HTMLElement,
        snapHost: HTMLElement,
        delegateToPageScroll: boolean,
    ) {
        let scrollerId = Number.parseInt(leaf.getAttribute("pax_id") ?? "", 10);
        if (!Number.isFinite(scrollerId)) {
            scrollerId = -1;
        }
        let snapPointsChanged = false;
        if (patch.snapPointsX != null) {
            state.snapPointsX = [...patch.snapPointsX];
            snapPointsChanged = true;
        }
        if (patch.snapPointsY != null) {
            state.snapPointsY = [...patch.snapPointsY];
            snapPointsChanged = true;
        }
        let snapHostChanged = state.snapHostDelegatesToPageScroll !== delegateToPageScroll;
        state.snapHostDelegatesToPageScroll = delegateToPageScroll;
        let snapPointsX = state.snapPointsX ?? [];
        let snapPointsY = state.snapPointsY ?? [];
        let hasX = snapPointsX.length > 0;
        let hasY = snapPointsY.length > 0;
        let snapType = "none";
        if (hasX && hasY) {
            snapType = "both mandatory";
        } else if (hasY) {
            snapType = "y mandatory";
        } else if (hasX) {
            snapType = "x mandatory";
        }

        let targetHost = snapHost;
        if (delegateToPageScroll) {
            this.applyScrollSnapType(leaf, "none");
            this.applyPageScrollSnapType(snapType);
            let pageHost = this.ensurePageSnapHost();
            if (pageHost != null) {
                targetHost = pageHost;
            }
            if (scrollerId >= 0) {
                this.pageSnapOwnerScrollerId = scrollerId;
            }
        } else {
            this.applyPageScrollSnapType("none");
            this.applyScrollSnapType(leaf, snapType);
            if (this.pageSnapHost != null && this.pageSnapOwnerScrollerId === scrollerId) {
                while (this.pageSnapHost.firstChild) {
                    this.pageSnapHost.removeChild(this.pageSnapHost.firstChild);
                }
                this.pageSnapOwnerScrollerId = undefined;
            }
        }
        if (snapHostChanged && targetHost !== snapHost) {
            this.renderSnapMarkers(snapHost, [], []);
        }
        if (snapPointsChanged || snapHostChanged) {
            this.renderSnapMarkers(targetHost, snapPointsX, snapPointsY);
        }
    }

    private applyScrollSnapType(target: HTMLElement, snapType: string) {
        if (snapType === "none") {
            target.style.scrollSnapType = "";
            target.style.scrollSnapStop = "";
            return;
        }
        target.style.scrollSnapType = snapType;
        target.style.scrollSnapStop = "always";
    }

    private applyPageScrollSnapType(snapType: string) {
        let root = document.documentElement;
        let body = document.body;
        if (snapType === "none") {
            root.style.scrollSnapType = "";
            root.style.scrollSnapStop = "";
            body.style.scrollSnapType = "";
            body.style.scrollSnapStop = "";
            return;
        }
        root.style.scrollSnapType = snapType;
        root.style.scrollSnapStop = "always";
        body.style.scrollSnapType = snapType;
        body.style.scrollSnapStop = "always";
    }

    private renderSnapMarkers(
        snapHost: HTMLElement,
        snapPointsX: number[],
        snapPointsY: number[],
    ) {
        while (snapHost.firstChild) {
            snapHost.removeChild(snapHost.firstChild);
        }
        let hasX = snapPointsX.length > 0;
        let hasY = snapPointsY.length > 0;
        if (!hasX && !hasY) {
            return;
        }

        let pointsX = hasX ? snapPointsX : [0];
        let pointsY = hasY ? snapPointsY : [0];
        // CSS scroll snapping needs actual child boxes at each snap coordinate.
        // The 1px markers are inert layout anchors for those browser snap targets.
        for (let x of pointsX) {
            for (let y of pointsY) {
                let marker = document.createElement("div");
                marker.dataset.role = "scroller-snap-point";
                marker.style.position = "absolute";
                marker.style.width = "1px";
                marker.style.height = "1px";
                marker.style.left = `${x}px`;
                marker.style.top = `${y}px`;
                marker.style.pointerEvents = "none";
                marker.style.scrollSnapAlign = "start";
                snapHost.appendChild(marker);
            }
        }
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
        if (state.contentLayerId == null) {
            return false;
        }
        let shouldClip = patch.clipContent ?? state.clipContent;
        if (shouldClip == null) {
            return false;
        }
        if (!shouldClip) {
            return false;
        }
        let transform = patch.transform ?? state.transform;
        if (!isViewportAnchoredTransform(transform)) {
            return false;
        }
        let viewportWidth = patch.sizeX ?? state.viewportWidth;
        let viewportHeight = patch.sizeY ?? state.viewportHeight;
        let contentWidth = patch.sizeInnerPaneX ?? state.contentWidth;
        let contentHeight = patch.sizeInnerPaneY ?? state.contentHeight;
        if (
            viewportWidth == null
            || viewportHeight == null
            || contentWidth == null
            || contentHeight == null
        ) {
            return false;
        }
        if (contentWidth > viewportWidth + 0.5) {
            // Defer to the scroller when horizontal overflow is involved.
            return false;
        }
        if (contentHeight <= viewportHeight + 0.5) {
            // No vertical scroll area to delegate.
            return false;
        }
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
            "ScrollerPosition": {
                "id": id,
                "scroll_x": measurement.scrollX,
                "scroll_y": measurement.scrollY,
                "presentation_scroll_x": measurement.presentationScrollX,
                "presentation_scroll_y": measurement.presentationScrollY,
            }
        };
        this.chassis!.interrupt(message, undefined);
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
        if (patch.borderRadius != null) {
            queued.borderRadius = patch.borderRadius;
        }
        if (patch.sizeInnerPaneX != null) {
            queued.sizeInnerPaneX = patch.sizeInnerPaneX;
        }
        if (patch.sizeInnerPaneY != null) {
            queued.sizeInnerPaneY = patch.sizeInnerPaneY;
        }
        if (patch.snapPointsX != null) {
            queued.snapPointsX = [...patch.snapPointsX];
        }
        if (patch.snapPointsY != null) {
            queued.snapPointsY = [...patch.snapPointsY];
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
        console.assert(patch.renderLayerId != null);

        let scrollerDiv: HTMLDivElement = this.objectManager.getFromPool(DIV);
        let innerPane: HTMLDivElement = this.objectManager.getFromPool(DIV);
        let snapHost: HTMLDivElement = this.objectManager.getFromPool(DIV);
        let canvasHost: HTMLDivElement = this.objectManager.getFromPool(DIV);
        let contentHost: HTMLDivElement = this.objectManager.getFromPool(DIV);
        let scrollerId = patch.id!;
        let vectorIslandEnabled = browserOwnedVectorScrollerIslandsEnabled();
        innerPane.setAttribute("class", INNER_PANE);
        scrollerDiv.style.background = "transparent";
        scrollerDiv.style.backgroundColor = "transparent";
        innerPane.style.background = "transparent";
        innerPane.style.backgroundColor = "transparent";
        snapHost.dataset.role = "scroller-snap-host";
        snapHost.style.position = "absolute";
        snapHost.style.top = "0";
        snapHost.style.left = "0";
        snapHost.style.width = "100%";
        snapHost.style.height = "100%";
        snapHost.style.pointerEvents = "none";
        snapHost.style.zIndex = "0";
        snapHost.style.background = "transparent";
        snapHost.style.backgroundColor = "transparent";
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
        canvasHost.style.background = "transparent";
        canvasHost.style.backgroundColor = "transparent";
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
        contentHost.style.background = "transparent";
        contentHost.style.backgroundColor = "transparent";
        // Keep the native overlay host above the canvas host inside browser-owned scroller
        // islands. Individual layer canvases/native elements can still order locally within those
        // hosts, but the hosts themselves should never invert.
        contentHost.style.zIndex = "1";
        let scheduleMeasurement = () => this.activateScrollerMeasurement(scrollerId);
        scrollerDiv.addEventListener("scroll", scheduleMeasurement, { passive: true });
        scrollerDiv.addEventListener("wheel", scheduleMeasurement, { passive: true });
        scrollerDiv.addEventListener("touchmove", scheduleMeasurement, { passive: true });
        scrollerDiv.addEventListener("touchstart", scheduleMeasurement, { passive: true });

        innerPane.appendChild(snapHost);
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


        if(patch.id != undefined && patch.renderLayerId != undefined){
            this.layers.addElement(scrollerDiv, patch.parentFrame, patch.renderLayerId);
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
                snapHost,
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
                snapHostDelegatesToPageScroll: false,
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
                lastWarmAt:
                    typeof performance !== "undefined" && typeof performance.now === "function"
                        ? performance.now()
                        : Date.now(),
                snapPointsX: undefined,
                snapPointsY: undefined,
            });
            this.replayPendingScrollerUpdate(patch.id);
        } else {
            throw new Error("undefined id or renderLayer");
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
        let snapHost = this.getScrollerSnapHost(leaf);
        let canvasHost = this.getScrollerCanvasHost(leaf);
        let contentHost = this.getScrollerContentHost(leaf);
        if (scrollerInner == null || snapHost == null || canvasHost == null || contentHost == null) {
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
            this.surfaceRefreshPending = true;
        }
        if (patch.sizeInnerPaneY != null) {
            scrollerInner.style.height = patch.sizeInnerPaneY + "px";
            canvasHost.style.height = patch.sizeInnerPaneY + "px";
            contentHost.style.height = patch.sizeInnerPaneY + "px";
            this.surfaceRefreshPending = true;
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
        if (state != null) {
            // Scroller patches are delta-shaped, so root page-scroll delegation needs to remember
            // the last engine-approved layout metrics instead of falling back to transient DOM
            // scroll widths/heights during the first autosize settle.
            if (patch.contentLayerId != null) {
                state.contentLayerId = patch.contentLayerId;
            }
            if (patch.sizeX != null) {
                state.viewportWidth = patch.sizeX;
            }
            if (patch.sizeY != null) {
                state.viewportHeight = patch.sizeY;
            }
            if (patch.sizeInnerPaneX != null) {
                state.contentWidth = patch.sizeInnerPaneX;
            }
            if (patch.sizeInnerPaneY != null) {
                state.contentHeight = patch.sizeInnerPaneY;
            }
            if (patch.clipContent != null) {
                state.clipContent = patch.clipContent;
            }
        }
        if (hosts?.vectorIslandEnabled && patch.contentLayerId != null) {
            let claimed = this.layers.claimLayerForScrollerIsland(patch.contentLayerId, patch.id!);
            if (claimed) {
                this.surfaceRefreshPending = true;
            }
        }
        let delegateToPageScroll = state != null && this.shouldDelegateToPageScroll(leaf, patch, state);
        const ignoreActiveScrollPatch =
            isIOSWebKitBrowser() && !delegateToPageScroll && state?.active;
        let approvedScrollX = patch.presentationScrollX
            ?? patch.scrollX
            ?? (state?.lastMeasuredPresentationScrollX ?? leaf.scrollLeft);
        let approvedScrollY = patch.presentationScrollY
            ?? patch.scrollY
            ?? (state?.lastMeasuredPresentationScrollY ?? leaf.scrollTop);
        if (ignoreActiveScrollPatch) {
            if (delegateToPageScroll) {
                let pageScroll = this.readPageScrollPosition();
                approvedScrollX = pageScroll.scrollX;
                approvedScrollY = pageScroll.scrollY;
            } else {
                approvedScrollX = state?.lastMeasuredPresentationScrollX ?? leaf.scrollLeft;
                approvedScrollY = state?.lastMeasuredPresentationScrollY ?? leaf.scrollTop;
            }
        }
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
        if (state != null) {
            this.syncScrollerSnap(patch, state, leaf, snapHost, delegateToPageScroll);
        }
        if (!delegateToPageScroll) {
            leaf.style.overflowX = shouldClip
                ? (contentWidth > viewportWidth ? "auto" : "hidden")
                : "visible";
            leaf.style.overflowY = shouldClip
                ? (contentHeight > viewportHeight ? "auto" : "hidden")
                : "visible";
        }
        if (patch.borderRadius != null) {
            leaf.style.borderRadius = `${patch.borderRadius}px`;
        }

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
        if (this.pageSnapOwnerScrollerId === id && this.pageSnapHost != null) {
            while (this.pageSnapHost.firstChild) {
                this.pageSnapHost.removeChild(this.pageSnapHost.firstChild);
            }
            this.pageSnapOwnerScrollerId = undefined;
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
        hosts?.snapHost.parentElement?.removeChild(hosts.snapHost);
        this.scrollerHosts.delete(id);
        let parent = oldNode.parentElement!;
        parent.removeChild(oldNode);
        this.nodesLookup.delete(id);
    }

    eventBlockerCreate(patch: AnyCreatePatch){
        console.assert(patch.id != null);
        console.assert(patch.renderLayerId != null);

        let eventBlockerDiv: HTMLDivElement = this.objectManager.getFromPool(DIV);
        // let eventBlocker: HTMLDivElement = this.objectManager.getFromPool(DIV);
        eventBlockerDiv.setAttribute("class", NATIVE_LEAF_CLASS)
        eventBlockerDiv.setAttribute("pax_id", String(patch.id));


        if(patch.id != undefined && patch.renderLayerId != undefined){
            this.layers.addElement(eventBlockerDiv, patch.parentFrame, patch.renderLayerId);
            this.nodesLookup.set(patch.id!, eventBlockerDiv);
        } else {
            throw new Error("undefined id or renderLayer");
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
        if (patch.background != null) {
            leaf.style.backgroundColor = toCssColor(patch.background);
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

            chassis.interrupt(message, pixels);
            responded = true;
            if (document.hidden) {
                this.postAsyncInterruptFlush?.();
            }
        };

        try {
            const mount = this.layers.parent;
            if (!(mount instanceof HTMLElement)) {
                console.warn('Could not resolve screenshot mount');
                return;
            }

            const scale = patch.scale ?? 1;
            canvas = document.createElement('canvas');
            canvas.width = Math.max(1, Math.round(mount.clientWidth * scale));
            canvas.height = Math.max(1, Math.round(mount.clientHeight * scale));
            const useHiddenTabCanvasFallback = document.hidden;

            ctx = canvas.getContext('2d');
            if (!ctx) {
                console.warn('Could not get canvas context');
                return;
            }

            const canvasLayers = Array
                .from(mount.querySelectorAll(`canvas.${CANVAS_CLASS}`))
                .reduce<ScreenshotCanvasLayer[]>((acc, element, index) => {
                    if (!(element instanceof HTMLCanvasElement) || !isRenderableScreenshotCanvas(element, mount)) {
                        return acc;
                    }
                    const layerId = screenshotCanvasLayerId(element);
                    if (layerId == null) {
                        return acc;
                    }
                    acc.push({
                        element,
                        index,
                        key: screenshotCanvasTileKey(element),
                        layerId,
                        zIndex: screenshotCanvasZIndex(element),
                    });
                    return acc;
                }, [])
                .sort(compareScreenshotCanvasLayers);

            const canvasLayerKeys = new Map<number, Set<string>>();
            const requestId = patch.id!;
            if (!useHiddenTabCanvasFallback) {
                for (const { layerId, key } of canvasLayers) {
                    let layerKeys = canvasLayerKeys.get(layerId);
                    if (layerKeys == null) {
                        layerKeys = new Set<string>();
                        canvasLayerKeys.set(layerId, layerKeys);
                    }
                    layerKeys.add(key);
                }
            }

            for (const layerId of canvasLayerKeys.keys()) {
                chassis.request_layer_screenshot(layerId, requestId);
            }

            const nextAnimationFrame = () => waitForDocumentFrame(
                document,
                HIDDEN_TAB_FRAME_FALLBACK_MS,
            );
            const waitForPaint = async (frames: number = 1) => {
                for (let frame = 0; frame < frames; frame += 1) {
                    await nextAnimationFrame();
                }
            };

            const layerCanvasCache = new Map<string, HTMLCanvasElement>();
            const canvasForSurfaceScreenshot = (
                layerId: number,
                screenshot: LayerSurfaceScreenshotData,
            ) => {
                const cacheKey = `${layerId}:${screenshot.key}:${screenshot.width}x${screenshot.height}`;
                const cached = layerCanvasCache.get(cacheKey);
                if (cached) {
                    return cached;
                }
                const layerCanvas = document.createElement('canvas');
                layerCanvas.width = screenshot.width;
                layerCanvas.height = screenshot.height;
                const layerContext = layerCanvas.getContext('2d');
                if (!layerContext) {
                    return null;
                }
                const imageData = new ImageData(
                    new Uint8ClampedArray(screenshot.data),
                    screenshot.width,
                    screenshot.height,
                );
                layerContext.putImageData(imageData, 0, 0);
                layerCanvasCache.set(cacheKey, layerCanvas);
                return layerCanvas;
            };

            const drawCanvasSourceForLayer = (
                layer: ScreenshotCanvasLayer,
                source: CanvasImageSource,
                sourcePixelWidth: number,
                sourcePixelHeight: number,
            ) => {
                if (!canvas || !ctx) {
                    return;
                }

                const destination = screenshotRectInMount(layer.element, mount);
                const clip = screenshotClipRectInMount(layer.element, mount);
                const visible = intersectScreenshotRects(destination, clip);
                if (
                    visible == null
                    || destination.right <= destination.left
                    || destination.bottom <= destination.top
                ) {
                    return;
                }

                const scaleX = canvas.width / Math.max(mount.clientWidth, 1);
                const scaleY = canvas.height / Math.max(mount.clientHeight, 1);
                const sourceX = ((visible.left - destination.left) / screenshotRectWidth(destination))
                    * sourcePixelWidth;
                const sourceY = ((visible.top - destination.top) / screenshotRectHeight(destination))
                    * sourcePixelHeight;
                const sourceWidth = (screenshotRectWidth(visible) / screenshotRectWidth(destination))
                    * sourcePixelWidth;
                const sourceHeight = (screenshotRectHeight(visible) / screenshotRectHeight(destination))
                    * sourcePixelHeight;

                ctx.save();
                applyScreenshotCanvasClips(ctx, layer.element, mount, scaleX, scaleY);
                ctx.drawImage(
                    source,
                    sourceX,
                    sourceY,
                    sourceWidth,
                    sourceHeight,
                    visible.left * scaleX,
                    visible.top * scaleY,
                    screenshotRectWidth(visible) * scaleX,
                    screenshotRectHeight(visible) * scaleY,
                );
                ctx.restore();
            };

            const drawLayerSurfaceScreenshot = (
                layer: ScreenshotCanvasLayer,
                screenshot: LayerSurfaceScreenshotData,
            ) => {
                const layerCanvas = canvasForSurfaceScreenshot(layer.layerId, screenshot);
                if (!layerCanvas) {
                    return;
                }
                drawCanvasSourceForLayer(layer, layerCanvas, screenshot.width, screenshot.height);
            };

            const drawLayerCanvasFallback = (layer: ScreenshotCanvasLayer) => {
                try {
                    drawCanvasSourceForLayer(
                        layer,
                        layer.element,
                        layer.element.width,
                        layer.element.height,
                    );
                } catch (err) {
                    console.warn(
                        `Could not draw Pax canvas fallback for layer ${layer.layerId} tile ${layer.key}`,
                        err,
                    );
                }
            };

            const waitForLayerSurfaceScreenshots = async (
                layerId: number,
                expectedKeys: Set<string>,
            ) => {
                const captures = new Map<string, LayerSurfaceScreenshotData>();
                for (let attempt = 0; attempt < 10; attempt += 1) {
                    const screenshots = normalizeLayerSurfaceScreenshots(
                        chassis.take_layer_surface_screenshots(layerId, requestId),
                    );
                    screenshots.forEach((screenshot) => {
                        captures.set(screenshot.key ?? "single", screenshot);
                    });
                    const complete = Array
                        .from(expectedKeys)
                        .every((key) => captures.has(key));
                    if (complete) {
                        return captures;
                    }
                    await nextAnimationFrame();
                }
                const missingKeys = Array
                    .from(expectedKeys)
                    .filter((key) => !captures.has(key));
                console.warn(
                    `Timed out waiting for Pax canvas screenshot for layer ${layerId}`
                    + (missingKeys.length > 0 ? ` tiles ${missingKeys.join(",")}` : ""),
                );
                return captures;
            };

            if (useHiddenTabCanvasFallback) {
                // Hidden tabs can stop servicing off-screen render/screenshot requests. Fall back to
                // the mounted DOM canvases plus native/text fallbacks, which preserves the last
                // active visual state without depending on background rendering support.
                for (const layer of canvasLayers) {
                    drawLayerCanvasFallback(layer);
                }
            } else {
                await waitForRegisteredFonts();
                await waitForPaint(2);

                const surfaceScreenshots = new Map<string, LayerSurfaceScreenshotData>();
                for (const [layerId, expectedKeys] of canvasLayerKeys) {
                    const screenshots = await waitForLayerSurfaceScreenshots(layerId, expectedKeys);
                    screenshots.forEach((screenshot, key) => {
                        surfaceScreenshots.set(layerSurfaceScreenshotKey(layerId, key), screenshot);
                    });
                }

                for (const layer of canvasLayers) {
                    const screenshot = surfaceScreenshots.get(
                        layerSurfaceScreenshotKey(layer.layerId, layer.key),
                    );
                    if (screenshot === undefined) {
                        drawLayerCanvasFallback(layer);
                        continue;
                    }
                    drawLayerSurfaceScreenshot(layer, screenshot);
                }
            }

            const hasNativeOverlayContent = Array
                .from(mount.querySelectorAll(`.${NATIVE_OVERLAY_CLASS}`))
                .some((element) => element.childElementCount > 0);

            if (hasNativeOverlayContent) {
                if (!document.hidden) {
                    const overlayRoot = Array
                        .from(mount.children)
                        .find((element) => element instanceof HTMLElement && element.classList.contains(NATIVE_OVERLAY_CLASS));
                    const overlayCanvas = await captureTransparentNativeOverlay(
                        (overlayRoot ?? mount) as HTMLElement,
                        mount,
                        scale,
                    ).catch((err) => {
                        console.warn('Proceeding without native overlay capture', err);
                        return null;
                    });
                    if (overlayCanvas) {
                        ctx.drawImage(overlayCanvas, 0, 0, canvas.width, canvas.height);
                    }
                }
                // html2canvas can stall in background tabs; the control/text fallbacks keep `pax dev look`
                // responsive even when the inspected page is hidden.
                drawNativeControlFallbacks(ctx, mount, canvas.width, canvas.height);
                drawPlainTextNativeLeaves(ctx, mount, canvas.width, canvas.height);
            }

            sendScreenshot();
        } catch (err) {
            console.error('html2canvas error:', err);
            sendScreenshot();
        }
    }
    
    async imageLoad(patch: ImageLoadPatch, chassis: PaxChassisWeb) {
        if (this.chassis !== chassis) {
            return
        }
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
        // A cartridge reload may dispose this pool while image decoding is in
        // flight. Never call back into a freed or superseded Wasm chassis.
        if (this.chassis !== chassis) {
            return
        }
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
        chassis.interrupt(message, image_data.pixels);
    }

    navigate(patch: NavigationPatch) {
        let destination = patch.url;
        if (!destination) {
            console.error("no valid url target!");
            return;
        }

        try {
            let url = new URL(destination, window.location.href);
            if (patch.target === "current" && url.origin === window.location.origin) {
                pushRouteHistoryState(url);
                this.chassis?.interrupt({
                    "RouteChange": serializeRouteLocation(url),
                }, []);
                return;
            }
        } catch (_err) {
            // Fall back to ordinary browser navigation for malformed or unsupported URLs.
        }

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
        window.open(destination, name);
    }

    setCursor(patch: SetCursorPatch) {
        document.body.style.cursor = patch.cursor!;
    }
}

type ScreenshotCanvasLayer = {
    element: HTMLCanvasElement;
    index: number;
    key: string;
    layerId: number;
    zIndex: number;
};

type LayerSurfaceScreenshotData = {
    data: Uint8Array | number[];
    height: number;
    id: number;
    key: string;
    logical_height?: number;
    logical_width?: number;
    origin_x?: number;
    origin_y?: number;
    width: number;
};

type ScreenshotRect = {
    left: number;
    top: number;
    right: number;
    bottom: number;
};

function normalizeLayerSurfaceScreenshots(value: any): LayerSurfaceScreenshotData[] {
    if (Array.isArray(value)) {
        return value.filter(isLayerSurfaceScreenshotData);
    }
    if (isLayerSurfaceScreenshotData(value)) {
        return [value];
    }
    return [];
}

function isLayerSurfaceScreenshotData(value: any): value is LayerSurfaceScreenshotData {
    return value != null
        && typeof value === "object"
        && value.data != null
        && Number.isFinite(value.width)
        && Number.isFinite(value.height);
}

function layerSurfaceScreenshotKey(layerId: number, key: string) {
    return `${layerId}:${key}`;
}

function screenshotCanvasLayerId(canvas: HTMLCanvasElement): number | undefined {
    const fromDataset = Number.parseInt(canvas.dataset.layerId ?? "", 10);
    if (Number.isFinite(fromDataset)) {
        return fromDataset;
    }
    const fromId = Number.parseInt(canvas.id, 10);
    return Number.isFinite(fromId) ? fromId : undefined;
}

function screenshotCanvasTileKey(canvas: HTMLCanvasElement) {
    return canvas.dataset.tileKey ?? "single";
}

function screenshotCanvasZIndex(canvas: HTMLCanvasElement) {
    const zIndex = Number.parseInt(window.getComputedStyle(canvas).zIndex || "0", 10);
    if (Number.isFinite(zIndex)) {
        return zIndex;
    }
    const layerId = screenshotCanvasLayerId(canvas);
    return layerId == null ? 0 : layerId * 2;
}

function compareScreenshotCanvasLayers(left: ScreenshotCanvasLayer, right: ScreenshotCanvasLayer) {
    if (left.zIndex !== right.zIndex) {
        return left.zIndex - right.zIndex;
    }
    if (left.layerId !== right.layerId) {
        return left.layerId - right.layerId;
    }
    return left.index - right.index;
}

function isRenderableScreenshotCanvas(canvas: HTMLCanvasElement, mount: HTMLElement) {
    if (canvas.closest('[data-role="canvas-pool-host"]')) {
        return false;
    }
    if (canvas.width <= 1 || canvas.height <= 1) {
        return false;
    }
    const rect = canvas.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) {
        return false;
    }

    let current: HTMLElement | null = canvas;
    while (current != null) {
        const style = window.getComputedStyle(current);
        if (
            style.display === "none"
            || style.visibility === "hidden"
            || style.visibility === "collapse"
        ) {
            return false;
        }
        if (current === mount) {
            return true;
        }
        current = current.parentElement;
    }
    return false;
}

function screenshotRectInMount(element: Element, mount: HTMLElement): ScreenshotRect {
    const elementRect = element.getBoundingClientRect();
    const mountRect = mount.getBoundingClientRect();
    return {
        left: elementRect.left - mountRect.left,
        top: elementRect.top - mountRect.top,
        right: elementRect.right - mountRect.left,
        bottom: elementRect.bottom - mountRect.top,
    };
}

function screenshotClipRectInMount(element: Element, mount: HTMLElement): ScreenshotRect {
    let clip: ScreenshotRect = {
        left: 0,
        top: 0,
        right: mount.clientWidth,
        bottom: mount.clientHeight,
    };

    let current = element.parentElement;
    while (current instanceof HTMLElement) {
        const style = window.getComputedStyle(current);
        if (current === mount || screenshotElementOverflowClips(style)) {
            const nextClip = intersectScreenshotRects(clip, screenshotRectInMount(current, mount));
            if (nextClip == null) {
                return { left: 0, top: 0, right: 0, bottom: 0 };
            }
            clip = nextClip;
        }
        const clipPathRect = screenshotClipPathRectInMount(current, mount, style);
        if (clipPathRect != null) {
            const nextClip = intersectScreenshotRects(clip, clipPathRect);
            if (nextClip == null) {
                return { left: 0, top: 0, right: 0, bottom: 0 };
            }
            clip = nextClip;
        }
        if (current === mount) {
            break;
        }
        current = current.parentElement;
    }

    return clip;
}

function applyScreenshotCanvasClips(
    ctx: CanvasRenderingContext2D,
    element: Element,
    mount: HTMLElement,
    scaleX: number,
    scaleY: number,
    includeSelf: boolean = false,
) {
    let current: Element | null = includeSelf ? element : element.parentElement;
    while (current instanceof HTMLElement) {
        const style = window.getComputedStyle(current);
        if (current === mount || screenshotElementOverflowClips(style)) {
            clipCanvasToScreenshotRect(
                ctx,
                screenshotRectInMount(current, mount),
                scaleX,
                scaleY,
            );
        }
        screenshotClipPathData(style).forEach((pathData) => {
            const scrollerScroll = screenshotAncestorScrollerScrollOffset(current, mount);
            ctx.setTransform(
                scaleX,
                0,
                0,
                scaleY,
                -scrollerScroll.x * scaleX,
                -scrollerScroll.y * scaleY,
            );
            ctx.clip(new Path2D(pathData));
            ctx.setTransform(1, 0, 0, 1, 0, 0);
        });
        if (current === mount) {
            break;
        }
        current = current.parentElement;
    }
}

function screenshotAncestorScrollerScrollOffset(element: Element, mount: HTMLElement) {
    let x = 0;
    let y = 0;
    let current = element.parentElement;
    while (current instanceof HTMLElement) {
        if (current.classList.contains(SCROLLER_CONTAINER)) {
            x += current.scrollLeft;
            y += current.scrollTop;
        }
        if (current === mount) {
            break;
        }
        current = current.parentElement;
    }
    return { x, y };
}

function clipCanvasToScreenshotRect(
    ctx: CanvasRenderingContext2D,
    rect: ScreenshotRect,
    scaleX: number,
    scaleY: number,
) {
    ctx.beginPath();
    ctx.rect(
        rect.left * scaleX,
        rect.top * scaleY,
        screenshotRectWidth(rect) * scaleX,
        screenshotRectHeight(rect) * scaleY,
    );
    ctx.clip();
}

function screenshotElementOverflowClips(style: CSSStyleDeclaration) {
    return screenshotOverflowClips(style.overflow)
        || screenshotOverflowClips(style.overflowX)
        || screenshotOverflowClips(style.overflowY);
}

function screenshotOverflowClips(value: string) {
    return value === "hidden"
        || value === "scroll"
        || value === "auto"
        || value === "clip";
}

function screenshotClipPathRectInMount(
    element: Element,
    mount: HTMLElement,
    style: CSSStyleDeclaration,
): ScreenshotRect | null {
    const scrollerScroll = screenshotAncestorScrollerScrollOffset(element, mount);
    const pathRects = screenshotClipPathData(style)
        .map(svgPathBounds)
        .map((rect) => rect == null
            ? null
            : translateScreenshotRect(rect, -scrollerScroll.x, -scrollerScroll.y))
        .filter((rect): rect is ScreenshotRect => rect != null);
    if (pathRects.length === 0) {
        return null;
    }
    return pathRects.reduce<ScreenshotRect | null>(
        (acc, rect) => acc == null ? rect : unionScreenshotRects(acc, rect),
        null,
    );
}

function screenshotClipPathData(style: CSSStyleDeclaration) {
    const clipPath = screenshotClipPathElement(style);
    if (clipPath == null) {
        return [];
    }

    const paths: string[] = [];
    collectScreenshotClipPathData(clipPath, paths);
    return paths;
}

function collectScreenshotClipPathData(element: Element, paths: string[]) {
    if (element instanceof SVGPathElement) {
        const pathData = element.getAttribute("d");
        if (pathData != null && pathData.length > 0) {
            paths.push(pathData);
        }
        return;
    }

    Array.from(element.children).forEach((child) => {
        collectScreenshotClipPathData(child, paths);
    });
}

function screenshotClipPathElement(style: CSSStyleDeclaration): SVGClipPathElement | null {
    const clipPathValue = screenshotClipPathValue(style);
    if (clipPathValue == null) {
        return null;
    }

    const match = clipPathValue.match(/url\(["']?#([^"')]+)["']?\)/);
    if (match == null) {
        return null;
    }
    const element = document.getElementById(match[1]);
    return element instanceof SVGClipPathElement ? element : null;
}

function screenshotClipPathValue(style: CSSStyleDeclaration): string | null {
    const webkitClipPath = (style as CSSStyleDeclaration & { webkitClipPath?: string }).webkitClipPath;
    const clipPath = style.clipPath && style.clipPath !== "none"
        ? style.clipPath
        : webkitClipPath;
    return clipPath && clipPath !== "none" ? clipPath : null;
}

function svgPathBounds(pathData: string): ScreenshotRect | null {
    const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
    path.setAttribute("d", pathData);
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.style.position = "absolute";
    svg.style.width = "0";
    svg.style.height = "0";
    svg.style.pointerEvents = "none";
    svg.appendChild(path);
    document.body.appendChild(svg);
    let rect: ScreenshotRect | null = null;
    try {
        const bounds = path.getBBox();
        if (bounds.width > 0 && bounds.height > 0) {
            rect = {
                left: bounds.x,
                top: bounds.y,
                right: bounds.x + bounds.width,
                bottom: bounds.y + bounds.height,
            };
        }
    } finally {
        svg.remove();
    }
    return rect;
}

function intersectScreenshotRects(
    left: ScreenshotRect,
    right: ScreenshotRect,
): ScreenshotRect | null {
    const intersection = {
        left: Math.max(left.left, right.left),
        top: Math.max(left.top, right.top),
        right: Math.min(left.right, right.right),
        bottom: Math.min(left.bottom, right.bottom),
    };
    if (intersection.right <= intersection.left || intersection.bottom <= intersection.top) {
        return null;
    }
    return intersection;
}

function unionScreenshotRects(left: ScreenshotRect, right: ScreenshotRect): ScreenshotRect {
    return {
        left: Math.min(left.left, right.left),
        top: Math.min(left.top, right.top),
        right: Math.max(left.right, right.right),
        bottom: Math.max(left.bottom, right.bottom),
    };
}

function translateScreenshotRect(
    rect: ScreenshotRect,
    x: number,
    y: number,
): ScreenshotRect {
    return {
        left: rect.left + x,
        top: rect.top + y,
        right: rect.right + x,
        bottom: rect.bottom + y,
    };
}

function screenshotRectWidth(rect: ScreenshotRect) {
    return Math.max(0, rect.right - rect.left);
}

function screenshotRectHeight(rect: ScreenshotRect) {
    return Math.max(0, rect.bottom - rect.top);
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

async function captureTransparentNativeOverlay(
    target: HTMLElement,
    mount: HTMLElement,
    scale: number,
) {
    const black = await captureNativeOverlayAgainstBackground(
        target,
        mount,
        scale,
        SCREENSHOT_OVERLAY_BLACK,
    );
    const white = await captureNativeOverlayAgainstBackground(
        target,
        mount,
        scale,
        SCREENSHOT_OVERLAY_WHITE,
    );
    return reconstructTransparentOverlay(black, white);
}

function captureNativeOverlayAgainstBackground(
    target: HTMLElement,
    mount: HTMLElement,
    scale: number,
    backgroundColor: string,
) {
    const scrollerPositions = collectScreenshotScrollerPositions(target);
    return html2canvas(target, {
        backgroundColor,
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
            clonedDocument.documentElement.style.backgroundColor = backgroundColor;
            clonedDocument.body.style.width = `${mount.clientWidth}px`;
            clonedDocument.body.style.height = `${mount.clientHeight}px`;
            clonedDocument.body.style.backgroundColor = backgroundColor;
            clonedMount.style.width = `${mount.clientWidth}px`;
            clonedMount.style.height = `${mount.clientHeight}px`;
            clonedMount.style.backgroundColor = backgroundColor;

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

            restoreClonedScrollerPositions(clonedMount, scrollerPositions);
            await waitForClonedAnimationFrame(clonedDocument);
        }) as unknown as (document: Document, element: HTMLElement) => void,
        ignoreElements: (overlayElement: Element) => isScreenshotIgnoredElement(overlayElement),
    });
}

type ScreenshotScrollerPosition = {
    id: string;
    scrollLeft: number;
    scrollTop: number;
};

function collectScreenshotScrollerPositions(root: HTMLElement): ScreenshotScrollerPosition[] {
    const scrollers = root.classList.contains(SCROLLER_CONTAINER)
        ? [root, ...Array.from(root.querySelectorAll(`.${SCROLLER_CONTAINER}`))]
        : Array.from(root.querySelectorAll(`.${SCROLLER_CONTAINER}`));
    return scrollers
        .reduce<ScreenshotScrollerPosition[]>((positions, element) => {
            if (!(element instanceof HTMLElement)) {
                return positions;
            }
            const id = element.getAttribute("pax_id");
            if (id == null) {
                return positions;
            }
            positions.push({
                id,
                scrollLeft: element.scrollLeft,
                scrollTop: element.scrollTop,
            });
            return positions;
        }, []);
}

function restoreClonedScrollerPositions(
    clonedRoot: HTMLElement,
    positions: ScreenshotScrollerPosition[],
) {
    positions.forEach((position) => {
        const selector = `.${SCROLLER_CONTAINER}[pax_id="${CSS.escape(position.id)}"]`;
        const clone = clonedRoot.querySelector(selector);
        if (!(clone instanceof HTMLElement)) {
            return;
        }
        clone.scrollLeft = position.scrollLeft;
        clone.scrollTop = position.scrollTop;
    });
}

function waitForClonedAnimationFrame(clonedDocument: Document) {
    return waitForDocumentFrame(clonedDocument, HIDDEN_TAB_FRAME_FALLBACK_MS);
}

function reconstructTransparentOverlay(
    blackCanvas: HTMLCanvasElement,
    whiteCanvas: HTMLCanvasElement,
) {
    if (blackCanvas.width !== whiteCanvas.width || blackCanvas.height !== whiteCanvas.height) {
        return blackCanvas;
    }

    const blackContext = blackCanvas.getContext('2d');
    const whiteContext = whiteCanvas.getContext('2d');
    if (!blackContext || !whiteContext) {
        return blackCanvas;
    }

    const blackImage = blackContext.getImageData(0, 0, blackCanvas.width, blackCanvas.height);
    const whiteImage = whiteContext.getImageData(0, 0, whiteCanvas.width, whiteCanvas.height);
    const blackData = blackImage.data;
    const whiteData = whiteImage.data;

    for (let index = 0; index < blackData.length; index += 4) {
        const inverseAlpha = clampByte(Math.max(
            whiteData[index] - blackData[index],
            whiteData[index + 1] - blackData[index + 1],
            whiteData[index + 2] - blackData[index + 2],
        ));
        const alpha = 255 - inverseAlpha;
        if (alpha <= 0) {
            blackData[index] = 0;
            blackData[index + 1] = 0;
            blackData[index + 2] = 0;
            blackData[index + 3] = 0;
            continue;
        }

        blackData[index] = clampByte(Math.round((blackData[index] * 255) / alpha));
        blackData[index + 1] = clampByte(Math.round((blackData[index + 1] * 255) / alpha));
        blackData[index + 2] = clampByte(Math.round((blackData[index + 2] * 255) / alpha));
        blackData[index + 3] = alpha;
    }

    blackContext.putImageData(blackImage, 0, 0);
    return blackCanvas;
}

function clampByte(value: number) {
    return Math.min(255, Math.max(0, value));
}

function drawNativeControlFallbacks(
    ctx: CanvasRenderingContext2D,
    mount: HTMLElement,
    outputWidth: number,
    outputHeight: number,
) {
    const scaleX = outputWidth / Math.max(mount.clientWidth, 1);
    const scaleY = outputHeight / Math.max(mount.clientHeight, 1);

    mount.querySelectorAll(`.${NATIVE_LEAF_CLASS}`).forEach((leafNode) => {
        if (!(leafNode instanceof HTMLElement)) {
            return;
        }
        const control = leafNode.firstElementChild;
        if (!isDrawableNativeControl(control)) {
            return;
        }

        const leafStyle = window.getComputedStyle(leafNode);
        const transform = leafStyle.transform !== 'none'
            ? new DOMMatrixReadOnly(leafStyle.transform)
            : new DOMMatrixReadOnly();
        const scrollerScroll = screenshotAncestorScrollerScrollOffset(leafNode, mount);

        ctx.save();
        applyScreenshotCanvasClips(ctx, leafNode, mount, scaleX, scaleY, true);
        ctx.globalAlpha *= Number.parseFloat(leafStyle.opacity || '1') || 1;
        ctx.setTransform(scaleX, 0, 0, scaleY, 0, 0);
        ctx.transform(
            transform.a,
            transform.b,
            transform.c,
            transform.d,
            transform.e - scrollerScroll.x,
            transform.f - scrollerScroll.y,
        );

        if (control instanceof HTMLButtonElement) {
            drawButtonControlFallback(ctx, control);
        } else if (control instanceof HTMLSelectElement) {
            drawSelectControlFallback(ctx, control);
        } else if (control instanceof HTMLInputElement) {
            if (control.type === 'checkbox') {
                drawCheckboxControlFallback(ctx, control);
            } else if (control.type === 'range') {
                drawRangeControlFallback(ctx, control);
            } else {
                drawTextboxControlFallback(ctx, control);
            }
        } else if (control instanceof HTMLFieldSetElement) {
            drawRadioListControlFallback(ctx, leafNode, control);
        }

        ctx.restore();
    });
}

function isDrawableNativeControl(element: Element | null): element is HTMLElement {
    return element instanceof HTMLButtonElement
        || element instanceof HTMLSelectElement
        || element instanceof HTMLInputElement
        || element instanceof HTMLFieldSetElement;
}

function drawButtonControlFallback(ctx: CanvasRenderingContext2D, button: HTMLButtonElement) {
    const style = window.getComputedStyle(button);
    const box = controlLocalBox(button);
    drawStyledControlBox(ctx, box, style);

    const textContainer = button.querySelector(`.${BUTTON_TEXT_CONTAINER_CLASS}`);
    const textElement = textContainer?.firstElementChild instanceof HTMLElement
        ? textContainer.firstElementChild
        : button;
    const textStyle = window.getComputedStyle(textElement);
    const text = (textElement.textContent ?? button.textContent ?? '').trim();
    drawControlText(ctx, text, textStyle, {
        x: box.x + parseCssPx(style.paddingLeft, 0),
        y: box.y,
        width: Math.max(0, box.width - parseCssPx(style.paddingLeft, 0) - parseCssPx(style.paddingRight, 0)),
        height: box.height,
    }, 'center');
}

function drawSelectControlFallback(ctx: CanvasRenderingContext2D, select: HTMLSelectElement) {
    const style = window.getComputedStyle(select);
    const box = controlLocalBox(select);
    drawStyledControlBox(ctx, box, style);

    const selected = select.options.item(select.selectedIndex);
    const label = selected?.textContent ?? '';
    const paddingLeft = parseCssPx(style.paddingLeft, 10);
    const arrowWidth = Math.min(28, box.width * 0.18);
    drawControlText(ctx, label, style, {
        x: box.x + paddingLeft,
        y: box.y,
        width: Math.max(0, box.width - paddingLeft - arrowWidth),
        height: box.height,
    }, 'left');

    const arrowCenterX = box.x + box.width - arrowWidth * 0.5;
    const arrowCenterY = box.y + box.height * 0.5;
    ctx.save();
    ctx.strokeStyle = nonTransparentColor(style.color, '#000000');
    ctx.lineWidth = 1.5;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.beginPath();
    ctx.moveTo(arrowCenterX - 5, arrowCenterY - 3);
    ctx.lineTo(arrowCenterX, arrowCenterY + 3);
    ctx.lineTo(arrowCenterX + 5, arrowCenterY - 3);
    ctx.stroke();
    ctx.restore();
}

function drawTextboxControlFallback(ctx: CanvasRenderingContext2D, input: HTMLInputElement) {
    const style = window.getComputedStyle(input);
    const box = controlLocalBox(input);
    drawStyledControlBox(ctx, box, style);

    const paddingLeft = parseCssPx(style.paddingLeft, 8);
    const paddingRight = parseCssPx(style.paddingRight, 8);
    const value = input.value.length > 0 ? input.value : input.placeholder;
    drawControlText(ctx, value, style, {
        x: box.x + paddingLeft,
        y: box.y,
        width: Math.max(0, box.width - paddingLeft - paddingRight),
        height: box.height,
    }, textAlignForControl(style, 'left'));
}

function drawCheckboxControlFallback(ctx: CanvasRenderingContext2D, input: HTMLInputElement) {
    const style = window.getComputedStyle(input);
    const box = controlLocalBox(input);
    drawStyledControlBox(ctx, box, style);

    if (!input.checked) {
        return;
    }

    ctx.save();
    ctx.strokeStyle = '#ffffff';
    ctx.lineWidth = Math.max(2, Math.min(box.width, box.height) * 0.12);
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.beginPath();
    ctx.moveTo(box.x + box.width * 0.24, box.y + box.height * 0.53);
    ctx.lineTo(box.x + box.width * 0.43, box.y + box.height * 0.72);
    ctx.lineTo(box.x + box.width * 0.76, box.y + box.height * 0.30);
    ctx.stroke();
    ctx.restore();
}

function drawRadioListControlFallback(
    ctx: CanvasRenderingContext2D,
    leaf: HTMLElement,
    fieldset: HTMLFieldSetElement,
) {
    Array.from(fieldset.children).forEach((row) => {
        if (!(row instanceof HTMLElement)) {
            return;
        }
        const input = row.querySelector('input[type="radio"]');
        const label = row.querySelector('label');
        if (!(input instanceof HTMLInputElement)) {
            return;
        }
        const inputStyle = window.getComputedStyle(input);
        const inputBox = controlLocalBox(input, leaf);
        drawStyledControlBox(ctx, inputBox, inputStyle);
        if (input.checked) {
            ctx.save();
            ctx.fillStyle = '#ffffff';
            ctx.beginPath();
            ctx.arc(
                inputBox.x + inputBox.width * 0.5,
                inputBox.y + inputBox.height * 0.5,
                Math.min(inputBox.width, inputBox.height) * 0.20,
                0,
                Math.PI * 2,
            );
            ctx.fill();
            ctx.restore();
        }

        if (label instanceof HTMLElement) {
            const labelStyle = window.getComputedStyle(label);
            const labelBox = controlLocalBox(label, leaf);
            drawControlText(ctx, label.textContent ?? '', labelStyle, {
                x: labelBox.x,
                y: labelBox.y,
                width: Math.max(labelBox.width, leaf.offsetWidth - labelBox.x),
                height: Math.max(labelBox.height, inputBox.height),
            }, 'left');
        }
    });
}

function drawRangeControlFallback(ctx: CanvasRenderingContext2D, input: HTMLInputElement) {
    const style = window.getComputedStyle(input);
    const box = controlLocalBox(input);
    const min = Number.parseFloat(input.min || '0');
    const max = Number.parseFloat(input.max || '100');
    const value = Number.parseFloat(input.value || '0');
    const t = max > min ? Math.min(1, Math.max(0, (value - min) / (max - min))) : 0;
    const trackHeight = Math.max(4, Math.min(8, box.height * 0.28));
    const radius = Math.max(trackHeight * 0.5, parseCssPx(style.borderRadius, 0));
    const trackY = box.y + (box.height - trackHeight) * 0.5;
    const thumbRadius = Math.max(7, Math.min(11, box.height * 0.42));
    const thumbX = box.x + thumbRadius + t * Math.max(0, box.width - thumbRadius * 2);
    const trackX = box.x + thumbRadius;
    const trackWidth = Math.max(0, box.width - thumbRadius * 2);
    const accent = nonTransparentColor(style.accentColor, '#899006');
    const background = nonTransparentColor(style.backgroundColor, '#f7d894');

    ctx.save();
    ctx.fillStyle = background;
    roundedRectPath(ctx, trackX, trackY, trackWidth, trackHeight, radius);
    ctx.fill();
    ctx.fillStyle = accent;
    roundedRectPath(ctx, trackX, trackY, Math.max(0, thumbX - trackX), trackHeight, radius);
    ctx.fill();
    ctx.fillStyle = accent;
    ctx.beginPath();
    ctx.arc(thumbX, box.y + box.height * 0.5, thumbRadius, 0, Math.PI * 2);
    ctx.fill();
    ctx.restore();
}

type NativeControlBox = {
    x: number;
    y: number;
    width: number;
    height: number;
};

function controlLocalBox(element: HTMLElement, ancestor?: HTMLElement): NativeControlBox {
    const targetAncestor = ancestor ?? element.parentElement;
    if (targetAncestor != null && targetAncestor !== element.parentElement) {
        return {
            x: Math.max(0, element.offsetLeft),
            y: Math.max(0, element.offsetTop),
            width: element.offsetWidth || parseCssPx(window.getComputedStyle(element).width, 0),
            height: element.offsetHeight || parseCssPx(window.getComputedStyle(element).height, 0),
        };
    }
    const style = window.getComputedStyle(element);
    return {
        x: element.offsetLeft || 0,
        y: element.offsetTop || 0,
        width: element.offsetWidth || parseCssPx(style.width, element.getBoundingClientRect().width),
        height: element.offsetHeight || parseCssPx(style.height, element.getBoundingClientRect().height),
    };
}

function drawStyledControlBox(
    ctx: CanvasRenderingContext2D,
    box: NativeControlBox,
    style: CSSStyleDeclaration,
) {
    if (box.width <= 0 || box.height <= 0) {
        return;
    }

    const radius = parseCssPx(style.borderRadius, 0);
    const background = nonTransparentColor(style.backgroundColor, '#ffffff');
    const borderWidth = parseCssPx(style.borderWidth, 0);
    const borderColor = nonTransparentColor(style.borderColor, 'transparent');

    ctx.save();
    if (background !== 'transparent') {
        ctx.fillStyle = background;
        roundedRectPath(ctx, box.x, box.y, box.width, box.height, radius);
        ctx.fill();
    }
    if (borderWidth > 0 && borderColor !== 'transparent') {
        ctx.strokeStyle = borderColor;
        ctx.lineWidth = borderWidth;
        roundedRectPath(
            ctx,
            box.x + borderWidth * 0.5,
            box.y + borderWidth * 0.5,
            Math.max(0, box.width - borderWidth),
            Math.max(0, box.height - borderWidth),
            Math.max(0, radius - borderWidth * 0.5),
        );
        ctx.stroke();
    }
    ctx.restore();
}

function drawControlText(
    ctx: CanvasRenderingContext2D,
    text: string,
    style: CSSStyleDeclaration,
    box: NativeControlBox,
    fallbackAlign: CanvasTextAlign,
) {
    if (text.length === 0 || box.width <= 0 || box.height <= 0) {
        return;
    }

    ctx.save();
    ctx.fillStyle = nonTransparentColor(style.color, '#000000');
    ctx.font = style.font;
    ctx.textBaseline = 'middle';
    ctx.textAlign = textAlignForControl(style, fallbackAlign);
    ctx.direction = (style.direction as CanvasDirection) || 'ltr';
    applyCanvasTypography(ctx, style);

    let x = box.x;
    if (ctx.textAlign === 'center') {
        x += box.width * 0.5;
    } else if (ctx.textAlign === 'right' || ctx.textAlign === 'end') {
        x += box.width;
    }
    ctx.fillText(text, x, box.y + box.height * 0.5, box.width);
    ctx.restore();
}

function textAlignForControl(
    style: CSSStyleDeclaration,
    fallback: CanvasTextAlign,
): CanvasTextAlign {
    const textAlign = style.textAlign as CanvasTextAlign;
    return textAlign && textAlign !== 'start' ? textAlign : fallback;
}

function roundedRectPath(
    ctx: CanvasRenderingContext2D,
    x: number,
    y: number,
    width: number,
    height: number,
    radius: number,
) {
    const r = Math.max(0, Math.min(radius, width * 0.5, height * 0.5));
    ctx.beginPath();
    ctx.moveTo(x + r, y);
    ctx.lineTo(x + width - r, y);
    ctx.quadraticCurveTo(x + width, y, x + width, y + r);
    ctx.lineTo(x + width, y + height - r);
    ctx.quadraticCurveTo(x + width, y + height, x + width - r, y + height);
    ctx.lineTo(x + r, y + height);
    ctx.quadraticCurveTo(x, y + height, x, y + height - r);
    ctx.lineTo(x, y + r);
    ctx.quadraticCurveTo(x, y, x + r, y);
    ctx.closePath();
}

function parseCssPx(value: string | null | undefined, fallback: number) {
    if (value == null || value.length === 0) {
        return fallback;
    }
    const parsed = Number.parseFloat(value);
    return Number.isFinite(parsed) ? parsed : fallback;
}

function nonTransparentColor(value: string | null | undefined, fallback: string) {
    if (value == null || value.length === 0 || value === 'transparent') {
        return fallback;
    }
    if (/rgba\([^)]*,\s*0(?:\.0+)?\s*\)$/i.test(value)) {
        return fallback;
    }
    return value;
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

function injectScrollerChromeCss(targetDocument: Document) {
    if (targetDocument.head?.querySelector(`style[${SCROLLER_CHROME_STYLE_ATTRIBUTE}]`)) {
        return;
    }

    const style = targetDocument.createElement('style');
    style.setAttribute(SCROLLER_CHROME_STYLE_ATTRIBUTE, 'true');
    style.textContent = `
        .${SCROLLER_CONTAINER} {
            scrollbar-color: rgba(128, 138, 150, 0.72) transparent;
            scrollbar-width: thin;
        }

        .${SCROLLER_CONTAINER}::-webkit-scrollbar {
            width: 12px;
            height: 12px;
            background: transparent;
        }

        .${SCROLLER_CONTAINER}::-webkit-scrollbar-track,
        .${SCROLLER_CONTAINER}::-webkit-scrollbar-corner {
            background: transparent;
        }

        .${SCROLLER_CONTAINER}::-webkit-scrollbar-thumb {
            background-color: rgba(128, 138, 150, 0.72);
            border-radius: 999px;
            border: 3px solid transparent;
            background-clip: padding-box;
        }
    `;
    (targetDocument.head ?? targetDocument.documentElement).appendChild(style);
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
        applyScreenshotCanvasClips(ctx, leafNode, mount, scaleX, scaleY, true);
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
        const scrollerScroll = screenshotAncestorScrollerScrollOffset(leafNode, mount);
        const fontSize = Number.parseFloat(textStyle.fontSize || '16');
        const lineHeight = Number.parseFloat(textStyle.lineHeight || '') || fontSize * 1.2;
        const localWidth = Number.parseFloat(leafStyle.width || '') || leafNode.offsetWidth || leafRect.width;
        const localHeight = Number.parseFloat(leafStyle.height || '') || leafNode.offsetHeight || leafRect.height;
        const preservesWhitespace = textStyle.whiteSpace.includes("pre");
        const lines = preservesWhitespace ? text.split('\n') : wrapPlainText(text, localWidth, ctx);
        const totalTextHeight = lineHeight * lines.length;

        ctx.setTransform(scaleX, 0, 0, scaleY, 0, 0);
        ctx.transform(
            transform.a,
            transform.b,
            transform.c,
            transform.d,
            transform.e - scrollerScroll.x,
            transform.f - scrollerScroll.y,
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
            if (preservesWhitespace) {
                ctx.fillText(line, x, startY + index * lineHeight);
            } else {
                ctx.fillText(line, x, startY + index * lineHeight, localWidth);
            }
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
    snapHostDelegatesToPageScroll: boolean;
    // `ScrollerUpdate` messages only include contentLayerId when it changes, but root page-scroll
    // delegation also needs the last engine-approved viewport/content metrics across later
    // scroll-only deltas.
    contentLayerId?: number;
    viewportWidth?: number;
    viewportHeight?: number;
    contentWidth?: number;
    contentHeight?: number;
    clipContent?: boolean;
    transform: number[];
    lastMeasuredScrollX: number;
    lastMeasuredScrollY: number;
    lastMeasuredPresentationScrollX: number;
    lastMeasuredPresentationScrollY: number;
    lastSentScrollX: number;
    lastSentScrollY: number;
    lastSentPresentationScrollX: number;
    lastSentPresentationScrollY: number;
    lastWarmAt: number;
    snapPointsX?: number[];
    snapPointsY?: number[];
};

type PendingScrollerUpdate = {
    id: number;
    sizeX?: number;
    sizeY?: number;
    clipContent?: boolean;
    borderRadius?: number;
    sizeInnerPaneX?: number;
    sizeInnerPaneY?: number;
    snapPointsX?: number[];
    snapPointsY?: number[];
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
    snapHost: HTMLDivElement;
    canvasHost: HTMLDivElement;
    contentHost: HTMLDivElement;
    parked: boolean;
    warmed: boolean;
    vectorIslandEnabled: boolean;
};

type ScrollerWarmIntent = "active" | "prewarm" | "cold";

type ScrollerWarmCandidate = {
    id: number;
    hosts: ScrollerDomHosts;
    state: ScrollerMeasurementState;
    record: {
        presentedBounds: AxisAlignedRect;
        presentedClipBounds: AxisAlignedRect;
    };
    parentId?: number;
    intent: ScrollerWarmIntent;
    distance: number;
    surfaceCost: number;
    mandatory: boolean;
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

const IOS_WARM_LINGER_MS = 3000;
const DEFAULT_ACTIVE_SCROLLABLE_PAD_X_MULTIPLIER = 4.0;
const DEFAULT_ACTIVE_SCROLLABLE_PAD_Y_MULTIPLIER = 4.0;
const DEFAULT_ACTIVE_SCROLLABLE_MIN_PAD_X = 960;
const DEFAULT_ACTIVE_SCROLLABLE_MIN_PAD_Y = 1024;
const DEFAULT_PREWARM_SCROLLABLE_PAD_X_MULTIPLIER = 8.0;
const DEFAULT_PREWARM_SCROLLABLE_PAD_Y_MULTIPLIER = 8.0;
const DEFAULT_PREWARM_SCROLLABLE_MIN_PAD_X = 1920;
const DEFAULT_PREWARM_SCROLLABLE_MIN_PAD_Y = 2048;
const IOS_ACTIVE_SCROLLABLE_PAD_X_MULTIPLIER = 8.0;
const IOS_ACTIVE_SCROLLABLE_PAD_Y_MULTIPLIER = 4.0;
const IOS_ACTIVE_SCROLLABLE_MIN_PAD_X = 1920;
const IOS_ACTIVE_SCROLLABLE_MIN_PAD_Y = 1024;
const IOS_PREWARM_SCROLLABLE_PAD_X_MULTIPLIER = 16.0;
const IOS_PREWARM_SCROLLABLE_PAD_Y_MULTIPLIER = 8.0;
const IOS_PREWARM_SCROLLABLE_MIN_PAD_X = 3840;
const IOS_PREWARM_SCROLLABLE_MIN_PAD_Y = 2048;
// The ordinary iOS path now uses Piet/2D canvas, not one WebGL context per tile. Keep enough
// browser canvases around to prevent fast scrolls from exposing cold, uninitialized tile slots.
const IOS_NESTED_BROWSER_SURFACE_BUDGET = 64;
const IOS_TOTAL_CANVAS_BUDGET = 64;
const DEFAULT_TOTAL_CANVAS_BUDGET = 64;

function browserNestedScrollerSurfaceBudget() {
    return isIOSWebKitBrowser() ? IOS_NESTED_BROWSER_SURFACE_BUDGET : undefined;
}

function browserTotalCanvasBudget() {
    return isIOSWebKitBrowser() ? IOS_TOTAL_CANVAS_BUDGET : undefined;
}

function browserCanvasPoolBudget() {
    let budget = browserTotalCanvasBudget();
    if (budget != null) {
        return budget;
    }
    return DEFAULT_TOTAL_CANVAS_BUDGET;
}

function browserCanvasPoolReuseCooldownMs() {
    // WebGPU on desktop browsers can glitch if a canvas is rebound to a new device immediately
    // after release. Add a small cooldown to reduce rapid reuse across layers.
    return isIOSWebKitBrowser() ? 0 : 200;
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

function expectedCanvasBackingSize(
    logicalWidth: number,
    logicalHeight: number,
    desiredDpr: number,
    maxSurfaceDimension: number,
    minimumDpr: number,
) {
    let minDpr = clampNumber(minimumDpr, 0.1, 1.0);
    let requestedDpr = Math.max(desiredDpr, minDpr);
    let dimensionLimit = Number.isFinite(maxSurfaceDimension)
        ? Math.max(1, maxSurfaceDimension)
        : Number.POSITIVE_INFINITY;
    let widthLimit = logicalWidth > 0 ? dimensionLimit / logicalWidth : requestedDpr;
    let heightLimit = logicalHeight > 0 ? dimensionLimit / logicalHeight : requestedDpr;
    let requestedDprX = Math.max(minDpr, Math.min(requestedDpr, widthLimit));
    let requestedDprY = Math.max(minDpr, Math.min(requestedDpr, heightLimit));
    let maxBacking = Number.isFinite(dimensionLimit)
        ? dimensionLimit
        : Number.MAX_SAFE_INTEGER;
    return {
        width: Math.max(1, Math.min(maxBacking, Math.round(logicalWidth * requestedDprX))),
        height: Math.max(1, Math.min(maxBacking, Math.round(logicalHeight * requestedDprY))),
    };
}

function clampNumber(value: number, min: number, max: number) {
    return Math.max(min, Math.min(max, value));
}

function scrollerWarmIntentRank(intent: ScrollerWarmIntent) {
    switch (intent) {
        case "active":
            return 2;
        case "prewarm":
            return 1;
        default:
            return 0;
    }
}

function activeScrollablePadX(viewportWidth: number) {
    let multiplier = DEFAULT_ACTIVE_SCROLLABLE_PAD_X_MULTIPLIER;
    let minPad = DEFAULT_ACTIVE_SCROLLABLE_MIN_PAD_X;
    if (isIOSWebKitBrowser()) {
        multiplier = IOS_ACTIVE_SCROLLABLE_PAD_X_MULTIPLIER;
        minPad = IOS_ACTIVE_SCROLLABLE_MIN_PAD_X;
    }
    return Math.max(viewportWidth * multiplier, minPad);
}

function activeScrollablePadY(viewportHeight: number) {
    let multiplier = DEFAULT_ACTIVE_SCROLLABLE_PAD_Y_MULTIPLIER;
    let minPad = DEFAULT_ACTIVE_SCROLLABLE_MIN_PAD_Y;
    if (isIOSWebKitBrowser()) {
        multiplier = IOS_ACTIVE_SCROLLABLE_PAD_Y_MULTIPLIER;
        minPad = IOS_ACTIVE_SCROLLABLE_MIN_PAD_Y;
    }
    return Math.max(viewportHeight * multiplier, minPad);
}

function prewarmScrollablePadX(viewportWidth: number) {
    let multiplier = DEFAULT_PREWARM_SCROLLABLE_PAD_X_MULTIPLIER;
    let minPad = DEFAULT_PREWARM_SCROLLABLE_MIN_PAD_X;
    if (isIOSWebKitBrowser()) {
        multiplier = IOS_PREWARM_SCROLLABLE_PAD_X_MULTIPLIER;
        minPad = IOS_PREWARM_SCROLLABLE_MIN_PAD_X;
    }
    return Math.max(viewportWidth * multiplier, minPad);
}

function prewarmScrollablePadY(viewportHeight: number) {
    let multiplier = DEFAULT_PREWARM_SCROLLABLE_PAD_Y_MULTIPLIER;
    let minPad = DEFAULT_PREWARM_SCROLLABLE_MIN_PAD_Y;
    if (isIOSWebKitBrowser()) {
        multiplier = IOS_PREWARM_SCROLLABLE_PAD_Y_MULTIPLIER;
        minPad = IOS_PREWARM_SCROLLABLE_MIN_PAD_Y;
    }
    return Math.max(viewportHeight * multiplier, minPad);
}

function rectWidth(rect: AxisAlignedRect) {
    return Math.max(0, rect.right - rect.left);
}

function rectHeight(rect: AxisAlignedRect) {
    return Math.max(0, rect.bottom - rect.top);
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

const PAX_GENERATED_CODE_MARKUP_PREFIX = '<pre data-pax-code-markup="example-host" ';

function renderMarkdownTextContent(content: string): string {
    if (content.startsWith(PAX_GENERATED_CODE_MARKUP_PREFIX)) {
        return content;
    }
    return snarkdown(content);
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
