/* tslint:disable */
/* eslint-disable */
/**
* @returns {any}
*/
export function wasm_memory(): any;
/**
*/
export class MemorySlice {
  free(): void;
/**
* @returns {number}
*/
  ptr(): number;
/**
* @returns {number}
*/
  len(): number;
}
/**
*/
export class PaxChassisWeb {
  free(): void;
/**
* @returns {PaxChassisWeb}
*/
  static new(): PaxChassisWeb;
/**
* @param {string} id
*/
  add_context(id: string): void;
/**
* @param {number} width
* @param {number} height
*/
  send_viewport_update(width: number, height: number): void;
/**
* @param {string} id
*/
  remove_context(id: string): void;
/**
* @param {string} native_interrupt
* @param {any} additional_payload
*/
  interrupt(native_interrupt: string, additional_payload: any): void;
/**
* @param {MemorySlice} slice
*/
  deallocate(slice: MemorySlice): void;
/**
* @returns {MemorySlice}
*/
  tick(): MemorySlice;
/**
*/
  render(): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly wasm_memory: () => number;
  readonly __wbg_paxchassisweb_free: (a: number) => void;
  readonly paxchassisweb_new: () => number;
  readonly paxchassisweb_add_context: (a: number, b: number, c: number) => void;
  readonly paxchassisweb_send_viewport_update: (a: number, b: number, c: number) => void;
  readonly paxchassisweb_remove_context: (a: number, b: number, c: number) => void;
  readonly paxchassisweb_interrupt: (a: number, b: number, c: number, d: number) => void;
  readonly paxchassisweb_deallocate: (a: number, b: number) => void;
  readonly paxchassisweb_tick: (a: number) => number;
  readonly __wbg_memoryslice_free: (a: number) => void;
  readonly memoryslice_ptr: (a: number) => number;
  readonly memoryslice_len: (a: number) => number;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_exn_store: (a: number) => void;
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