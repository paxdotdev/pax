import type {PaxChassisWeb} from "../types/pax-chassis-web";

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


export function setupEventListeners(chassis: PaxChassisWeb) {

    let lastPositions = new Map<number, {x: number, y: number}>();
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

    window.addEventListener('click', (evt) => {

        let clickEvent = {
            "Click": {
                "x": evt.clientX,
                "y": evt.clientY,
                "button": getMouseButton(evt),
                "modifiers": convertModifiers(evt)
            }
        };
        let r1 = chassis.interrupt(JSON.stringify(clickEvent), []);
        let clapEvent = {
            "Clap": {
                "x": evt.clientX,
                "y": evt.clientY,
            }
        };
        let r2 = chassis.interrupt(JSON.stringify(clapEvent), []);
        if (r1.prevent_default || r2.prevent_default) {
            evt.preventDefault();
        }
    }, true);
    window.addEventListener('dblclick', (evt) => {
        let event = {
            "DoubleClick": {
                "x": evt.clientX,
                "y": evt.clientY,
                "button": getMouseButton(evt),
                "modifiers": convertModifiers(evt)
            }
        };
        let res = chassis.interrupt(JSON.stringify(event), []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, true);
    window.addEventListener('selectstart', (evt) => {

        // NOTE: this shouldn't be needed once selectionstart can be
        // fired only on active/focused element instead of global
        // Check if the target is an input or textarea
        if (evt.target instanceof HTMLInputElement || evt.target instanceof HTMLTextAreaElement) {
            // Allow default behavior for inputs and textareas
            return;
        }        
        let event = {
            "SelectStart": {}
        };
        let res = chassis.interrupt(JSON.stringify(event), []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, true);
    window.addEventListener('mousemove', (evt) => {
        // this value was previously set on window
        let button = (window as any).current_button || 'Left';
        let event = {
            "MouseMove": {
                "x": evt.clientX,
                "y": evt.clientY,
                "button": button,
                "modifiers": convertModifiers(evt)
            }
        };
        let res = chassis.interrupt(JSON.stringify(event), []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, true);
    window.addEventListener('wheel', (evt) => {
        let event = {
            "Wheel": {
                "x": evt.clientX,
                "y": evt.clientY,
                "delta_x": evt.deltaX,
                "delta_y": evt.deltaY,
                "modifiers": convertModifiers(evt)
            }
        };
        let res = chassis.interrupt(JSON.stringify(event), []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, {"passive": false, "capture": true});
    window.addEventListener('mousedown', (evt) => {
        let button = getMouseButton(evt);
        // set non-existent window prop to keep track of value
        (window as any).current_button = button;
        let event = {
            "MouseDown": {
                "x": evt.clientX,
                "y": evt.clientY,
                "button": getMouseButton(evt),
                "modifiers": convertModifiers(evt)
            }
        };
        let res = chassis.interrupt(JSON.stringify(event), []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, true);
    window.addEventListener('mouseup', (evt) => {
        let event = {
            "MouseUp": {
                "x": evt.clientX,
                "y": evt.clientY,
                "button": getMouseButton(evt),
                "modifiers": convertModifiers(evt)
            }
        };
        let res = chassis.interrupt(JSON.stringify(event), []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, true);
    window.addEventListener('contextmenu', (evt) => {
        let event = {
            "ContextMenu": {
                "x": evt.clientX,
                "y": evt.clientY,
                "button": getMouseButton(evt),
                "modifiers": convertModifiers(evt)
            }
        };
        let res = chassis.interrupt(JSON.stringify(event), []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, true);
    window.addEventListener('touchstart', (evt) => {
        let event = {
            "TouchStart": {
                "touches": getTouchMessages(evt.touches)
            }
        };
        Array.from(evt.changedTouches).forEach(touch => {
            lastPositions.set(touch.identifier, { x: touch.clientX, y: touch.clientY });
        });
        let r1 = chassis.interrupt(JSON.stringify(event), []);

        let clapEvent = {
            "Clap": {
                "x": evt.touches[0].clientX,
                "y": evt.touches[0].clientY,
            }
        };
        let r2 = chassis.interrupt(JSON.stringify(clapEvent), []);
        if (r1.prevent_default || r2.prevent_default) {
            evt.preventDefault();
        }
    }, {"passive": true, "capture": true});
    window.addEventListener('touchmove', (evt) => {
        let touches = getTouchMessages(evt.touches);
        let event = {
            "TouchMove": {
                "touches": touches
            }
        };
        let res = chassis.interrupt(JSON.stringify(event), []);
        if (res.prevent_default) {
            evt.preventDefault();
        }

    }, {"passive": true, "capture": true});
    window.addEventListener('touchend', (evt) => {
        let event = {
            "TouchEnd": {
                "touches": getTouchMessages(evt.changedTouches)
            }
        };
        let res = chassis.interrupt(JSON.stringify(event), []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
        Array.from(evt.changedTouches).forEach(touch => {
            lastPositions.delete(touch.identifier);
        });
    }, {"passive": true, "capture": true});
    window.addEventListener('keydown', (evt) => {
        let dom_node_selected = document.activeElement != document.body;
        // TODO figure out how to handle this more robustly
        if (dom_node_selected) {
            return;
        }
        
        let event = {
            "KeyDown": {
                "key": evt.key,
                "modifiers": convertModifiers(evt),
                "is_repeat": evt.repeat
            }
        };
        let res = chassis.interrupt(JSON.stringify(event), []);
        if (res.prevent_default && !dom_node_selected) {
            evt.preventDefault();
        }
    }, true);
    window.addEventListener('keyup', (evt) => {
        if (document.activeElement != document.body) {
            return;
        }
        let event = {
            "KeyUp": {
                "key": evt.key,
                "modifiers": convertModifiers(evt),
                "is_repeat": evt.repeat
            }
        };
        let res = chassis.interrupt(JSON.stringify(event), []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, true);
    window.addEventListener('keypress', (evt) => {
        if (document.activeElement != document.body) {
            return;
        }
        let event = {
            "KeyPress": {
                "key": evt.key,
                "modifiers": convertModifiers(evt),
                "is_repeat": evt.repeat
            }
        };
        let res = chassis.interrupt(JSON.stringify(event), []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, true);
    window.addEventListener('focus', (evt) => {
        if (document.activeElement != document.body) {
            return;
        }
        let event = {
            "Focus": {}
        };
        let res = chassis.interrupt(JSON.stringify(event), []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, true);
    window.addEventListener('drop', async (evt) => {
        evt.stopPropagation();
        evt.preventDefault();
        if (document.activeElement != document.body) {
            return;
        }
        let file = evt.dataTransfer?.files[0]!;
        let bytes = await readFileAsByteArray(file);
        let event = {
            "DropFile": {
                "x": evt.clientX,
                "y": evt.clientY,
                "name": file.name,
                "mime_type": file.type,
                "size": file.size,
            }
        };
        let res = chassis.interrupt(JSON.stringify(event), bytes);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, true);
    window.addEventListener('dragover', (evt) => {
        evt.stopPropagation();
        evt.preventDefault();
        evt.dataTransfer!.dropEffect = 'copy';
    }, {"passive": false, "capture": true});
}

function readFileAsByteArray(file: File): Promise<Uint8Array> {
    return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = (event: ProgressEvent<FileReader>) => {
            if (event.target && event.target.result instanceof ArrayBuffer) {
                const arrayBuffer: ArrayBuffer = event.target.result;
                const byteArray: Uint8Array = new Uint8Array(arrayBuffer);
                resolve(byteArray); // Resolve the promise with the byte array
            } else {
                reject(new Error('File reading did not return an ArrayBuffer'));
            }
        };
        reader.onerror = () => reject(reader.error); // Reject the promise on error
        reader.readAsArrayBuffer(file); // Read the file as an ArrayBuffer
    });
}
