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


export function setupEventListeners(chassis: PaxChassisWeb): () => void {
    let disposers: Array<() => void> = [];
    let addDisposer = (disposer: () => void) => {
        disposers.push(disposer);
    };
    let addWindowListener = (
        type: string,
        listener: (event: any) => void,
        options?: boolean | AddEventListenerOptions,
    ) => {
        window.addEventListener(type, listener, options);
        disposers.push(() => window.removeEventListener(type, listener, options));
    };

    let lastPositions = new Map<number, {x: number, y: number}>();
    let lastTouchTap: {x: number, y: number, timestamp: number} | undefined;
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

    function isTouchGeneratedClick(evt: MouseEvent) {
        let sourceCapabilities = (evt as any).sourceCapabilities;
        if (sourceCapabilities?.firesTouchEvents) {
            return true;
        }
        if (lastTouchTap == undefined) {
            return false;
        }
        let elapsed = performance.now() - lastTouchTap.timestamp;
        let distanceX = Math.abs(evt.clientX - lastTouchTap.x);
        let distanceY = Math.abs(evt.clientY - lastTouchTap.y);
        return elapsed < 750 && distanceX < 25 && distanceY < 25;
    }

    addWindowListener('click', (evt) => {
        if (isTouchGeneratedClick(evt)) {
            lastTouchTap = undefined;
            return;
        }

        let clickEvent = {
            "Click": {
                "x": evt.clientX,
                "y": evt.clientY,
                "button": getMouseButton(evt),
                "modifiers": convertModifiers(evt)
            }
        };
        let res = chassis.interrupt(clickEvent, []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, true);
    addWindowListener('dblclick', (evt) => {
        let event = {
            "DoubleClick": {
                "x": evt.clientX,
                "y": evt.clientY,
                "button": getMouseButton(evt),
                "modifiers": convertModifiers(evt)
            }
        };
        let res = chassis.interrupt(event, []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, true);
    addWindowListener('selectstart', (evt) => {

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
        let res = chassis.interrupt(event, []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, true);
    addWindowListener('mousemove', (evt) => {
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
        let res = chassis.interrupt(event, []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, true);
    addWindowListener('wheel', (evt) => {
        let event = {
            "Wheel": {
                "x": evt.clientX,
                "y": evt.clientY,
                "delta_x": evt.deltaX,
                "delta_y": evt.deltaY,
                "modifiers": convertModifiers(evt)
            }
        };
        let res = chassis.interrupt(event, []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, {"passive": false, "capture": true});
    addWindowListener('mousedown', (evt) => {
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
        let res = chassis.interrupt(event, []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, true);
    addWindowListener('mouseup', (evt) => {
        let event = {
            "MouseUp": {
                "x": evt.clientX,
                "y": evt.clientY,
                "button": getMouseButton(evt),
                "modifiers": convertModifiers(evt)
            }
        };
        let res = chassis.interrupt(event, []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, true);
    addWindowListener('contextmenu', (evt) => {
        let event = {
            "ContextMenu": {
                "x": evt.clientX,
                "y": evt.clientY,
                "button": getMouseButton(evt),
                "modifiers": convertModifiers(evt)
            }
        };
        let res = chassis.interrupt(event, []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, true);
    addWindowListener('touchstart', (evt) => {
        let event = {
            "TouchStart": {
                "touches": getTouchMessages(evt.touches)
            }
        };
        Array.from(evt.changedTouches).forEach(touch => {
            lastPositions.set(touch.identifier, { x: touch.clientX, y: touch.clientY });
        });
        let r1 = chassis.interrupt(event, []);

        let tapPreventDefault = false;
        if (evt.touches.length === 1) {
            let touch = evt.touches[0];
            lastTouchTap = {
                x: touch.clientX,
                y: touch.clientY,
                timestamp: performance.now(),
            };
            let tapEvent = {
                "Tap": {
                    "x": touch.clientX,
                    "y": touch.clientY,
                }
            };
            tapPreventDefault = chassis.interrupt(tapEvent, []).prevent_default;
        }
        if (r1.prevent_default || tapPreventDefault) {
            evt.preventDefault();
        }
    }, {"passive": true, "capture": true});
    addWindowListener('touchmove', (evt) => {
        let touches = getTouchMessages(evt.touches);
        let event = {
            "TouchMove": {
                "touches": touches
            }
        };
        let res = chassis.interrupt(event, []);
        if (res.prevent_default) {
            evt.preventDefault();
        }

    }, {"passive": false, "capture": true});
    addWindowListener('touchend', (evt) => {
        let event = {
            "TouchEnd": {
                "touches": getTouchMessages(evt.changedTouches)
            }
        };
        let res = chassis.interrupt(event, []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
        Array.from(evt.changedTouches).forEach(touch => {
            lastPositions.delete(touch.identifier);
        });
    }, {"passive": true, "capture": true});
    addWindowListener('keydown', (evt) => {
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
        let res = chassis.interrupt(event, []);
        if (res.prevent_default && !dom_node_selected) {
            evt.preventDefault();
        }
    }, true);
    addWindowListener('keyup', (evt) => {
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
        let res = chassis.interrupt(event, []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, true);
    addWindowListener('keypress', (evt) => {
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
        let res = chassis.interrupt(event, []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, true);
    addWindowListener('focus', (evt) => {
        if (document.activeElement != document.body) {
            return;
        }
        let event = {
            "Focus": {}
        };
        let res = chassis.interrupt(event, []);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, true);
    addWindowListener('drop', async (evt) => {
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
        let res = chassis.interrupt(event, bytes);
        if (res.prevent_default) {
            evt.preventDefault();
        }
    }, true);
    addWindowListener('dragover', (evt) => {
        evt.stopPropagation();
        evt.preventDefault();
        evt.dataTransfer!.dropEffect = 'copy';
    }, {"passive": false, "capture": true});
    setupDeviceSensorListeners(chassis, addWindowListener, addDisposer);

    return () => {
        while (disposers.length > 0) {
            let dispose = disposers.pop();
            dispose?.();
        }
    };
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

type AddWindowListener = (
    type: string,
    listener: (event: any) => void,
    options?: boolean | AddEventListenerOptions,
) => void;

function setupDeviceSensorListeners(
    chassis: PaxChassisWeb,
    addWindowListener: AddWindowListener,
    addDisposer: (disposer: () => void) => void,
) {
    let sensorsStarted = false;
    let permissionRequested = false;
    let orientation = (window as any).DeviceOrientationEvent;
    let motion = (window as any).DeviceMotionEvent;
    let needsPermission =
        typeof orientation?.requestPermission === 'function' ||
        typeof motion?.requestPermission === 'function';

    let numericOrZero = (value: number | null | undefined) => value ?? 0;
    let hasAnyAxis = (x: number | null | undefined, y: number | null | undefined, z: number | null | undefined) =>
        x != null || y != null || z != null;

    let startSensors = () => {
        if (sensorsStarted) {
            return;
        }
        sensorsStarted = true;
        addWindowListener('deviceorientation', (evt: DeviceOrientationEvent) => {
            if (!hasAnyAxis(evt.beta, evt.gamma, evt.alpha)) {
                return;
            }
            chassis.interrupt({
                "Gyro": {
                    "x": numericOrZero(evt.beta),
                    "y": numericOrZero(evt.gamma),
                    "z": numericOrZero(evt.alpha),
                }
            }, []);
        }, true);
        addWindowListener('devicemotion', (evt: DeviceMotionEvent) => {
            let acceleration = evt.accelerationIncludingGravity ?? evt.acceleration;
            if (acceleration == null || !hasAnyAxis(acceleration.x, acceleration.y, acceleration.z)) {
                return;
            }
            chassis.interrupt({
                "Accel": {
                    "x": numericOrZero(acceleration.x),
                    "y": numericOrZero(acceleration.y),
                    "z": numericOrZero(acceleration.z),
                }
            }, []);
        }, true);
    };

    let requestPermissionAndStart = (evt?: Event) => {
        evt?.preventDefault();
        evt?.stopPropagation();
        if (sensorsStarted) {
            return;
        }
        if (!needsPermission) {
            startSensors();
            return;
        }
        if (permissionRequested) {
            return;
        }
        permissionRequested = true;
        let requests: Array<Promise<string>> = [];
        if (typeof orientation?.requestPermission === 'function') {
            requests.push(orientation.requestPermission().catch(() => 'denied'));
        }
        if (typeof motion?.requestPermission === 'function') {
            requests.push(motion.requestPermission().catch(() => 'denied'));
        }
        if (requests.length === 0) {
            return;
        }
        Promise.all(requests).then(results => {
            if (results.some(result => result === 'granted')) {
                startSensors();
            } else {
                permissionRequested = false;
            }
        }).catch(() => {
            permissionRequested = false;
        });
    };

    addDisposer(installSensorPermissionRequester(requestPermissionAndStart));
    if (!needsPermission) {
        startSensors();
    }
}

function installSensorPermissionRequester(onRequest: (evt?: Event) => void): () => void {
    let previousRequester = (window as any).paxRequestDeviceSensorPermissions;
    let eventType = 'pax-request-device-sensor-permission';
    let requestFromJs = () => onRequest();
    let requestFromEvent = (evt: Event) => onRequest(evt);
    (window as any).paxRequestDeviceSensorPermissions = requestFromJs;
    window.addEventListener(eventType, requestFromEvent, {"capture": true});

    return () => {
        window.removeEventListener(eventType, requestFromEvent, {"capture": true});
        if ((window as any).paxRequestDeviceSensorPermissions === requestFromJs) {
            if (previousRequester === undefined) {
                delete (window as any).paxRequestDeviceSensorPermissions;
            } else {
                (window as any).paxRequestDeviceSensorPermissions = previousRequester;
            }
        }
    };
}
