const {chromium}=require('playwright'),fs=require('fs'),assert=require('assert/strict');
const out=process.argv[2],results=[];
(async()=>{const browser=await chromium.launch({headless:true,executablePath:'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'});try{
 for(const variant of ['baseline','parallel','deferred']){
  const page=await browser.newPage({viewport:{width:1280,height:900},ignoreHTTPSErrors:true}),errors=[];
  page.on('pageerror',e=>errors.push(String(e)));
  await page.goto(`https://127.0.0.1:${variant==='baseline'?18756:18757}/?pax_scroll_perf=1&policy=${variant}`);
  await page.waitForFunction(()=>window.probe?.timings.startupSettled!==undefined);
  if(variant!=='baseline')await page.waitForFunction(()=>probe.timings.packageReady!==undefined&&probe.raw.probe_stats(5)>0);
  const text=s=>page.getByText(s,{exact:true}).first();
  await text('Your next great watch').waitFor();
  await text('+ More info').scrollIntoViewIfNeeded();await page.waitForTimeout(700);
  async function click(s){await text(s).scrollIntoViewIfNeeded();const b=await text(s).boundingBox();assert(b,s);await page.mouse.click(b.x+b.width/2,b.y+b.height/2)}
  await click('+ More info');await text('Paxflix / FEATURE').waitFor();
  await page.keyboard.press('Escape');await page.waitForTimeout(250);assert(await text('Paxflix / FEATURE').isVisible(),'embedded keyboard remains disabled');
  await page.screenshot({path:out+'/'+variant+'-website-detail.png'});
  await page.setViewportSize({width:390,height:844});
  await page.waitForFunction(()=>!document.body.textContent.includes('Your next great watch'));
  if(variant!=='baseline')await page.waitForFunction(()=>[0,1,2,5].every(i=>probe.raw.probe_stats(i)===0));
  if(variant!=='baseline')assert.deepEqual(await page.evaluate(()=>[0,1,2,5].map(i=>probe.raw.probe_stats(i))),[0,0,0,0],'mobile breakpoint releases the live module scene');
  await page.setViewportSize({width:1280,height:900});await page.waitForFunction(()=>document.body.textContent.includes('Your next great watch'));
  assert.deepEqual(errors,[]);results.push({variant,passed:true,errors,timings:await page.evaluate(()=>probe.timings)});await page.close();
 }
 const mobile=await browser.newPage({viewport:{width:390,height:844},ignoreHTTPSErrors:true});
 await mobile.goto('https://127.0.0.1:18757/?pax_scroll_perf=1&policy=manual');await mobile.waitForFunction(()=>window.probe?.timings.startupSettled!==undefined);
 assert.equal(await mobile.evaluate(()=>performance.getEntriesByType('resource').some(r=>r.name.endsWith('/paxflix.wasm'))),false);
 assert.equal(await mobile.getByText('Your next great watch',{exact:true}).count(),0);
 await mobile.screenshot({path:out+'/mobile-unloaded.png'});results.push({variant:'mobile-manual',passed:true});await mobile.close();
 fs.writeFileSync(out+'/checks.json',JSON.stringify(results,null,2));console.log(results);
}finally{await browser.close()}})().catch(e=>{console.error(e);process.exitCode=1});
