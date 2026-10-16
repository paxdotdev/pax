/* tslint:disable */
/* eslint-disable */
/**
* @returns {any}
*/
export function wasm_memory(): any;
/**
*/
export class PaxChassisWeb {
  request(bytes: Uint8Array): Promise<Uint8Array>;
  free(): void;
  set_property_waker(callback: () => void): void;
  has_pending_properties(): boolean;
  activate_application(): void;
  shutdown_for_replacement(): void;
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
  startup_input_pending(): boolean;
  replay_startup_input(): void;
  startup_pending(): boolean;
  prepare_startup_presentation(): boolean;
  finish_startup_presentation(): void;
  service_startup(budget_ms: number): [any[], {pending: boolean; units: number; mounted: number; effects: number; elapsed_ms: number; longest_unit_ms: number; longest_unit: string}];
  tick(): any;
  tick_profiled(): [any, [string, number][], {passes: number; rebuilds: number; structural_reconciliations: number; records_retained: number; orders_updated: number; records_updated: number; masks_evaluated: number; mask_candidates: number; rebuild_reason: string | null; rebuild_node: number | null; rebuild_origins: ({file: string; line: number; node: number | null; count: number} | null)[]}];
  set_surface_ready_callback(callback: (() => void) | undefined): void;
  present_ready_surfaces(): {submitted: number; deferred: number; cache_misses: number; blocked: boolean; blocker: string | null; elapsed_ms: number};
/**
*/
  render(): void;
  begin_render_work(epoch: number): void;
  render_work_is_settled(): boolean;
  service_render_work(slice_ms: number): import('../utils/cooperative-render').RenderWorkResult & {
    blocker: string | null; units: number; ordinary_layers: number; atomic_fallbacks: number;
    elapsed_ms: number; max_unit_ms: number; preparation: object;
  };
  render_profiled(): [[string, number][], [{initializations: number; submitted: number; deferred: number; dirty_layers: number; cache_misses: number; blocked: boolean; blocker: string | null; elapsed_ms: number}, {initializations: number; submitted: number; deferred: number; dirty_layers: number; cache_misses: number; blocked: boolean; blocker: string | null; elapsed_ms: number}]];
/**
* @returns {number}
*/
  layer_canvas_plan_generation(): number;
  surface_policy_revision(): number;
  install_surface_plan(snapshot: import('../classes/surface-coordinator').InstalledSurfacePlan): void;
  begin_surface_plan(): void;
/**
* @param {number} layer
* @returns {any}
*/
  get_layer_canvas_plan(layer: number): any;
  get_layer_canvas_plan_for_transform(layer: number, a: number, b: number, c: number, d: number, now: number): any;
  schedule_surfaces(snapshot: any): any;
  prepare_surface_hosts(snapshot: any): {id: number; visible: number[]; required: number[]; prepared: number[]; horizonMs: number; unavailableReason?: string}[];
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
  image_load_complete(path: string, width: number, height: number, rgba: Uint8Array | Uint8ClampedArray, ids: Uint32Array): void;
  image_ready(ids: Uint32Array): void;
}


export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InterruptResult {
  readonly prevent_default: boolean;  
}

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly init_web: (host: number) => number;
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
