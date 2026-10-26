import init, * as glue from './pax-cartridge.js';
const time=()=>performance.now(),start=time(),timings={},status=document.querySelector('#status'),load=document.querySelector('#load');
const {mode}=await (await fetch('./mode.json')).json();
// Instrument the interface's single ordinary fetch; do not preload it a second time.
const originalFetch=window.fetch,originalInstantiate=WebAssembly.instantiate;
window.fetch=async function(input,options){
 const url=String(input instanceof Request?input.url:input);
 if(!url.endsWith('pax-cartridge_bg.wasm'))return originalFetch.call(this,input,options);
 const begin=time(),response=await originalFetch.call(this,input,options),read=response.arrayBuffer.bind(response);
 response.arrayBuffer=async()=>{const bytes=await read();timings.baseFetchMs=time()-begin;return bytes};return response;
};
WebAssembly.instantiate=async function(bytes,imports){
 let begin=time();const compiled=await WebAssembly.compile(bytes);timings.baseCompileMs=time()-begin;
 begin=time();const instance=await originalInstantiate.call(WebAssembly,compiled,imports);timings.baseInstantiateMs=time()-begin;
 if(new URL(location.href).searchParams.has('eager'))instance.exports.probe_eager(1);
 return {module:compiled,instance};
};
const initStart=time();let app;
try{app=await window.Pax.init_web({element:document.querySelector('#app'),cartridgeUrl:new URL('./pax-cartridge',location.href).href})}finally{window.fetch=originalFetch;WebAssembly.instantiate=originalInstantiate}
const raw=await init();timings.baseInitMs=time()-initStart-timings.baseFetchMs-timings.baseCompileMs-timings.baseInstantiateMs;timings.baseReadyMs=time()-start;timings.baseReadyAt=time();
let module,pending;
async function prefetch(){if(module)return module;if(pending)return pending;pending=(async()=>{
 let t=time();const response=await fetch('./paxflix.wasm');if(!response.ok)throw Error(`HTTP ${response.status}`);const bytes=await response.arrayBuffer();timings.childFetchMs=time()-t;
 t=time();const compiled=await WebAssembly.compile(bytes);timings.childCompileMs=time()-t;
 t=time();module=await WebAssembly.instantiate(compiled,{pax_module:{module_host_bridge:raw.module_host_bridge}});timings.childInstantiateMs=time()-t;
 const e=module.exports,data=new Uint8Array(e.memory.buffer);if(e.module_data(4)!==3)throw Error('ABI mismatch');
 const definition=data.slice(e.module_data(0),e.module_data(0)+e.module_data(1));const schema=new TextDecoder('utf-8',{fatal:true}).decode(data.subarray(e.module_data(2),e.module_data(2)+e.module_data(3)));
 timings.definitionBytes=definition.length;timings.schemaBytes=e.module_data(3);
 t=time();glue.probe_stage(definition,schema);timings.definitionRegistrationMs=time()-t;
 const table=glue.probe_table(),slot=table.grow(1);table.set(slot,e.module_dispatch);if(raw.probe_register(slot)!==1)throw Error('Registration rejected');
 status.textContent='Package loaded; ready to mount';return module;
})();try{return await pending}finally{pending=undefined}}
function activate(){const t=time();raw.probe_activate();timings.activateCallMs=time()-t;timings.activationAt=time();load.disabled=true;document.querySelector('#unmount').disabled=false;status.textContent=mode+' · Paxflix mounted';}
function unmount(){raw.probe_unmount();load.disabled=false;status.textContent='Paxflix unmounted';}
load.disabled=mode==='shell';load.onclick=async()=>{try{if(mode==='split')await prefetch();activate()}catch(e){status.textContent=String(e);console.error(e)}};document.querySelector('#unmount').onclick=unmount;
status.textContent=mode+' · base ready';window.probe={raw,app,prefetch,activate,unmount,timings,mode,get module(){return module},stats:()=>raw.probe_stats?Array.from({length:6},(_,i)=>raw.probe_stats(i)):[]};
