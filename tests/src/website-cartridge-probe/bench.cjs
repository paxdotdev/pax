const {chromium}=require('playwright'),fs=require('fs'),path=require('path');
const output=process.argv[2];if(!output||fs.existsSync(output))throw Error('Provide a new output directory; never overwrite trials');fs.mkdirSync(output,{recursive:true});
const variants=(process.env.VARIANTS||'baseline,parallel,sequential,deferred').split(',');
const profiles=(process.env.PROFILES||'local,network').split(',');
const count=Number(process.env.TRIALS||5),rows=[];
async function run(profile,trial,variant){
 const loadBefore=require('os').loadavg();
 const browser=await chromium.launch({headless:true,executablePath:'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'});
 const errors=[],failed=[],requests=[],messages=[],pending=new Set();let lastFinished=Date.now();
 try{
  const page=await browser.newPage({viewport:{width:1280,height:900},ignoreHTTPSErrors:true});
  page.on('pageerror',e=>errors.push(String(e)));
  page.on('console',m=>{if(m.type()==='warning'||m.type()==='error'||/renderer/i.test(m.text()))messages.push({type:m.type(),text:m.text()})});
  page.on('request',r=>pending.add(r));
  page.on('requestfinished',r=>{pending.delete(r);lastFinished=Date.now()});
  page.on('requestfailed',r=>{pending.delete(r);lastFinished=Date.now();failed.push({url:r.url(),error:r.failure()})});
  page.on('response',r=>{if(r.status()>=400)failed.push({url:r.url(),status:r.status()})});
  const cdp=await page.context().newCDPSession(page);await cdp.send('Network.enable');await cdp.send('Network.setCacheDisabled',{cacheDisabled:process.env.CACHE_DISABLED==='1'});
  cdp.on('Network.responseReceived',e=>requests.push({url:e.response.url,protocol:e.response.protocol,status:e.response.status,connectionId:e.response.connectionId,fromDiskCache:e.response.fromDiskCache,fromPrefetchCache:e.response.fromPrefetchCache}));
  if(profile==='network')await cdp.send('Network.emulateNetworkConditions',{offline:false,latency:80,downloadThroughput:1250000,uploadThroughput:1250000,connectionType:'wifi'});
  const port={baseline:18756,pruned:18759,'mount-only':18761}[variant]||18757;
  await page.goto(`https://127.0.0.1:${port}/?pax_scroll_perf=1&policy=${variant}`,{waitUntil:'domcontentloaded'});
  await page.waitForFunction(()=>window.probe?.timings.startupSettled!==undefined,null,{timeout:120000});
  if(!['baseline','pruned','manual','mount-only'].includes(variant))await page.waitForFunction(()=>window.probe?.timings.packageReady!==undefined&&probe.raw.probe_stats(5)>0,null,{timeout:120000});
  if(variant==='mount-only')await page.waitForFunction(()=>probe.timings.activation!==undefined&&document.body.textContent.includes('Your next great watch'));
  await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(()=>{probe.timings.activatedSceneFrame=performance.now();r()}))));
  // A previously reached networkidle lifecycle is not evidence for later loads.
  // Observe every request and require a fresh quiet interval after settlement.
  const deadline=Date.now()+120000;
  while(pending.size||Date.now()-lastFinished<1000){if(Date.now()>deadline)throw Error('resources did not settle: '+[...pending].map(x=>x.url()));await page.waitForTimeout(100)}
  const row=await page.evaluate(()=>({timings:probe.timings,probeErrors:probe.errors,at:performance.now(),navigation:performance.getEntriesByType('navigation')[0].toJSON(),
   resources:performance.getEntriesByType('resource').map(r=>({url:r.name,start:r.startTime,end:r.responseEnd,encoded:r.encodedBodySize,decoded:r.decodedBodySize,transfer:r.transferSize,wireBody:r.transferSize>0?r.encodedBodySize:0,protocol:r.nextHopProtocol})),
   measures:performance.getEntriesByType('measure').filter(e=>e.name.startsWith('pax:startup')).map(e=>({name:e.name,start:e.startTime,duration:e.duration})),
   text:document.querySelector('#app').textContent,canvases:[...document.querySelectorAll('canvas')].map(c=>({width:c.width,height:c.height})),
   hostMemory:probe.raw.memory.buffer.byteLength,childMemory:probe.child?.exports.memory.buffer.byteLength||0,
   stats:probe.child?Array.from({length:6},(_,i)=>probe.raw.probe_stats(i)):[],
   gpuAvailable:!!navigator.gpu,fonts:document.fonts.status
  }));
  Object.assign(row,{profile,trial,variant,browser:browser.version(),cacheDisabled:process.env.CACHE_DISABLED==='1',errors,failed,requests,messages,loadBefore,loadAfter:require('os').loadavg()});
  await page.screenshot({path:path.join(output,`${profile}-${trial}-${variant}.png`)});
  rows.push(row);fs.writeFileSync(path.join(output,'bench.json'),JSON.stringify(rows,null,2));
  console.log(profile,trial,variant,JSON.stringify(row.timings),'errors',errors.length,failed.length);
  if(errors.length||failed.length||row.probeErrors.length)throw Error('browser validation failed');
 }finally{await browser.close()}
}
(async()=>{for(const profile of profiles)for(let trial=0;trial<count;trial++)for(let offset=0;offset<variants.length;offset++)await run(profile,trial,variants[(offset+trial)%variants.length]);})().catch(e=>{console.error(e);process.exitCode=1});
