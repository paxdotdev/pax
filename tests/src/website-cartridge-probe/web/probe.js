import init, * as glue from './pax-cartridge.js';
const now=()=>performance.now(), timings={}, errors=[];
const {mode}=await (await fetch('/mode.json')).json();
const policy=new URL(location.href).searchParams.get('policy')||'parallel';
let raw, child, pending, app, deferredStarted=false;
window.probe={mode,policy,timings,errors,get raw(){return raw},get app(){return app},get child(){return child}};
window.addEventListener('error',e=>errors.push(e.message));
window.addEventListener('unhandledrejection',e=>errors.push(String(e.reason)));
const originalFetch=window.fetch, originalInstantiate=WebAssembly.instantiate;
window.fetch=async function(input,options){
 const url=String(input instanceof Request?input.url:input);
 if(!url.endsWith('pax-cartridge_bg.wasm'))return originalFetch.call(this,input,options);
 timings.baseFetchStart=now();const response=await originalFetch.call(this,input,options),read=response.arrayBuffer.bind(response);
 response.arrayBuffer=async()=>{const bytes=await read();timings.baseFetchEnd=now();return bytes};return response;
};
WebAssembly.instantiate=async function(bytes,imports){
 // Only the interface's byte-based base instantiation uses this override.
 if(bytes instanceof WebAssembly.Module)return originalInstantiate.call(WebAssembly,bytes,imports);
 let t=now();const compiled=await WebAssembly.compile(bytes);timings.baseCompileMs=now()-t;
 t=now();const instance=await originalInstantiate.call(WebAssembly,compiled,imports);timings.baseInstantiateMs=now()-t;
 return {module:compiled,instance};
};
async function fetchChild(){
 if(pending)return pending;
 pending=(async()=>{
  timings.childFetchStart=now();const response=await originalFetch('/paxflix.wasm');if(!response.ok)throw Error('child HTTP '+response.status);
  const bytes=await response.arrayBuffer();timings.childFetchEnd=now();
  const t=now(), compiled=await WebAssembly.compile(bytes);timings.childCompileMs=now()-t;return compiled;
 })();return pending;
}
async function activate(){
 if(mode==='mount-only'){timings.activation=now();raw.probe_activate();return}
 const compiled=await fetchChild();let t=now();
 child=await originalInstantiate.call(WebAssembly,compiled,{pax_module:{module_host_bridge:raw.module_host_bridge}});
 timings.childInstantiateMs=now()-t;
 const e=child.exports,data=new Uint8Array(e.memory.buffer);if(e.module_data(4)!==3)throw Error('ABI mismatch');
 const definition=data.slice(e.module_data(0),e.module_data(0)+e.module_data(1));
 const schema=new TextDecoder('utf-8',{fatal:true}).decode(data.subarray(e.module_data(2),e.module_data(2)+e.module_data(3)));
 t=now();glue.probe_stage(definition,schema);timings.registrationMs=now()-t;
 const table=glue.probe_table(),slot=table.grow(1);table.set(slot,e.module_dispatch);
 if(raw.probe_register(slot)!==1)throw Error('registration failed');
 timings.packageReady=now();raw.probe_activate();timings.activation=now();
}
window.probe.activate=activate;
function afterFirstCanvas(){
 if(((mode==='split'&&policy==='deferred')||mode==='mount-only')&&!deferredStarted){
  deferredStarted=true;
  // Leave one paint opportunity for hero artwork before the package can mount.
  requestAnimationFrame(()=>requestAnimationFrame(()=>activate().catch(e=>errors.push(String(e)))));
 }
}
new PerformanceObserver(list=>{
 for(const e of list.getEntries()){
  if(e.name==='pax:startupPresentation'&&timings.firstViewportPresentation===undefined){
   timings.firstViewportPresentation=e.startTime+e.duration;
   requestAnimationFrame(()=>{timings.firstViewportNextFrame=now()});
  }
 }
}).observe({type:'measure',buffered:true});
window.probeBeforeMount=async exports=>{
 raw=exports;timings.baseInitialized=now();
 // Measure the existing renderer's first actual nonempty submission separately
 // from native text's coordinated viewport reveal.
 const proto=glue.PaxChassisWeb.prototype,original=proto.present_ready_surfaces;
 proto.present_ready_surfaces=function(...args){const r=original.apply(this,args);if(r.submitted>0&&timings.firstCanvasSubmission===undefined){timings.firstCanvasSubmission=now();afterFirstCanvas()}return r};
 if(mode==='split'&&policy!=='deferred'&&policy!=='manual')await activate();
 timings.mountStart=now();
};
if(mode==='split'&&policy==='parallel')fetchChild();
try{
 app=await window.Pax.init_web({element:document.querySelector('#app'),cartridgeUrl:new URL('/pax-cartridge',location.href).href,yieldMount:true});
 timings.startupSettled=now();
}catch(e){errors.push(String(e));throw e}
finally{window.fetch=originalFetch;WebAssembly.instantiate=originalInstantiate}
raw=await init();
