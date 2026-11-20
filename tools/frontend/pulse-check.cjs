// Run only against browser_fixture. All credentials and traffic are synthetic.
const {chromium} = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
(async () => {
  const browser = await chromium.launch({executablePath:process.env.CHROMIUM_PATH,headless:true,args:['--no-sandbox']});
  const page = await browser.newPage({viewport:{width:1440,height:1050},colorScheme:'dark'});
  const errors=[];
  page.on('pageerror',e=>errors.push(e.message));
  await page.addInitScript(() => {
    const Native = window.EventSource;
    window.testStreams=[];
    window.EventSource=class extends Native {constructor(...args){super(...args);window.testStreams.push(this);}};
  });
  await page.goto('http://127.0.0.1:19581');
  await page.getByLabel('Username',{exact:true}).fill('browser-test');
  await page.getByLabel('Password',{exact:true}).fill(process.env.BROWSER_TEST_PASSWORD);
  await page.getByRole('button',{name:'Sign in',exact:true}).click();
  await page.locator('.chart-frame.rainbow').waitFor();
  await page.getByLabel('Color theme').selectOption('dark');
  assert.match(await page.locator('.capacity-note').innerText(),/200.0 Mbps.*80.0 Mbps/);
  const geometry=await page.locator('linearGradient').evaluateAll(nodes=>nodes.map(n=>({units:n.getAttribute('gradientUnits'),y:n.getAttribute('y1'),stops:n.querySelectorAll(':scope > stop').length,red:getComputedStyle(n.querySelector('stop')).stopColor})));
  assert.deepEqual(geometry.map(g=>g.y),['0','240']);
  assert.ok(geometry.every(g=>g.units==='userSpaceOnUse'&&g.stops===7&&g.red==='rgb(239, 68, 68)'));
  let path=await page.locator('.actual.download-line').getAttribute('d');
  const started=Date.now();
  for(let i=0;i<3;i++){
    await page.waitForFunction(path=>document.querySelector('.actual.download-line')?.getAttribute('d')!==path,path,{timeout:3500});
    path=await page.locator('.actual.download-line').getAttribute('d');
  }
  assert.ok(Date.now()-started<5000,'three live updates must arrive without five-second polling');
  const out=process.env.PULSE_EVIDENCE_DIR || '/tmp/lqosync-pulse-evidence';
  fs.mkdirSync(out,{recursive:true});
  await page.screenshot({path:`${out}/dark.png`,fullPage:true});
  await page.getByLabel('Color theme').selectOption('light');
  await page.screenshot({path:`${out}/light.png`,fullPage:true});
  for(const width of [768,320]){
    await page.setViewportSize({width,height:1000});
    assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),`overflow at ${width}`);
    if(width===320)await page.screenshot({path:`${out}/mobile.png`,fullPage:true});
  }
  await page.setViewportSize({width:1440,height:1050});
  const count=await page.evaluate(()=>window.testStreams.length);
  await page.evaluate(()=>{
    Object.defineProperty(document,'hidden',{configurable:true,get:()=>true});
    document.dispatchEvent(new Event('visibilitychange'));
  });
  assert.equal(await page.evaluate(()=>window.testStreams.at(-1).readyState),2);
  await page.waitForTimeout(2200);
  assert.equal(await page.evaluate(()=>window.testStreams.length),count);
  await page.evaluate(()=>{
    Object.defineProperty(document,'hidden',{configurable:true,get:()=>false});
    document.dispatchEvent(new Event('visibilitychange'));
  });
  await page.waitForFunction(count=>window.testStreams.length>count,count);
  await page.waitForFunction(()=>window.testStreams.at(-1).readyState===1);
  await page.evaluate(async()=>{
    const view=await (await fetch('/api/v1/overview')).json();
    window.testOverview=view;
    const source=window.testStreams.at(-1);source.close();
    source.dispatchEvent(new MessageEvent('message',{data:JSON.stringify({...view,wan_capacity:null})}));
  });
  await page.getByText('WAN capacity unavailable.',{exact:false}).waitFor();
  assert.equal(await page.locator('.chart-frame.rainbow').count(),0);
  await page.evaluate(()=>{
    const view=window.testOverview;
    view.traffic.at(-1).down_mbps=220;
    window.testStreams.at(-1).dispatchEvent(new MessageEvent('message',{data:JSON.stringify(view)}));
  });
  await page.waitForFunction(()=>document.querySelector('.capacity-note')?.textContent.includes('110.0%'));
  assert.ok(Number(await page.locator('linearGradient').first().getAttribute('y1'))>0,'red remains at capacity when traffic exceeds it');
  await page.emulateMedia({forcedColors:'active'});
  assert.equal(await page.locator('.traffic-area').first().evaluate(n=>getComputedStyle(n).display),'none');
  await page.emulateMedia({forcedColors:'none',reducedMotion:'reduce'});
  await page.getByRole('button',{name:'Sign out',exact:true}).click();
  await page.getByRole('heading',{name:'Sign in',exact:true}).waitFor();
  assert.ok(await page.evaluate(()=>window.testStreams.every(s=>s.readyState===2)));
  assert.deepEqual(errors,[]);
  await browser.close();
  console.log('PASS: one-second live updates, direction capacity gradients, overflow, missing capacity, hidden-page cleanup, reconnect, themes, 768/320px, forced colors, logout cleanup.');
})().catch(error=>{console.error(error);process.exit(1);});
