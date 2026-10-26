// Fresh browser processes prevent cross-trial compiled-module cache reuse.
const {chromium}=require('playwright'),fs=require('fs'),assert=require('assert/strict');
const base=process.argv[2]||'http://127.0.0.1:18753',out=process.argv[3]||'/private/tmp/pax-1034-paxflix';
(async()=>{const rows=[];const profiles=(process.env.PROFILES||'local,network').split(',');for(const profile of profiles){const n=Number(process.env.TRIALS|| (profile==='local'?7:5));for(let i=0;i<n;i++){for(const mode of (i%2?['split','baseline']:['baseline','split'])){
 const browser=await chromium.launch({headless:true,executablePath:process.env.CHROME||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'});try{const page=await browser.newPage({viewport:{width:1280,height:900}}),errors=[];page.on('pageerror',e=>errors.push(String(e)));const cdp=await page.context().newCDPSession(page);await cdp.send('Network.enable');await cdp.send('Network.setCacheDisabled',{cacheDisabled:true});if(profile==='network')await cdp.send('Network.emulateNetworkConditions',{offline:false,latency:80,downloadThroughput:1250000,uploadThroughput:1250000,connectionType:'wifi'});
 await page.goto(base+'/'+mode+'/?pax_scroll_perf=1');await page.waitForFunction(()=>window.probe);await page.waitForTimeout(100);
 const initial=await page.evaluate(()=>({at:performance.now(),resources:performance.getEntriesByType('resource').map(r=>({name:r.name.split('/').pop(),encoded:r.encodedBodySize,decoded:r.decodedBodySize,start:r.startTime,end:r.responseEnd}))}));
 const mounting=await page.evaluate(async()=>{
  const requestedAt=performance.now();if(probe.mode==='split')await probe.prefetch();
  performance.clearMeasures();const activation=performance.now();probe.activate();
  const domReady=await new Promise(resolve=>{function check(){if(document.querySelector('#app').textContent.includes('Your next great watch'))resolve(performance.now());else requestAnimationFrame(check)}check()});
  await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));const afterFrames=performance.now();
  await new Promise(r=>setTimeout(r,1000));
  const measures=performance.getEntriesByType('measure').map(x=>({name:x.name,start:x.startTime,duration:x.duration}));
  return {requestToDomMs:domReady-requestedAt,mountToDomMs:domReady-activation,mountToTwoFramesMs:afterFrames-activation,activation,measures,timings:probe.timings,stats:probe.stats(),hostMemory:probe.raw.memory.buffer.byteLength,childMemory:probe.module?.exports.memory.buffer.byteLength||0,resources:performance.getEntriesByType('resource').map(r=>({name:r.name.split('/').pop(),encoded:r.encodedBodySize,decoded:r.decodedBodySize,start:r.startTime,end:r.responseEnd}))};
 });assert.deepEqual(errors,[]);await page.waitForFunction(()=>performance.getEntriesByType('resource').filter(r=>/\.(jpg|png)$/.test(r.name)).length===37,null,{timeout:60000});const settled=await page.evaluate(()=>({at:performance.now(),resources:performance.getEntriesByType('resource').map(r=>({name:r.name.split('/').pop(),encoded:r.encodedBodySize,decoded:r.decodedBodySize,start:r.startTime,end:r.responseEnd}))}));let row={profile,trial:i,mode,browser:browser.version(),initial,...mounting,settled};rows.push(row);fs.writeFileSync(out+'/bench.json',JSON.stringify(rows,null,2));console.log(profile,i,mode,'mount',mounting.mountToDomMs.toFixed(1),'stage',mounting.timings.definitionRegistrationMs);
 }finally{await browser.close()}
}}} })().catch(e=>{console.error(e);process.exitCode=1});
