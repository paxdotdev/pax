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
        chassis.interrupt(JSON.stringify(clickEvent), []);
        let clapEvent = {
            "Clap": {
                "x": evt.clientX,
                "y": evt.clientY,
            }
        };
        chassis.interrupt(JSON.stringify(clapEvent), []);
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
        chassis.interrupt(JSON.stringify(event), []);
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
        chassis.interrupt(JSON.stringify(event), []);
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
        chassis.interrupt(JSON.stringify(event), []);
    }, {"passive": true, "capture": true});
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
        chassis.interrupt(JSON.stringify(event), []);
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
        chassis.interrupt(JSON.stringify(event), []);
    }, true);
    window.addEventListener('mouseover', (evt) => {
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
    window.addEventListener('mouseout', (evt) => {
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
        chassis.interrupt(JSON.stringify(event), []);

        let clapEvent = {
            "Clap": {
                "x": evt.touches[0].clientX,
                "y": evt.touches[0].clientY,
            }
        };
        chassis.interrupt(JSON.stringify(clapEvent), []);
    }, {"passive": true, "capture": true});
    window.addEventListener('touchmove', (evt) => {
        let touches = getTouchMessages(evt.touches);
        let event = {
            "TouchMove": {
                "touches": touches
            }
        };
        chassis.interrupt(JSON.stringify(event), []);

    }, {"passive": true, "capture": true});
    window.addEventListener('touchend', (evt) => {
        let event = {
            "TouchEnd": {
                "touches": getTouchMessages(evt.changedTouches)
            }
        };
        chassis.interrupt(JSON.stringify(event), []);
        Array.from(evt.changedTouches).forEach(touch => {
            lastPositions.delete(touch.identifier);
        });
    }, {"passive": true, "capture": true});
    window.addEventListener('keydown', (evt) => {
        if (document.activeElement != document.body) {
            return;
        }
        let event = {
            "KeyDown": {
                "key": evt.key,
                "modifiers": convertModifiers(evt),
                "is_repeat": evt.repeat
            }
        };
        chassis.interrupt(JSON.stringify(event), []);
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
        chassis.interrupt(JSON.stringify(event), []);
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
        chassis.interrupt(JSON.stringify(event), []);
    }, true);
}
