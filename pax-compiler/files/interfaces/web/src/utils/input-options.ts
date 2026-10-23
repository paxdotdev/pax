import { readWebConfig } from './startup-options';

export type WheelEvents = 'passive' | 'cancellable';

/** Resolve before attaching listeners: cancellation cannot be chosen after dispatch. */
export function resolveWheelEvents(config: unknown, override?: WheelEvents): WheelEvents {
    const configured = typeof config === 'object' && config !== null
        ? (config as {wheel_events?: unknown}).wheel_events : undefined;
    const mode = override ?? configured ?? 'passive';
    if (mode !== 'passive' && mode !== 'cancellable') {
        throw new Error('wheelEvents / wheel_events must be "passive" or "cancellable"');
    }
    return mode;
}

export function readWheelEvents(override?: WheelEvents): WheelEvents {
    return resolveWheelEvents(readWebConfig(), override);
}
