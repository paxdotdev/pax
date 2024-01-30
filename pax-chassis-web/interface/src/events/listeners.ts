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


export function setupEventListeners(chassis: PaxChassisWeb, layer: any) {

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
        let clapEvent = {
            "Clap": {
                "x": evt.clientX,
                "y": evt.clientY,
            }
        };
        chassis.interrupt(JSON.stringify(clapEvent), []);
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
            "Mousemove": {
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
    }, {"passive": true, "capture": true});
    // @ts-ignore
    layer.addEventListener('mousedown', (evt) => {
        let event = {
            "Mousedown": {
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
            "Mouseup": {
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

        let clapEvent = {
            "Clap": {
                "x": evt.touches[0].clientX,
                "y": evt.touches[0].clientY,
            }
        };
        chassis.interrupt(JSON.stringify(clapEvent), []);
    }, {"passive": true, "capture": true});
    // @ts-ignore
    layer.addEventListener('touchmove', (evt) => {
        let touches = getTouchMessages(evt.touches);
        let event = {
            "TouchMove": {
                "touches": touches
            }
        };
        chassis.interrupt(JSON.stringify(event), []);

    }, {"passive": true, "capture": true});
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
    }, {"passive": true, "capture": true});
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