import {AnyCreatePatch} from "../classes/messages/any-create-patch";
import {FrameUpdatePatch} from "../classes/messages/frame-update-patch";
import {TextUpdatePatch} from "../classes/messages/text-update-patch";
import {ScrollerUpdatePatch} from "../classes/messages/scroller-update-patch";
import {ImageLoadPatch} from "../classes/messages/image-load-patch";
import {CANVAS_CLASS} from "../utils/constants";
import {ObjectManager} from "./object-manager";
import {Layer} from "../classes/layer";
import {OcclusionLayerManager} from "../classes/occlusion-context";
import {Font, TextStyle} from "../classes/text";
import { CheckboxUpdatePatch } from "../classes/messages/checkbox-update-patch";
import { OcclusionUpdatePatch } from "../classes/messages/occlusion-update-patch";
import { ButtonUpdatePatch } from "../classes/messages/button-update-patch";
import { TextboxUpdatePatch } from "../classes/messages/textbox-update-patch";
import { DropdownUpdatePatch } from "../classes/messages/dropdown-update-patch";
import { SliderUpdatePatch } from "../classes/messages/slider-update-patch";

export const OBJECT = "Object";
export const ARRAY = "Array";
export const DIV = "DIV";
export const INPUT = "Input";
export const SELECT = "Select";
export const BUTTON = "Button";
export const CANVAS = "Canvas";
export const ANY_CREATE_PATCH = "Any Create Patch";
export const OCCLUSION_UPDATE_PATCH = "Occlusion Update Patch";
export const FRAME_UPDATE_PATCH = "Frame Update Patch";
export const IMAGE_LOAD_PATCH = "IMAGE LOAD PATCH";
export const SCROLLER_UPDATE_PATCH = "Scroller Update Patch";
export const TEXT_UPDATE_PATCH = "Text Update Patch";
export const CHECKBOX_UPDATE_PATCH = "Checkbox Update Patch";
export const TEXTBOX_UPDATE_PATCH = "Textbox Update Patch";
export const DROPDOWN_UPDATE_PATCH = "Dropdown Update Patch";
export const BUTTON_UPDATE_PATCH = "Button Update Patch";
export const SLIDER_UPDATE_PATCH = "Slider Update Patch";

export const LAYER = "LAYER";
export const OCCLUSION_CONTEXT = "Occlusion Context";

export const FONT = "Font";

export const TEXT_STYLE = "Text Style";

export const UINT32ARRAY = "UInt32Array";

export let SUPPORTED_OBJECTS = [{
    name: OBJECT,
    factory: () => ({}),
    cleanUp: (obj: any) => {
        for (let prop in obj) {
            if (obj.hasOwnProperty(prop)) {
                delete obj[prop];
            }
        }
    }
    },
    {
        name: INPUT,
        factory: () => document.createElement("input"),
        cleanUp: (input: HTMLInputElement) => {
            input.removeAttribute("style");
            input.innerHTML= "";
        }
    },
    {
        name: SELECT,
        factory: () => document.createElement("select"),
        cleanUp: (input: HTMLSelectElement) => {
            input.removeAttribute("style");
            input.innerHTML= "";
        }
    },
    {
        name: BUTTON,
        factory: () => document.createElement("button"),
        cleanUp: (input: HTMLButtonElement) => {
            input.removeAttribute("style");
            input.innerHTML= "";
        }
    },
    {
        name: ARRAY,
        factory: () => ([]),
        cleanUp: (arr: any[]) => {
            arr.length = 0;
        },
    },
    {
        name: DIV,
        factory: () => document.createElement('div'),
        cleanUp: (div: HTMLDivElement) => {
            div.removeAttribute('style');
            div.innerHTML = '';
        },
    },
    {
        name: CANVAS,
        factory: () => {
            let canvas = document.createElement('canvas');
            canvas.className = CANVAS_CLASS;
            return canvas
        },
        cleanUp: (canvas: HTMLCanvasElement) => {
            let ctx = canvas.getContext('2d');
            ctx && ctx.clearRect(0, 0, canvas.width, canvas.height);
            canvas.width = 0;
            canvas.height = 0;
            canvas.id = '';
            canvas.removeAttribute('style');
        },
    },
    {
        name: ANY_CREATE_PATCH,
        factory: () => new AnyCreatePatch(),
        cleanUp: (patch: AnyCreatePatch) => {
            patch.cleanUp()
        }
    },
    {
        name: OCCLUSION_UPDATE_PATCH,
        factory: () => new OcclusionUpdatePatch(),
        cleanUp: (patch: OcclusionUpdatePatch) => {
            patch.cleanUp()
        }
    },
    {
        name: FRAME_UPDATE_PATCH,
        factory: () => new FrameUpdatePatch(),
        cleanUp: (patch: FrameUpdatePatch) => {patch.cleanUp()},
    },
    {
        name: TEXT_UPDATE_PATCH,
        factory: (objectManager: ObjectManager) => new TextUpdatePatch(objectManager),
        cleanUp: (patch: TextUpdatePatch) => {patch.cleanUp()},
    },
    {
        name: CHECKBOX_UPDATE_PATCH,
        factory: (objectManager: ObjectManager) => new CheckboxUpdatePatch(objectManager),
        cleanUp: (patch: CheckboxUpdatePatch) => {patch.cleanUp()},
    },
    {
        name: TEXTBOX_UPDATE_PATCH,
        factory: (objectManager: ObjectManager) => new TextboxUpdatePatch(objectManager),
        cleanUp: (patch: TextboxUpdatePatch) => {patch.cleanUp()},
    },
    {
        name: DROPDOWN_UPDATE_PATCH,
        factory: (objectManager: ObjectManager) => new DropdownUpdatePatch(objectManager),
        cleanUp: (patch: DropdownUpdatePatch) => {patch.cleanUp()},
    },
    {
        name: BUTTON_UPDATE_PATCH,
        factory: (objectManager: ObjectManager) => new ButtonUpdatePatch(objectManager),
        cleanUp: (patch: ButtonUpdatePatch) => {patch.cleanUp()},
    },
    {
        name: SLIDER_UPDATE_PATCH,
        factory: (objectManager: ObjectManager) => new SliderUpdatePatch(objectManager),
        cleanUp: (patch: SliderUpdatePatch) => {patch.cleanUp()},
    },
    {
        name: IMAGE_LOAD_PATCH,
        factory: () => new ImageLoadPatch(),
        cleanUp: (patch: ImageLoadPatch) => {patch.cleanUp()},
    },
    {
        name: SCROLLER_UPDATE_PATCH,
        factory: () => new ScrollerUpdatePatch(),
        cleanUp: (patch: ScrollerUpdatePatch) => {patch.cleanUp()},
    },
    {
        name: LAYER,
        factory: (objectManager: ObjectManager) => new Layer(objectManager),
        cleanUp: (layer: Layer) => {layer.cleanUp()},
    },
    {
        name: OCCLUSION_CONTEXT,
        factory: (objectManager: ObjectManager) => new OcclusionLayerManager(objectManager),
        cleanUp: (oc: OcclusionLayerManager) => {oc.cleanUp()},
    },
    {
        name: FONT,
        factory: () => new Font(),
        cleanUp: (font: Font) => {font.cleanUp()},
    },
    {
        name: TEXT_STYLE,
        factory: (objectManager: ObjectManager) => new TextStyle(objectManager),
        cleanUp: (ts: TextStyle) => {ts.cleanUp()},
    },
    {
        name: UINT32ARRAY,
        factory: () => new Uint32Array(128),
        cleanUp: (array: Uint32Array) => {array.fill(0)},
    },
]
