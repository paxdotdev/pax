
// const rust = import('./dist/pax_chassis_web');
import {PaxChassisWeb} from './dist/pax_chassis_web';
// @ts-ignore
import snarkdown from 'snarkdown';

const MOUNT_ID = "mount";
const NATIVE_OVERLAY_CLASS = "native-overlay";
const CANVAS_CLASS = "canvas";
const SCROLLER_CONTAINER = "scroller-container"
const INNER_PANE = "inner-pane"

const NATIVE_LEAF_CLASS = "native-leaf";
const NATIVE_CLIPPING_CLASS = "native-clipping";

const CLIP_PREFIX = "clip"

const registeredFontFaces = new Set<string>();

let initializedChassis = false;


let layers: { "native": HTMLDivElement[], "canvas": HTMLCanvasElement[] } = { "native": [], "canvas": [] };

let is_mobile_device = false;

class Layer {
    readonly canvas: HTMLCanvasElement;
    readonly native: HTMLDivElement;
    readonly scrollerId : BigUint64Array | undefined;
    readonly zIndex: number;
    lastCanvas: Element | undefined;
    chassis: PaxChassisWeb

    constructor(parent: Element, zIndex: number, scroller_id: BigUint64Array | undefined, chassis: PaxChassisWeb,
                lastCanvas: Element | undefined) {
        //console.log(scroller_id, zIndex);
        this.zIndex = zIndex;
        this.scrollerId = scroller_id;
        this.chassis = chassis;
        this.lastCanvas = lastCanvas;


        this.canvas = document.createElement("canvas");
        this.canvas .className = CANVAS_CLASS
        this.canvas.id = PaxChassisWeb.generate_location_id(scroller_id, zIndex);
        this.canvas.style.zIndex = String(1000-zIndex);
        parent.appendChild(this.canvas);

        console.log("Adding Context", this.canvas.id);
        chassis.add_context(scroller_id, zIndex);

        this.native = document.createElement("div");
        this.native.className = NATIVE_OVERLAY_CLASS;
        this.native.style.zIndex = String(1000-zIndex);
        parent.appendChild(this.native);
        if(scroller_id != undefined){
            this.canvas.style.position = "sticky";
            if(zIndex > 0){
                this.canvas.style.marginTop = String(-this.canvas.style.height)+'px';
            }
            this.native.style.position = "sticky";
            this.native.style.marginTop = String(-this.canvas.style.height)+'px';
        }
    }

    public remove(){
        let parent = this.canvas.parentElement;
        this.chassis.remove_context(this.scrollerId, this.zIndex);
        parent!.removeChild(this.native);
        parent!.removeChild(this.canvas);
    }

    public updateCanvas(width: number, height: number){
        let dpr = window.devicePixelRatio;
        requestAnimationFrame(() => {
            if(this.scrollerId != undefined && this.zIndex > 0){
                this.canvas.style.marginTop = String(-height)+"px";
            }
            this.canvas.width = (width * dpr);
            this.canvas.height = (height * dpr);
            this.canvas.style.width = String(width)+'px';
            this.canvas.style.height = String(height)+'px';
            const context = this.canvas.getContext('2d');
            if (context) {
                context.scale(dpr, dpr);
            }
        });
    }

    public updateNativeOverlay(width: number, height: number){
        requestAnimationFrame(()=> {
            if(this.scrollerId != undefined){
                this.native.style.marginTop = String(-height)+"px";
            }
            this.native.style.width = String(width)+'px';
            this.native.style.height = String(height)+'px';
        });
    }
}

class OcclusionContext {
    private layers: Layer[];
    private parent: Element;
    private zIndex: number;
    private scrollerId: BigUint64Array | undefined;
    chassis: PaxChassisWeb;

    constructor(parent: Element, scrollerId : BigUint64Array | undefined, chassis: PaxChassisWeb) {
        this.layers = [];
        this.parent = parent;
        this.zIndex = -1;
        this.scrollerId = scrollerId;
        this.chassis = chassis;
        this.growTo(0);
    }

    growTo(zIndex: number) {
        if(this.zIndex < zIndex){
            for(let i = this.zIndex+1; i <= zIndex; i++) {
                let lastCanvas = undefined;
                if(i > 0){
                    lastCanvas = this.layers[i-1].canvas;
                }
                let newLayer = new Layer(this.parent, i, this.scrollerId, this.chassis,
                    lastCanvas);
                this.layers.push(newLayer);
            }
            this.zIndex = zIndex;
        }
    }

    shrinkTo(zIndex: number){
        if(this.zIndex > zIndex) {
            for(let i = this.zIndex; i > zIndex; i--){
                this.layers[i].remove();
                this.layers.pop();
            }
            this.zIndex = zIndex;
        }
    }

    addElement(element: Element, zIndex: number){
        if(zIndex > this.zIndex){
            this.growTo(zIndex);
        }
        this.layers[zIndex].native.appendChild(element);
    }

    updateCanvases(width: number, height: number){
        this.layers.forEach((layer)=>{layer.updateCanvas(width, height)});
    }

    updateNativeOverlays(width: number, height: number){
        this.layers.forEach((layer)=>{layer.updateNativeOverlay(width, height)});
    }
}


class Scroller {
    readonly idChain: BigUint64Array;
    readonly parentScrollerId: BigUint64Array | undefined;
    readonly zIndex : number;
    readonly container: HTMLDivElement;
    innerPane: HTMLDivElement;
    occlusionContext: OcclusionContext;
    sizeX?: number;
    sizeY?: number;
    sizeInnerPaneX?: number;
    sizeInnerPaneY?: number;
    transform?: number[];
    scrollX?: boolean;
    scrollY?: boolean;
    scrollOffsetX: number;
    scrollOffsetY: number;
    subtreeDepth?: number;


    constructor(idChain: BigUint64Array, zIndex: number, scrollerId: BigUint64Array | undefined, chassis: PaxChassisWeb) {
        this.idChain = idChain;
        this.parentScrollerId = scrollerId;
        this.zIndex = zIndex;
        this.scrollOffsetX = 0;
        this.scrollOffsetY = 0;

        this.container = document.createElement("div");
        this.container.className = SCROLLER_CONTAINER;
        addNativeElement(this.container, idChain, scrollerId, zIndex);

        this.container.addEventListener("scroll", () => {
            let scrollEvent = {
                "Scroll": {
                    "delta_x": this.container.scrollLeft - this.scrollOffsetX,
                    "delta_y": this.container.scrollTop - this.scrollOffsetY,
                }
            };
            this.scrollOffsetX = this.container.scrollLeft;
            this.scrollOffsetY = this.container.scrollTop;
            chassis.interrupt(JSON.stringify(scrollEvent), []);
        });

        this.innerPane = document.createElement("div");
        this.innerPane.className = INNER_PANE;
        this.container.appendChild(this.innerPane);

        this.occlusionContext = new OcclusionContext(this.container, idChain, chassis);

        if(scrollerId != null){
            // @ts-ignore
            let scrollerParent = scrollers[scrollerId];
            scrollerParent.occlusionContext.addElement(this.container, zIndex);
        } else {
            baseOcclusionContext.addElement(this.container, zIndex);
        }
    }

    handleScrollerUpdate(msg: ScrollerUpdatePatch){
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
        if(msg.sizeX != null && msg.sizeY != null){
            this.occlusionContext.updateCanvases(msg.sizeX, msg.sizeY);
            this.occlusionContext.updateNativeOverlays(msg.sizeX, msg.sizeY);
        }
        if(msg.sizeInnerPaneX != null && msg.sizeInnerPaneY != null){
            this.sizeInnerPaneX = msg.sizeInnerPaneX;
            this.sizeInnerPaneY = msg.sizeInnerPaneY;
            this.innerPane.style.width = String(this.sizeInnerPaneX)+'px';
            this.innerPane.style.height = String(this.sizeInnerPaneY)+'px';
        }
    }

    addElement(elem: Element, zIndex: number){
        this.occlusionContext.addElement(elem, zIndex);
    }
}


// global map of scrollers
// scroller id_chain -> Scroller object
let scrollers = {};
let elemToScroller = {};
let baseOcclusionContext: OcclusionContext;

function addNativeElement(elem: Element, idChain: BigUint64Array , scrollerIdChain: BigUint64Array | undefined, zIndex: number){
    //console.log(elem, idChain, scrollerIdChain, zIndex);
    if(scrollerIdChain != undefined){
        // @ts-ignore
        let scroller = scrollers[scrollerIdChain];
        scroller.addElement(elem, zIndex);
        // @ts-ignore
        elemToScroller[idChain] = scroller;
    } else {
        baseOcclusionContext.addElement(elem, zIndex);
    }
}


//handle {click, mouseover, ...} on {canvas element, native elements}
//for both virtual and native events, pass:
//  - global (screen) coordinates
//  - local (canvas) coordinates
//  - element offset (where within element, top-left-0,0)
//for native events, pass also an ID for the native element (e.g. DOM node) that
//can be used by engine to resolve virtual element
//This ID mechanism will also likely knock out most of the work for DOM element pooling/recycling

async function main(wasmMod: typeof import('./dist/pax_chassis_web')) {

    is_mobile_device = /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(navigator.userAgent);


    //Initialize chassis & engine
    let chassis = await wasmMod.PaxChassisWeb.new();

    let mount = document.querySelector("#" + MOUNT_ID)!;
    baseOcclusionContext = new OcclusionContext(mount, undefined, chassis);


    //Kick off render loop
    requestAnimationFrame(renderLoop.bind(renderLoop, chassis))

}

function convertModifiers(event: MouseEvent | KeyboardEvent) {
    let modifiers = [];
    if (event.shiftKey) modifiers.push('Shift');
    if (event.ctrlKey) modifiers.push('Control');
    if (event.altKey) modifiers.push('Alt');
    if (event.metaKey) modifiers.push('Command');
    return modifiers;
}

function getMouseButton(event: MouseEvent) {
    switch (event.button) {
        case 0: return 'Left';
        case 1: return 'Middle';
        case 2: return 'Right';
        default: return 'Unknown';
    }
}



function setupEventListeners(chassis: any, layer: any) {
    // Need to make the layer focusable it can receive keyboard events
    layer.setAttribute('tabindex', '1000');
    layer.focus();

    let lastPositions = new Map<number, {x: number, y: number}>();
    // @ts-ignore
    function getTouchMessages(touchList: TouchList) {
        return Array.from(touchList).map(touch => {
            let lastPosition = lastPositions.get(touch.identifier) || { x: touch.clientX, y: touch.clientY };
            let delta_x = touch.clientX - lastPosition.x;
            let delta_y = touch.clientY - lastPosition.y;
            lastPositions.set(touch.identifier, { x: touch.clientX, y: touch.clientY });
            return {
                x: touch.clientX,
                y: touch.clientY,
                identifier: touch.identifier,
                delta_x: delta_x,
                delta_y: delta_y
            };
        });
    }

    // @ts-ignore
    layer.addEventListener('click', (evt) => {
        let clickEvent = {
            "Click": {
                "x": evt.clientX,
                "y": evt.clientY,
                "button": getMouseButton(evt),
                "modifiers": convertModifiers(evt)
            }
        };
        chassis.interrupt(JSON.stringify(clickEvent), []);
        let jabEvent = {
            "Jab": {
                "x": evt.clientX,
                "y": evt.clientY,
            }
        };
        chassis.interrupt(JSON.stringify(jabEvent), []);
    }, true);
    // @ts-ignore
    layer.addEventListener('dblclick', (evt) => {
        let event = {
            "DoubleClick": {
                "x": evt.clientX,
                "y": evt.clientY,
                "button": getMouseButton(evt),
                "modifiers": convertModifiers(evt)
            }
        };
        chassis.interrupt(JSON.stringify(event), []);
    }, true);
    // @ts-ignore
    layer.addEventListener('mousemove', (evt) => {
        let event = {
            "MouseMove": {
                "x": evt.clientX,
                "y": evt.clientY,
                "button": getMouseButton(evt),
                "modifiers": convertModifiers(evt)
            }
        };
        chassis.interrupt(JSON.stringify(event), []);
    }, true);
    // @ts-ignore
    layer.addEventListener('wheel', (evt) => {
        let event = {
            "Wheel": {
                "x": evt.clientX,
                "y": evt.clientY,
                "delta_x": evt.deltaX,
                "delta_y": evt.deltaY,
                "modifiers": convertModifiers(evt)
            }
        };
        chassis.interrupt(JSON.stringify(event), []);
    }, true);
    // @ts-ignore
    layer.addEventListener('mousedown', (evt) => {
        let event = {
            "MouseDown": {
                "x": evt.clientX,
                "y": evt.clientY,
                "button": getMouseButton(evt),
                "modifiers": convertModifiers(evt)
            }
        };
        chassis.interrupt(JSON.stringify(event), []);
    }, true);
    // @ts-ignore
    layer.addEventListener('mouseup', (evt) => {
        let event = {
            "MouseUp": {
                "x": evt.clientX,
                "y": evt.clientY,
                "button": getMouseButton(evt),
                "modifiers": convertModifiers(evt)
            }
        };
        chassis.interrupt(JSON.stringify(event), []);
    }, true);
    // @ts-ignore
    layer.addEventListener('mouseover', (evt) => {
        let event = {
            "MouseOver": {
                "x": evt.clientX,
                "y": evt.clientY,
                "button": getMouseButton(evt),
                "modifiers": convertModifiers(evt)
            }
        };
        chassis.interrupt(JSON.stringify(event), []);
    }, true);
    // @ts-ignore
    layer.addEventListener('mouseout', (evt) => {
        let event = {
            "MouseOut": {
                "x": evt.clientX,
                "y": evt.clientY,
                "button": getMouseButton(evt),
                "modifiers": convertModifiers(evt)
            }
        };
        chassis.interrupt(JSON.stringify(event), []);
    }, true);
    // @ts-ignore
    layer.addEventListener('contextmenu', (evt) => {
        let event = {
            "ContextMenu": {
                "x": evt.clientX,
                "y": evt.clientY,
                "button": getMouseButton(evt),
                "modifiers": convertModifiers(evt)
            }
        };
        chassis.interrupt(JSON.stringify(event), []);
    }, true);
    // @ts-ignore
    layer.addEventListener('touchstart', (evt) => {
        let event = {
            "TouchStart": {
                "touches": getTouchMessages(evt.touches)
            }
        };
        Array.from(evt.changedTouches).forEach(touch => { // @ts-ignore
            lastPositions.set(touch.identifier, { x: touch.clientX, y: touch.clientY });
        });
        chassis.interrupt(JSON.stringify(event), []);

        let jabEvent = {
            "Jab": {
                "x": evt.touches[0].clientX,
                "y": evt.touches[0].clientY,
            }
        };
        chassis.interrupt(JSON.stringify(jabEvent), []);
    }, true);
    // @ts-ignore
    layer.addEventListener('touchmove', (evt) => {
        let touches = getTouchMessages(evt.touches);
        let event = {
            "TouchMove": {
                "touches": touches
            }
        };
        chassis.interrupt(JSON.stringify(event), []);

    }, true);
    // @ts-ignore
    layer.addEventListener('touchend', (evt) => {
        let event = {
            "TouchEnd": {
                "touches": getTouchMessages(evt.changedTouches)
            }
        };
        chassis.interrupt(JSON.stringify(event), []);
        Array.from(evt.changedTouches).forEach(touch => { // @ts-ignore
            lastPositions.delete(touch.identifier);
        });
    }, true);
    // @ts-ignore
    layer.addEventListener('keydown', (evt) => {
        let event = {
            "KeyDown": {
                "key": evt.key,
                "modifiers": convertModifiers(evt),
                "is_repeat": evt.repeat
            }
        };
        chassis.interrupt(JSON.stringify(event), []);
    }, true);
    // @ts-ignore
    layer.addEventListener('keyup', (evt) => {
        let event = {
            "KeyUp": {
                "key": evt.key,
                "modifiers": convertModifiers(evt),
                "is_repeat": evt.repeat
            }
        };
        chassis.interrupt(JSON.stringify(event), []);
    }, true);
    // @ts-ignore
    layer.addEventListener('keypress', (evt) => {
        let event = {
            "KeyPress": {
                "key": evt.key,
                "modifiers": convertModifiers(evt),
                "is_repeat": evt.repeat
            }
        };
        chassis.interrupt(JSON.stringify(event), []);
    }, true);
}


function addEventListenersToNativeLayers(starting_index: number, count: number, chassis: PaxChassisWeb){
    for (let i = starting_index; i < starting_index+count; i++) {
        setupEventListeners(chassis,layers.native[i]);
    }
}

function initializeLayers(num: number){
    let mount = document.querySelector("#" + MOUNT_ID); // FUTURE: make more general; see approach used by Vue & React
    // Set the position of the container to relative
    // @ts-ignore
    mount.style.position = 'relative';

    let starting_index = layers.canvas.length;

    for (let i = 0; i < num; i++) {
        let index = starting_index + i;
        //Create canvas element for piet drawing.
        //Note that width and height are set by the chassis each frame.
        let canvas = document.createElement("canvas");
        canvas.className = CANVAS_CLASS
        canvas.id = CANVAS_CLASS + "_" + index.toString();
        layers.canvas.push(canvas)

        if(index != 0) {
            // Ignore pointer events on the canvas
            canvas.style.pointerEvents = 'none';
        }

        //Create layer for native (DOM) rendering
        let nativeLayer = document.createElement("div");
        nativeLayer.className = NATIVE_OVERLAY_CLASS;
        nativeLayer.id = NATIVE_OVERLAY_CLASS +"_"+ index.toString();
        nativeLayer.style.pointerEvents = 'none';
        layers.native.push(nativeLayer)

        //Attach layers to mount
        //FIRST-APPLIED IS LOWEST
        mount?.appendChild(canvas)
        mount?.appendChild(nativeLayer);
    }
}

function getStringIdFromClippingId(prefix: string, id_chain: BigUint64Array) {
    return prefix + "_" + id_chain.join("_");
}

function escapeHtml(content: string){
    return new Option(content).innerHTML;
}

function clearCanvases(): void {
    const canvases: HTMLCollectionOf<HTMLCanvasElement> = document.getElementsByTagName('canvas');

    for (let i = 0; i < canvases.length; i++) {
        const canvas = canvases[i];
        const context = canvas.getContext('2d');
        if (context) {
            context.clearRect(0, 0, canvas.width, canvas.height);
        }
    }
}


function renderLoop (chassis: PaxChassisWeb) {
     clearCanvases()
     let messages : string = chassis.tick();
     messages = JSON.parse(messages);
    console.log(messages);
     if(!initializedChassis){
         //Handle events on mount
         let mount = document.querySelector("#" + MOUNT_ID)!;
         window.addEventListener('resize', () => {
             let width = window.innerWidth;
             let height = window.innerHeight;
             chassis.sendViewportUpdate(width, height);
             baseOcclusionContext.updateCanvases(width, height);
         });
         setupEventListeners(chassis, mount);
         initializedChassis = true;
     }
     // @ts-ignore
     processMessages(messages, chassis);
     messages;
     requestAnimationFrame(renderLoop.bind(renderLoop, chassis))
}

enum TextAlignHorizontal {
    Left = "Left",
    Center = "Center",
    Right = "Right",
}

function getJustifyContent(horizontalAlignment: string): string {
    switch (horizontalAlignment) {
        case TextAlignHorizontal.Left:
            return 'flex-start';
        case TextAlignHorizontal.Center:
            return 'center';
        case TextAlignHorizontal.Right:
            return 'flex-end';
        default:
            return 'flex-start';
    }
}

function getTextAlign(paragraphAlignment: string): string {
    switch (paragraphAlignment) {
        case TextAlignHorizontal.Left:
            return 'left';
        case TextAlignHorizontal.Center:
            return 'center';
        case TextAlignHorizontal.Right:
            return 'right';
        default:
            return 'left';
    }
}

enum TextAlignVertical {
    Top = "Top",
    Center = "Center",
    Bottom = "Bottom",
}

function getAlignItems(verticalAlignment: string): string {
    switch (verticalAlignment) {
        case TextAlignVertical.Top:
            return 'flex-start';
        case TextAlignVertical.Center:
            return 'center';
        case TextAlignVertical.Bottom:
            return 'flex-end';
        default:
            return 'flex-start';
    }
}




class NativeElementPool {
    private textNodes : any = {};
    private clippingNodes : any = {};
    private clippingValueCache : any = {};

    textCreate(patch: AnyCreatePatch) {
        console.assert(patch.idChain != null);
        console.assert(patch.clippingIds != null);
        console.assert(patch.scrollerIds != null);
        console.assert(patch.zIndex != null);


        let runningChain = document.createElement("div")
        let textChild = document.createElement('div');
        runningChain.appendChild(textChild);
        runningChain.setAttribute("class", NATIVE_LEAF_CLASS)

        let scrollerId = undefined;
        if(patch.scrollerIds.length > 0){
            scrollerId = patch.scrollerIds[patch.scrollerIds.length-1];
        }
        addNativeElement(runningChain, patch.idChain, scrollerId, patch.zIndex);

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




    textDelete(id_chain: BigUint64Array) {
        // @ts-ignore
        let oldNode = this.textNodes[id_chain];
        if (oldNode){
            let parent = oldNode.parentElement;
            requestAnimationFrame(()=> {
                parent.removeChild(oldNode);
            });
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

    frameDelete(id_chain: BigUint64Array) {
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
        let scroller = new Scroller(patch.idChain, patch.zIndex, scroller_id, chassis);
        // @ts-ignore
        scrollers[patch.idChain] = scroller;
    }

    scrollerUpdate(patch: ScrollerUpdatePatch){
        //console.log(patch);
        // @ts-ignore
        scrollers[patch.idChain].handleScrollerUpdate(patch);
    }

    scrollerDelete(idChain: BigUint64Array){
        //unimplemented
    }

    async imageLoad(patch: ImageLoadPatch, chassis: PaxChassisWeb) {

        let path = patch.path;
        let image_data = await readImageToByteBuffer(path);
        let message = {
            "Image": {
                "Data": {
                    "id_chain": Array.from(patch.id_chain, value => Number(value)),
                    "width": image_data.width,
                    "height": image_data.height,
                }
            }
        }
        chassis.interrupt(JSON.stringify(message), image_data.pixels);
    }

}


async function readImageToByteBuffer(imagePath: string): Promise<{ pixels: Uint8ClampedArray, width: number, height: number }> {
    const response = await fetch(imagePath);
    const blob = await response.blob();
    const img = await createImageBitmap(blob);
    const canvas = new OffscreenCanvas(img.width+1000, img.height);
    const ctx = canvas.getContext('2d');
    // @ts-ignore
    ctx.drawImage(img, 0, 0, img.width, img.height);
    // @ts-ignore
    const imageData = ctx.getImageData(0, 0, img.width, img.height);
    let pixels = imageData.data;
    return { pixels, width: img.width, height: img.height };
}

class ImageLoadPatch {
    public id_chain: BigUint64Array;
    public path: string;

    constructor(jsonMessage: any) {
        this.id_chain = new BigUint64Array(
            jsonMessage["id_chain"].map((id: number | string) => BigInt(id))
        );
        this.path = jsonMessage["path"];
    }
}

class TextStyle {
    public font?: Font;
    public fill?: ColorGroup;
    public font_size?: number;
    public underline?: boolean;
    public align_multiline?: TextAlignHorizontal;
    public align_horizontal?: TextAlignHorizontal;
    public align_vertical?: TextAlignVertical;

    constructor(styleMessage: any) {
        if (styleMessage["font"]) {
            const font = new Font();
            font.fromFontPatch(styleMessage["font"]);
            this.font = font;
        }
        this.fill = styleMessage["fill"];
        this.font_size = styleMessage["font_size"];
        this.underline = styleMessage["underline"];
        this.align_multiline = styleMessage["align_multiline"];
        this.align_horizontal = styleMessage["align_horizontal"];
        this.align_vertical = styleMessage["align_vertical"];
    }
}

//Type-safe wrappers around JSON representation
class TextUpdatePatch {
    public id_chain: BigUint64Array;
    public content?: string;
    public size_x?: number;
    public size_y?: number;
    public transform?: number[];
    public style?: TextStyle;
    public style_link?: TextStyle;
    public depth?: number;

    constructor(jsonMessage: any) {
        this.id_chain = new BigUint64Array(
            jsonMessage["id_chain"].map((id: number | string) => BigInt(id))
        );
        this.content = jsonMessage["content"];
        this.size_x = jsonMessage["size_x"];
        this.size_y = jsonMessage["size_y"];
        this.transform = jsonMessage["transform"];
        this.depth = jsonMessage["depth"];

        const styleMessage = jsonMessage["style"];
        if (styleMessage) {
            this.style = new TextStyle(styleMessage);
        }

        const styleLinkMessage = jsonMessage["style_link"];
        if (styleLinkMessage) {
            this.style_link = new TextStyle(styleLinkMessage);
        }
    }
}

class ScrollerUpdatePatch {
    public idChain: BigUint64Array;
    public sizeX?: number;
    public sizeY?: number;
    public sizeInnerPaneX? : number;
    public sizeInnerPaneY? : number;
    public transform? : number[];
    public scrollX? : boolean;
    public scrollY? : boolean;
    public subtreeDepth?: number;

    constructor(jsonMessage: any) {
        this.idChain = new BigUint64Array(
            jsonMessage["id_chain"].map((id: number | string) => BigInt(id))
        );
        this.sizeX = jsonMessage["size_x"];
        this.sizeY = jsonMessage["size_y"];
        this.sizeInnerPaneX = jsonMessage["size_inner_pane_x"];
        this.sizeInnerPaneY = jsonMessage["size_inner_pane_y"];
        this.transform = jsonMessage["transform"];
        this.scrollX = jsonMessage["scroll_x"];
        this.scrollY = jsonMessage["scroll_y"];
        this.subtreeDepth = jsonMessage["subtree_depth"];
    }
}



class ColorGroup {
    Hsla?: number[];
    Rgba?: number[];
}


class Font {
    public type?: string;
    public family?: string;
    public style?: FontStyle;
    public weight?: FontWeight;
    public url?: string; // for WebFontMessage
    public path?: string; // for LocalFontMessage

    mapFontWeight(fontWeight : FontWeight) {
        switch (fontWeight) {
            case FontWeight.Thin:
                return 100;
            case FontWeight.ExtraLight:
                return 200;
            case FontWeight.Light:
                return 300;
            case FontWeight.Normal:
                return 400;
            case FontWeight.Medium:
                return 500;
            case FontWeight.SemiBold:
                return 600;
            case FontWeight.Bold:
                return 700;
            case FontWeight.ExtraBold:
                return 800;
            case FontWeight.Black:
                return 900;
            default:
                return 400; // Return a default value if fontWeight is not found
        }
    }

    mapFontStyle(fontStyle: FontStyle) {
        switch (fontStyle) {
            case FontStyle.Normal:
                return 'normal';
            case FontStyle.Italic:
                return 'italic';
            case FontStyle.Oblique:
                return 'oblique';
            default:
                return 'normal'; // Return a default value if fontStyle is not found
        }
    }
    fromFontPatch(fontPatch: any) {
        const type = Object.keys(fontPatch)[0];
        const data = fontPatch[type];
        this.type = type;
        if (type === "System") {
            this.family = data.family;
            this.style = FontStyle[data.style as keyof typeof FontStyle];
            this.weight = FontWeight[data.weight as keyof typeof FontWeight];
        } else if (type === "Web") {
            this.family = data.family;
            this.url = data.url;
            this.style = FontStyle[data.style as keyof typeof FontStyle];
            this.weight = FontWeight[data.weight as keyof typeof FontWeight];
        } else if (type === "Local") {
            this.family = data.family;
            this.path = data.path;
            this.style = FontStyle[data.style as keyof typeof FontStyle];
            this.weight = FontWeight[data.weight as keyof typeof FontWeight];
        }
        this.registerFontFace();
    }


    private fontKey(): string {
        return `${this.type}-${this.family}-${this.style}-${this.weight}`;
    }

    registerFontFace() {
        const fontKey = this.fontKey();
        if (!registeredFontFaces.has(fontKey)) {
            registeredFontFaces.add(fontKey);

            if (this.type === "Web" && this.url && this.family) {
                if (this.url.includes("fonts.googleapis.com/css")) {
                    // Fetch the Google Fonts CSS file and create a <style> element to insert its content
                    fetch(this.url)
                        .then(response => response.text())
                        .then(css => {
                            const style = document.createElement("style");
                            style.textContent = css;
                            document.head.appendChild(style);
                        });
                } else {
                    const fontFace = new FontFace(this.family, `url(${this.url})`, {
                        style: this.style ? FontStyle[this.style] : undefined,
                        weight: this.weight ? FontWeight[this.weight] : undefined,
                    });

                    fontFace.load().then(loadedFontFace => {
                        document.fonts.add(loadedFontFace);
                    });
                }
            } else if (this.type === "Local" && this.path && this.family) {
                const fontFace = new FontFace(this.family, `url(${this.path})`, {
                    style: this.style ? FontStyle[this.style] : undefined,
                    weight: this.weight ? FontWeight[this.weight] : undefined,
                });

                fontFace.load().then(loadedFontFace => {
                    document.fonts.add(loadedFontFace);
                });
            }
        }
    }


    applyFontToDiv(div: HTMLDivElement) {
        if (this.family != undefined) {
            div.style.fontFamily = this.family;
        }
        if (this.style != undefined) {
            div.style.fontStyle = this.mapFontStyle(this.style);
        }
        if (this.weight != undefined) {
            div.style.fontWeight = String(this.mapFontWeight(this.weight));
        }
    }
}

enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    Normal,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}



class FrameUpdatePatch {
    public id_chain?: BigUint64Array;
    public size_x?: number;
    public size_y?: number;
    public transform?: number[];
    constructor(jsonMessage: any) {
        if(jsonMessage != null) {
            this.id_chain = new BigUint64Array(
                jsonMessage["id_chain"].map((id: number | string) => BigInt(id))
            );
            this.size_x = jsonMessage["size_x"];
            this.size_y = jsonMessage["size_y"];
            this.transform = jsonMessage["transform"];
        }
    }
}

class AnyCreatePatch {
    public idChain: BigUint64Array;
    public clippingIds: BigUint64Array[];
    public scrollerIds: BigUint64Array[];
    public zIndex: number;

    constructor(jsonMessage: any) {
        // Convert idChain
        this.idChain = new BigUint64Array(
            jsonMessage["id_chain"].map((id: number | string) => BigInt(id))
        );


        // Convert clippingIds - Array of BigUint64Array
        this.clippingIds = jsonMessage["clipping_ids"].map((idArray: (number | string)[]) =>
            new BigUint64Array(idArray.map(id => BigInt(id)))
        );

        // Convert scrollerIds - Array of BigUint64Array
        this.scrollerIds = jsonMessage["scroller_ids"].map((idArray: (number | string)[]) =>
            new BigUint64Array(idArray.map(id => BigInt(id)))
        );

        this.zIndex = jsonMessage["z_index"];
    }
}

// function getQuadClipPathCommand(width: number, height: number, transform: number[]) {
//     let point0 = affineMultiply([0, 0], transform);
//     let point1 = affineMultiply([width, 0], transform);
//     let point2 = affineMultiply([width, height], transform);
//     let point3 = affineMultiply([0, height], transform);

//     let command = `path('M ${point0[0]} ${point0[1]} L ${point1[0]} ${point1[1]} L ${point2[0]} ${point2[1]} L ${point3[0]} ${point3[1]} Z')`
//     return command;
// }

//Rectilinear-affine alternative to `clip-path: path(...)` clipping.  Might be faster than `path`
function getQuadClipPolygonCommand(width: number, height: number, transform: number[]) {
    let point0 = affineMultiply([0, 0], transform);
    let point1 = affineMultiply([width, 0], transform);
    let point2 = affineMultiply([width, height], transform);
    let point3 = affineMultiply([0, height], transform);

    let polygon = `polygon(${point0[0]}px ${point0[1]}px, ${point1[0]}px ${point1[1]}px, ${point2[0]}px ${point2[1]}px, ${point3[0]}px ${point3[1]}px)`
    return polygon;
}

let nativePool = new NativeElementPool();

function processMessages(messages: any[], chassis: PaxChassisWeb) {

    messages?.forEach((unwrapped_msg) => {
        // if (Math.random() < .1) console.log(unwrapped_msg)
       // console.log(unwrapped_msg);
        if(unwrapped_msg["TextCreate"]) {
            let msg = unwrapped_msg["TextCreate"]
            nativePool.textCreate(new AnyCreatePatch(msg));
        }else if (unwrapped_msg["TextUpdate"]){
            let msg = unwrapped_msg["TextUpdate"]
            nativePool.textUpdate(new TextUpdatePatch(msg));
        }else if (unwrapped_msg["TextDelete"]) {
            let msg = unwrapped_msg["TextDelete"];
            nativePool.textDelete(msg)
        } else if(unwrapped_msg["FrameCreate"]) {
            let msg = unwrapped_msg["FrameCreate"]
            nativePool.frameCreate(new AnyCreatePatch(msg));
        }else if (unwrapped_msg["FrameUpdate"]){
            let msg = unwrapped_msg["FrameUpdate"]
            nativePool.frameUpdate(new FrameUpdatePatch(msg));
        }else if (unwrapped_msg["FrameDelete"]) {
            let msg = unwrapped_msg["FrameDelete"];
            nativePool.frameDelete(msg["id_chain"])
        }else if (unwrapped_msg["ImageLoad"]){
            let msg = unwrapped_msg["ImageLoad"];
            nativePool.imageLoad(new ImageLoadPatch(msg), chassis)
        }else if (unwrapped_msg["LayerAdd"]){
            let msg = unwrapped_msg["LayerAdd"];
            let layersToAdd = msg["num_layers_to_add"];
            let old_length = layers.native.length;
            if (!is_mobile_device || (old_length+layersToAdd) < 5) {
                initializeLayers(layersToAdd);
                addEventListenersToNativeLayers(old_length, layersToAdd, chassis);
            } else {
                layersToAdd = 0;
            }
            let event = {
                "AddedLayer": {
                    "num_layers_added": layersToAdd,
                }
            }
            chassis.interrupt(JSON.stringify(event), []);
        }else if(unwrapped_msg["ScrollerCreate"]) {
            let msg = unwrapped_msg["ScrollerCreate"]
            nativePool.scrollerCreate(new AnyCreatePatch(msg), chassis);
        }else if (unwrapped_msg["ScrollerUpdate"]){
            let msg = unwrapped_msg["ScrollerUpdate"]
            nativePool.scrollerUpdate(new ScrollerUpdatePatch(msg));
        }else if (unwrapped_msg["ScrollerDelete"]) {
            let msg = unwrapped_msg["ScrollerDelete"];
            nativePool.scrollerDelete(msg["id_chain"])
        }
    })
}

/// Our 2D affine transform comes across the wire as an array of
/// floats in column-major order, (a,b,c,d,e,f) representing the
/// augmented matrix:
///  | a c e |
///  | b d f |
///  | 0 0 1 |
///
///  In order to pack this into a CSS-ready matrix3d format, we must
///  imagine packing into the following matrix for a "dont-care Z"
///
///  | a c 0 e |
///  | b d 0 f |
///  | 0 0 1 0 | //note that 1 will preserve a dont-care z, vs 0 will 'flatten' it
///  | 0 0 0 1 |
///
///  and then unroll into a comma-separated list, following column-major order
///
function packAffineCoeffsIntoMatrix3DString(coeffs: number[]) : string {
    return "matrix3d(" + [
        //begin column 0
        coeffs[0].toFixed(6),
        coeffs[1].toFixed(6),
        0,
        0,
        //begin column 1
        coeffs[2].toFixed(6),
        coeffs[3].toFixed(6),
        0,
        0,
        //begin column 2
        0,
        0,
        1,
        0,
        //begin column 3
        coeffs[4].toFixed(6),
        coeffs[5].toFixed(6),
        0,
        1
    ].join(",") + ")";
}

function packAffineCoeffsIntoMatrix2DString(coeffs: number[]) : string {
    return "matrix(" + [
        //begin column 0
        coeffs[0].toFixed(6),
        coeffs[1].toFixed(6),
        //begin column 1
        coeffs[2].toFixed(6),
        coeffs[3].toFixed(6),
        //begin column 2
        coeffs[4].toFixed(6),
        coeffs[5].toFixed(6),
    ].join(",") + ")";
}

function getAttachPointFromClippingIds(clipping_ids: BigUint64Array[]) {
    // If there's a clipping context, attach to it.  Otherwise, attach directly to the native element layer.
    let attachPoint = (() => {
        if (clipping_ids.length > 0) {
            let clippingLeaf = document.querySelector("#" + getStringIdFromClippingId(CLIP_PREFIX, clipping_ids[clipping_ids.length - 1]));
            console.assert(clippingLeaf != null);
            return clippingLeaf
        }else {
            return document.querySelector("#" + NATIVE_OVERLAY_CLASS+"_0")
        }
    })();
    return attachPoint;
}

//Required due to Safari bug, unable to clip DOM elements to SVG=>`transform: matrix(...)` elements; see https://bugs.webkit.org/show_bug.cgi?id=126207
//  and repro in this repo: `878576bf0e9`
//Work-around is to manually affine-multiply coordinates of relevant elements and plot as `Path`s (without `transform`) in SVG.
//
//For the point V [x,y]
//And the affine coefficients in column-major order, (a,b,c,d,e,f) representing the matrix M:
//  | a c e |
//  | b d f |
//  | 0 0 1 |
//Return the product `V * M`
// Given a matrix A∈ℝm×n and vector x∈ℝn the matrix-vector multiplication of A and x is defined as
// Ax:=x1a∗,1+x2a∗,2+⋯+xna∗,n
// where a∗,i is the ith column vector of A.
function affineMultiply(point: number[], matrix: number[]) : number[] {
    let x = point[0];
    let y = point[1];
    let a = matrix[0];
    let b = matrix[1];
    let c = matrix[2];
    let d = matrix[3];
    let e = matrix[4];
    let f = matrix[5];
    let xOut = a*x + c*y + e;
    let yOut = b*x + d*y + f;
    return [xOut, yOut];
}


// Wasm + TS Bootstrapping boilerplate
async function load() {
    main(await import('./dist/pax_chassis_web'));
}
load().then();