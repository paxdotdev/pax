/* tslint:disable */
/* eslint-disable */
/**
* @returns {any}
*/
export function wasm_memory(): any;
/**
*/
export class PaxChassisWeb {
  free(): void;
/**
* @param {number} id
*/
  resize_layers_to(id: number): void;
/**
* @param {number} width
* @param {number} height
*/
  send_viewport_update(width: number, height: number): void;
/**
*/
  refresh_render_surfaces(): void;
/**
* @param {Uint32Array} layer_ids
*/
  refresh_render_surfaces_for_layers(layer_ids: Uint32Array): void;
/**
* @returns {[number]}
*/
  get_dirty_canvases(): [number];
/**
* @param {any} native_interrupt
* @param {any} additional_payload
* @returns {InterruptResult}
*/
  interrupt(native_interrupt: any, additional_payload: any): InterruptResult;
/**
*/
  update_userland_component(): void;
/**
*/
  handle_recv_designtime(): void;
/**
*/
  designtime_tick(): void;
/**
* @returns {any}
*/
  tick(): any;
/**
*/
  render(): void;
/**
* @returns {number}
*/
  layer_canvas_plan_generation(): number;
/**
* @param {number} layer
* @returns {any}
*/
  get_layer_canvas_plan(layer: number): any;
/**
* @param {number} layer
* @param {number} request_id
*/
  request_layer_screenshot(layer: number, request_id: number): void;
/**
* @param {number} layer
* @param {number} request_id
* @returns {any}
*/
  take_layer_screenshot(layer: number, request_id: number): any;
/**
* @param {number} layer
* @param {number} request_id
* @returns {any}
*/
  take_layer_surface_screenshots(layer: number, request_id: number): any;
/**
* @returns {any}
*/
  take_prepare_app_revisions(): any;
/**
* @param {string} logic_revision_id
* @returns {boolean}
*/
  request_app_revision_activation(logic_revision_id: string): boolean;
/**
* @param {string} logic_revision_id
* @returns {string}
*/
  poll_app_revision_activation(logic_revision_id: string): string;
/**
* @param {string} logic_revision_id
* @returns {boolean}
*/
  cancel_app_revision_activation(logic_revision_id: string): boolean;
/**
* @param {string} logic_revision_id
* @returns {boolean}
*/
  activate_app_revision(logic_revision_id: string): boolean;
/**
* @param {string} path
* @returns {boolean}
*/
  image_loaded(path: string): boolean;
}


export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InterruptResult {
  readonly prevent_default: boolean;  
}

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly pax_init: () => number;
  readonly wasm_memory: () => number;
  readonly __wbg_paxchassisweb_free: (a: number, b: number) => void;
  readonly __wbg_interruptresult_free: (a: number, b: number) => void;
  readonly __wbg_get_interruptresult_prevent_default: (a: number) => number;
  readonly __wbg_set_interruptresult_prevent_default: (a: number, b: number) => void;
  readonly paxchassisweb_resize_layers_to: (a: number, b: number) => void;
  readonly paxchassisweb_send_viewport_update: (a: number, b: number, c: number) => void;
  readonly paxchassisweb_refresh_render_surfaces: (a: number) => void;
  readonly paxchassisweb_refresh_render_surfaces_for_layers: (a: number, b: number, c: number) => void;
  readonly paxchassisweb_get_dirty_canvases: (a: number, b: number) => void;
  readonly paxchassisweb_interrupt: (a: number, b: number, c: number) => number;
  readonly paxchassisweb_update_userland_component: (a: number) => void;
  readonly paxchassisweb_handle_recv_designtime: (a: number) => void;
  readonly paxchassisweb_designtime_tick: (a: number) => void;
  readonly paxchassisweb_tick: (a: number) => number;
  readonly paxchassisweb_render: (a: number) => void;
  readonly paxchassisweb_layer_canvas_plan_generation: (a: number) => number;
  readonly paxchassisweb_get_layer_canvas_plan: (a: number, b: number) => number;
  readonly paxchassisweb_request_layer_screenshot: (a: number, b: number, c: number) => void;
  readonly paxchassisweb_take_layer_screenshot: (a: number, b: number, c: number) => number;
  readonly paxchassisweb_take_layer_surface_screenshots: (a: number, b: number, c: number) => number;
  readonly paxchassisweb_take_prepare_app_revisions: (a: number) => any;
  readonly paxchassisweb_request_app_revision_activation: (a: number, b: number, c: number) => number;
  readonly paxchassisweb_poll_app_revision_activation: (a: number, b: number, c: number, d: number) => void;
  readonly paxchassisweb_cancel_app_revision_activation: (a: number, b: number, c: number) => number;
  readonly paxchassisweb_activate_app_revision: (a: number, b: number, c: number) => number;
  readonly paxchassisweb_image_loaded: (a: number, b: number, c: number) => number;
  readonly slugify: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly _dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__he1f207449096521a: (a: number, b: number, c: number) => void;
  readonly _dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h3b0323705541ab89: (a: number, b: number, c: number) => void;
  readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly wasm_bindgen__convert__closures__invoke2_mut__h66b98066a75f2829: (a: number, b: number, c: number, d: number) => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {SyncInitInput} module
*
* @returns {InitOutput}
*/
export function initSync(module: SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {InitInput | Promise<InitInput>} module_or_path
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: InitInput | Promise<InitInput>): Promise<InitOutput>;
